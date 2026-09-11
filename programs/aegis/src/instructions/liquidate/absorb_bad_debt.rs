//! `absorb_bad_debt` — loss recognition and protocol first-loss socialization
//! (`instruction-catalogue.md` §18, `economic-model.md` §8.2).
//!
//! **Design-critical properties, all load-bearing:**
//! - **Permissionless** — no signer account at all, matching `accrue_interest`'s precedent
//!   (`instructions/borrow/accrue.rs`): the transaction fee payer is not a named/validated
//!   account, so there is no identity to restrict.
//! - **No oracle** — bad-debt absorption is pure accounting over already-accrued shares/assets;
//!   this instruction's `Accounts` struct declares no price-update field at all, structurally
//!   (INV-ORA-02's counterpart for this instruction: loss recognition must never depend on the
//!   thing that may have caused the loss).
//! - **Unpausable** — no pause bit exists here, and none ever will (INV-ADM-04's list; Phase 12
//!   must never add one).
//! - **`position.collateral_amount == 0` EXACTLY** — no dust tolerance. This guarantees every
//!   liquidator has already had the chance to extract every recoverable unit, so there is no
//!   discretion and nothing to front-run.
//! - **`fee_position` is required, not optional**, PDA-constrained to `PDA(market,
//!   market.fee_recipient)` — a caller cannot omit or substitute it to skip protocol first-loss
//!   and push more loss onto lenders (a real griefing vector, closed by construction, same as
//!   every other instruction's mandatory `fee_position`).
//! - **No tokens move.** Both `total_supply_assets` and `total_borrow_assets` fall by exactly
//!   `bad_assets`, so `total_supply_assets − total_borrow_assets` (free liquidity) is unchanged
//!   and INV-CUS-01 holds through this instruction with zero CPI.

use crate::constants::{MARKET_SEED, POSITION_SEED};
use crate::error::AegisError;
use crate::events::BadDebtAbsorbed;
use crate::state::{Market, Position};
use aegis_math::{to_assets_down, to_assets_up, to_shares_up};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct AbsorbBadDebt<'info> {
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

    #[account(
        mut,
        has_one = market @ AegisError::PositionMarketMismatch,
    )]
    pub position: Account<'info, Position>,

    /// Mandatory, PDA-constrained -- never a caller-supplied account (T-11).
    #[account(
        mut,
        seeds = [POSITION_SEED, market.key().as_ref(), market.fee_recipient.as_ref()],
        bump = fee_position.bump,
        has_one = market @ AegisError::PositionMarketMismatch,
    )]
    pub fee_position: Account<'info, Position>,
}

pub fn handler(ctx: Context<AbsorbBadDebt>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    ctx.accounts
        .market
        .accrue_mut(&mut ctx.accounts.fee_position, now)?;

    let market = &ctx.accounts.market;
    let position = &ctx.accounts.position;

    // INV-SOLV-03: exact zero, no dust exception.
    require!(
        position.collateral_amount == 0,
        AegisError::BadDebtRequiresZeroCollateral
    );
    require!(
        position.borrow_shares > 0,
        AegisError::BadDebtRequiresOutstandingDebt
    );

    let bad_assets = to_assets_up(
        position.borrow_shares,
        market.total_borrow_assets,
        market.total_borrow_shares,
    )
    .map_err(AegisError::from)?;

    // 1. Protocol first-loss: burn the fee recipient's supply shares up to the loss
    //    (economic-model.md §8.2, INV-SOLV-06).
    let fee_position_account = &ctx.accounts.fee_position;
    let fee_assets = to_assets_down(
        fee_position_account.supply_shares,
        market.total_supply_assets,
        market.total_supply_shares,
    )
    .map_err(AegisError::from)?;
    let absorbed_by_protocol = bad_assets.min(fee_assets);
    let mut burn_shares = to_shares_up(
        absorbed_by_protocol,
        market.total_supply_assets,
        market.total_supply_shares,
    )
    .map_err(AegisError::from)?;
    burn_shares = burn_shares.min(fee_position_account.supply_shares);

    let fee_position = &mut ctx.accounts.fee_position;
    fee_position.supply_shares = fee_position
        .supply_shares
        .checked_sub(burn_shares)
        .ok_or(AegisError::ArithmeticOverflow)?;

    let market = &mut ctx.accounts.market;
    market.total_supply_shares = market
        .total_supply_shares
        .checked_sub(burn_shares)
        .ok_or(AegisError::ArithmeticOverflow)?;

    // 2. Socialize the remainder across the market's lenders. Both totals fall by exactly
    //    `bad_assets`, so the loan-vault reconciliation identity is preserved through this
    //    instruction without any token movement (INV-ACC-05, INV-SOLV-04).
    market.total_supply_assets = market.total_supply_assets.saturating_sub(bad_assets);
    market.total_borrow_assets = market.total_borrow_assets.saturating_sub(bad_assets);
    market.total_borrow_shares = market
        .total_borrow_shares
        .checked_sub(position.borrow_shares)
        .ok_or(AegisError::ArithmeticOverflow)?;

    let position = &mut ctx.accounts.position;
    let socialized = bad_assets.saturating_sub(absorbed_by_protocol);
    position.borrow_shares = 0;

    emit!(BadDebtAbsorbed {
        market: market.key(),
        position: position.key(),
        bad_assets,
        absorbed_by_protocol,
        socialized,
        fee_shares_burned: burn_shares,
    });

    Ok(())
}
