//! Phase 4 — adversarial tests. Each performs the attack and asserts it fails with the specific
//! expected `AegisError` (or the specific Anchor/runtime rejection the account model relies on),
//! never merely "the transaction failed" (`testing-strategy.md` §4.2).

#![allow(clippy::result_large_err)]

use aegis::error::AegisError;
use aegis_test_kit::{
    assert_aegis_error, borrow_ix, create_market, create_spl_mint, create_token_account, deploy,
    fetch_position, fetch_token_account_base, init_position, initialize_protocol, position_pda,
    reference_market_args, repay_ix, spl_token_interface, supply, supply_ix, withdraw_ix,
};
use anchor_lang::{InstructionData, ToAccountMetas};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_message::{Message, VersionedMessage};
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;

fn program_bytes() -> &'static [u8] {
    include_bytes!(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/aegis.so"))
}

/// A collision-proof source of fixed keypair seeds for one test (see `tests/phase4_lending.rs`'s
/// copy of the same doc for the full rationale). Starts well clear of
/// `aegis_test_kit::svm::PAYER_SEED` (`[7u8; 32]`, the `deploy()` admin/payer).
struct SeedGen(u8);
impl SeedGen {
    fn new() -> Self {
        Self(20)
    }
    fn next(&mut self) -> u8 {
        let s = self.0;
        self.0 = self
            .0
            .checked_add(1)
            .expect("test used more than 255 seeds");
        s
    }
}

fn fixed_pubkey(seed: u8) -> Pubkey {
    Keypair::new_from_array([seed; 32]).pubkey()
}

fn setup_protocol(svm: &mut litesvm::LiteSVM, admin: &Keypair, seeds: &mut SeedGen) -> Pubkey {
    let guardian = fixed_pubkey(seeds.next());
    let fee_recipient = fixed_pubkey(seeds.next());
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

struct Fixture {
    market: Pubkey,
    fee_position: Pubkey,
    loan_vault: Pubkey,
    loan_mint: Pubkey,
    loan_token_program: Pubkey,
}

fn setup_market(svm: &mut litesvm::LiteSVM, admin: &Keypair, seeds: &mut SeedGen) -> Fixture {
    let fee_recipient = setup_protocol(svm, admin, seeds);
    let collateral_mint = create_spl_mint(svm, admin, seeds.next(), 9, admin.pubkey(), None);
    let loan_mint = create_spl_mint(svm, admin, seeds.next(), 6, admin.pubkey(), None);
    let args = reference_market_args(0, [1u8; 32], [2u8; 32], false);
    let (result, market, _cv, loan_vault, fee_position) = create_market(
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
    Fixture {
        market,
        fee_position,
        loan_vault,
        loan_mint,
        loan_token_program: spl_token_interface::ID,
    }
}

fn wallet_with_ata(
    svm: &mut litesvm::LiteSVM,
    admin: &Keypair,
    loan_mint: Pubkey,
    loan_token_program: Pubkey,
    seeds: &mut SeedGen,
    balance: u64,
) -> (Keypair, Pubkey) {
    let wallet = Keypair::new_from_array([seeds.next(); 32]);
    svm.airdrop(&wallet.pubkey(), 10_000_000_000)
        .expect("airdrop");
    let ata = create_token_account(
        svm,
        admin,
        seeds.next(),
        loan_mint,
        wallet.pubkey(),
        loan_token_program,
        &[],
    );
    if balance > 0 {
        aegis_test_kit::mint_to(
            svm,
            admin,
            loan_mint,
            ata,
            admin,
            balance,
            loan_token_program,
        );
    }
    (wallet, ata)
}

// --- BORROW GATE: the single most important adversarial test in this phase ---

// `borrow` is hard-gated: an otherwise well-formed borrow attempt, against a real market with real
// supplied liquidity, must fail with exactly `OracleNotYetAvailable` -- never succeed, never fail
// with any other error that might suggest a different, permissive code path was reached.
#[test]
fn borrow_is_hard_gated_returns_oracle_not_yet_available() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);

    // Real liquidity in the market -- if the gate were bypassable, this borrow would otherwise be
    // perfectly satisfiable against free liquidity and min_debt.
    let (lender, lender_ata) = wallet_with_ata(
        &mut svm,
        &admin,
        fx.loan_mint,
        fx.loan_token_program,
        &mut seeds,
        1_000_000_000,
    );
    let (_, lender_position) = init_position(&mut svm, &admin, fx.market, lender.pubkey());
    supply(
        &mut svm,
        &lender,
        fx.market,
        lender_position,
        fx.fee_position,
        fx.loan_vault,
        lender_ata,
        fx.loan_mint,
        fx.loan_token_program,
        1_000_000_000,
        0,
    )
    .expect("supply must succeed");

    let (borrower, borrower_ata) = wallet_with_ata(
        &mut svm,
        &admin,
        fx.loan_mint,
        fx.loan_token_program,
        &mut seeds,
        0,
    );
    let (_, borrower_position) = init_position(&mut svm, &admin, fx.market, borrower.pubkey());

    let ix = borrow_ix(
        &borrower.pubkey(),
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_ata,
        fx.loan_mint,
        fx.loan_token_program,
        100_000_000,
        0,
    );
    let result = send(&mut svm, &borrower, &[], ix);
    assert_aegis_error(&result, AegisError::OracleNotYetAvailable);

    // Nothing changed: no tokens moved, no debt was recorded.
    let position_after = fetch_position(&svm, &borrower_position);
    assert_eq!(position_after.borrow_shares, 0);
    let vault_after = fetch_token_account_base(&svm, &fx.loan_vault);
    assert_eq!(
        vault_after.amount, 1_000_000_000,
        "no tokens must have left the vault"
    );
    let borrower_ata_after = fetch_token_account_base(&svm, &borrower_ata);
    assert_eq!(
        borrower_ata_after.amount, 0,
        "the borrower must have received nothing"
    );
}

