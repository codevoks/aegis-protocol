//! `repay` — reduces a position's debt (`instruction-catalogue.md` §15).
//!
//! **Design-critical properties, all load-bearing (INV-ADM-04, INV-REP-01/02):**
//! - **No owner signature required** — `payer` is any signer; repaying someone else's debt is a
//!   gift, not an attack (`account-model.md` §5.1).
//! - **No oracle** — repayment never needs a price.
//! - **Unpausable** — no pause bit exists here at all, and none ever will (Phase 12 must never add
//!   one; INV-ADM-04 is structural, not policy).
//! - **Clamped to actual debt** — the computed repayment is capped at `position.borrow_shares`,
//!   and the token amount actually pulled is recomputed from the *clamped* shares, so this
//!   instruction **never pulls more tokens than the position's outstanding debt requires** (E-06).

use crate::constants::{LOAN_VAULT_SEED, MARKET_SEED, POSITION_SEED};
use crate::error::AegisError;
use crate::events::Repaid;
use crate::guards::require_exactly_one_amount;
use crate::state::{Market, Position};
use crate::token::transfer::transfer_checked_in;
use aegis_math::{to_assets_up, to_shares_down};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(Accounts)]
pub struct Repay<'info> {
    /// Anyone — repaying someone else's debt requires no permission (INV-AUTH-03).
    #[account(mut)]
    pub payer: Signer<'info>,

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

    /// No `has_one = owner` — `repay` targets any existing position in this market, not
    /// necessarily one owned by `payer`.
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
        constraint = payer_loan_ata.mint == market.loan_mint @ AegisError::VaultMintMismatch,
    )]
    pub payer_loan_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(address = market.loan_mint @ AegisError::VaultMintMismatch)]
    pub loan_mint: InterfaceAccount<'info, Mint>,

    pub loan_token_program: Interface<'info, TokenInterface>,
}

pub fn handler(ctx: Context<Repay>, assets: u64, shares: u128) -> Result<()> {
    require_exactly_one_amount(assets, shares)?;
    require_keys_eq!(
        ctx.accounts.loan_token_program.key(),
        ctx.accounts.market.loan_token_program,
        AegisError::TokenProgramMismatch
    );

    let now = Clock::get()?.unix_timestamp;
    let market = &mut ctx.accounts.market;
    let fee_position = &mut ctx.accounts.fee_position;
    market.accrue_mut(fee_position, now)?;

    // repay(assets) -> shares floored (row 4); repay(shares) -> taken as given, then both forms
    // are clamped to the position's actual outstanding debt (E-06) BEFORE recomputing the exact
    // token amount required, so this instruction can never pull more than the debt.
    let requested_shares = if assets > 0 {
        to_shares_down(
            assets,
            market.total_borrow_assets,
            market.total_borrow_shares,
        )
        .map_err(AegisError::from)?
    } else {
        shares
    };
    let clamped_shares = requested_shares.min(ctx.accounts.position.borrow_shares);

    // repay(shares) -> assets required, ceiled (row 8) -- the exact cost of paying down exactly
    // `clamped_shares` of debt, never more than what the position actually owes.
    let assets_to_pull = to_assets_up(
        clamped_shares,
        market.total_borrow_assets,
        market.total_borrow_shares,
    )
    .map_err(AegisError::from)?;

    let credited = transfer_checked_in(
        &ctx.accounts.payer_loan_ata.to_account_info(),
        &ctx.accounts.loan_mint.to_account_info(),
        &mut ctx.accounts.loan_vault,
        &ctx.accounts.payer.to_account_info(),
        &ctx.accounts.loan_token_program.to_account_info(),
        assets_to_pull,
        ctx.accounts.loan_mint.decimals,
    )?;
    // As in `supply`: loan assets are fee-free by policy, so credited == assets_to_pull is
    // expected but verified, never assumed.
    require_eq!(credited, assets_to_pull, AegisError::VaultAccountingError);

    let market = &mut ctx.accounts.market;
    market.total_borrow_assets = market
        .total_borrow_assets
        .checked_sub(assets_to_pull)
        .ok_or(AegisError::ArithmeticOverflow)?;
    market.total_borrow_shares = market
        .total_borrow_shares
        .checked_sub(clamped_shares)
        .ok_or(AegisError::ArithmeticOverflow)?;

    let position = &mut ctx.accounts.position;
    position.borrow_shares = position
        .borrow_shares
        .checked_sub(clamped_shares)
        .ok_or(AegisError::ArithmeticOverflow)?;

    emit!(Repaid {
        market: market.key(),
        position: position.key(),
        payer: ctx.accounts.payer.key(),
        assets_in: assets_to_pull,
        credited,
        shares_burned: clamped_shares,
    });

    Ok(())
}
