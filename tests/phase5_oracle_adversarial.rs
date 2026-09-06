//! Phase 5 — oracle adversarial tests (`docs/oracle-design.md`, `docs/phases/phase-05-oracle.md`).
//!
//! Every check O-1..O-11 individually violated, the two mandatory positive safety tests
//! (`A-ORACLE-01/02`), the state-atomicity test (`A-ORACLE-13`), and the boundary cases. Each test
//! asserts a **specific** `AegisError` (or, for account-level rejections Anchor's own account
//! loading performs, a specific structural failure) and, where the check requires it, that **no
//! state changed** — never merely "the transaction failed" (`testing-strategy.md` §4.2).
//!
//! All prices are deterministic `PriceUpdateV2` fixtures constructed with the real
//! `pyth-solana-receiver-sdk` (`aegis_test_kit::pyth_fixture`) and injected directly via
//! `LiteSVM::set_account` — no Hermes, no RPC, no Pyth program deployment (ADR-0008).

#![allow(clippy::result_large_err)]

use aegis::error::AegisError;
use aegis_test_kit::{
    assert_aegis_error, borrow, borrow_ix, create_market, create_spl_mint, create_token_account,
    deploy, deposit_collateral, fetch_market, fetch_position, init_position, initialize_protocol,
    inject_price_update, invariants, mint_to, pyth_solana_receiver_sdk, reference_market_args,
    repay, set_price, spl_token_interface, supply, withdraw_collateral, PriceFixture,
};
use pyth_solana_receiver_sdk::price_update::VerificationLevel;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_message::{Message, VersionedMessage};
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;

fn program_bytes() -> &'static [u8] {
    include_bytes!(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/aegis.so"))
}

const COLLATERAL_FEED_ID: [u8; 32] = [0xAAu8; 32];
const LOAN_FEED_ID: [u8; 32] = [0xBBu8; 32];
const MAX_PRICE_AGE_SECS: i64 = 60; // reference_market_args' own value
const MAX_CONF_BPS: u128 = 100; // reference_market_args' own value

/// A collision-proof source of fixed keypair/pubkey seeds for one test.
struct SeedGen(u8);
impl SeedGen {
    fn new() -> Self {
        Self(20)
    }
    fn next(&mut self) -> u8 {
        let s = self.0;
        self.0 = self.0.checked_add(1).expect("used more than 255 seeds");
        s
    }
}

fn fixed_pubkey(seed: u8) -> Pubkey {
    Keypair::new_from_array([seed; 32]).pubkey()
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

fn now(svm: &litesvm::LiteSVM) -> i64 {
    svm.get_sysvar::<solana_clock::Clock>().unix_timestamp
}

fn warp_seconds(svm: &mut litesvm::LiteSVM, dt: i64) {
    let mut clock = svm.get_sysvar::<solana_clock::Clock>();
    clock.unix_timestamp += dt;
    svm.set_sysvar(&clock);
}

/// SOL (9dp) collateral / USDC (6dp) loan market with the reference risk/IRM/oracle parameters.
/// SOL priced at exactly $150.00, USDC at exactly $1.00, both at `expo = -8` and zero confidence
/// so every downstream WAD value in these tests is a clean round number.
struct Fixture {
    market: Pubkey,
    fee_position: Pubkey,
    collateral_vault: Pubkey,
    loan_vault: Pubkey,
    collateral_mint: Pubkey,
    loan_mint: Pubkey,
}

fn setup_market(svm: &mut litesvm::LiteSVM, admin: &Keypair, seeds: &mut SeedGen) -> Fixture {
    let guardian = fixed_pubkey(seeds.next());
    let fee_recipient = fixed_pubkey(seeds.next());
    initialize_protocol(svm, admin, guardian, fee_recipient).expect("initialize_protocol");

    let collateral_mint = create_spl_mint(svm, admin, seeds.next(), 9, admin.pubkey(), None);
    let loan_mint = create_spl_mint(svm, admin, seeds.next(), 6, admin.pubkey(), None);
    let args = reference_market_args(0, COLLATERAL_FEED_ID, LOAN_FEED_ID, false);
    let (result, market, collateral_vault, loan_vault, fee_position) = create_market(
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
        collateral_vault,
        loan_vault,
        collateral_mint,
        loan_mint,
    }
}

fn wallet_with_ata(
    svm: &mut litesvm::LiteSVM,
    admin: &Keypair,
    mint: Pubkey,
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
        mint,
        wallet.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    if balance > 0 {
        mint_to(
            svm,
            admin,
            mint,
            ata,
            admin,
            balance,
            spl_token_interface::ID,
        );
    }
    (wallet, ata)
}

/// Injects a valid $150.00 SOL / $1.00 USDC price pair, published `at`, and returns their pubkeys.
fn valid_prices(svm: &mut litesvm::LiteSVM, seeds: &mut SeedGen, at: i64) -> (Pubkey, Pubkey) {
    let c = set_price(
        svm,
        seeds.next(),
        PriceFixture::valid(COLLATERAL_FEED_ID, 15_000_000_000, 0, -8, at),
    );
    let l = set_price(
        svm,
        seeds.next(),
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 0, -8, at),
    );
    (c, l)
}

