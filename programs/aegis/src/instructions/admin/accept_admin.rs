//! `accept_admin()` — the second step of the two-step admin transfer (`instruction-catalogue.md`
//! §2-5, INV-ADM-02, INV-AUTH-05). The signer must equal `protocol.pending_admin` exactly; on
//! success `admin = pending_admin` and `pending_admin` is cleared back to `Pubkey::default()` in
//! the same instruction, which is also what makes a replay of this instruction fail: the second
//! call's signer (the now-*former* pending admin) no longer matches the cleared field.

use crate::constants::PROTOCOL_SEED;
use crate::error::AegisError;
use crate::events::AdminTransferred;
use crate::state::Protocol;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct AcceptAdmin<'info> {
    /// **Must be `protocol.pending_admin`** (`A-AUTH-05`) — not checked via `has_one` because the
    /// relevant field is `pending_admin`, not `admin`; enforced explicitly in the handler instead.
    pub pending_admin: Signer<'info>,

    #[account(
        mut,
        seeds = [PROTOCOL_SEED],
        bump = protocol.bump,
    )]
    pub protocol: Account<'info, Protocol>,
}

pub fn handler(ctx: Context<AcceptAdmin>) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.pending_admin.key(),
        ctx.accounts.protocol.pending_admin,
        AegisError::NotPendingAdmin
    );

    let protocol = &mut ctx.accounts.protocol;
    let old_admin = protocol.admin;
    let new_admin = protocol.pending_admin;
    protocol.admin = new_admin;
    protocol.pending_admin = Pubkey::default();

    emit!(AdminTransferred {
        protocol: protocol.key(),
        old_admin,
        new_admin,
    });

    Ok(())
}
