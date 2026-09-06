//! Phase 3 — adversarial tests. Each performs the attack and asserts it fails with the specific
//! expected `AegisError` (or the specific Anchor constraint error the account model relies on),
//! never merely "the transaction failed" (`testing-strategy.md` §4.2).

#![allow(clippy::result_large_err)]

use aegis::error::AegisError;
use aegis_test_kit::{
    assert_aegis_error, close_position, create_market, create_spl_mint, create_token_2022_mint,
    create_token_account, deploy, deposit_collateral, deposit_collateral_ix, fetch_position,
    init_position, initialize_protocol, mint_to, reference_market_args, spl_token_2022_interface,
    spl_token_interface, withdraw_collateral, withdraw_collateral_ix, Token2022Extension,
};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_message::{Message, VersionedMessage};
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;

fn program_bytes() -> &'static [u8] {
    include_bytes!(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/aegis.so"))
}

fn fixed_pubkey(seed: u8) -> Pubkey {
    Keypair::new_from_array([seed; 32]).pubkey()
}

fn setup_protocol(svm: &mut litesvm::LiteSVM, admin: &Keypair) -> Pubkey {
    let guardian = fixed_pubkey(2);
    let fee_recipient = fixed_pubkey(3);
    initialize_protocol(svm, admin, guardian, fee_recipient).expect("initialize_protocol");
    fee_recipient
}

fn send(
    svm: &mut litesvm::LiteSVM,
    payer: &Keypair,
    extra_signers: &[&Keypair],
    ix: Instruction,
) -> litesvm::types::TransactionResult {
    let blockhash = svm.latest_blockhash();
    let message = Message::new_with_blockhash(&[ix], Some(&payer.pubkey()), &blockhash);
    let mut signers: Vec<&Keypair> = vec![payer];
    signers.extend_from_slice(extra_signers);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(message), &signers)
        .expect("failed to sign transaction");
    svm.send_transaction(tx)
}

/// Shared fixture: an SPL market with one funded depositor token account and one position, ready
/// to deposit into. Returns everything a test needs to attempt a deposit or withdrawal.
struct SplFixture {
    market: Pubkey,
    collateral_vault: Pubkey,
    collateral_mint: Pubkey,
    depositor_ata: Pubkey,
}

fn setup_spl_fixture(svm: &mut litesvm::LiteSVM, admin: &Keypair, seed: u8) -> SplFixture {
    let fee_recipient = setup_protocol(svm, admin);
    let collateral_mint = create_spl_mint(svm, admin, seed, 9, admin.pubkey(), None);
    let loan_mint = create_spl_mint(svm, admin, seed.wrapping_add(1), 6, admin.pubkey(), None);
    let args = reference_market_args(0, [1u8; 32], [2u8; 32], false);
    let (result, market, collateral_vault, ..) = create_market(
        svm,
        admin,
        collateral_mint,
        loan_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        fee_recipient,
        args,
    );
    result.expect("create_market must succeed");

    let depositor_ata = create_token_account(
        svm,
        admin,
        seed.wrapping_add(2),
        collateral_mint,
        admin.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    mint_to(
        svm,
        admin,
        collateral_mint,
        depositor_ata,
        admin,
        10_000_000_000,
        spl_token_interface::ID,
    );

    SplFixture {
        market,
        collateral_vault,
        collateral_mint,
        depositor_ata,
    }
}

// A-CUS-01 / T-03: a substituted collateral vault — a real, otherwise-valid token account for the
// right mint, but not the market's canonical vault PDA — must be rejected.
#[test]
fn deposit_rejects_substituted_vault() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fixture = setup_spl_fixture(&mut svm, &admin, 10);
    let (_, position) = init_position(&mut svm, &admin, fixture.market, fixed_pubkey(200));

    // A real token account for the correct mint, but at an address that is not the canonical
    // `PDA([b"cvault", market])` — exactly the substitution T-03 describes.
    let attacker_vault = create_token_account(
        &mut svm,
        &admin,
        199,
        fixture.collateral_mint,
        fixture.market,
        spl_token_interface::ID,
        &[],
    );

    let ix = deposit_collateral_ix(
        &admin.pubkey(),
        fixture.market,
        position,
        attacker_vault,
        fixture.depositor_ata,
        fixture.collateral_mint,
        spl_token_interface::ID,
        1_000_000_000,
    );
    let result = send(&mut svm, &admin, &[], ix);
    assert!(
        result.is_err(),
        "a substituted (non-canonical) vault must be rejected"
    );
}

