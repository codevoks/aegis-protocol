//! `initialize_protocol` — creates the singleton `Protocol` account (`instruction-catalogue.md` §1).

use crate::constants::PROTOCOL_SEED;
use crate::error::AegisError;
use crate::events::ProtocolInitialized;
use crate::guards::require_non_default_pubkey;
use crate::state::Protocol;
use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct InitProtocolArgs {
    pub guardian: Pubkey,
    pub fee_recipient: Pubkey,
}

#[derive(Accounts)]
pub struct InitializeProtocol<'info> {
    /// Becomes `protocol.admin`. Whoever lands this transaction first wins — the deployment
    /// checklist (`I-DEPLOY-01`) requires asserting `protocol.admin` immediately after, since
    /// `instruction-catalogue.md` §1 names front-running initialization as the attack here.
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = Protocol::LEN,
        seeds = [PROTOCOL_SEED],
        bump,
    )]
    pub protocol: Account<'info, Protocol>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<InitializeProtocol>, args: InitProtocolArgs) -> Result<()> {
    require_non_default_pubkey(args.guardian, AegisError::DefaultPubkeyNotAllowed)?;
    require_non_default_pubkey(args.fee_recipient, AegisError::DefaultPubkeyNotAllowed)?;

    let protocol = &mut ctx.accounts.protocol;
    protocol.admin = ctx.accounts.payer.key();
    protocol.pending_admin = Pubkey::default();
    protocol.guardian = args.guardian;
    protocol.fee_recipient = args.fee_recipient;
    protocol.paused = 0;
    protocol.bump = ctx.bumps.protocol;
    protocol._reserved = [0u8; 64];

    emit!(ProtocolInitialized {
        protocol: protocol.key(),
        admin: protocol.admin,
        guardian: protocol.guardian,
        fee_recipient: protocol.fee_recipient,
    });

    Ok(())
}
