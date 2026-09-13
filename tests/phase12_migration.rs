//! Phase 12 — `migrate_protocol_v2`, the real Anchor 1.2.0 `Migration<'info, From, To>` account
//! schema migration (`docs/phases/phase-12-governance.md`, `docs/governance.md` §6,
//! `docs/invariants.md` INV-UPG-01..05).

#![allow(clippy::result_large_err)]

use aegis::error::AegisError;
use aegis::state::{Protocol, ProtocolV1};
use aegis_test_kit::{
    assert_aegis_error, deploy, fetch_protocol, migrate_protocol_v2, protocol_pda,
};
use anchor_lang::AccountSerialize;
use solana_account::Account as RawAccount;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

fn program_bytes() -> &'static [u8] {
    include_bytes!(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/aegis.so"))
}

fn fixed_pubkey(seed: u8) -> Pubkey {
    Keypair::new_from_array([seed; 32]).pubkey()
}

/// Injects a real, well-formed `ProtocolV1` account at the canonical protocol PDA -- simulating
/// "an account created before `schema_version` existed," which no real transaction in this
/// already-Phase-12 codebase can produce anymore (the same legitimate technique
/// `state_injection.rs` already establishes for otherwise-unreachable prior-schema states).
fn inject_v1_protocol(
    svm: &mut litesvm::LiteSVM,
    admin: Pubkey,
    pending_admin: Pubkey,
    guardian: Pubkey,
    fee_recipient: Pubkey,
    paused: u8,
) -> Pubkey {
    let (protocol_pubkey, bump) = protocol_pda();
    let v1 = ProtocolV1 {
        admin,
        pending_admin,
        guardian,
        fee_recipient,
        paused,
        bump,
        _reserved: [0u8; 64],
    };
    let mut data = Vec::new();
    v1.try_serialize(&mut data).expect("serialize ProtocolV1");
    assert_eq!(data.len(), ProtocolV1::LEN);
    svm.set_account(
        protocol_pubkey,
        RawAccount {
            lamports: svm.minimum_balance_for_rent_exemption(data.len()),
            data,
            owner: aegis::id(),
            executable: false,
            rent_epoch: 0,
        },
    )
    .expect("inject ProtocolV1 fixture");
    protocol_pubkey
}

// ============================================================================================
// I-UPG-01: migration correctness.
// ============================================================================================

#[test]
fn i_upg_01_migration_preserves_every_field_and_initializes_schema_version() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let pending_admin = fixed_pubkey(90);
    let guardian = fixed_pubkey(91);
    let fee_recipient = fixed_pubkey(92);
    let protocol_pubkey = inject_v1_protocol(
        &mut svm,
        admin.pubkey(),
        pending_admin,
        guardian,
        fee_recipient,
        aegis::constants::PAUSE_SUPPLY,
    );

    migrate_protocol_v2(&mut svm, &admin).expect("migration must succeed on a real V1 account");

    let migrated = fetch_protocol(&svm, &protocol_pubkey);
    // Preserved fields, identical.
    assert_eq!(migrated.admin, admin.pubkey());
    assert_eq!(migrated.pending_admin, pending_admin);
    assert_eq!(migrated.guardian, guardian);
    assert_eq!(migrated.fee_recipient, fee_recipient);
    assert_eq!(migrated.paused, aegis::constants::PAUSE_SUPPLY);
    let (_, expected_bump) = protocol_pda();
    assert_eq!(migrated.bump, expected_bump);
    // New field initialized correctly.
    assert_eq!(
        migrated.schema_version,
        aegis::constants::PROTOCOL_SCHEMA_VERSION
    );
    // PDA identity preserved -- same address, still owned by the program.
    let account = svm.get_account(&protocol_pubkey).unwrap();
    assert_eq!(account.owner, aegis::id());
    assert_eq!(account.data.len(), Protocol::LEN);
    // No realloc: the account is exactly the same size before and after.
    assert_eq!(Protocol::LEN, ProtocolV1::LEN);
}

// Authority preserved: only the account's own (pre-migration) admin may migrate it -- an
// unrelated signer is rejected, exactly as every other admin-gated instruction.
#[test]
fn migration_requires_the_real_admin_signer() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    inject_v1_protocol(
        &mut svm,
        admin.pubkey(),
        Pubkey::default(),
        fixed_pubkey(2),
        fixed_pubkey(3),
        0,
    );

    let attacker = Keypair::new_from_array([77u8; 32]);
    svm.airdrop(&attacker.pubkey(), 10_000_000_000).unwrap();
    let result = migrate_protocol_v2(&mut svm, &attacker);
    assert_aegis_error(&result, AegisError::NotProtocolAdmin);
}

// ============================================================================================
// I-UPG-02: migration idempotence -- a second attempt on an already-migrated account is
// rejected, not silently no-op'd.
// ============================================================================================