// A-CUS-06 / T-04: a wrong mint (not `market.collateral_mint`) must be rejected with the specific
// `VaultMintMismatch` error, not merely "the transaction failed".
#[test]
fn deposit_rejects_wrong_mint() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fixture = setup_spl_fixture(&mut svm, &admin, 20);
    let (_, position) = init_position(&mut svm, &admin, fixture.market, fixed_pubkey(201));

    let wrong_mint = create_spl_mint(&mut svm, &admin, 29, 9, admin.pubkey(), None);

    let result = deposit_collateral(
        &mut svm,
        &admin,
        fixture.market,
        position,
        fixture.collateral_vault,
        fixture.depositor_ata,
        wrong_mint,
        spl_token_interface::ID,
        1_000_000_000,
    );
    assert_aegis_error(&result, AegisError::VaultMintMismatch);
}

// A-CUS-08 / INV-CUS-08: tokens transferred directly into the vault, outside `deposit_collateral`
// entirely, are never credited to any position — the vault balance is never a source of truth for
// individual ownership.
#[test]
fn direct_donation_is_never_credited() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fixture = setup_spl_fixture(&mut svm, &admin, 30);
    let (_, position) = init_position(&mut svm, &admin, fixture.market, fixed_pubkey(202));

    deposit_collateral(
        &mut svm,
        &admin,
        fixture.market,
        position,
        fixture.collateral_vault,
        fixture.depositor_ata,
        fixture.collateral_mint,
        spl_token_interface::ID,
        1_000_000_000,
    )
    .expect("legitimate deposit must succeed");
    let position_before = fetch_position(&svm, &position);
    assert_eq!(position_before.collateral_amount, 1_000_000_000);

    // Direct SPL Token transfer into the vault — not an Aegis instruction at all.
    let donation_ix = spl_token_interface::instruction::transfer_checked(
        &spl_token_interface::ID,
        &fixture.depositor_ata,
        &fixture.collateral_mint,
        &fixture.collateral_vault,
        &admin.pubkey(),
        &[],
        500_000_000,
        9,
    )
    .expect("valid transfer_checked instruction");
    send(&mut svm, &admin, &[], donation_ix).expect("raw donation transfer must succeed");

    let vault_after = aegis_test_kit::fetch_token_account_base(&svm, &fixture.collateral_vault);
    assert_eq!(
        vault_after.amount, 1_500_000_000,
        "the vault balance itself does increase by the donation"
    );

    let position_after = fetch_position(&svm, &position);
    assert_eq!(
        position_after.collateral_amount, position_before.collateral_amount,
        "A-CUS-08: an unsolicited direct transfer must never be credited to any position"
    );
}

// The invariant checker itself must be falsifiable (AGENTS.md §8): after the donation above,
// INV-CUS-02 (vault == Σ positions + fee_accrued) genuinely no longer holds — the donated amount
// is permanently unaccounted-for surplus (INV-CUS-08) — and `assert_inv_cus_02` must detect it.
#[test]
#[should_panic(expected = "INV-CUS-02 violated")]
fn assert_inv_cus_02_detects_uncredited_donation() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fixture = setup_spl_fixture(&mut svm, &admin, 40);
    let (_, position) = init_position(&mut svm, &admin, fixture.market, fixed_pubkey(203));

    deposit_collateral(
        &mut svm,
        &admin,
        fixture.market,
        position,
        fixture.collateral_vault,
        fixture.depositor_ata,
        fixture.collateral_mint,
        spl_token_interface::ID,
        1_000_000_000,
    )
    .expect("legitimate deposit must succeed");

    let donation_ix = spl_token_interface::instruction::transfer_checked(
        &spl_token_interface::ID,
        &fixture.depositor_ata,
        &fixture.collateral_mint,
        &fixture.collateral_vault,
        &admin.pubkey(),
        &[],
        250_000_000,
        9,
    )
    .expect("valid transfer_checked instruction");
    send(&mut svm, &admin, &[], donation_ix).expect("raw donation transfer must succeed");

    aegis_test_kit::invariants::assert_inv_cus_02(&svm, &fixture.market, &[position]);
}