/// Lender supplies `supply_amount`, borrower deposits `collateral_amount`, and a position is
/// ready to borrow against. Returns (borrower keypair, borrower position, borrower loan ATA).
#[allow(clippy::too_many_arguments)]
fn setup_borrower(
    svm: &mut litesvm::LiteSVM,
    admin: &Keypair,
    fx: &Fixture,
    seeds: &mut SeedGen,
    supply_amount: u64,
    collateral_amount: u64,
) -> (Keypair, Pubkey, Pubkey) {
    let (lender, lender_ata) = wallet_with_ata(svm, admin, fx.loan_mint, seeds, supply_amount);
    let (_, lender_position) = init_position(svm, admin, fx.market, lender.pubkey());
    supply(
        svm,
        &lender,
        fx.market,
        lender_position,
        fx.fee_position,
        fx.loan_vault,
        lender_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        supply_amount,
        0,
    )
    .expect("supply must succeed");

    let (borrower, borrower_collateral_ata) =
        wallet_with_ata(svm, admin, fx.collateral_mint, seeds, collateral_amount);
    let (_, borrower_position) = init_position(svm, admin, fx.market, borrower.pubkey());
    deposit_collateral(
        svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.collateral_vault,
        borrower_collateral_ata,
        fx.collateral_mint,
        spl_token_interface::ID,
        collateral_amount,
    )
    .expect("deposit_collateral must succeed");

    let borrower_loan_ata = create_token_account(
        svm,
        admin,
        seeds.next(),
        fx.loan_mint,
        borrower.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    (borrower, borrower_position, borrower_loan_ata)
}

// ============================================================================================
// A-ORACLE-01 / A-ORACLE-02: risk-reducing operations succeed under a maximally broken oracle.
// ============================================================================================

// A-ORACLE-01 / INV-ORA-02 / E-10: deposit_collateral succeeds with a maximally broken oracle.
// `deposit_collateral`'s account list contains no price-update field at all -- there is
// structurally nothing a caller could break. This test still exercises a genuinely adversarial
// environment (a market whose configured feeds have no valid price posted anywhere in this
// LiteSVM instance at all -- not merely omitted from this call) to prove the operation is truly
// independent of oracle state, not merely untested against it.
#[test]
fn a_oracle_01_deposit_collateral_succeeds_with_broken_oracle() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    // No price account is ever injected into this LiteSVM instance for either feed -- the
    // market's configured feed IDs correspond to nothing postable.

    let (depositor, depositor_ata) = wallet_with_ata(
        &mut svm,
        &admin,
        fx.collateral_mint,
        &mut seeds,
        5_000_000_000,
    );
    let (_, position) = init_position(&mut svm, &admin, fx.market, depositor.pubkey());

    let result = deposit_collateral(
        &mut svm,
        &depositor,
        fx.market,
        position,
        fx.collateral_vault,
        depositor_ata,
        fx.collateral_mint,
        spl_token_interface::ID,
        5_000_000_000,
    );
    result.expect("A-ORACLE-01: deposit_collateral must succeed with no oracle available at all");

    let position_state = fetch_position(&svm, &position);
    assert_eq!(position_state.collateral_amount, 5_000_000_000);
}