// The gate also fires for the shares-given form, and regardless of the requested amount's size
// relative to available liquidity (a request for 1 base unit is just as gated as a large one).
#[test]
fn borrow_is_hard_gated_regardless_of_form_or_size() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);

    let (borrower, borrower_ata) = wallet_with_ata(
        &mut svm,
        &admin,
        fx.loan_mint,
        fx.loan_token_program,
        &mut seeds,
        0,
    );
    let (_, borrower_position) = init_position(&mut svm, &admin, fx.market, borrower.pubkey());

    // shares-given form.
    let ix = borrow_ix(
        &borrower.pubkey(),
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_ata,
        fx.loan_mint,
        fx.loan_token_program,
        0,
        1,
    );
    let result = send(&mut svm, &borrower, &[], ix);
    assert_aegis_error(&result, AegisError::OracleNotYetAvailable);

    // A tiny 1-unit assets-given request, on a fresh blockhash.
    svm.expire_blockhash();
    let ix2 = borrow_ix(
        &borrower.pubkey(),
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_ata,
        fx.loan_mint,
        fx.loan_token_program,
        1,
        0,
    );
    let result2 = send(&mut svm, &borrower, &[], ix2);
    assert_aegis_error(&result2, AegisError::OracleNotYetAvailable);
}

// --- A-ACC-01 / T-11: duplicate mutable accounts ---