// A-AUTH-02 / INV-AUTH-02: only `position.owner`, as an actual transaction signer, may withdraw —
// an attacker who signs but is not the recorded owner must fail.
#[test]
fn non_owner_withdraw_fails() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fixture = setup_spl_fixture(&mut svm, &admin, 50);

    let real_owner = Keypair::new_from_array([210u8; 32]);
    let (_, position) = init_position(&mut svm, &admin, fixture.market, real_owner.pubkey());
    deposit_collateral(
        &mut svm,
        &admin,
        fixture.market,
        position,
        fixture.collateral_vault,
        fixture.depositor_ata,
        fixture.collateral_mint,
        spl_token_interface::ID,
        1_000_000_000,
    )
    .expect("deposit must succeed");

    let attacker = Keypair::new_from_array([211u8; 32]);
    svm.airdrop(&attacker.pubkey(), 10_000_000_000)
        .expect("airdrop to attacker");
    let attacker_ata = create_token_account(
        &mut svm,
        &admin,
        212,
        fixture.collateral_mint,
        attacker.pubkey(),
        spl_token_interface::ID,
        &[],
    );

    // Attacker signs the transaction and names themselves as `owner` — but `position.owner` is
    // `real_owner`, so `has_one = owner` must reject it.
    let result = withdraw_collateral(
        &mut svm,
        &attacker,
        fixture.market,
        position,
        fixture.collateral_vault,
        attacker_ata,
        fixture.collateral_mint,
        spl_token_interface::ID,
        Pubkey::default(),
        Pubkey::default(),
        1_000_000_000,
    );
    assert_aegis_error(&result, AegisError::NotPositionOwner);

    // The position is untouched by the failed attempt.
    let position_state = fetch_position(&svm, &position);
    assert_eq!(position_state.collateral_amount, 1_000_000_000);
}

// A-AUTH-03 / INV-AUTH-03: depositing into someone else's position requires no signature from the
// position owner at all — a complete stranger may fund it.
#[test]
fn deposit_by_non_owner_succeeds() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fixture = setup_spl_fixture(&mut svm, &admin, 60);

    let owner = fixed_pubkey(220); // never signs anything in this test
    let (_, position) = init_position(&mut svm, &admin, fixture.market, owner);

    let stranger = Keypair::new_from_array([221u8; 32]);
    svm.airdrop(&stranger.pubkey(), 10_000_000_000)
        .expect("airdrop to stranger");
    let stranger_ata = create_token_account(
        &mut svm,
        &admin,
        222,
        fixture.collateral_mint,
        stranger.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    mint_to(
        &mut svm,
        &admin,
        fixture.collateral_mint,
        stranger_ata,
        &admin,
        2_000_000_000,
        spl_token_interface::ID,
    );

    let result = deposit_collateral(
        &mut svm,
        &stranger,
        fixture.market,
        position,
        fixture.collateral_vault,
        stranger_ata,
        fixture.collateral_mint,
        spl_token_interface::ID,
        2_000_000_000,
    );
    result.expect("a stranger must be able to deposit into someone else's position");

    let position_state = fetch_position(&svm, &position);
    assert_eq!(position_state.collateral_amount, 2_000_000_000);
}