// A-ORACLE-02 / INV-ORA-02 / INV-REP-01 / E-11: repay succeeds with a maximally broken oracle.
// Real debt is established via real `borrow` under a valid price (Phase 4's state-injection
// technique is no longer needed now that borrow is real), the price is then made maximally
// broken (stale AND wrong-owner AND wrong-feed simultaneously is unnecessary -- staleness alone
// is representative of an outage, per oracle-design.md §8's own framing), and `repay` is proven
// to succeed regardless -- `repay`'s account list contains no price-update field at all.
#[test]
fn a_oracle_02_repay_succeeds_with_broken_oracle() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (borrower, borrower_position, borrower_loan_ata) = setup_borrower(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        10_000_000_000,
    );

    let n = now(&svm);
    let (c, l) = valid_prices(&mut svm, &mut seeds, n);
    borrow(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        c,
        l,
        900_000_000, // 900 USDC, well within LTV
        0,
    )
    .expect("borrow must succeed against a valid oracle");
    let debt_before = fetch_position(&svm, &borrower_position).borrow_shares;
    assert!(debt_before > 0, "fixture must actually create debt");

    // Break the oracle: warp far past max_price_age_secs, representative of a real outage
    // (oracle-design.md §4.1's "prolonged oracle outage" framing).
    warp_seconds(&mut svm, MAX_PRICE_AGE_SECS + 10_000);

    let repay_result = repay(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        1_000_000_000, // more than the debt; repay clamps
        0,
    );
    repay_result.expect("A-ORACLE-02: repay must succeed with a broken (stale) oracle");

    let debt_after = fetch_position(&svm, &borrower_position).borrow_shares;
    assert_eq!(debt_after, 0, "the full debt must have been repaid");
}

// ============================================================================================
// A-ORACLE-03: stale oracle blocks borrow and debt-bearing withdraw, but not repay/deposit.
// ============================================================================================

#[test]
fn a_oracle_03_stale_oracle_blocks_borrow_and_debt_withdraw_but_not_repay_or_deposit() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (borrower, borrower_position, borrower_loan_ata) = setup_borrower(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        10_000_000_000,
    );
    let borrower_collateral_ata = create_token_account(
        &mut svm,
        &admin,
        seeds.next(),
        fx.collateral_mint,
        borrower.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    mint_to(
        &mut svm,
        &admin,
        fx.collateral_mint,
        borrower_collateral_ata,
        &admin,
        1_000_000_000,
        spl_token_interface::ID,
    );

    let n = now(&svm);
    let (c, l) = valid_prices(&mut svm, &mut seeds, n);
    borrow(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        c,
        l,
        900_000_000,
        0,
    )
    .expect("initial borrow must succeed");

    // Make the SAME price accounts stale (do not re-publish).
    warp_seconds(&mut svm, MAX_PRICE_AGE_SECS + 1);

    // borrow fails closed.
    svm.expire_blockhash();
    let borrow_result = borrow(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        c,
        l,
        1_000_000,
        0,
    );
    assert_aegis_error(&borrow_result, AegisError::OraclePriceStale);

    // debt-bearing withdraw_collateral fails closed.
    svm.expire_blockhash();
    let withdraw_result = withdraw_collateral(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.collateral_vault,
        borrower_collateral_ata,
        fx.collateral_mint,
        spl_token_interface::ID,
        c,
        l,
        1_000_000,
    );
    assert_aegis_error(&withdraw_result, AegisError::OraclePriceStale);

    // repay still works (no oracle needed).
    svm.expire_blockhash();
    repay(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        1_000_000,
        0,
    )
    .expect("repay must still succeed while the oracle is stale");

    // deposit_collateral still works (no oracle needed).
    svm.expire_blockhash();
    deposit_collateral(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.collateral_vault,
        borrower_collateral_ata,
        fx.collateral_mint,
        spl_token_interface::ID,
        1_000_000,
    )
    .expect("deposit_collateral must still succeed while the oracle is stale");
}

// Boundary: age exactly at max_price_age_secs must pass; +1 must fail.
#[test]
fn a_oracle_03_boundary_age_exactly_at_threshold_vs_plus_one() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (borrower, borrower_position, borrower_loan_ata) = setup_borrower(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        10_000_000_000,
    );

    let publish_time = now(&svm);
    let (c, l) = valid_prices(&mut svm, &mut seeds, publish_time);

    // Warp so that (now - publish_time) == max_price_age_secs exactly -> must still pass.
    warp_seconds(&mut svm, MAX_PRICE_AGE_SECS);
    let ok = borrow(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        c,
        l,
        10_000_000,
        0,
    );
    ok.expect("age exactly at max_price_age_secs must be accepted");

    // One more second -> must fail.
    warp_seconds(&mut svm, 1);
    svm.expire_blockhash();
    let err = borrow(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        c,
        l,
        10_000_000,
        0,
    );
    assert_aegis_error(&err, AegisError::OraclePriceStale);
}

