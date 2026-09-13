//! `migrate_protocol_v2()` — the Phase 12 account-schema migration demonstration (INV-UPG-01..03,
//! ADR-0014), using Anchor 1.2.0's real `Migration<'info, From, To>` primitive rather than a
//! hand-rolled realloc-and-reinterpret scheme.
//!
//! Migrates a `Protocol` account created before `schema_version` existed
//! (`state::protocol::ProtocolV1`, byte-identical to the current `Protocol` minus that one field)
//! into the current schema (`state::protocol::Protocol`). No realloc is used or needed: both
//! layouts are exactly 202 bytes (`ProtocolV1::LEN == Protocol::LEN`, asserted in
//! `state/protocol.rs`'s own tests) — the new field is carved out of what was previously all
//! `_reserved` space, exactly the "additive first" rule `governance.md` §6 requires.
//!
//! Moves no tokens and touches no other account (`governance.md` §6 rule 3 / this phase's "no
//! rescue path" mandate) — this instruction transforms one account's schema and nothing else.

use crate::constants::{PROTOCOL_SCHEMA_VERSION, PROTOCOL_SEED};
use crate::error::AegisError;
use crate::events::ProtocolMigrated;
use crate::state::{Protocol, ProtocolV1};
use anchor_lang::accounts::migration::Migration;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct MigrateProtocolV2<'info> {
    /// Must equal the pre-migration account's own `admin` field (checked in the handler via
    /// `try_as_from`, since the account's type is not yet known to be `Protocol` or `ProtocolV1`
    /// at the point Anchor's declarative constraints run).
    pub admin: Signer<'info>,

    /// `Migration::try_from` itself rejects an account that is uninitialized (wrong owner) or
    /// already in the `Protocol` (`To`) format — the concrete mechanism behind INV-UPG-03: running
    /// this instruction a second time on an already-migrated account fails at account
    /// deserialization, before the handler body even runs.
    #[account(mut)]
    pub protocol: Migration<'info, ProtocolV1, Protocol>,
}

pub fn handler(ctx: Context<MigrateProtocolV2>) -> Result<()> {
    // Bind the canonical PDA explicitly -- `Migration<'info, ...>` does not support a declarative
    // `seeds = [...], bump` constraint the way `Account<'info, T>` does, so this is a manual,
    // equivalent check rather than an omitted one.
    let (expected_protocol, _bump) = Pubkey::find_program_address(&[PROTOCOL_SEED], &crate::ID);
    require_keys_eq!(
        ctx.accounts.protocol.key(),
        expected_protocol,
        AegisError::NotCanonicalProtocol
    );

    let old = ctx.accounts.protocol.try_as_from()?;
    require_keys_eq!(
        ctx.accounts.admin.key(),
        old.admin,
        AegisError::NotProtocolAdmin
    );

    let new_data = Protocol {
        admin: old.admin,
        pending_admin: old.pending_admin,
        guardian: old.guardian,
        fee_recipient: old.fee_recipient,
        paused: old.paused,
        bump: old.bump,
        schema_version: PROTOCOL_SCHEMA_VERSION,
        _reserved: [0u8; 63],
    };
    let migrated_admin = new_data.admin;

    // Unconditional: `AccountsExit` (Anchor's own `exit` hook, run automatically after this
    // handler returns) fails the whole transaction with `AccountNotMigrated` if `.migrate()` was
    // never called -- there is no path through this instruction that leaves the account
    // unmigrated and still succeeds.
    ctx.accounts.protocol.migrate(new_data)?;

    emit!(ProtocolMigrated {
        protocol: expected_protocol,
        admin: migrated_admin,
        schema_version: PROTOCOL_SCHEMA_VERSION,
    });

    Ok(())
}
