//! `accrue_interest` — standalone, permissionless interest accrual (`instruction-catalogue.md`
//! §16). No privileged signer is required merely to bring `Market` accounting current; interest
//! already accrues implicitly inside `supply`/`withdraw`/`repay`, but a standalone instruction
//! makes accrual independently observable, testable, and keeper-friendly. Writes only two
//! accounts (`Market`, `fee_position`) and is unpausable (accrual itself never moves risk).

use crate::constants::{MARKET_SEED, POSITION_SEED};
use crate::error::AegisError;
use crate::events::InterestAccrued;
use crate::state::{Market, Position};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct AccrueInterest<'info> {
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
        seeds = [POSITION_SEED, market.key().as_ref(), market.fee_recipient.as_ref()],
        bump = fee_position.bump,
        has_one = market @ AegisError::PositionMarketMismatch,
    )]
    pub fee_position: Account<'info, Position>,
}

pub fn handler(ctx: Context<AccrueInterest>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let market = &mut ctx.accounts.market;
    let fee_position = &mut ctx.accounts.fee_position;
    let (outcome, fee_shares) = market.accrue_mut(fee_position, now)?;

    emit!(InterestAccrued {
        market: market.key(),
        interest: outcome.interest,
        fee_amount: outcome.fee_amount,
        fee_shares,
        total_borrow_assets: outcome.total_borrow_assets,
        total_supply_assets: outcome.total_supply_assets,
    });

    Ok(())
}
