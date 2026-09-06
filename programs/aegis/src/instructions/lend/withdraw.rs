//! `withdraw` — redeems the lender's own supply shares for loan-asset tokens
//! (`instruction-catalogue.md` §13).
//!
//! Bounded by **free liquidity** (`total_supply_assets - total_borrow_assets`) — the vault
//! reconciliation identity of `economic-model.md` §2, not a separate rule. Direct token donations
//! to `loan_vault` never manufacture protocol-accounted liquidity rights (INV-CUS-08): this check
//! is against the market's own accounting scalars, never the vault's raw token balance.

use crate::constants::{LOAN_VAULT_SEED, MARKET_SEED, POSITION_SEED};
use crate::error::AegisError;
use crate::events::Withdrawn;
use crate::guards::require_exactly_one_amount;
use crate::state::{Market, Position};
use crate::token::transfer::transfer_checked_out;
use aegis_math::{to_assets_down, to_shares_up};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(Accounts)]
pub struct Withdraw<'info> {
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

pub fn handler(ctx: Context<Withdraw>, assets: u64, shares: u128) -> Result<()> {
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

    let (assets_out, shares_burned) = if assets > 0 {
        let shares_burned = to_shares_up(
            assets,
            market.total_supply_assets,
            market.total_supply_shares,
        )
        .map_err(AegisError::from)?;
        (assets, shares_burned)
    } else {
        let assets_out = to_assets_down(
            shares,
            market.total_supply_assets,
            market.total_supply_shares,
        )
        .map_err(AegisError::from)?;
        (assets_out, shares)
    };

    require!(
        shares_burned <= ctx.accounts.position.supply_shares,
        AegisError::InsufficientShares
    );

    // The vault-reconciliation identity itself (economic-model.md §2), not a separate rule:
    // withdrawing more than free liquidity would mean paying out assets that are actually lent
    // out to borrowers. E-05.
    let free_liquidity = market
        .total_supply_assets
        .checked_sub(market.total_borrow_assets)
        .ok_or(AegisError::ArithmeticOverflow)?;
    require!(
        assets_out <= free_liquidity,
        AegisError::InsufficientLiquidity
    );

    market.total_supply_assets = market
        .total_supply_assets
        .checked_sub(assets_out)
        .ok_or(AegisError::ArithmeticOverflow)?;
    market.total_supply_shares = market
        .total_supply_shares
        .checked_sub(shares_burned)
        .ok_or(AegisError::ArithmeticOverflow)?;

    let position = &mut ctx.accounts.position;
    position.supply_shares = position
        .supply_shares
        .checked_sub(shares_burned)
        .ok_or(AegisError::ArithmeticOverflow)?;

    let market_key = market.key();
    let collateral_mint = market.collateral_mint;
    let loan_mint_key = market.loan_mint;
    let config_id_bytes = market.config_id.to_le_bytes();
    let market_bump = market.bump;
    let signer_seeds: &[&[u8]] = &[
        MARKET_SEED,
        collateral_mint.as_ref(),
        loan_mint_key.as_ref(),
        &config_id_bytes,
        &[market_bump],
    ];

    transfer_checked_out(
        &ctx.accounts.loan_vault.to_account_info(),
        &ctx.accounts.loan_mint.to_account_info(),
        &ctx.accounts.owner_loan_ata.to_account_info(),
        &ctx.accounts.market.to_account_info(),
        &ctx.accounts.loan_token_program.to_account_info(),
        assets_out,
        ctx.accounts.loan_mint.decimals,
        signer_seeds,
    )?;

    emit!(Withdrawn {
        market: market_key,
        position: ctx.accounts.position.key(),
        owner: ctx.accounts.owner.key(),
        assets_out,
        shares_burned,
    });

    Ok(())
}