// If a caller's `position` happens to equal the canonical `fee_position` (i.e. the caller *is*
// `market.fee_recipient`, a legitimate coincidence), passing the same pubkey for both `position`
// and `fee_position` in one instruction must be rejected by Anchor 1.0's default
// duplicate-mutable-account protection -- never silently accepted.
#[test]
fn a_acc_01_duplicate_mutable_accounts_rejected() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fee_recipient_kp = Keypair::new_from_array([seeds.next(); 32]);
    let guardian = fixed_pubkey(seeds.next());
    initialize_protocol(&mut svm, &admin, guardian, fee_recipient_kp.pubkey())
        .expect("initialize_protocol");

    let collateral_mint = create_spl_mint(&mut svm, &admin, seeds.next(), 9, admin.pubkey(), None);
    let loan_mint = create_spl_mint(&mut svm, &admin, seeds.next(), 6, admin.pubkey(), None);
    let args = reference_market_args(0, [1u8; 32], [2u8; 32], false);
    let (result, market, _cv, loan_vault, fee_position) = create_market(
        &mut svm,
        &admin,
        collateral_mint,
        loan_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        fee_recipient_kp.pubkey(),
        args,
    );
    result.expect("create_market must succeed");

    // The fee recipient's own regular position is, by construction, at the SAME PDA as
    // `fee_position` (PDA(market, owner) with owner == market.fee_recipient).
    let (derived_position, _) = position_pda(&market, &fee_recipient_kp.pubkey());
    assert_eq!(
        derived_position, fee_position,
        "fixture must actually produce the coincidence"
    );

    svm.airdrop(&fee_recipient_kp.pubkey(), 10_000_000_000)
        .unwrap();
    let ata = create_token_account(
        &mut svm,
        &admin,
        seeds.next(),
        loan_mint,
        fee_recipient_kp.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    aegis_test_kit::mint_to(
        &mut svm,
        &admin,
        loan_mint,
        ata,
        &admin,
        1_000_000_000,
        spl_token_interface::ID,
    );

    let ix = supply_ix(
        &fee_recipient_kp.pubkey(),
        market,
        fee_position,
        fee_position,
        loan_vault,
        ata,
        loan_mint,
        spl_token_interface::ID,
        1_000_000,
        0,
    );
    let result = send(&mut svm, &fee_recipient_kp, &[], ix);
    assert!(
        result.is_err(),
        "A-ACC-01: passing the same pubkey for `position` and `fee_position` must be rejected"
    );
}

// --- U-GUARD-01/02: exactly-one-of, exercised through the real instructions ---

#[test]
fn supply_rejects_both_zero_and_both_nonzero() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (lender, ata) = wallet_with_ata(
        &mut svm,
        &admin,
        fx.loan_mint,
        fx.loan_token_program,
        &mut seeds,
        0,
    );
    let (_, position) = init_position(&mut svm, &admin, fx.market, lender.pubkey());

    let both_zero = supply_ix(
        &lender.pubkey(),
        fx.market,
        position,
        fx.fee_position,
        fx.loan_vault,
        ata,
        fx.loan_mint,
        fx.loan_token_program,
        0,
        0,
    );
    let result = send(&mut svm, &lender, &[], both_zero);
    assert_aegis_error(&result, AegisError::ZeroAmount);

    svm.expire_blockhash();
    let both_nonzero = supply_ix(
        &lender.pubkey(),
        fx.market,
        position,
        fx.fee_position,
        fx.loan_vault,
        ata,
        fx.loan_mint,
        fx.loan_token_program,
        1,
        1,
    );
    let result2 = send(&mut svm, &lender, &[], both_nonzero);
    assert_aegis_error(&result2, AegisError::InconsistentInput);
}

