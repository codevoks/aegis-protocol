//! Phase 12 — two-step admin transfer, guardian assignment, and the cross-instruction authority
//! matrix (`docs/phases/phase-12-governance.md`, `docs/governance.md` §1, `docs/invariants.md`
//! INV-ADM-02, INV-AUTH-05).

#![allow(clippy::result_large_err)]

use aegis::error::AegisError;
use aegis_test_kit::{
    accept_admin, assert_aegis_error, deploy, fetch_protocol, initialize_protocol, protocol_pda,
    set_guardian, set_pending_admin, set_protocol_pause,
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

fn program_bytes() -> &'static [u8] {
    include_bytes!(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/aegis.so"))
}

fn fixed_pubkey(seed: u8) -> Pubkey {
    Keypair::new_from_array([seed; 32]).pubkey()
}

fn setup(svm: &mut litesvm::LiteSVM, admin: &Keypair) -> (Pubkey, Pubkey) {
    let guardian = fixed_pubkey(2);
    let fee_recipient = fixed_pubkey(3);
    initialize_protocol(svm, admin, guardian, fee_recipient).expect("initialize_protocol");
    (guardian, fee_recipient)
}

// ============================================================================================
// set_pending_admin / accept_admin -- two-step transfer
// ============================================================================================

#[test]
fn set_pending_admin_writes_only_pending_admin_and_preserves_everything_else() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    setup(&mut svm, &admin);
    let (protocol_pubkey, _) = protocol_pda();

    let before = fetch_protocol(&svm, &protocol_pubkey);
    let new_admin = fixed_pubkey(9);
    set_pending_admin(&mut svm, &admin, new_admin).expect("set_pending_admin must succeed");

    let after = fetch_protocol(&svm, &protocol_pubkey);
    assert_eq!(after.pending_admin, new_admin);
    // Everything else preserved -- set_pending_admin does not transfer authority itself.
    assert_eq!(after.admin, before.admin);
    assert_eq!(after.guardian, before.guardian);
    assert_eq!(after.fee_recipient, before.fee_recipient);
    assert_eq!(after.paused, before.paused);
    assert_eq!(after.bump, before.bump);
}

#[test]
fn set_pending_admin_rejects_default_pubkey() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    setup(&mut svm, &admin);
    let result = set_pending_admin(&mut svm, &admin, Pubkey::default());
    assert_aegis_error(&result, AegisError::DefaultPubkeyNotAllowed);
}

#[test]
fn set_pending_admin_rejects_non_admin_signer() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    setup(&mut svm, &admin);
    let attacker = Keypair::new_from_array([77u8; 32]);
    svm.airdrop(&attacker.pubkey(), 10_000_000_000).unwrap();
    let result = set_pending_admin(&mut svm, &attacker, fixed_pubkey(9));
    assert_aegis_error(&result, AegisError::NotProtocolAdmin);
}

// A-AUTH-05: only the pending admin may accept.
#[test]
fn a_auth_05_only_pending_admin_can_accept() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    setup(&mut svm, &admin);
    let (protocol_pubkey, _) = protocol_pda();

    let new_admin = Keypair::new_from_array([9u8; 32]);
    svm.airdrop(&new_admin.pubkey(), 10_000_000_000).unwrap();
    set_pending_admin(&mut svm, &admin, new_admin.pubkey()).expect("set_pending_admin");

    // An unrelated signer, NOT the pending admin, must be rejected with the exact error.
    let attacker = Keypair::new_from_array([88u8; 32]);
    svm.airdrop(&attacker.pubkey(), 10_000_000_000).unwrap();
    let result = accept_admin(&mut svm, &attacker);
    assert_aegis_error(&result, AegisError::NotPendingAdmin);

    // The OLD admin itself is also not the pending admin and must be rejected the same way.
    let result = accept_admin(&mut svm, &admin);
    assert_aegis_error(&result, AegisError::NotPendingAdmin);

    // The real pending admin succeeds.
    accept_admin(&mut svm, &new_admin).expect("the real pending admin must be able to accept");

    let after = fetch_protocol(&svm, &protocol_pubkey);
    assert_eq!(after.admin, new_admin.pubkey());
    assert_eq!(after.pending_admin, Pubkey::default());
}

