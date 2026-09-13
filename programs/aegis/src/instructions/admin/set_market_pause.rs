//! `set_market_pause(flags: u8)` — per-market pause bits, scoped to one market
//! (`instruction-catalogue.md` §2-5, §8). Same admin/guardian asymmetry as `set_protocol_pause`
//! (`guards::require_authorized_pause_change`), applied against `market.paused` instead of
//! `protocol.paused`. `governance.md` §7 R-5: pausing one market never affects any other
//! (ADR-0004's isolation paying off operationally).

use crate::constants::{MARKET_SEED, PROTOCOL_SEED};
use crate::events::MarketPauseSet;
use crate::guards::require_authorized_pause_change;
use crate::state::{Market, Protocol};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct SetMarketPause<'info> {
    pub authority: Signer<'info>,

    #[account(
        seeds = [PROTOCOL_SEED],
        bump = protocol.bump,
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
}

pub fn handler(ctx: Context<SetMarketPause>, flags: u8) -> Result<()> {
    require_authorized_pause_change(
        ctx.accounts.authority.key(),
        ctx.accounts.protocol.admin,
        ctx.accounts.protocol.guardian,
        ctx.accounts.market.paused,
        flags,
    )?;

    let market = &mut ctx.accounts.market;
    let old_paused = market.paused;
    market.paused = flags;

    emit!(MarketPauseSet {
        market: market.key(),
        authority: ctx.accounts.authority.key(),
        old_paused,
        new_paused: flags,
    });

    Ok(())
}