// ============================================================================================
// A-ORACLE-04: zero / negative / absurd price.
// ============================================================================================

#[test]
fn a_oracle_04_zero_price_is_rejected() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (borrower, borrower_position, borrower_loan_ata) = setup_borrower(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        10_000_000_000,
    );

    let n = now(&svm);
    let c = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(COLLATERAL_FEED_ID, 0, 0, -8, n),
    );
    let l = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 0, -8, n),
    );

    let result = borrow(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        c,
        l,
        10_000_000,
        0,
    );
    assert_aegis_error(&result, AegisError::OraclePriceNotPositive);
}

#[test]
fn a_oracle_04_negative_price_is_rejected() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (borrower, borrower_position, borrower_loan_ata) = setup_borrower(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        10_000_000_000,
    );

    let n = now(&svm);
    let c = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(COLLATERAL_FEED_ID, -15_000_000_000, 0, -8, n),
    );
    let l = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 0, -8, n),
    );

    let result = borrow(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        c,
        l,
        10_000_000,
        0,
    );
    assert_aegis_error(&result, AegisError::OraclePriceNotPositive);
}

// An absurdly large price (scales far past MAX_PRICE_WAD) is rejected cleanly, never an
// arithmetic abort.
#[test]
fn a_oracle_04_absurd_price_is_rejected() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (borrower, borrower_position, borrower_loan_ata) = setup_borrower(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        10_000_000_000,
    );

    let n = now(&svm);
    // price = 1_000_000, expo = 10 -> WAD value = 1_000_000 * 10^28 = 1e34 > MAX_PRICE_WAD (1e30).
    let c = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(COLLATERAL_FEED_ID, 1_000_000, 0, 10, n),
    );
    let l = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 0, -8, n),
    );

    let result = borrow(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        c,
        l,
        10_000_000,
        0,
    );
    assert_aegis_error(&result, AegisError::OraclePriceOutOfBounds);
}

// ============================================================================================
// A-ORACLE-05: confidence over threshold, plus the exact boundary.
// ============================================================================================

#[test]
fn a_oracle_05_confidence_over_threshold_is_rejected() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (borrower, borrower_position, borrower_loan_ata) = setup_borrower(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        10_000_000_000,
    );

    let n = now(&svm);
    // price = 15_000_000_000, max_conf_bps = 100 (1%) -> max_conf = 150_000_000 exactly.
    let c = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(COLLATERAL_FEED_ID, 15_000_000_000, 150_000_001, -8, n),
    );
    let l = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 0, -8, n),
    );

    let result = borrow(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        c,
        l,
        10_000_000,
        0,
    );
    assert_aegis_error(&result, AegisError::OracleConfidenceTooWide);
}

// Boundary: confidence exactly at max_conf_bps must pass; +1 (an otherwise-valid update rejected
// solely because of confidence) must fail.
#[test]
fn a_oracle_05_boundary_confidence_exactly_at_threshold_vs_plus_one() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (borrower, borrower_position, borrower_loan_ata) = setup_borrower(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        10_000_000_000,
    );

    let n = now(&svm);
    // 15_000_000_000 * MAX_CONF_BPS / 10_000 = 150_000_000 exactly.
    let max_conf = (15_000_000_000u128 * MAX_CONF_BPS / 10_000) as u64;
    let c_ok = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(COLLATERAL_FEED_ID, 15_000_000_000, max_conf, -8, n),
    );
    let l = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 0, -8, n),
    );
    borrow(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        c_ok,
        l,
        10_000_000,
        0,
    )
    .expect("confidence exactly at max_conf_bps must be accepted");

    svm.expire_blockhash();
    let c_bad = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(COLLATERAL_FEED_ID, 15_000_000_000, max_conf + 1, -8, n),
    );
    let err = borrow(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        c_bad,
        l,
        10_000_000,
        0,
    );
    assert_aegis_error(&err, AegisError::OracleConfidenceTooWide);
}

// ============================================================================================
// A-ORACLE-06: fake / wrong-owner account.
// ============================================================================================