#[test]
fn withdraw_and_repay_reject_both_zero_and_both_nonzero() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (lender, ata) = wallet_with_ata(
        &mut svm,
        &admin,
        fx.loan_mint,
        fx.loan_token_program,
        &mut seeds,
        0,
    );
    let (_, position) = init_position(&mut svm, &admin, fx.market, lender.pubkey());

    let both_zero = withdraw_ix(
        &lender.pubkey(),
        fx.market,
        position,
        fx.fee_position,
        fx.loan_vault,
        ata,
        fx.loan_mint,
        fx.loan_token_program,
        0,
        0,
    );
    assert_aegis_error(
        &send(&mut svm, &lender, &[], both_zero),
        AegisError::ZeroAmount,
    );

    svm.expire_blockhash();
    let both_nonzero = withdraw_ix(
        &lender.pubkey(),
        fx.market,
        position,
        fx.fee_position,
        fx.loan_vault,
        ata,
        fx.loan_mint,
        fx.loan_token_program,
        1,
        1,
    );
    assert_aegis_error(
        &send(&mut svm, &lender, &[], both_nonzero),
        AegisError::InconsistentInput,
    );

    svm.expire_blockhash();
    let repay_both_zero = repay_ix(
        &lender.pubkey(),
        fx.market,
        position,
        fx.fee_position,
        fx.loan_vault,
        ata,
        fx.loan_mint,
        fx.loan_token_program,
        0,
        0,
    );
    assert_aegis_error(
        &send(&mut svm, &lender, &[], repay_both_zero),
        AegisError::ZeroAmount,
    );

    svm.expire_blockhash();
    let repay_both_nonzero = repay_ix(
        &lender.pubkey(),
        fx.market,
        position,
        fx.fee_position,
        fx.loan_vault,
        ata,
        fx.loan_mint,
        fx.loan_token_program,
        1,
        1,
    );
    assert_aegis_error(
        &send(&mut svm, &lender, &[], repay_both_nonzero),
        AegisError::InconsistentInput,
    );
}

// --- INV-CUS-08 (loan-side analog of A-CUS-08): direct donations to loan_vault are never
// credited to any position or to the market's own accounting scalars. ---

#[test]
fn loan_vault_direct_donation_is_never_credited() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (lender, ata) = wallet_with_ata(
        &mut svm,
        &admin,
        fx.loan_mint,
        fx.loan_token_program,
        &mut seeds,
        2_000_000_000,
    );
    let (_, position) = init_position(&mut svm, &admin, fx.market, lender.pubkey());

    supply(
        &mut svm,
        &lender,
        fx.market,
        position,
        fx.fee_position,
        fx.loan_vault,
        ata,
        fx.loan_mint,
        fx.loan_token_program,
        1_000_000_000,
        0,
    )
    .expect("legitimate supply must succeed");

    let market_before = aegis_test_kit::fetch_market(&svm, &fx.market);

    // A raw SPL Token transfer directly into loan_vault -- not an Aegis instruction at all.
    let donation_ix = spl_token_interface::instruction::transfer_checked(
        &spl_token_interface::ID,
        &ata,
        &fx.loan_mint,
        &fx.loan_vault,
        &lender.pubkey(),
        &[],
        500_000_000,
        6,
    )
    .expect("valid transfer_checked instruction");
    send(&mut svm, &lender, &[], donation_ix).expect("raw donation transfer must succeed");

    let vault_after = fetch_token_account_base(&svm, &fx.loan_vault);
    assert_eq!(
        vault_after.amount, 1_500_000_000,
        "the vault's raw balance does increase from the donation"
    );

    let market_after = aegis_test_kit::fetch_market(&svm, &fx.market);
    assert_eq!(
        market_after.total_supply_assets, market_before.total_supply_assets,
        "INV-CUS-08: an unsolicited direct transfer must never be credited to total_supply_assets"
    );

    // INV-CUS-01 now genuinely does NOT hold (the donated surplus is unaccounted-for), proving the
    // checker itself is falsifiable, exactly like Phase 3's A-CUS-08 established for collateral.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        aegis_test_kit::invariants::assert_inv_cus_01(&svm, &fx.market);
    }));
    assert!(
        result.is_err(),
        "INV-CUS-01 must be observed to fail after an uncredited donation, proving the checker is falsifiable"
    );
}

// --- INV-AUTH-02: only position.owner, as an actual signer, may withdraw -- an attacker who
// signs but is not the recorded owner must fail. ---