// A one-transaction `set_admin(new_admin)` instruction must not exist at all -- there is no
// instruction discriminator for it. The two-step path is the *only* path; this is a structural
// property of `lib.rs`'s `#[program]` module (which functions exist), not a runtime check, so it
// is proven by grep rather than a transaction.
#[test]
fn no_single_step_set_admin_instruction_exists() {
    let lib_rs = std::fs::read_to_string("programs/aegis/src/lib.rs").expect("read lib.rs");
    assert!(
        !lib_rs.contains("fn set_admin("),
        "a single-step set_admin instruction must never exist -- only set_pending_admin/accept_admin"
    );
}

// Replay: after a successful accept_admin, pending_admin is cleared, so calling accept_admin
// again (same signer, now stale) must fail -- not silently no-op, and not succeed a second time.
#[test]
fn replay_after_successful_acceptance_fails() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    setup(&mut svm, &admin);

    let new_admin = Keypair::new_from_array([9u8; 32]);
    svm.airdrop(&new_admin.pubkey(), 10_000_000_000).unwrap();
    set_pending_admin(&mut svm, &admin, new_admin.pubkey()).expect("set_pending_admin");
    accept_admin(&mut svm, &new_admin).expect("first accept must succeed");

    // A fresh blockhash, not a stale-blockhash/duplicate-signature rejection, is what should make
    // this second attempt distinguishable from the first at the transaction layer -- the program
    // itself must be what rejects the replay (NotPendingAdmin), matching every other `expire_
    // blockhash` precedent in this test suite (e.g. tests/phase4_adversarial.rs).
    svm.expire_blockhash();
    let result = accept_admin(&mut svm, &new_admin);
    assert_aegis_error(&result, AegisError::NotPendingAdmin);
}

// A chained transfer: admin -> B -> C, checking the old admin loses authority immediately at
// each step and cannot short-circuit by accepting again or acting as admin.
#[test]
fn chained_transfer_and_old_admin_loses_authority_immediately() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    setup(&mut svm, &admin);

    let b = Keypair::new_from_array([9u8; 32]);
    svm.airdrop(&b.pubkey(), 10_000_000_000).unwrap();
    set_pending_admin(&mut svm, &admin, b.pubkey()).expect("set_pending_admin to B");
    accept_admin(&mut svm, &b).expect("B accepts");

    // The old admin can no longer perform admin actions.
    let result = set_guardian(&mut svm, &admin, fixed_pubkey(50));
    assert_aegis_error(&result, AegisError::NotProtocolAdmin);

    let c = Keypair::new_from_array([10u8; 32]);
    svm.airdrop(&c.pubkey(), 10_000_000_000).unwrap();
    set_pending_admin(&mut svm, &b, c.pubkey()).expect("B sets pending admin to C");
    accept_admin(&mut svm, &c).expect("C accepts");

    let (protocol_pubkey, _) = protocol_pda();
    let after = fetch_protocol(&svm, &protocol_pubkey);
    assert_eq!(after.admin, c.pubkey());
}

// ============================================================================================
// set_guardian
// ============================================================================================

#[test]
fn set_guardian_admin_only_and_writes_only_guardian() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    setup(&mut svm, &admin);
    let (protocol_pubkey, _) = protocol_pda();
    let before = fetch_protocol(&svm, &protocol_pubkey);

    let new_guardian = fixed_pubkey(50);
    set_guardian(&mut svm, &admin, new_guardian).expect("set_guardian must succeed");

    let after = fetch_protocol(&svm, &protocol_pubkey);
    assert_eq!(after.guardian, new_guardian);
    assert_eq!(after.admin, before.admin);
    assert_eq!(after.pending_admin, before.pending_admin);
    assert_eq!(after.fee_recipient, before.fee_recipient);
    assert_eq!(
        after.paused, before.paused,
        "no implicit pause/unpause behavior"
    );
}