// A genuine, otherwise-well-formed PriceUpdateV2 account (correct discriminator, correct feed,
// correct everything) but owned by a program that is NOT the Pyth receiver -- this validates the
// actual ownership boundary, not merely malformed deserialization.
#[test]
fn a_oracle_06_wrong_owner_account_is_rejected() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (borrower, borrower_position, borrower_loan_ata) = setup_borrower(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        10_000_000_000,
    );

    let n = now(&svm);
    let fake_owner_fixture = PriceFixture {
        owner: spl_token_interface::ID, // a real, deployed, but WRONG program.
        ..PriceFixture::valid(COLLATERAL_FEED_ID, 15_000_000_000, 0, -8, n)
    };
    let c = set_price(&mut svm, seeds.next(), fake_owner_fixture);
    let l = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 0, -8, n),
    );

    let result = borrow(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        c,
        l,
        10_000_000,
        0,
    );
    assert_aegis_error(&result, AegisError::OracleAccountOwnerMismatch);
}

// ============================================================================================
// A-ORACLE-07: wrong feed ID.
// ============================================================================================

// An otherwise-valid price-update account, correctly owned and fully verified, but for a
// DIFFERENT feed than the market's configured collateral feed.
#[test]
fn a_oracle_07_wrong_feed_id_is_rejected() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (borrower, borrower_position, borrower_loan_ata) = setup_borrower(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        10_000_000_000,
    );

    let n = now(&svm);
    let wrong_feed = [0x77u8; 32]; // not COLLATERAL_FEED_ID
    let c = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(wrong_feed, 15_000_000_000, 0, -8, n),
    );
    let l = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 0, -8, n),
    );

    let result = borrow(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        c,
        l,
        10_000_000,
        0,
    );
    assert_aegis_error(&result, AegisError::OracleFeedMismatch);
}

// ============================================================================================
// A-ORACLE-08: partial verification level.
// ============================================================================================

#[test]
fn a_oracle_08_partial_verification_level_is_rejected() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (borrower, borrower_position, borrower_loan_ata) = setup_borrower(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        10_000_000_000,
    );

    let n = now(&svm);
    let partial_fixture = PriceFixture {
        verification_level: VerificationLevel::Partial { num_signatures: 5 },
        ..PriceFixture::valid(COLLATERAL_FEED_ID, 15_000_000_000, 0, -8, n)
    };
    let c = set_price(&mut svm, seeds.next(), partial_fixture);
    let l = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 0, -8, n),
    );

    let result = borrow(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        c,
        l,
        10_000_000,
        0,
    );
    assert_aegis_error(&result, AegisError::OracleVerificationLevelNotFull);
}

// ============================================================================================
// A-ORACLE-09: future publish time.
// ============================================================================================

#[test]
fn a_oracle_09_future_publish_time_is_rejected() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (borrower, borrower_position, borrower_loan_ata) = setup_borrower(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        10_000_000_000,
    );

    let n = now(&svm);
    // 61s ahead of the runtime clock -- 1s past MAX_FUTURE_PRICE_SKEW_SECS (60).
    let c = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(COLLATERAL_FEED_ID, 15_000_000_000, 0, -8, n + 61),
    );
    let l = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 0, -8, n),
    );

    let result = borrow(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        c,
        l,
        10_000_000,
        0,
    );
    assert_aegis_error(&result, AegisError::OraclePriceInFuture);
}

// Boundary: exactly 60s in the future must pass; 61s must fail (asserted above).
#[test]
fn a_oracle_09_boundary_exactly_at_max_future_skew() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (borrower, borrower_position, borrower_loan_ata) = setup_borrower(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        10_000_000_000,
    );

    let n = now(&svm);
    let c = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(COLLATERAL_FEED_ID, 15_000_000_000, 0, -8, n + 60),
    );
    let l = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 0, -8, n),
    );
    borrow(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        c,
        l,
        10_000_000,
        0,
    )
    .expect("publish_time exactly 60s in the future must be accepted");
}

// ============================================================================================
// A-ORACLE-10: outage across a price move; accounting stays consistent on recovery.
// ============================================================================================

