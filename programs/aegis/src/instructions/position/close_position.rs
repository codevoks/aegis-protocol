//! `close_position` — defunds and closes an empty `Position` (`instruction-catalogue.md` §20,
//! `account-model.md` §10).
//!
//! Closable only when `supply_shares == 0 && borrow_shares == 0 && collateral_amount == 0`,
//! checked as **exact** equalities — never a dust tolerance. Uses Anchor's `close = owner`
//! (lamports to `owner`, discriminator zeroed, account defunded), not a hand-rolled pattern:
//! `CLOSED_ACCOUNT_DISCRIMINATOR` was removed in Anchor 1.0 and manual closes are the known
//! revival vector (T-13). Because the position PDA is deterministic, a closed position can be
//! recreated later by `init_position`, which is safe: it can only ever come back empty.

use crate::constants::MARKET_SEED;
use crate::error::AegisError;
use crate::events::PositionClosed;
use crate::state::{Market, Position};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct ClosePosition<'info> {
    #[account(mut)]
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
        close = owner,
    )]
    pub position: Account<'info, Position>,
}

pub fn handler(ctx: Context<ClosePosition>) -> Result<()> {
    let position = &ctx.accounts.position;
    require!(
        position.supply_shares == 0
            && position.borrow_shares == 0
            && position.collateral_amount == 0,
        AegisError::PositionNotEmpty
    );

    emit!(PositionClosed {
        market: ctx.accounts.market.key(),
        position: position.key(),
        owner: ctx.accounts.owner.key(),
    });

    Ok(())
}