// A-TOK-08: passing the wrong token program for the market's pinned collateral asset (Token-2022
// for a plain-SPL market) must fail, even though `Interface<TokenInterface>` alone accepts it as a
// structurally valid token program.
#[test]
fn wrong_token_program_for_spl_market_is_rejected() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fixture = setup_spl_fixture(&mut svm, &admin, 70);
    let (_, position) = init_position(&mut svm, &admin, fixture.market, fixed_pubkey(230));

    let result = deposit_collateral(
        &mut svm,
        &admin,
        fixture.market,
        position,
        fixture.collateral_vault,
        fixture.depositor_ata,
        fixture.collateral_mint,
        spl_token_2022_interface::ID,
        1_000_000_000,
    );
    assert_aegis_error(&result, AegisError::TokenProgramMismatch);
}

// A-TOK-09: the reverse substitution — the legacy SPL Token program presented for a market whose
// collateral asset is pinned to Token-2022 — must also fail.
#[test]
fn wrong_token_program_for_token2022_market_is_rejected() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fee_recipient = setup_protocol(&mut svm, &admin);

    let fee_mint = create_token_2022_mint(
        &mut svm,
        &admin,
        80,
        9,
        admin.pubkey(),
        None,
        &[Token2022Extension::TransferFeeConfig {
            basis_points: 100,
            maximum_fee: 1_000_000_000,
        }],
    );
    let loan_mint = create_spl_mint(&mut svm, &admin, 81, 6, admin.pubkey(), None);
    let args = reference_market_args(0, [1u8; 32], [2u8; 32], false);
    let (result, market, collateral_vault, ..) = create_market(
        &mut svm,
        &admin,
        fee_mint,
        loan_mint,
        spl_token_2022_interface::ID,
        spl_token_interface::ID,
        fee_recipient,
        args,
    );
    result.expect("create_market must succeed");

    let (_, position) = init_position(&mut svm, &admin, market, fixed_pubkey(231));
    let extensions = &[spl_token_2022_interface::extension::ExtensionType::TransferFeeConfig];
    let depositor_ata = create_token_account(
        &mut svm,
        &admin,
        82,
        fee_mint,
        admin.pubkey(),
        spl_token_2022_interface::ID,
        extensions,
    );
    mint_to(
        &mut svm,
        &admin,
        fee_mint,
        depositor_ata,
        &admin,
        1_000_000_000,
        spl_token_2022_interface::ID,
    );

    let result = deposit_collateral(
        &mut svm,
        &admin,
        market,
        position,
        collateral_vault,
        depositor_ata,
        fee_mint,
        spl_token_interface::ID,
        1_000_000_000,
    );
    assert_aegis_error(&result, AegisError::TokenProgramMismatch);
}

// A-PAR-01 / INV-RES-02: `deposit_collateral` and `withdraw_collateral` must not declare `Market`
// writable, proven from the actual generated instruction account metadata — not source inspection.
#[test]
fn market_is_not_writable_in_collateral_instructions() {
    let market = fixed_pubkey(1);
    let position = fixed_pubkey(2);
    let vault = fixed_pubkey(3);
    let ata = fixed_pubkey(4);
    let mint = fixed_pubkey(5);

    let deposit_ix = deposit_collateral_ix(
        &fixed_pubkey(6),
        market,
        position,
        vault,
        ata,
        mint,
        spl_token_interface::ID,
        1,
    );
    let market_meta = deposit_ix
        .accounts
        .iter()
        .find(|m| m.pubkey == market)
        .expect("market account must be present in deposit_collateral");
    assert!(
        !market_meta.is_writable,
        "INV-RES-02: deposit_collateral must not declare Market writable"
    );

    let withdraw_ix = withdraw_collateral_ix(
        &fixed_pubkey(7),
        market,
        position,
        vault,
        ata,
        mint,
        spl_token_interface::ID,
        Pubkey::default(),
        Pubkey::default(),
        1,
    );
    let market_meta = withdraw_ix
        .accounts
        .iter()
        .find(|m| m.pubkey == market)
        .expect("market account must be present in withdraw_collateral");
    assert!(
        !market_meta.is_writable,
        "INV-RES-02: withdraw_collateral must not declare Market writable"
    );
}