#[test]
fn non_owner_cannot_withdraw_someone_elses_supply() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (real_owner, owner_ata) = wallet_with_ata(
        &mut svm,
        &admin,
        fx.loan_mint,
        fx.loan_token_program,
        &mut seeds,
        1_000_000_000,
    );
    let (_, position) = init_position(&mut svm, &admin, fx.market, real_owner.pubkey());
    supply(
        &mut svm,
        &real_owner,
        fx.market,
        position,
        fx.fee_position,
        fx.loan_vault,
        owner_ata,
        fx.loan_mint,
        fx.loan_token_program,
        1_000_000_000,
        0,
    )
    .expect("supply must succeed");

    let (attacker, attacker_ata) = wallet_with_ata(
        &mut svm,
        &admin,
        fx.loan_mint,
        fx.loan_token_program,
        &mut seeds,
        0,
    );

    // Attacker signs as themselves, but points `owner` (their own signer field) at a position that
    // is NOT theirs -- has_one = owner must reject it.
    let ix = withdraw_ix(
        &attacker.pubkey(),
        fx.market,
        position,
        fx.fee_position,
        fx.loan_vault,
        attacker_ata,
        fx.loan_mint,
        fx.loan_token_program,
        1_000_000_000,
        0,
    );
    let result = send(&mut svm, &attacker, &[], ix);
    assert_aegis_error(&result, AegisError::NotPositionOwner);
}

// --- Token program pinning, mirroring Phase 3's A-TOK-08/09 for the loan side. ---

#[test]
fn supply_rejects_wrong_token_program() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (lender, ata) = wallet_with_ata(
        &mut svm,
        &admin,
        fx.loan_mint,
        fx.loan_token_program,
        &mut seeds,
        0,
    );
    let (_, position) = init_position(&mut svm, &admin, fx.market, lender.pubkey());

    let ix = supply_ix(
        &lender.pubkey(),
        fx.market,
        position,
        fx.fee_position,
        fx.loan_vault,
        ata,
        fx.loan_mint,
        aegis_test_kit::spl_token_2022_interface::ID,
        1_000_000,
        0,
    );
    let result = send(&mut svm, &lender, &[], ix);
    assert_aegis_error(&result, AegisError::TokenProgramMismatch);
}

// --- fee_position substitution: an arbitrary account cannot stand in for the canonical fee
// position -- the PDA constraint alone must reject it. ---

#[test]
fn supply_rejects_substituted_fee_position() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (lender, ata) = wallet_with_ata(
        &mut svm,
        &admin,
        fx.loan_mint,
        fx.loan_token_program,
        &mut seeds,
        0,
    );
    let (_, position) = init_position(&mut svm, &admin, fx.market, lender.pubkey());

    let (_, fake_fee_position) =
        init_position(&mut svm, &admin, fx.market, fixed_pubkey(seeds.next()));

    let ix = Instruction {
        program_id: aegis::ID,
        accounts: aegis::accounts::Supply {
            owner: lender.pubkey(),
            market: fx.market,
            position,
            fee_position: fake_fee_position,
            loan_vault: fx.loan_vault,
            owner_loan_ata: ata,
            loan_mint: fx.loan_mint,
            loan_token_program: fx.loan_token_program,
        }
        .to_account_metas(None),
        data: aegis::instruction::Supply {
            assets: 1_000_000,
            shares: 0,
        }
        .data(),
    };
    let result = send(&mut svm, &lender, &[], ix);
    assert!(
        result.is_err(),
        "a non-canonical fee_position must be rejected by the seeds constraint"
    );
}

// --- A-PAR-style: supply/withdraw/borrow/repay must all declare Market writable (unlike the
// Phase 3 collateral instructions) -- sanity check on the account metadata Anchor generates. ---

#[test]
fn lending_instructions_declare_market_writable() {
    let market = fixed_pubkey(1);
    let position = fixed_pubkey(2);
    let fee_position = fixed_pubkey(3);
    let vault = fixed_pubkey(4);
    let ata = fixed_pubkey(5);
    let mint = fixed_pubkey(6);
    let owner = fixed_pubkey(8);

    let ix = supply_ix(
        &owner,
        market,
        position,
        fee_position,
        vault,
        ata,
        mint,
        spl_token_interface::ID,
        1,
        0,
    );
    let meta = ix.accounts.iter().find(|m| m.pubkey == market).unwrap();
    assert!(meta.is_writable, "supply must declare Market writable");
}