#[test]
fn set_guardian_rejects_non_admin() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let (guardian, _) = setup(&mut svm, &admin);
    let guardian_kp_result = set_guardian(&mut svm, &admin, guardian);
    guardian_kp_result.expect("admin itself may still call set_guardian with any valid target");

    let attacker = Keypair::new_from_array([77u8; 32]);
    svm.airdrop(&attacker.pubkey(), 10_000_000_000).unwrap();
    let result = set_guardian(&mut svm, &attacker, fixed_pubkey(51));
    assert_aegis_error(&result, AegisError::NotProtocolAdmin);
}

#[test]
fn set_guardian_rejects_default_pubkey() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    setup(&mut svm, &admin);
    let result = set_guardian(&mut svm, &admin, Pubkey::default());
    assert_aegis_error(&result, AegisError::DefaultPubkeyNotAllowed);
}

// ============================================================================================
// Authority matrix: admin / guardian / pending admin / random user, across the governance
// instructions (docs/phases/phase-12-governance.md item 32).
// ============================================================================================

#[test]
fn authority_matrix_set_pending_admin() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let (guardian_key, _) = setup(&mut svm, &admin);
    let guardian = Keypair::new_from_array([2u8; 32]);
    svm.airdrop(&guardian.pubkey(), 10_000_000_000).unwrap();
    assert_eq!(guardian.pubkey(), guardian_key);
    let random = Keypair::new_from_array([99u8; 32]);
    svm.airdrop(&random.pubkey(), 10_000_000_000).unwrap();

    assert_aegis_error(
        &set_pending_admin(&mut svm, &guardian, fixed_pubkey(60)),
        AegisError::NotProtocolAdmin,
    );
    assert_aegis_error(
        &set_pending_admin(&mut svm, &random, fixed_pubkey(60)),
        AegisError::NotProtocolAdmin,
    );
    set_pending_admin(&mut svm, &admin, fixed_pubkey(60)).expect("admin may always call it");
}

#[test]
fn authority_matrix_set_guardian_and_set_protocol_pause() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let (guardian_key, _) = setup(&mut svm, &admin);
    let guardian = Keypair::new_from_array([2u8; 32]);
    svm.airdrop(&guardian.pubkey(), 10_000_000_000).unwrap();
    assert_eq!(guardian.pubkey(), guardian_key);
    let random = Keypair::new_from_array([99u8; 32]);
    svm.airdrop(&random.pubkey(), 10_000_000_000).unwrap();

    // set_guardian: admin-only. Guardian and random user both denied.
    assert_aegis_error(
        &set_guardian(&mut svm, &guardian, fixed_pubkey(61)),
        AegisError::NotProtocolAdmin,
    );
    assert_aegis_error(
        &set_guardian(&mut svm, &random, fixed_pubkey(61)),
        AegisError::NotProtocolAdmin,
    );

    // set_protocol_pause: admin and guardian both allowed to SET; random user denied.
    set_protocol_pause(&mut svm, &admin, aegis::constants::PAUSE_SUPPLY).expect("admin may pause");
    set_protocol_pause(
        &mut svm,
        &guardian,
        aegis::constants::PAUSE_SUPPLY | aegis::constants::PAUSE_BORROW,
    )
    .expect("guardian may add bits");
    assert_aegis_error(
        &set_protocol_pause(&mut svm, &random, aegis::constants::PAUSE_SUPPLY),
        AegisError::NotAdminOrGuardian,
    );
}

// A pending admin (proposed but not yet accepted) has NO authority yet -- confirms the two-step
// design's core property directly against the admin-gated instructions.
#[test]
fn pending_admin_has_no_authority_before_accepting() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    setup(&mut svm, &admin);
    let pending = Keypair::new_from_array([9u8; 32]);
    svm.airdrop(&pending.pubkey(), 10_000_000_000).unwrap();
    set_pending_admin(&mut svm, &admin, pending.pubkey()).expect("set_pending_admin");

    // The pending admin cannot act as admin yet.
    assert_aegis_error(
        &set_guardian(&mut svm, &pending, fixed_pubkey(62)),
        AegisError::NotProtocolAdmin,
    );
    assert_aegis_error(
        &set_pending_admin(&mut svm, &pending, fixed_pubkey(63)),
        AegisError::NotProtocolAdmin,
    );
}
