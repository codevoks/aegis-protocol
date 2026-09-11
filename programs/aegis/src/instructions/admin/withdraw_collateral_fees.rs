//! `withdraw_collateral_fees` — admin withdrawal of protocol-owned accrued collateral fees only
//! (`instruction-catalogue.md` §19).
//!
//! **The concrete mechanism behind INV-ADM-01/NFR-9 ("the admin cannot move user funds"):** the
//! withdrawal is bounded by `market.collateral_fee_accrued`, a field that increases **only**
//! inside `liquidate`, by exactly `protocol_cut` (INV-CUS-09) — never by a fraction of, or
//! anything derived from, `Σ(position.collateral_amount)`. There is no code path here that reads
//! or touches any `Position` account at all. `A-ADM-02` is the adversarial proof.

use crate::constants::{COLLATERAL_VAULT_SEED, MARKET_SEED, PROTOCOL_SEED};
use crate::error::AegisError;
use crate::events::CollateralFeesWithdrawn;
use crate::state::{Market, Protocol};
use crate::token::transfer::transfer_checked_out;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(Accounts)]
pub struct WithdrawCollateralFees<'info> {
    pub admin: Signer<'info>,

    #[account(
        seeds = [PROTOCOL_SEED],
        bump = protocol.bump,
        has_one = admin @ AegisError::NotProtocolAdmin,
    )]
    pub protocol: Account<'info, Protocol>,

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
        seeds = [COLLATERAL_VAULT_SEED, market.key().as_ref()],
        bump = market.collateral_vault_bump,
        address = market.collateral_vault @ AegisError::VaultMismatch,
    )]
    pub collateral_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = admin_collateral_ata.mint == market.collateral_mint @ AegisError::VaultMintMismatch,
    )]
    pub admin_collateral_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(address = market.collateral_mint @ AegisError::VaultMintMismatch)]
    pub collateral_mint: InterfaceAccount<'info, Mint>,

    pub collateral_token_program: Interface<'info, TokenInterface>,
}

pub fn handler(ctx: Context<WithdrawCollateralFees>, amount: u64) -> Result<()> {
    require!(amount > 0, AegisError::ZeroAmount);
    require_keys_eq!(
        ctx.accounts.collateral_token_program.key(),
        ctx.accounts.market.collateral_token_program,
        AegisError::TokenProgramMismatch
    );

    // INV-ADM-08 / A-ADM-02: never more than the protocol's own accrued cut. No Position account
    // is even present in this instruction's account list -- there is nothing else to bound
    // against.
    require!(
        amount <= ctx.accounts.market.collateral_fee_accrued,
        AegisError::InsufficientCollateralFees
    );

    ctx.accounts.market.collateral_fee_accrued = ctx
        .accounts
        .market
        .collateral_fee_accrued
        .checked_sub(amount)
        .ok_or(AegisError::ArithmeticOverflow)?;

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
        &ctx.accounts.admin_collateral_ata.to_account_info(),
        &market.to_account_info(),
        &ctx.accounts.collateral_token_program.to_account_info(),
        amount,
        ctx.accounts.collateral_mint.decimals,
        signer_seeds,
    )?;

    emit!(CollateralFeesWithdrawn {
        market: market_key,
        admin: ctx.accounts.admin.key(),
        amount,
        remaining_collateral_fee_accrued: ctx.accounts.market.collateral_fee_accrued,
    });

    Ok(())
}