#[test]
fn a_oracle_10_outage_across_a_price_move_then_recovery() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (borrower, borrower_position, borrower_loan_ata) = setup_borrower(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        10_000_000_000,
    );
    let borrower_collateral_ata = create_token_account(
        &mut svm,
        &admin,
        seeds.next(),
        fx.collateral_mint,
        borrower.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    mint_to(
        &mut svm,
        &admin,
        fx.collateral_mint,
        borrower_collateral_ata,
        &admin,
        1_000_000_000,
        spl_token_interface::ID,
    );

    // 1. Valid price, protocol enters a valid state (a real borrow).
    let n1 = now(&svm);
    let (c1, l1) = valid_prices(&mut svm, &mut seeds, n1);
    borrow(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        c1,
        l1,
        900_000_000,
        0,
    )
    .expect("initial borrow must succeed");
    invariants::assert_inv_cus_01(&svm, &fx.market);

    // 2. Oracle becomes stale (outage) while the underlying economic price, hypothetically, has
    // moved -- Aegis cannot observe that move because the feed is not refreshed.
    warp_seconds(&mut svm, MAX_PRICE_AGE_SECS + 1);

    // 3. Permitted risk-reducing actions occur during the outage: repay part of the debt and top
    // up collateral. Neither requires the oracle.
    svm.expire_blockhash();
    repay(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        100_000_000,
        0,
    )
    .expect("repay during outage must succeed");
    svm.expire_blockhash();
    deposit_collateral(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.collateral_vault,
        borrower_collateral_ata,
        fx.collateral_mint,
        spl_token_interface::ID,
        1_000_000_000,
    )
    .expect("deposit during outage must succeed");
    invariants::assert_inv_cus_01(&svm, &fx.market);

    // Meanwhile, borrowing more remains fail-closed during the outage.
    svm.expire_blockhash();
    let blocked = borrow(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        c1,
        l1,
        1_000_000,
        0,
    );
    assert_aegis_error(&blocked, AegisError::OraclePriceStale);

    // 4. Oracle recovers with an updated (moved) price -- SOL now $120.00 instead of $150.00.
    let recovery_time = now(&svm);
    let (c2, l2) = (
        set_price(
            &mut svm,
            seeds.next(),
            PriceFixture::valid(COLLATERAL_FEED_ID, 12_000_000_000, 0, -8, recovery_time),
        ),
        set_price(
            &mut svm,
            seeds.next(),
            PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 0, -8, recovery_time),
        ),
    );

    // 5. Protocol accounting remains internally consistent through the whole episode.
    invariants::assert_inv_cus_01(&svm, &fx.market);
    let market_state = fetch_market(&svm, &fx.market);
    let position_state = fetch_position(&svm, &borrower_position);
    assert_eq!(
        position_state.collateral_amount, 11_000_000_000,
        "10 SOL deposited + 1 SOL topped up during the outage"
    );
    assert!(
        market_state.total_borrow_assets < 900_000_000,
        "the partial repay during the outage must be reflected"
    );

    // 6. A fresh, valid, non-stale borrow now succeeds again, using the recovered price -- proving
    // the position's accounting (not just its price feed) survived the episode intact.
    svm.expire_blockhash();
    borrow(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        c2,
        l2,
        1_000_000,
        0,
    )
    .expect("borrow must succeed again once the oracle recovers");
    invariants::assert_inv_cus_01(&svm, &fx.market);
}

// ============================================================================================
// A-ORACLE-11: same-transaction price timing -- a price read is a single, self-consistent
// account read within one instruction; a later transaction's price update never retroactively
// changes an already-executed transaction's outcome.
// ============================================================================================

#[test]
fn a_oracle_11_same_transaction_price_read_is_self_consistent() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (borrower, borrower_position, borrower_loan_ata) = setup_borrower(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        10_000_000_000,
    );

    // A price under which a 1,200 USDC borrow is within max_ltv (collateral_value = 1500e18,
    // max borrowable debt_value = 1125e18 at 0.75 -> up to 1125 USDC).
    let n = now(&svm);
    let (c, l) = valid_prices(&mut svm, &mut seeds, n);

    // Execute a borrow transaction against this exact, fixed price.
    let first = borrow(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        c,
        l,
        1_000_000_000, // 1,000 USDC -- within LTV at $150/SOL, would NOT be at a much lower price
        0,
    );
    first.expect("borrow within LTV at the price read during this transaction must succeed");
    let debt_after_first = fetch_position(&svm, &borrower_position).borrow_shares;

    // Publish a NEW, much lower price at the same feed accounts, in a LATER transaction. This
    // must have no effect whatsoever on the transaction that already executed and committed --
    // there is no retroactive re-evaluation of a past instruction against a price that did not
    // exist when it ran.
    let later_time = now(&svm);
    inject_price_update(
        &mut svm,
        c,
        PriceFixture::valid(COLLATERAL_FEED_ID, 1_500_000_000, 0, -8, later_time), // $15.00
    );
    let debt_unchanged_by_later_price_publication =
        fetch_position(&svm, &borrower_position).borrow_shares;
    assert_eq!(
        debt_after_first, debt_unchanged_by_later_price_publication,
        "a price update published after a transaction commits must never retroactively affect it"
    );
}

