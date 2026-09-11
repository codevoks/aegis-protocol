//! `liquidate` — the most dangerous instruction in the protocol (`instruction-catalogue.md` §17,
//! `docs/phases/phase-06-liquidation.md`, `economic-model.md` §7).
//!
//! **Ordering is security-critical (INV-ORA-07), exactly mirroring `borrow`'s precedent:**
//! oracle validation happens first, strictly before `accrue_mut` or any other state write, so a
//! failed oracle check leaves nothing modified. `HF < WAD` is checked **strictly** (E-12,
//! INV-LIQ-01/INV-SOLV-02) — `HF == WAD` is never liquidatable, and nothing in this file uses
//! `<=` against a health factor.
//!
//! No owner signature is required (`account-model.md` §5.1, INV-AUTH-03): liquidation is
//! permissionless by design, including self-liquidation by the position's own owner (`U-LIQ-07`,
//! `threat-model.md` T-22) — not blocked with a special case, since doing so would also block
//! legitimate third-party liquidator bots that happen to share a signer with the position owner
//! in a test fixture, and self-liquidation is provably unprofitable rather than dangerous.

use crate::constants::{COLLATERAL_VAULT_SEED, LOAN_VAULT_SEED, MARKET_SEED, POSITION_SEED};
use crate::error::AegisError;
use crate::events::Liquidated;
use crate::guards::require_exactly_one_u64;
use crate::oracle;
use crate::state::{Market, Position};
use crate::token::transfer::{transfer_checked_in, transfer_checked_out};
use aegis_math::{
    collateral_value, compute_liquidation_by_repay, compute_liquidation_by_seize, debt_value,
    health_factor, is_liquidatable, to_assets_up, to_shares_down, LiquidationParams,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(Accounts)]
pub struct Liquidate<'info> {
    /// Anyone -- permissionless, including the position's own owner (self-liquidation, U-LIQ-07).
    #[account(mut)]
    pub liquidator: Signer<'info>,

    #[account(
        mut,
        seeds = [
            MARKET_SEED,
            market.collateral_mint.as_ref(),
            market.loan_mint.as_ref(),
            &market.config_id.to_le_bytes(),
        ],
        bump = market.bump,
    )]
    pub market: Box<Account<'info, Market>>,

    /// No `has_one = owner` -- the liquidator need not be, and is never required to be, the
    /// position owner.
    #[account(
        mut,
        has_one = market @ AegisError::PositionMarketMismatch,
    )]
    pub position: Account<'info, Position>,

    #[account(
        mut,
        seeds = [POSITION_SEED, market.key().as_ref(), market.fee_recipient.as_ref()],
        bump = fee_position.bump,
        has_one = market @ AegisError::PositionMarketMismatch,
    )]
    pub fee_position: Account<'info, Position>,

    #[account(
        mut,
        seeds = [LOAN_VAULT_SEED, market.key().as_ref()],
        bump = market.loan_vault_bump,
        address = market.loan_vault @ AegisError::VaultMismatch,
    )]
    pub loan_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [COLLATERAL_VAULT_SEED, market.key().as_ref()],
        bump = market.collateral_vault_bump,
        address = market.collateral_vault @ AegisError::VaultMismatch,
    )]
    pub collateral_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = liquidator_loan_ata.mint == market.loan_mint @ AegisError::VaultMintMismatch,
    )]
    pub liquidator_loan_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Not constrained to any particular owner (`instruction-catalogue.md` §17: "a liquidator may
    /// direct proceeds anywhere" -- the amount is fully determined by protocol state, not by who
    /// controls this account).
    #[account(
        mut,
        constraint = liquidator_collateral_ata.mint == market.collateral_mint @ AegisError::VaultMintMismatch,
    )]
    pub liquidator_collateral_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(address = market.loan_mint @ AegisError::VaultMintMismatch)]
    pub loan_mint: InterfaceAccount<'info, Mint>,
    #[account(address = market.collateral_mint @ AegisError::VaultMintMismatch)]
    pub collateral_mint: InterfaceAccount<'info, Mint>,

    pub loan_token_program: Interface<'info, TokenInterface>,
    pub collateral_token_program: Interface<'info, TokenInterface>,

    /// CHECK: validated field-by-field by `oracle::require_valid_price` (O-1..O-11).
    pub collateral_price_update: UncheckedAccount<'info>,
    /// CHECK: as `collateral_price_update`, for the loan asset's feed.
    pub loan_price_update: UncheckedAccount<'info>,
}

