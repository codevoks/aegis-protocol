//! Phase 12 — pause architecture: guardian asymmetry, undefined-bit rejection, the full-pause
//! safety-exit proof (`A-ADM-01`), and paused-borrow (`A-ADM-03`)
//! (`docs/phases/phase-12-governance.md`, `docs/governance.md` §3, `docs/invariants.md`
//! INV-ADM-03/04, INV-AUTH-04, INV-BOR-04).

#![allow(clippy::result_large_err)]

use aegis::constants::{
    PAUSE_ALL_BITS, PAUSE_BORROW, PAUSE_LIQUIDATE, PAUSE_SUPPLY, PAUSE_WITHDRAW,
};
use aegis::error::AegisError;
use aegis_test_kit::{
    absorb_bad_debt, assert_aegis_error, borrow, close_position, create_market, create_spl_mint,
    create_token_account, deploy, deposit_collateral, fetch_market, fetch_position, fetch_protocol,
    init_position, initialize_protocol, invariants, liquidate, mint_to, protocol_pda,
    reference_market_args, repay, set_market_pause, set_price, set_protocol_pause,
    spl_token_interface, supply, withdraw, withdraw_collateral, PriceFixture,
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

fn program_bytes() -> &'static [u8] {
    include_bytes!(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/aegis.so"))
}

const COLLATERAL_FEED_ID: [u8; 32] = [0xAAu8; 32];
const LOAN_FEED_ID: [u8; 32] = [0xBBu8; 32];

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

fn now(svm: &litesvm::LiteSVM) -> i64 {
    svm.get_sysvar::<solana_clock::Clock>().unix_timestamp
}

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

fn crash_prices(svm: &mut litesvm::LiteSVM, seeds: &mut SeedGen, at: i64) -> (Pubkey, Pubkey) {
    let c = set_price(
        svm,
        seeds.next(),
        PriceFixture::valid(COLLATERAL_FEED_ID, 9_500_000_000, 20_000_000, -8, at),
    );
    let l = set_price(
        svm,
        seeds.next(),
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 20_000, -8, at),
    );
    (c, l)
}

// ============================================================================================
// A-AUTH-04: the guardian may SET pause bits but may never CLEAR one.
// ============================================================================================

#[test]
fn a_auth_04_guardian_can_pause_protocol_but_cannot_unpause() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let mut seeds = SeedGen::new();
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let guardian = Keypair::new_from_array([20u8; 32]);
    svm.airdrop(&guardian.pubkey(), 10_000_000_000).unwrap();

    set_protocol_pause(&mut svm, &guardian, PAUSE_ALL_BITS).expect("guardian may set every bit");
    let (protocol_key, _) = protocol_pda();
    assert_eq!(fetch_protocol(&svm, &protocol_key).paused, PAUSE_ALL_BITS);

    // The guardian attempts to unpause -- must fail with the EXACT intended error.
    let result = set_protocol_pause(&mut svm, &guardian, 0);
    assert_aegis_error(&result, AegisError::GuardianCannotClearPause);
    assert_eq!(
        fetch_protocol(&svm, &protocol_key).paused,
        PAUSE_ALL_BITS,
        "state must be unchanged after the rejected clear attempt"
    );

    // The admin CAN unpause.
    set_protocol_pause(&mut svm, &admin, 0).expect("admin may clear pause bits");
    assert_eq!(fetch_protocol(&svm, &protocol_key).paused, 0);

    let _ = fx;
}

#[test]
fn a_auth_04_guardian_can_pause_market_but_cannot_unpause() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let mut seeds = SeedGen::new();
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let guardian = Keypair::new_from_array([20u8; 32]);
    svm.airdrop(&guardian.pubkey(), 10_000_000_000).unwrap();

    set_market_pause(&mut svm, &guardian, fx.market, PAUSE_ALL_BITS)
        .expect("guardian may set every market bit");
    assert_eq!(fetch_market(&svm, &fx.market).paused, PAUSE_ALL_BITS);

    let result = set_market_pause(&mut svm, &guardian, fx.market, PAUSE_SUPPLY);
    assert_aegis_error(&result, AegisError::GuardianCannotClearPause);

    set_market_pause(&mut svm, &admin, fx.market, 0).expect("admin may clear market pause bits");
    assert_eq!(fetch_market(&svm, &fx.market).paused, 0);
}