// ============================================================================================
// A-ORACLE-12: same account passed for both feeds.
// ============================================================================================

#[test]
fn a_oracle_12_same_account_for_both_feeds_is_rejected() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (borrower, borrower_position, borrower_loan_ata) = setup_borrower(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        10_000_000_000,
    );

    // A single, otherwise-valid price account for the COLLATERAL feed, physically passed for
    // both roles -- not merely two differently-named arguments that happen to differ.
    let n = now(&svm);
    let same_account = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(COLLATERAL_FEED_ID, 15_000_000_000, 0, -8, n),
    );

    let ix = borrow_ix(
        &borrower.pubkey(),
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        same_account,
        same_account,
        10_000_000,
        0,
    );
    let result = send(&mut svm, &borrower, &[], ix);
    assert_aegis_error(&result, AegisError::OracleDuplicatePriceAccounts);
}

// ============================================================================================
// A-ORACLE-13: a failed oracle check leaves no state modified (INV-ORA-07).
// ============================================================================================

#[test]
fn a_oracle_13_failed_oracle_check_leaves_state_byte_identical() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (borrower, borrower_position, borrower_loan_ata) = setup_borrower(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        10_000_000_000,
    );

    // Full before-snapshot: market, position, both vaults, borrower's own loan ATA -- everything
    // this instruction could conceivably touch.
    let market_before = svm.get_account(&fx.market).unwrap();
    let position_before = svm.get_account(&borrower_position).unwrap();
    let loan_vault_before = svm.get_account(&fx.loan_vault).unwrap();
    let collateral_vault_before = svm.get_account(&fx.collateral_vault).unwrap();
    let borrower_ata_before = svm.get_account(&borrower_loan_ata).unwrap();

    // A wrong-owner price account -- a genuine O-1 failure, not a happy path with a side check.
    let n = now(&svm);
    let bad_owner_fixture = PriceFixture {
        owner: spl_token_interface::ID,
        ..PriceFixture::valid(COLLATERAL_FEED_ID, 15_000_000_000, 0, -8, n)
    };
    let c = set_price(&mut svm, seeds.next(), bad_owner_fixture);
    let l = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 0, -8, n),
    );

    let result = borrow(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        c,
        l,
        900_000_000,
        0,
    );
    assert_aegis_error(&result, AegisError::OracleAccountOwnerMismatch);

    // Full after-snapshot: byte-exact equality, not merely "the instruction returned an error".
    let market_after = svm.get_account(&fx.market).unwrap();
    let position_after = svm.get_account(&borrower_position).unwrap();
    let loan_vault_after = svm.get_account(&fx.loan_vault).unwrap();
    let collateral_vault_after = svm.get_account(&fx.collateral_vault).unwrap();
    let borrower_ata_after = svm.get_account(&borrower_loan_ata).unwrap();

    assert_eq!(
        market_before.data, market_after.data,
        "A-ORACLE-13: market data changed"
    );
    assert_eq!(
        position_before.data, position_after.data,
        "A-ORACLE-13: position data changed"
    );
    assert_eq!(
        loan_vault_before.data, loan_vault_after.data,
        "A-ORACLE-13: loan_vault data changed"
    );
    assert_eq!(
        collateral_vault_before.data, collateral_vault_after.data,
        "A-ORACLE-13: collateral_vault data changed"
    );
    assert_eq!(
        borrower_ata_before.data, borrower_ata_after.data,
        "A-ORACLE-13: borrower's own loan ATA changed"
    );
    assert_eq!(market_before.lamports, market_after.lamports);
    assert_eq!(position_before.lamports, position_after.lamports);
    assert_eq!(loan_vault_before.lamports, loan_vault_after.lamports);
    assert_eq!(
        collateral_vault_before.lamports,
        collateral_vault_after.lamports
    );
    assert_eq!(borrower_ata_before.lamports, borrower_ata_after.lamports);
}

