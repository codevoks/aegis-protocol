//! `withdraw_collateral` — the full instruction (`instruction-catalogue.md` §11,
//! `docs/phases/phase-05-oracle.md`).
//!
//! A debt-free position withdraws with **no oracle read at all** (E-08, INV-ORA-02) — this is the
//! unchanged Phase 3 path. A position with `borrow_shares > 0` now validates the oracle
//! (`oracle::require_valid_price`, O-1..O-11) and requires the **post-withdrawal** state to
//! satisfy `debt_value <= collateral_value * max_ltv / WAD` (INV-SOLV-01) before any mutation —
//! never the pre-withdrawal collateral amount.
//!
//! `Market` is still never written here (claim C2, `account-model.md` §8, `A-PAR-01`): the debt
//! figure for the health check uses `Market::accrue_view` (pure, `economic-model.md` §4.5), never
//! `accrue_mut`.

use crate::constants::{COLLATERAL_VAULT_SEED, MARKET_SEED};
use crate::error::AegisError;
use crate::events::CollateralWithdrawn;
use crate::oracle;
use crate::state::{Market, Position};
use crate::token::transfer::transfer_checked_out;
use aegis_math::{collateral_value, debt_value, is_within_max_ltv, to_assets_up};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(Accounts)]
pub struct WithdrawCollateral<'info> {
    /// **Required signer** (`instruction-catalogue.md` §11, INV-AUTH-02) — the asymmetric
    /// counterpart to `deposit_collateral`'s no-signer-required depositor (INV-AUTH-03): this
    /// operation can only reduce the position's safety, so the owner must authorize it.
    pub owner: Signer<'info>,

    #[account(
        seeds = [
            MARKET_SEED,
            market.collateral_mint.as_ref(),
            market.loan_mint.as_ref(),
            &market.config_id.to_le_bytes(),
        ],
        bump = market.bump,
    )]
    pub market: Box<Account<'info, Market>>,

    #[account(
        mut,
        has_one = market @ AegisError::PositionMarketMismatch,
        has_one = owner @ AegisError::NotPositionOwner,
    )]
    pub position: Account<'info, Position>,

    #[account(
        mut,
        seeds = [COLLATERAL_VAULT_SEED, market.key().as_ref()],
        bump = market.collateral_vault_bump,
        address = market.collateral_vault @ AegisError::VaultMismatch,
    )]
    pub collateral_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = owner_collateral_ata.mint == market.collateral_mint @ AegisError::VaultMintMismatch,
    )]
    pub owner_collateral_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(address = market.collateral_mint @ AegisError::VaultMintMismatch)]
    pub collateral_mint: InterfaceAccount<'info, Mint>,

    pub collateral_token_program: Interface<'info, TokenInterface>,

    /// CHECK: validated field-by-field by `oracle::require_valid_price` (O-1..O-11) only when
    /// `position.borrow_shares > 0`. A debt-free withdrawal never reads either price account
    /// (E-08) — a caller may pass any account here in that case.
    pub collateral_price_update: UncheckedAccount<'info>,
    /// CHECK: as `collateral_price_update`, for the loan asset's feed.
    pub loan_price_update: UncheckedAccount<'info>,
}

pub fn handler(ctx: Context<WithdrawCollateral>, amount: u64) -> Result<()> {
    require!(amount > 0, AegisError::ZeroAmount);

    require_keys_eq!(
        ctx.accounts.collateral_token_program.key(),
        ctx.accounts.market.collateral_token_program,
        AegisError::TokenProgramMismatch
    );

    require!(
        amount <= ctx.accounts.position.collateral_amount,
        AegisError::InsufficientCollateral
    );

    // INV-ORA-07 / E-08: only a debt-bearing position needs the oracle at all. Validate BEFORE
    // any state write, and evaluate health against the **post-withdrawal** collateral amount
    // (task requirement: "the proposed resulting state must be safe," never the pre-withdrawal
    // one).
    if ctx.accounts.position.borrow_shares > 0 {
        let now = Clock::get()?.unix_timestamp;
        let (collateral_band, loan_band) = oracle::require_valid_price(
            &ctx.accounts.market,
            &ctx.accounts.collateral_price_update.to_account_info(),
            &ctx.accounts.loan_price_update.to_account_info(),
            now,
        )?;

        // Market is never written by this instruction (C2) -- accrue_view is the pure,
        // non-mutating read economic-model.md §4.5 specifies for exactly this purpose.
        let accrued = ctx.accounts.market.accrue_view(now)?;
        let debt_assets = to_assets_up(
            ctx.accounts.position.borrow_shares,
            accrued.total_borrow_assets,
            ctx.accounts.market.total_borrow_shares,
        )
        .map_err(AegisError::from)?;

        let post_withdraw_collateral = ctx
            .accounts
            .position
            .collateral_amount
            .checked_sub(amount)
            .ok_or(AegisError::ArithmeticOverflow)?;

        let cv = collateral_value(
            post_withdraw_collateral,
            collateral_band.lo,
            ctx.accounts.market.collateral_decimals,
        )
        .map_err(AegisError::from)?;
        let dv = debt_value(debt_assets, loan_band.hi, ctx.accounts.market.loan_decimals)
            .map_err(AegisError::from)?;
        let within_ltv =
            is_within_max_ltv(cv, dv, ctx.accounts.market.max_ltv).map_err(AegisError::from)?;
        require!(within_ltv, AegisError::ExceedsMaxLtv);
    }

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

    // Read state -> write state -> move tokens (architecture.md §2): unlike the inbound path,
    // the outbound amount is exactly what Aegis debits — the recipient bears any transfer fee
    // (account-model.md §6.4) — so there is no post-CPI reload to sequence around.
    ctx.accounts.position.collateral_amount = ctx
        .accounts
        .position
        .collateral_amount
        .checked_sub(amount)
        .ok_or(AegisError::ArithmeticOverflow)?;

    transfer_checked_out(
        &ctx.accounts.collateral_vault.to_account_info(),
        &ctx.accounts.collateral_mint.to_account_info(),
        &ctx.accounts.owner_collateral_ata.to_account_info(),
        &market.to_account_info(),
        &ctx.accounts.collateral_token_program.to_account_info(),
        amount,
        ctx.accounts.collateral_mint.decimals,
        signer_seeds,
    )?;

    emit!(CollateralWithdrawn {
        market: market_key,
        position: ctx.accounts.position.key(),
        owner: ctx.accounts.owner.key(),
        amount,
    });

    Ok(())
}