#[test]
fn i_upg_02_second_migration_attempt_is_rejected() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    inject_v1_protocol(
        &mut svm,
        admin.pubkey(),
        Pubkey::default(),
        fixed_pubkey(2),
        fixed_pubkey(3),
        0,
    );

    migrate_protocol_v2(&mut svm, &admin).expect("first migration must succeed");

    svm.expire_blockhash();
    let result = migrate_protocol_v2(&mut svm, &admin);
    assert!(
        result.is_err(),
        "a second migration attempt on an already-migrated account must fail"
    );
    // This is Anchor's own account-discriminator-mismatch rejection (the account now serializes
    // with Protocol's discriminator, not ProtocolV1's) -- a real, specific, distinguishable
    // error, not a silent no-op. Anchor's `ErrorCode::AccountDiscriminatorMismatch` is error code
    // 3002 (`6000 + 3002` is NOT how Anchor's OWN internal errors are banded -- they use their
    // own 2000/3000 ranges, distinct from AegisError's 6000+ custom range).
    let logs = format!("{result:?}");
    assert!(
        logs.contains("3002") || logs.contains("AccountDiscriminatorMismatch"),
        "expected Anchor's AccountDiscriminatorMismatch, got: {logs}"
    );

    // State is exactly what the first migration produced -- the rejected second attempt changed
    // nothing.
    let (protocol_pubkey, _) = protocol_pda();
    let migrated = fetch_protocol(&svm, &protocol_pubkey);
    assert_eq!(migrated.admin, admin.pubkey());
    assert_eq!(
        migrated.schema_version,
        aegis::constants::PROTOCOL_SCHEMA_VERSION
    );
}

// ============================================================================================
// Hostile cases.
// ============================================================================================

#[test]
fn migration_rejects_account_owned_by_the_wrong_program() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let (protocol_pubkey, bump) = protocol_pda();
    let v1 = ProtocolV1 {
        admin: admin.pubkey(),
        pending_admin: Pubkey::default(),
        guardian: fixed_pubkey(2),
        fee_recipient: fixed_pubkey(3),
        paused: 0,
        bump,
        _reserved: [0u8; 64],
    };
    let mut data = Vec::new();
    v1.try_serialize(&mut data).unwrap();
    svm.set_account(
        protocol_pubkey,
        RawAccount {
            lamports: svm.minimum_balance_for_rent_exemption(data.len()),
            data,
            owner: anchor_lang::solana_program::system_program::ID, // wrong owner
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();

    let result = migrate_protocol_v2(&mut svm, &admin);
    assert!(
        result.is_err(),
        "an account not owned by the aegis program must be rejected"
    );
}

#[test]
fn migration_rejects_corrupted_old_data() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let (protocol_pubkey, _) = protocol_pda();
    // Real ProtocolV1 discriminator, but a truncated body -- Borsh's fixed-width field types
    // (no Vec/String) mean any full-length byte pattern deserializes fine, so the genuinely
    // hostile "corrupted data" case for a fixed layout is an outright truncated buffer.
    let disc = <ProtocolV1 as anchor_lang::Discriminator>::DISCRIMINATOR;
    let mut short_data = vec![0u8; disc.len() + 4];
    short_data[..disc.len()].copy_from_slice(disc);

    svm.set_account(
        protocol_pubkey,
        RawAccount {
            lamports: svm.minimum_balance_for_rent_exemption(ProtocolV1::LEN),
            data: short_data,
            owner: aegis::id(),
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();

    let result = migrate_protocol_v2(&mut svm, &admin);
    assert!(
        result.is_err(),
        "a truncated/corrupted account body must be rejected at deserialization"
    );
}

#[test]
fn migration_rejects_unsupported_version_garbage_discriminator() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let (protocol_pubkey, _) = protocol_pda();
    // Neither ProtocolV1's nor Protocol's real discriminator.
    let mut data = vec![0xFFu8; ProtocolV1::LEN];
    data[0..8].copy_from_slice(&[9, 9, 9, 9, 9, 9, 9, 9]);

    svm.set_account(
        protocol_pubkey,
        RawAccount {
            lamports: svm.minimum_balance_for_rent_exemption(data.len()),
            data,
            owner: aegis::id(),
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();

    let result = migrate_protocol_v2(&mut svm, &admin);
    assert!(
        result.is_err(),
        "an account with an unrecognized discriminator must be rejected"
    );
}

#[test]
fn migration_rejects_a_nonexistent_account() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    // No `initialize_protocol`, no injection -- the canonical PDA has never been created.
    let result = migrate_protocol_v2(&mut svm, &admin);
    assert!(
        result.is_err(),
        "migrating a nonexistent account must fail (AccountNotInitialized)"
    );
}

// No fund movement: structural, not runtime -- `MigrateProtocolV2`'s Accounts struct has no
// token account, no vault, and no mint field at all, so there is no code path through which this
// instruction could ever move collateral, loan tokens, or protocol fees (governance.md §6 rule,
// this phase's "no rescue path" mandate).
#[test]
fn migration_accounts_struct_has_no_token_or_vault_fields() {
    let source =
        std::fs::read_to_string("programs/aegis/src/instructions/admin/migrate_protocol_v2.rs")
            .expect("read migrate_protocol_v2.rs");
    for forbidden in ["TokenAccount", "TokenInterface", "vault", "mint"] {
        assert!(
            !source.to_lowercase().contains(&forbidden.to_lowercase()),
            "migrate_protocol_v2's Accounts struct must never reference {forbidden}"
        );
    }
}
