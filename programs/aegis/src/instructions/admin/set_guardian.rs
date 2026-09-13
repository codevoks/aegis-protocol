//! `set_guardian(new_guardian)` — admin-only guardian reassignment (`instruction-catalogue.md`
//! §2-5). Writes only `guardian`; never touches `paused` (no implicit pause/unpause behavior).

use crate::constants::PROTOCOL_SEED;
use crate::error::AegisError;
use crate::events::GuardianChanged;
use crate::guards::require_non_default_pubkey;
use crate::state::Protocol;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct SetGuardian<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [PROTOCOL_SEED],
        bump = protocol.bump,
        has_one = admin @ AegisError::NotProtocolAdmin,
    )]
    pub protocol: Account<'info, Protocol>,
}

pub fn handler(ctx: Context<SetGuardian>, new_guardian: Pubkey) -> Result<()> {
    require_non_default_pubkey(new_guardian, AegisError::DefaultPubkeyNotAllowed)?;

    let protocol = &mut ctx.accounts.protocol;
    let old_guardian = protocol.guardian;
    protocol.guardian = new_guardian;

    emit!(GuardianChanged {
        protocol: protocol.key(),
        admin: ctx.accounts.admin.key(),
        old_guardian,
        new_guardian,
    });

    Ok(())
}
