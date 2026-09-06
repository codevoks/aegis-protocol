//! `withdraw_collateral` — the Phase 3 **zero-debt path only** (`docs/phases/phase-03-collateral.md`,
//! `docs/phase-roadmap.md` "Sequencing the oracle dependency").
//!
//! The full, final instruction (`instruction-catalogue.md` §11) prices the position and enforces
//! `debt_value <= collateral_value * max_ltv / WAD` whenever `position.borrow_shares > 0`. That
//! branch does not exist yet — there is no oracle before Phase 5 and no borrowing before Phase 4.
//! Rather than ship a permissive placeholder (rejected outright by the phase roadmap), a position
//! with any outstanding debt is refused with `OracleNotYetAvailable`: a hard failure, strictly
//! *more* restrictive than the eventual behavior, never less. A debt-free position can always
//! withdraw its own collateral, oracle or no oracle (INV-ORA-02, E-08).
//!
//! `Market` is never written here, for the same parallelism reason as `deposit_collateral`
//! (claim C2, `account-model.md` §8) — enforced by `A-PAR-01`.

use crate::constants::{COLLATERAL_VAULT_SEED, MARKET_SEED};
use crate::error::AegisError;
use crate::events::CollateralWithdrawn;
use crate::state::{Market, Position};
use crate::token::transfer::transfer_checked_out;
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
}

pub fn handler(ctx: Context<WithdrawCollateral>, amount: u64) -> Result<()> {
    require!(amount > 0, AegisError::ZeroAmount);

    require_keys_eq!(
        ctx.accounts.collateral_token_program.key(),
        ctx.accounts.market.collateral_token_program,
        AegisError::TokenProgramMismatch
    );

    // Hard sequencing gate: no oracle exists before Phase 5, so any priced (debt-aware)
    // withdrawal is unreachable by construction, not merely unimplemented. This is the ONLY
    // check on the debt branch — there is deliberately no placeholder price or "assumed healthy"
    // path (docs/phase-roadmap.md).
    require!(
        ctx.accounts.position.borrow_shares == 0,
        AegisError::OracleNotYetAvailable
    );

    require!(
        amount <= ctx.accounts.position.collateral_amount,
        AegisError::InsufficientCollateral
    );

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