pub fn handler(ctx: Context<Liquidate>, repay_assets: u64, seize_collateral: u64) -> Result<()> {
    require_exactly_one_u64(repay_assets, seize_collateral)?;
    require_keys_eq!(
        ctx.accounts.loan_token_program.key(),
        ctx.accounts.market.loan_token_program,
        AegisError::TokenProgramMismatch
    );
    require_keys_eq!(
        ctx.accounts.collateral_token_program.key(),
        ctx.accounts.market.collateral_token_program,
        AegisError::TokenProgramMismatch
    );

    // INV-ORA-07: validate the oracle for BOTH assets before any state write -- the first
    // fallible operation that reads caller-supplied external data.
    let now = Clock::get()?.unix_timestamp;
    let (collateral_band, loan_band) = oracle::require_valid_price(
        &ctx.accounts.market,
        &ctx.accounts.collateral_price_update.to_account_info(),
        &ctx.accounts.loan_price_update.to_account_info(),
        now,
    )?;

    require!(
        ctx.accounts.position.collateral_amount > 0,
        AegisError::NothingToLiquidate
    );

    // Accrue under the already-validated price, exactly as `borrow` does -- this is the first
    // state write, and everything below uses the now-fully-accrued totals.
    ctx.accounts
        .market
        .accrue_mut(&mut ctx.accounts.fee_position, now)?;

    let market = &ctx.accounts.market;
    let position = &ctx.accounts.position;

    let debt_assets = to_assets_up(
        position.borrow_shares,
        market.total_borrow_assets,
        market.total_borrow_shares,
    )
    .map_err(AegisError::from)?;

    let cv = collateral_value(
        position.collateral_amount,
        collateral_band.lo,
        market.collateral_decimals,
    )
    .map_err(AegisError::from)?;
    let dv =
        debt_value(debt_assets, loan_band.hi, market.loan_decimals).map_err(AegisError::from)?;
    let hf_before = health_factor(cv, market.liq_threshold, dv).map_err(AegisError::from)?;

    // INV-LIQ-01 / INV-SOLV-02 / E-12: STRICT. HF == WAD is not liquidatable.
    require!(is_liquidatable(hf_before), AegisError::NotLiquidatable);

    let params = LiquidationParams {
        collateral_decimals: market.collateral_decimals,
        loan_decimals: market.loan_decimals,
        price_c_lo: collateral_band.lo,
        price_l_hi: loan_band.hi,
        liq_bonus: market.liq_bonus,
        liq_protocol_fee: market.liq_protocol_fee,
        close_factor: market.close_factor,
        full_liq_hf: market.full_liq_hf,
        min_debt: market.min_debt,
    };

    let outcome = if repay_assets > 0 {
        compute_liquidation_by_repay(
            &params,
            debt_assets,
            position.collateral_amount,
            hf_before,
            repay_assets,
        )
    } else {
        compute_liquidation_by_seize(
            &params,
            debt_assets,
            position.collateral_amount,
            hf_before,
            seize_collateral,
        )
    }
    .map_err(AegisError::from)?;

    // economic-model.md §7.3: repay_shares from repay_assets, floored, then clamped to what the
    // position actually owes.
    let mut repay_shares = to_shares_down(
        outcome.repay_assets,
        market.total_borrow_assets,
        market.total_borrow_shares,
    )
    .map_err(AegisError::from)?;
    repay_shares = repay_shares.min(position.borrow_shares);

    // --- receive the liquidator's repayment (measured delta; loan assets are fee-free by
    // policy, so credited == outcome.repay_assets is expected but verified, never assumed). ---
    let credited = transfer_checked_in(
        &ctx.accounts.liquidator_loan_ata.to_account_info(),
        &ctx.accounts.loan_mint.to_account_info(),
        &mut ctx.accounts.loan_vault,
        &ctx.accounts.liquidator.to_account_info(),
        &ctx.accounts.loan_token_program.to_account_info(),
        outcome.repay_assets,
        ctx.accounts.loan_mint.decimals,
    )?;
    require_eq!(
        credited,
        outcome.repay_assets,
        AegisError::VaultAccountingError
    );

    // --- update borrow shares/debt and collateral (economic-model.md §7.3) ---
    let market = &mut ctx.accounts.market;
    market.total_borrow_shares = market
        .total_borrow_shares
        .checked_sub(repay_shares)
        .ok_or(AegisError::ArithmeticOverflow)?;
    market.total_borrow_assets = market
        .total_borrow_assets
        .checked_sub(outcome.repay_assets)
        .ok_or(AegisError::ArithmeticOverflow)?;
    // total_supply_assets is deliberately unchanged: lenders are repaid, never enriched, by a
    // liquidation (INV-LIQ-09, U-LIQ-06).
    market.collateral_fee_accrued = market
        .collateral_fee_accrued
        .checked_add(outcome.protocol_cut)
        .ok_or(AegisError::ArithmeticOverflow)?;

    let position = &mut ctx.accounts.position;
    position.borrow_shares = position
        .borrow_shares
        .checked_sub(repay_shares)
        .ok_or(AegisError::ArithmeticOverflow)?;
    position.collateral_amount = position
        .collateral_amount
        .checked_sub(outcome.total_seize)
        .ok_or(AegisError::ArithmeticOverflow)?;

    // hf_after, for the event and as on-chain evidence for P-LIQ-1: recomputed from the
    // now-mutated position/market totals against the SAME already-validated price bands.
    let debt_after = to_assets_up(
        position.borrow_shares,
        market.total_borrow_assets,
        market.total_borrow_shares,
    )
    .map_err(AegisError::from)?;
    let cv_after = collateral_value(
        position.collateral_amount,
        collateral_band.lo,
        market.collateral_decimals,
    )
    .map_err(AegisError::from)?;
    let dv_after =
        debt_value(debt_after, loan_band.hi, market.loan_decimals).map_err(AegisError::from)?;
    let hf_after =
        health_factor(cv_after, market.liq_threshold, dv_after).map_err(AegisError::from)?;

    // --- transfer net seized collateral; protocol_cut stays physically in the vault ---
    let market = &ctx.accounts.market;
    let market_key = market.key();
    let config_id_bytes = market.config_id.to_le_bytes();
    let signer_seeds: &[&[u8]] = &[
        MARKET_SEED,
        market.collateral_mint.as_ref(),
        market.loan_mint.as_ref(),
        &config_id_bytes,
        &[market.bump],
    ];

    transfer_checked_out(
        &ctx.accounts.collateral_vault.to_account_info(),
        &ctx.accounts.collateral_mint.to_account_info(),
        &ctx.accounts.liquidator_collateral_ata.to_account_info(),
        &market.to_account_info(),
        &ctx.accounts.collateral_token_program.to_account_info(),
        outcome.to_liquidator,
        ctx.accounts.collateral_mint.decimals,
        signer_seeds,
    )?;

    emit!(Liquidated {
        market: market_key,
        position: ctx.accounts.position.key(),
        liquidator: ctx.accounts.liquidator.key(),
        repay_assets: outcome.repay_assets,
        repay_shares,
        base_seize: outcome.base_seize,
        total_seize: outcome.total_seize,
        bonus_amount: outcome.bonus_amount,
        protocol_cut: outcome.protocol_cut,
        to_liquidator: outcome.to_liquidator,
        clamped: outcome.clamped,
        hf_before,
        hf_after,
    });

    Ok(())
}