// Same evidence, on the withdraw_collateral debt path.
#[test]
fn a_oracle_13_failed_withdraw_collateral_oracle_check_leaves_state_byte_identical() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (borrower, borrower_position, borrower_loan_ata) = setup_borrower(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        10_000_000_000,
    );
    let borrower_collateral_ata = create_token_account(
        &mut svm,
        &admin,
        seeds.next(),
        fx.collateral_mint,
        borrower.pubkey(),
        spl_token_interface::ID,
        &[],
    );

    let n = now(&svm);
    let (c, l) = valid_prices(&mut svm, &mut seeds, n);
    borrow(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        c,
        l,
        900_000_000,
        0,
    )
    .expect("must have real debt for the debt-path oracle check to even run");

    let position_before = svm.get_account(&borrower_position).unwrap();
    let collateral_vault_before = svm.get_account(&fx.collateral_vault).unwrap();

    warp_seconds(&mut svm, MAX_PRICE_AGE_SECS + 1);
    svm.expire_blockhash();
    let result = withdraw_collateral(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.collateral_vault,
        borrower_collateral_ata,
        fx.collateral_mint,
        spl_token_interface::ID,
        c,
        l,
        1_000_000_000,
    );
    assert_aegis_error(&result, AegisError::OraclePriceStale);

    let position_after = svm.get_account(&borrower_position).unwrap();
    let collateral_vault_after = svm.get_account(&fx.collateral_vault).unwrap();
    assert_eq!(position_before.data, position_after.data);
    assert_eq!(collateral_vault_before.data, collateral_vault_after.data);
}

// ============================================================================================
// Debt-bearing withdraw_collateral: post-withdraw health check (replaces the removed Phase 3/4
// gate; see the NOTE in tests/phase3_adversarial.rs).
// ============================================================================================

#[test]
fn debt_bearing_withdraw_rejects_unsafe_post_withdraw_health() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (borrower, borrower_position, borrower_loan_ata) = setup_borrower(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        10_000_000_000,
    );
    let borrower_collateral_ata = create_token_account(
        &mut svm,
        &admin,
        seeds.next(),
        fx.collateral_mint,
        borrower.pubkey(),
        spl_token_interface::ID,
        &[],
    );

    let n = now(&svm);
    let (c, l) = valid_prices(&mut svm, &mut seeds, n);
    // Borrow right up near the max_ltv boundary: collateral_value = 1500e18, max_ltv = 0.75 ->
    // max debt_value = 1125e18 -> up to 1125 USDC.
    borrow(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        c,
        l,
        1_100_000_000,
        0,
    )
    .expect("borrow within LTV must succeed");

    // Withdrawing a large amount of collateral would push debt_value above the remaining
    // collateral_value * max_ltv -- must be rejected, using the POST-withdrawal collateral
    // amount, not the pre-withdrawal one.
    let result = withdraw_collateral(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.collateral_vault,
        borrower_collateral_ata,
        fx.collateral_mint,
        spl_token_interface::ID,
        c,
        l,
        8_000_000_000, // would leave 2 SOL = $300 value, far below the 1,100 USDC debt / 0.75
    );
    assert_aegis_error(&result, AegisError::ExceedsMaxLtv);

    // A small, safe withdrawal against the same debt succeeds.
    svm.expire_blockhash();
    withdraw_collateral(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.collateral_vault,
        borrower_collateral_ata,
        fx.collateral_mint,
        spl_token_interface::ID,
        c,
        l,
        1_000_000_000, // leaves 9 SOL = $1,350, still comfortably >= 1,100 / 0.75 = 1,466.67 ...
    )
    .expect_err(
        "even a modest withdrawal this close to the LTV boundary must be evaluated honestly",
    );
}

// A separate, generous fixture proving the positive case: a safe post-withdrawal state succeeds.
#[test]
fn debt_bearing_withdraw_succeeds_when_post_withdraw_health_is_safe() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (borrower, borrower_position, borrower_loan_ata) = setup_borrower(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        10_000_000_000,
    );
    let borrower_collateral_ata = create_token_account(
        &mut svm,
        &admin,
        seeds.next(),
        fx.collateral_mint,
        borrower.pubkey(),
        spl_token_interface::ID,
        &[],
    );

    let n = now(&svm);
    let (c, l) = valid_prices(&mut svm, &mut seeds, n);
    // Modest debt: 300 USDC against 10 SOL ($1,500).
    borrow(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        c,
        l,
        300_000_000,
        0,
    )
    .expect("borrow must succeed");

    // Withdraw half the collateral: leaves 5 SOL = $750 value, still >> 300 / 0.75 = 400.
    let result = withdraw_collateral(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.collateral_vault,
        borrower_collateral_ata,
        fx.collateral_mint,
        spl_token_interface::ID,
        c,
        l,
        5_000_000_000,
    );
    result.expect("a safe post-withdrawal state must be permitted");

    let position_state = fetch_position(&svm, &borrower_position);
    assert_eq!(position_state.collateral_amount, 5_000_000_000);
}