// A-ADM-05: undefined/reserved pause bits are rejected outright, for both protocol and market
// pause, and for both admin and guardian.
#[test]
fn a_adm_05_undefined_pause_bits_are_rejected() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let mut seeds = SeedGen::new();
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let undefined_bit: u8 = 0b0001_0000; // one bit above PAUSE_ALL_BITS (0b1111)

    let result = set_protocol_pause(&mut svm, &admin, undefined_bit);
    assert_aegis_error(&result, AegisError::InvalidPauseBits);

    let result = set_protocol_pause(&mut svm, &admin, PAUSE_ALL_BITS | undefined_bit);
    assert_aegis_error(&result, AegisError::InvalidPauseBits);

    let result = set_market_pause(&mut svm, &admin, fx.market, undefined_bit);
    assert_aegis_error(&result, AegisError::InvalidPauseBits);

    // No mask bits were silently persisted.
    let (protocol_key, _) = protocol_pda();
    assert_eq!(fetch_protocol(&svm, &protocol_key).paused, 0);
    assert_eq!(fetch_market(&svm, &fx.market).paused, 0);
}

// ============================================================================================
// A-ADM-03: paused borrow fails with the exact intended error; every other pause-gated
// instruction is checked too (docs/phases/phase-12-governance.md item 15).
// ============================================================================================

#[test]
fn a_adm_03_paused_borrow_fails_with_exact_error() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let mut seeds = SeedGen::new();
    let fx = setup_market(&mut svm, &admin, &mut seeds);

    let (lender, lender_ata) = wallet_with_ata(
        &mut svm,
        &admin,
        fx.loan_mint,
        &mut seeds,
        1_000_000_000_000,
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
        spl_token_interface::ID,
        1_000_000_000_000,
        0,
    )
    .expect("supply must succeed");

    let (borrower, borrower_collateral_ata) = wallet_with_ata(
        &mut svm,
        &admin,
        fx.collateral_mint,
        &mut seeds,
        10_000_000_000,
    );
    let (_, borrower_position) = init_position(&mut svm, &admin, fx.market, borrower.pubkey());
    deposit_collateral(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.collateral_vault,
        borrower_collateral_ata,
        fx.collateral_mint,
        spl_token_interface::ID,
        10_000_000_000,
    )
    .expect("deposit_collateral must succeed");
    let borrower_loan_ata = create_token_account(
        &mut svm,
        &admin,
        seeds.next(),
        fx.loan_mint,
        borrower.pubkey(),
        spl_token_interface::ID,
        &[],
    );

    set_protocol_pause(&mut svm, &admin, PAUSE_BORROW).expect("pause BORROW");

    let n = now(&svm);
    let (c, l) = valid_prices(&mut svm, &mut seeds, n);
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
    assert_aegis_error(&result, AegisError::OperationPaused);

    // Unrelated bits do not block borrow.
    set_protocol_pause(&mut svm, &admin, PAUSE_SUPPLY).expect("clear BORROW, set only SUPPLY");
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
    .expect("borrow must succeed when only an unrelated bit is paused");
}

#[test]
fn paused_supply_and_withdraw_and_withdraw_collateral_and_liquidate_fail_with_exact_errors() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let mut seeds = SeedGen::new();
    let fx = setup_market(&mut svm, &admin, &mut seeds);

    let (lender, lender_ata) = wallet_with_ata(
        &mut svm,
        &admin,
        fx.loan_mint,
        &mut seeds,
        1_000_000_000_000,
    );
    let (_, lender_position) = init_position(&mut svm, &admin, fx.market, lender.pubkey());

    // SUPPLY paused -> supply fails.
    set_protocol_pause(&mut svm, &admin, PAUSE_SUPPLY).expect("pause SUPPLY");
    let result = supply(
        &mut svm,
        &lender,
        fx.market,
        lender_position,
        fx.fee_position,
        fx.loan_vault,
        lender_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        1_000_000_000_000,
        0,
    );
    assert_aegis_error(&result, AegisError::OperationPaused);
    set_protocol_pause(&mut svm, &admin, 0).expect("unpause");
    svm.expire_blockhash();

    supply(
        &mut svm,
        &lender,
        fx.market,
        lender_position,
        fx.fee_position,
        fx.loan_vault,
        lender_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        1_000_000_000_000,
        0,
    )
    .expect("supply must succeed while unpaused");

    // WITHDRAW paused -> withdraw fails.
    set_protocol_pause(&mut svm, &admin, PAUSE_WITHDRAW).expect("pause WITHDRAW");
    let result = withdraw(
        &mut svm,
        &lender,
        fx.market,
        lender_position,
        fx.fee_position,
        fx.loan_vault,
        lender_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        1_000_000,
        0,
    );
    assert_aegis_error(&result, AegisError::OperationPaused);

    // WITHDRAW paused -> withdraw_collateral fails too (governance.md §3: WITHDRAW covers both).
    let (borrower, borrower_collateral_ata) = wallet_with_ata(
        &mut svm,
        &admin,
        fx.collateral_mint,
        &mut seeds,
        10_000_000_000,
    );
    let (_, borrower_position) = init_position(&mut svm, &admin, fx.market, borrower.pubkey());
    set_protocol_pause(&mut svm, &admin, 0).expect("unpause to deposit collateral");
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
    .expect("deposit_collateral");
    svm.expire_blockhash();
    set_protocol_pause(&mut svm, &admin, PAUSE_WITHDRAW).expect("pause WITHDRAW again");
    let n = now(&svm);
    let (c, l) = valid_prices(&mut svm, &mut seeds, n);
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
        1_000_000,
    );
    assert_aegis_error(&result, AegisError::OperationPaused);
    set_protocol_pause(&mut svm, &admin, 0).expect("unpause");

    // LIQUIDATE paused -> liquidate fails, using a real liquidatable position.
    let borrower_loan_ata = create_token_account(
        &mut svm,
        &admin,
        seeds.next(),
        fx.loan_mint,
        borrower.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    deposit_collateral(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.collateral_vault,
        borrower_collateral_ata,
        fx.collateral_mint,
        spl_token_interface::ID,
        9_000_000_000,
    )
    .expect("top up collateral to 10 SOL total");
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
    .expect("borrow must succeed");

    let (liquidator, liquidator_loan_ata) =
        wallet_with_ata(&mut svm, &admin, fx.loan_mint, &mut seeds, 1_000_000_000);
    let liquidator_collateral_ata = create_token_account(
        &mut svm,
        &admin,
        seeds.next(),
        fx.collateral_mint,
        liquidator.pubkey(),
        spl_token_interface::ID,
        &[],
    );

    set_protocol_pause(&mut svm, &admin, PAUSE_LIQUIDATE).expect("pause LIQUIDATE");
    let n = now(&svm);
    let (c, l) = crash_prices(&mut svm, &mut seeds, n);
    let result = liquidate(
        &mut svm,
        &liquidator,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        fx.collateral_vault,
        liquidator_loan_ata,
        liquidator_collateral_ata,
        fx.loan_mint,
        fx.collateral_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        c,
        l,
        400_000_000,
        0,
    );
    assert_aegis_error(&result, AegisError::OperationPaused);
}

