//! `set_pending_admin(new_admin)` — the first step of the two-step admin transfer
//! (`instruction-catalogue.md` §2-5, INV-ADM-02). Writes only `pending_admin`; the current admin
//! keeps full authority until `accept_admin` succeeds. There is deliberately no single-transaction
//! `set_admin(new_admin)` anywhere in this program — a typo'd or unreachable key would otherwise
//! permanently brick governance (ADR-0012).

use crate::constants::PROTOCOL_SEED;
use crate::error::AegisError;
use crate::events::AdminTransferStarted;
use crate::guards::require_non_default_pubkey;
use crate::state::Protocol;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct SetPendingAdmin<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [PROTOCOL_SEED],
        bump = protocol.bump,
        has_one = admin @ AegisError::NotProtocolAdmin,
    )]
    pub protocol: Account<'info, Protocol>,
}

pub fn handler(ctx: Context<SetPendingAdmin>, new_admin: Pubkey) -> Result<()> {
    require_non_default_pubkey(new_admin, AegisError::DefaultPubkeyNotAllowed)?;

    let protocol = &mut ctx.accounts.protocol;
    protocol.pending_admin = new_admin;

    emit!(AdminTransferStarted {
        protocol: protocol.key(),
        current_admin: protocol.admin,
        pending_admin: new_admin,
    });

    Ok(())
}
