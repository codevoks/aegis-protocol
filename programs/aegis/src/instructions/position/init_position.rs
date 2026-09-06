//! `init_position` — creates an empty `Position` for `(market, owner)` (`instruction-catalogue.md` §9).
//!
//! A separate, explicit instruction rather than the conditional-create pattern this repository
//! bans (see `account-model.md` §9): Anchor's `init` fails outright if the account already
//! exists, which makes reinitialization structurally impossible (INV-LIFE-01) rather than merely
//! discouraged.

use crate::constants::{MARKET_SEED, POSITION_SEED};
use crate::error::AegisError;
use crate::events::PositionInitialized;
use crate::guards::require_non_default_pubkey;
use crate::state::{Market, Position};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct InitPosition<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        seeds = [
            MARKET_SEED,
            market.collateral_mint.as_ref(),
            market.loan_mint.as_ref(),
            &market.config_id.to_le_bytes(),
        ],
        bump = market.bump,
    )]
    pub market: Account<'info, Market>,

    /// CHECK: intentionally not a signer — anyone may create an empty position for any owner
    /// (instruction-catalogue.md §9); it can only ever be created empty and only `owner` can
    /// later act on it.
    pub owner: UncheckedAccount<'info>,

    #[account(
        init,
        payer = payer,
        space = Position::LEN,
        seeds = [POSITION_SEED, market.key().as_ref(), owner.key().as_ref()],
        bump,
    )]
    pub position: Account<'info, Position>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<InitPosition>) -> Result<()> {
    require_non_default_pubkey(ctx.accounts.owner.key(), AegisError::InvalidPositionOwner)?;

    let position = &mut ctx.accounts.position;
    position.market = ctx.accounts.market.key();
    position.owner = ctx.accounts.owner.key();
    position.supply_shares = 0;
    position.borrow_shares = 0;
    position.collateral_amount = 0;
    position.bump = ctx.bumps.position;
    position._reserved = [0u8; 32];

    emit!(PositionInitialized {
        market: position.market,
        position: position.key(),
        owner: position.owner,
    });

    Ok(())
}