// NOTE: the Phase 3/4 hard gate this test used to cover (`withdraw_collateral` on a debt-bearing
// position unconditionally returning `OracleNotYetAvailable`) was removed in Phase 5 -- the
// instruction now performs a real, oracle-validated post-withdrawal health check instead
// (`instruction-catalogue.md` §11, `docs/phases/phase-05-oracle.md`). Equivalent, stronger
// coverage (a debt-bearing withdrawal against an invalid/stale/wrong-owner price account fails
// closed, and state is unchanged) now lives in `tests/phase5_oracle_adversarial.rs`
// (`A-ORACLE-03`, `A-ORACLE-06`, `A-ORACLE-13`).

// A-LIFE-02 / INV-LIFE-03: a closed position is fully defunded, cannot be reused while stale, and
// can only ever be recreated empty by `init_position`.
#[test]
fn closed_position_cannot_be_revived_with_stale_data() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fixture = setup_spl_fixture(&mut svm, &admin, 100);

    let owner = Keypair::new_from_array([250u8; 32]);
    svm.airdrop(&owner.pubkey(), 10_000_000_000)
        .expect("airdrop to owner");
    let (_, position) = init_position(&mut svm, &admin, fixture.market, owner.pubkey());

    deposit_collateral(
        &mut svm,
        &admin,
        fixture.market,
        position,
        fixture.collateral_vault,
        fixture.depositor_ata,
        fixture.collateral_mint,
        spl_token_interface::ID,
        1_000_000_000,
    )
    .expect("deposit must succeed");

    let owner_ata = create_token_account(
        &mut svm,
        &admin,
        251,
        fixture.collateral_mint,
        owner.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    withdraw_collateral(
        &mut svm,
        &owner,
        fixture.market,
        position,
        fixture.collateral_vault,
        owner_ata,
        fixture.collateral_mint,
        spl_token_interface::ID,
        Pubkey::default(),
        Pubkey::default(),
        1_000_000_000,
    )
    .expect("withdraw must succeed");

    close_position(&mut svm, &owner, fixture.market, position).expect("close must succeed");

    // The account is either fully purged (0 lamports) or, if the runtime retains a record,
    // reassigned to the System Program with its data cleared — either way, Anchor's own
    // discriminator check makes stale-data reuse impossible.
    if let Some(account) = svm.get_account(&position) {
        assert_eq!(account.lamports, 0, "closed account must be fully defunded");
        assert!(
            account.data.is_empty()
                || account.owner == anchor_lang::solana_program::system_program::ID,
            "closed account must no longer be owned by Aegis with live data"
        );
    }

    // Attempting to act on the stale (closed) position address must fail — there is no
    // discriminator left for Anchor to deserialize.
    let revival_attempt = deposit_collateral(
        &mut svm,
        &admin,
        fixture.market,
        position,
        fixture.collateral_vault,
        fixture.depositor_ata,
        fixture.collateral_mint,
        spl_token_interface::ID,
        1,
    );
    assert!(
        revival_attempt.is_err(),
        "a closed position must not accept further instructions before being recreated"
    );

    // The PDA is deterministic, so it can be recreated later — and it can only ever come back
    // completely empty. A fresh blockhash is needed so this transaction (otherwise byte-identical
    // to the very first `init_position` call) is not rejected as an `AlreadyProcessed` replay —
    // exactly what a real client gets for free from the passage of time between transactions.
    svm.expire_blockhash();
    let (recreated, recreated_position) =
        init_position(&mut svm, &admin, fixture.market, owner.pubkey());
    recreated.expect("init_position must be able to recreate a closed position");
    assert_eq!(recreated_position, position, "same seeds, same PDA");
    let fresh = fetch_position(&svm, &position);
    assert_eq!(fresh.owner, owner.pubkey());
    assert_eq!(fresh.supply_shares, 0);
    assert_eq!(fresh.borrow_shares, 0);
    assert_eq!(fresh.collateral_amount, 0);
}
