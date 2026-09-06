//! `supply` — deposits the loan asset into the market's pool, crediting the lender's own position
//! with supply shares (`instruction-catalogue.md` §12).
//!
//! **Exactly one** of `assets`/`shares` must be nonzero (`guards::require_exactly_one_amount`).
//! Interest accrues first (`Market::accrue_mut`, which may mint protocol fee shares to
//! `fee_position`), then the requested/computed amount is transferred in and measured
//! (`account-model.md` §6.4) — loan assets are policy-restricted to fee-free mints
//! (`token-compatibility.md`), so `credited` is expected to equal the requested amount, but this
//! is **verified, not assumed**: a mismatch aborts with `VaultAccountingError` rather than
//! silently minting shares against a different figure than what actually arrived.
//!
//! No pause check: `set_market_pause`/`set_protocol_pause` are Phase 12 scope and, before they
//! exist, no instruction can ever set a pause bit to nonzero — a check today would be dead code
//! with no way to exercise it honestly (same precedent `withdraw_collateral` set in Phase 3).

use crate::constants::{LOAN_VAULT_SEED, MARKET_SEED, POSITION_SEED};
use crate::error::AegisError;
use crate::events::Supplied;
use crate::guards::require_exactly_one_amount;
use crate::state::{Market, Position};
use crate::token::transfer::transfer_checked_in;
use aegis_math::{to_assets_up, to_shares_down};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(Accounts)]
pub struct Supply<'info> {
    /// The lender. `supply` always credits the signer's **own** position (there is no
    /// third-party-supply feature analogous to `deposit_collateral`'s flexible `depositor` —
    /// `account-model.md` §5.1 requires an owner signature for `supply`).
    #[account(mut)]
    pub owner: Signer<'info>,

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
        has_one = owner @ AegisError::NotPositionOwner,
    )]
    pub position: Account<'info, Position>,

    /// PDA-constrained to `PDA(market, market.fee_recipient)` (never a caller-supplied account) —
    /// Anchor 1.0's default duplicate-mutable-account protection rejects any transaction where
    /// this happens to equal `position` (T-11, `A-ACC-01`).
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
        constraint = owner_loan_ata.mint == market.loan_mint @ AegisError::VaultMintMismatch,
    )]
    pub owner_loan_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(address = market.loan_mint @ AegisError::VaultMintMismatch)]
    pub loan_mint: InterfaceAccount<'info, Mint>,

    pub loan_token_program: Interface<'info, TokenInterface>,
}

pub fn handler(ctx: Context<Supply>, assets: u64, shares: u128) -> Result<()> {
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

    // Compute the requested-transfer amount and the shares it will mint, using the
    // now-fully-accrued totals (economic-model.md §1.3 rows 1 and 5).
    let (requested_assets, shares_minted) = if assets > 0 {
        let shares_minted = to_shares_down(
            assets,
            market.total_supply_assets,
            market.total_supply_shares,
        )
        .map_err(AegisError::from)?;
        (assets, shares_minted)
    } else {
        let requested_assets = to_assets_up(
            shares,
            market.total_supply_assets,
            market.total_supply_shares,
        )
        .map_err(AegisError::from)?;
        (requested_assets, shares)
    };

    let credited = transfer_checked_in(
        &ctx.accounts.owner_loan_ata.to_account_info(),
        &ctx.accounts.loan_mint.to_account_info(),
        &mut ctx.accounts.loan_vault,
        &ctx.accounts.owner.to_account_info(),
        &ctx.accounts.loan_token_program.to_account_info(),
        requested_assets,
        ctx.accounts.loan_mint.decimals,
    )?;
    // Loan assets are policy-restricted to fee-free mints (create_market precondition 4), so
    // credited == requested is expected -- but verified, never assumed. A mismatch here would mean
    // either a policy-check bypass or a genuinely unexpected token-program behavior; either way,
    // minting shares against a different figure than what actually arrived would be a real
    // accounting bug, so this aborts rather than silently drifting.
    require_eq!(credited, requested_assets, AegisError::VaultAccountingError);

    let market = &mut ctx.accounts.market;
    market.total_supply_assets = market
        .total_supply_assets
        .checked_add(credited)
        .ok_or(AegisError::ArithmeticOverflow)?;
    market.total_supply_shares = market
        .total_supply_shares
        .checked_add(shares_minted)
        .ok_or(AegisError::ArithmeticOverflow)?;

    let position = &mut ctx.accounts.position;
    position.supply_shares = position
        .supply_shares
        .checked_add(shares_minted)
        .ok_or(AegisError::ArithmeticOverflow)?;

    emit!(Supplied {
        market: market.key(),
        position: position.key(),
        owner: ctx.accounts.owner.key(),
        assets_in: requested_assets,
        credited,
        shares_minted,
    });

    Ok(())
}
