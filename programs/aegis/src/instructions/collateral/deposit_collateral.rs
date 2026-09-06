//! `deposit_collateral` — moves collateral from a depositor's token account into the market's
//! collateral vault, crediting the position by measured delta (`instruction-catalogue.md` §10).
//!
//! **No oracle. No pause check. No health check. `Market` is never written.** Collateral deposit
//! is strictly risk-reducing, so it has no preconditions beyond identity and amount — and because
//! it writes only `Position`, `collateral_vault` and the depositor's own token account, it never
//! contends with any other collateral deposit/withdrawal in the same market except on the shared
//! vault (`account-model.md` §8, claim C2). **A future change that adds a `Market` write here
//! silently destroys that property** — `A-PAR-01` exists specifically to catch it.

use crate::constants::{COLLATERAL_VAULT_SEED, MARKET_SEED};
use crate::error::AegisError;
use crate::events::CollateralDeposited;
use crate::state::{Market, Position};
use crate::token::transfer::transfer_checked_in;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(Accounts)]
pub struct DepositCollateral<'info> {
    /// Need not be the position owner (`account-model.md` §5.1) — depositing into someone else's
    /// position is strictly risk-reducing for them and requires no permission. This account is
    /// the SPL/Token-2022 transfer *authority* for `depositor_collateral_ata`, which the token
    /// program itself verifies during the CPI below.
    #[account(mut)]
    pub depositor: Signer<'info>,

    /// Read-only (see module doc). Not boxed like `create_market`'s copy needs to be: this
    /// instruction has far fewer accounts in its `Accounts` struct.
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
    )]
    pub position: Account<'info, Position>,

    /// Double-validated (`account-model.md` §1 principle 5): the canonical PDA derivation
    /// (`seeds`/`bump`) and the stored pubkey on `Market` (`address`) must both agree (T-03).
    #[account(
        mut,
        seeds = [COLLATERAL_VAULT_SEED, market.key().as_ref()],
        bump = market.collateral_vault_bump,
        address = market.collateral_vault @ AegisError::VaultMismatch,
    )]
    pub collateral_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = depositor_collateral_ata.mint == market.collateral_mint @ AegisError::VaultMintMismatch,
    )]
    pub depositor_collateral_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(address = market.collateral_mint @ AegisError::VaultMintMismatch)]
    pub collateral_mint: InterfaceAccount<'info, Mint>,

    pub collateral_token_program: Interface<'info, TokenInterface>,
}

pub fn handler(ctx: Context<DepositCollateral>, amount: u64) -> Result<()> {
    require!(amount > 0, AegisError::ZeroAmount);

    // T-05/T-11/INV-CUS-07: `Interface<TokenInterface>` alone accepts either token program —
    // pin it to the specific program `Market` was created with for this asset.
    require_keys_eq!(
        ctx.accounts.collateral_token_program.key(),
        ctx.accounts.market.collateral_token_program,
        AegisError::TokenProgramMismatch
    );

    let credited = transfer_checked_in(
        &ctx.accounts.depositor_collateral_ata.to_account_info(),
        &ctx.accounts.collateral_mint.to_account_info(),
        &mut ctx.accounts.collateral_vault,
        &ctx.accounts.depositor.to_account_info(),
        &ctx.accounts.collateral_token_program.to_account_info(),
        amount,
        ctx.accounts.collateral_mint.decimals,
    )?;

    let position = &mut ctx.accounts.position;
    position.collateral_amount = position
        .collateral_amount
        .checked_add(credited)
        .ok_or(AegisError::ArithmeticOverflow)?;

    emit!(CollateralDeposited {
        market: ctx.accounts.market.key(),
        position: position.key(),
        depositor: ctx.accounts.depositor.key(),
        amount_in: amount,
        credited,
    });

    Ok(())
}
