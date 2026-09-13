//! `set_protocol_pause(flags: u8)` — protocol-wide pause bits (`instruction-catalogue.md` §2-5,
//! `governance.md` §1/§3). Callable by admin **or** guardian, asymmetrically
//! (`guards::require_authorized_pause_change`): the admin may set or clear any combination of the
//! four defined bits; the guardian may only add bits, never remove one (`A-AUTH-04`, INV-AUTH-04).
//! Undefined bits are rejected unconditionally (`A-ADM-05`, INV-ADM-03).

use crate::constants::PROTOCOL_SEED;
use crate::events::ProtocolPauseSet;
use crate::guards::require_authorized_pause_change;
use crate::state::Protocol;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct SetProtocolPause<'info> {
    /// Admin or guardian — checked in the handler (`require_authorized_pause_change`), not via
    /// `has_one`, since Anchor's `has_one` cannot express an OR of two relations.
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [PROTOCOL_SEED],
        bump = protocol.bump,
    )]
    pub protocol: Account<'info, Protocol>,
}

pub fn handler(ctx: Context<SetProtocolPause>, flags: u8) -> Result<()> {
    let protocol = &ctx.accounts.protocol;
    require_authorized_pause_change(
        ctx.accounts.authority.key(),
        protocol.admin,
        protocol.guardian,
        protocol.paused,
        flags,
    )?;

    let protocol = &mut ctx.accounts.protocol;
    let old_paused = protocol.paused;
    protocol.paused = flags;

    emit!(ProtocolPauseSet {
        protocol: protocol.key(),
        authority: ctx.accounts.authority.key(),
        old_paused,
        new_paused: flags,
    });

    Ok(())
}