// ============================================================================================
// A-ADM-01 -- the single most important test in this phase (docs/phases/phase-12-governance.md).
//
// Every protocol AND market pause bit set; repay/deposit_collateral/absorb_bad_debt/
// close_position all still succeed, using real prior state transitions (not source-inspection).
// ============================================================================================

#[test]
fn a_adm_01_safety_exits_succeed_with_every_pause_bit_set() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let mut seeds = SeedGen::new();
    let fx = setup_market(&mut svm, &admin, &mut seeds);

    // --- Establish meaningful lender/borrower state BEFORE pausing anything. ---
    let (lender, lender_ata) = wallet_with_ata(
        &mut svm,
        &admin,
        fx.loan_mint,
        &mut seeds,
        1_000_000_000_000,
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
        spl_token_interface::ID,
        1_000_000_000_000,
        0,
    )
    .expect("lender supply");

    // Borrower A: real collateral + real debt -- will exercise repay and deposit_collateral
    // (topping up) while fully paused, then reduce debt to zero and collateral to zero to also
    // exercise close_position while fully paused.
    let (borrower_a, borrower_a_collateral_ata) = wallet_with_ata(
        &mut svm,
        &admin,
        fx.collateral_mint,
        &mut seeds,
        20_000_000_000,
    );
    let (_, position_a) = init_position(&mut svm, &admin, fx.market, borrower_a.pubkey());
    deposit_collateral(
        &mut svm,
        &borrower_a,
        fx.market,
        position_a,
        fx.collateral_vault,
        borrower_a_collateral_ata,
        fx.collateral_mint,
        spl_token_interface::ID,
        10_000_000_000,
    )
    .expect("deposit_collateral A");
    let borrower_a_loan_ata = create_token_account(
        &mut svm,
        &admin,
        seeds.next(),
        fx.loan_mint,
        borrower_a.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    let n = now(&svm);
    let (c, l) = valid_prices(&mut svm, &mut seeds, n);
    borrow(
        &mut svm,
        &borrower_a,
        fx.market,
        position_a,
        fx.fee_position,
        fx.loan_vault,
        borrower_a_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        c,
        l,
        100_000_000,
        0,
    )
    .expect("borrow A");
    // Give the payer enough loan-asset balance to fully repay (plus a margin, since some was
    // already used for the borrow's own account rent).
    mint_to(
        &mut svm,
        &admin,
        fx.loan_mint,
        borrower_a_loan_ata,
        &admin,
        1_000_000_000,
        spl_token_interface::ID,
    );

    // Bad-debt position (borrower B): built via the same legitimate `seed_borrow_state` injection
    // technique Phase 4/6 already use to isolate one instruction's precondition
    // (`crates/aegis-test-kit/src/state_injection.rs`) -- collateral_amount == 0, borrow_shares >
    // 0 (bad debt), constructed as *real* on-chain account state, then absorbed via a real
    // transaction against the real handler.
    let (_, position_b) = init_position(&mut svm, &admin, fx.market, fixed_pubkey(seeds.next()));
    aegis_test_kit::seed_borrow_state(&mut svm, fx.market, position_b, 5_000_000, 5_000_000_000);

    // Borrower C: a position about to become fully empty, to exercise close_position while fully
    // paused (supply_shares == 0, borrow_shares == 0, collateral_amount == 0 is required).
    let (borrower_c, _) = wallet_with_ata(&mut svm, &admin, fx.collateral_mint, &mut seeds, 0);
    let (_, position_c) = init_position(&mut svm, &admin, fx.market, borrower_c.pubkey());

    // --- Set EVERY protocol pause bit and EVERY market pause bit. ---
    set_protocol_pause(&mut svm, &admin, PAUSE_ALL_BITS).expect("set every protocol pause bit");
    set_market_pause(&mut svm, &admin, fx.market, PAUSE_ALL_BITS)
        .expect("set every market pause bit");
    let (protocol_key, _) = protocol_pda();
    assert_eq!(fetch_protocol(&svm, &protocol_key).paused, PAUSE_ALL_BITS);
    assert_eq!(fetch_market(&svm, &fx.market).paused, PAUSE_ALL_BITS);

    // --- repay succeeds fully paused. ---
    repay(
        &mut svm,
        &borrower_a,
        fx.market,
        position_a,
        fx.fee_position,
        fx.loan_vault,
        borrower_a_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        0,
        u128::MAX, // full repay: burn every outstanding borrow share
    )
    .expect("repay must succeed with every pause bit set (INV-ADM-04)");
    let after_repay = fetch_position(&svm, &position_a);
    assert_eq!(
        after_repay.borrow_shares, 0,
        "full repay must zero out debt"
    );

    // --- deposit_collateral succeeds fully paused. ---
    deposit_collateral(
        &mut svm,
        &borrower_a,
        fx.market,
        position_a,
        fx.collateral_vault,
        borrower_a_collateral_ata,
        fx.collateral_mint,
        spl_token_interface::ID,
        1_000_000_000,
    )
    .expect("deposit_collateral must succeed with every pause bit set (INV-ADM-04)");

    // --- absorb_bad_debt succeeds fully paused, on a real (collateral==0, debt>0) position. ---
    absorb_bad_debt(&mut svm, &admin, fx.market, position_b, fx.fee_position)
        .expect("absorb_bad_debt must succeed with every pause bit set (INV-ADM-04)");
    let after_absorb = fetch_position(&svm, &position_b);
    assert_eq!(
        after_absorb.borrow_shares, 0,
        "bad debt must be fully absorbed"
    );

    // --- close_position succeeds fully paused, on an already-empty position. ---
    close_position(&mut svm, &borrower_c, fx.market, position_c)
        .expect("close_position must succeed with every pause bit set (INV-ADM-04)");

    // --- Meanwhile every risk-increasing instruction still fails while fully paused. ---
    let result = supply(
        &mut svm,
        &lender,
        fx.market,
        lender_position,
        fx.fee_position,
        fx.loan_vault,
        lender_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        1_000_000,
        0,
    );
    assert_aegis_error(&result, AegisError::OperationPaused);

    let n = now(&svm);
    let (c, l) = valid_prices(&mut svm, &mut seeds, n);
    let result = borrow(
        &mut svm,
        &borrower_a,
        fx.market,
        position_a,
        fx.fee_position,
        fx.loan_vault,
        borrower_a_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        c,
        l,
        1_000_000,
        0,
    );
    assert_aegis_error(&result, AegisError::OperationPaused);

    invariants::assert_inv_cus_01(&svm, &fx.market);
}

// Non-vacuity (item 69): confirm that if the pause guard were wired into a safety-exit
// instruction, this exact test would start failing -- i.e. this test is not vacuously passing
// because the fixture happens not to reach the paused check. Implemented as a direct call to the
// shared guard function with the fixture's own real pause state, proving the guard WOULD reject
// these calls if it were consulted; the instructions above prove it genuinely is not consulted.
#[test]
fn non_vacuity_pause_guard_would_reject_these_operations_if_it_were_consulted() {
    let paused = PAUSE_ALL_BITS;
    for bit in [PAUSE_SUPPLY, PAUSE_BORROW, PAUSE_WITHDRAW, PAUSE_LIQUIDATE] {
        let result = aegis::guards::require_pause_bit_clear(
            paused,
            paused,
            bit,
            AegisError::OperationPaused,
        );
        assert!(
            result.is_err(),
            "the guard itself must reject when consulted -- proving repay/deposit_collateral/\
             absorb_bad_debt/close_position succeed only because they never call it, not because \
             the guard is somehow toothless"
        );
    }
}
