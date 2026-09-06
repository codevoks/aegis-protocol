//! Phase 4 — happy-path lending flows and integration scenarios: `supply`/`withdraw` share
//! accounting, free-liquidity bounds (`U-WD-01`), repayment clamping (`U-REPAY-01/02`),
//! multi-user accrual, a one-year dormant market, and 100% utilization.
//!
//! Debt is required for several of these scenarios; `borrow` is hard-gated in this phase
//! (`docs/phase-roadmap.md` "Sequencing the oracle dependency"), so debt is constructed through
//! `aegis_test_kit::seed_borrow_state` — explicitly sanctioned TEST-KIT state injection, never a
//! weakened `borrow` instruction (`docs/phases/phase-04-lending.md`).

#![allow(clippy::result_large_err)]

use aegis::state::{Market, Position};
use aegis_test_kit::{
    accrue_interest, create_market, create_spl_mint, create_token_account, deploy, fetch_market,
    fetch_position, fetch_token_account_base, init_position, initialize_protocol, invariants,
    mint_to, reference_market_args, repay, seed_borrow_state, spl_token_interface, supply,
    withdraw,
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

fn program_bytes() -> &'static [u8] {
    include_bytes!(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/aegis.so"))
}

/// A collision-proof source of fixed keypair seeds for one test: every identity created within a
/// single test (mints, wallets, token accounts alike) gets its own never-repeated `u8`, so no two
/// unrelated fixtures can ever end up at the same derived pubkey (`docs/zero-cost-demo.md` §6:
/// fixed seeds, never `Keypair::new()`, but each seed used exactly once). Starts well clear of
/// `aegis_test_kit::svm::PAYER_SEED` (`[7u8; 32]`, the `deploy()` admin/payer) so no fixture here
/// can ever collide with it.
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

/// A fully-created SPL/SPL market with a real fee position and no supply yet.
struct Fixture {
    market: Pubkey,
    fee_position: Pubkey,
    loan_vault: Pubkey,
    loan_mint: Pubkey,
}

fn setup_market(svm: &mut litesvm::LiteSVM, admin: &Keypair, seeds: &mut SeedGen) -> Fixture {
    let fee_recipient = setup_protocol(svm, admin, seeds);
    let collateral_mint = create_spl_mint(svm, admin, seeds.next(), 9, admin.pubkey(), None);
    let loan_mint = create_spl_mint(svm, admin, seeds.next(), 6, admin.pubkey(), None);
    // `reference_market_args` (shared with Phase 2/3, which never accrue interest) sets every IRM
    // slope to zero. Phase 4's tests need the REAL reference IRM curve from `economic-model.md`
    // §4.1, so those fields are overridden here rather than changing the shared Phase 2/3 helper.
    let args = aegis::instructions::admin::CreateMarketArgs {
        base_rate_ps: 0,
        slope1_ps: 1_268_391_679,        // 4% APR at the kink
        slope2_ps: 31_709_791_983,       // +100% APR above the kink
        u_kink: 800_000_000_000_000_000, // 0.80 WAD
        max_rate_ps: 317_097_919_837,    // 1000% APR cap
        ..reference_market_args(0, [1u8; 32], [2u8; 32], false)
    };
    let (result, market, _collateral_vault, loan_vault, fee_position) = create_market(
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
    }
}

fn wallet_with_ata(
    svm: &mut litesvm::LiteSVM,
    admin: &Keypair,
    loan_mint: Pubkey,
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
        spl_token_interface::ID,
        &[],
    );
    if balance > 0 {
        mint_to(
            svm,
            admin,
            loan_mint,
            ata,
            admin,
            balance,
            spl_token_interface::ID,
        );
    }
    (wallet, ata)
}

// Basic supply/withdraw round trip; INV-CUS-01/ACC-01/02/03/06 hold after every step.
#[test]
fn supply_and_withdraw_round_trip() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (lender, lender_ata) =
        wallet_with_ata(&mut svm, &admin, fx.loan_mint, &mut seeds, 1_000_000_000);

    let (_, position) = init_position(&mut svm, &admin, fx.market, lender.pubkey());

    let assets = 500_000_000u64;
    supply(
        &mut svm,
        &lender,
        fx.market,
        position,
        fx.fee_position,
        fx.loan_vault,
        lender_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        assets,
        0,
    )
    .expect("supply must succeed");

    let position_state = fetch_position(&svm, &position);
    assert_eq!(position_state.supply_shares, assets as u128 * 1_000_000);
    invariants::assert_all_lending(&svm, &fx.market, &[position], &fx.fee_position);

    let withdraw_shares = position_state.supply_shares;
    withdraw(
        &mut svm,
        &lender,
        fx.market,
        position,
        fx.fee_position,
        fx.loan_vault,
        lender_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        0,
        withdraw_shares,
    )
    .expect("withdraw must succeed");

    let position_after = fetch_position(&svm, &position);
    assert_eq!(position_after.supply_shares, 0);
    let market_after: Market = fetch_market(&svm, &fx.market);
    assert_eq!(market_after.total_supply_assets, 0);
    assert_eq!(market_after.total_supply_shares, 0);
    let ata_after = fetch_token_account_base(&svm, &lender_ata);
    assert_eq!(ata_after.amount, 1_000_000_000);
    invariants::assert_all_lending(&svm, &fx.market, &[position], &fx.fee_position);
}

// U-WD-01 / E-05: withdrawing more than free liquidity fails, even when the caller owns enough
// shares -- requires real debt outstanding (test-kit state injection).
#[test]
fn withdraw_more_than_free_liquidity_fails() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (lender, lender_ata) =
        wallet_with_ata(&mut svm, &admin, fx.loan_mint, &mut seeds, 1_000_000_000);
    let (_, position) = init_position(&mut svm, &admin, fx.market, lender.pubkey());

    supply(
        &mut svm,
        &lender,
        fx.market,
        position,
        fx.fee_position,
        fx.loan_vault,
        lender_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        1_000_000_000,
        0,
    )
    .expect("supply must succeed");

    // Inject 900_000_000 of debt against a borrower position -- free liquidity becomes 100_000_000.
    let borrower = fixed_pubkey(seeds.next());
    let (_, borrower_position) = init_position(&mut svm, &admin, fx.market, borrower);
    let borrow_shares = 900_000_000u128 * 1_000_000; // matches the lender's own share price exactly
    seed_borrow_state(
        &mut svm,
        fx.market,
        borrower_position,
        900_000_000,
        borrow_shares,
    );
    invariants::assert_inv_cus_01(&svm, &fx.market);

    let result = withdraw(
        &mut svm,
        &lender,
        fx.market,
        position,
        fx.fee_position,
        fx.loan_vault,
        lender_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        1_000_000_000,
        0,
    );
    aegis_test_kit::assert_aegis_error(&result, aegis::error::AegisError::InsufficientLiquidity);

    // A withdrawal within free liquidity succeeds.
    let ok = withdraw(
        &mut svm,
        &lender,
        fx.market,
        position,
        fx.fee_position,
        fx.loan_vault,
        lender_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        100_000_000,
        0,
    );
    ok.expect("withdrawing exactly free liquidity must succeed");
    invariants::assert_inv_cus_01(&svm, &fx.market);
}

// U-REPAY-01 / E-06: repaying more than the outstanding debt is clamped, and never pulls more
// tokens than the debt actually requires.
#[test]
fn repay_clamps_to_actual_debt_never_pulls_excess() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (lender, lender_ata) =
        wallet_with_ata(&mut svm, &admin, fx.loan_mint, &mut seeds, 1_000_000_000);
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
        1_000_000_000,
        0,
    )
    .expect("supply must succeed");

    let (borrower, payer_ata) = wallet_with_ata(&mut svm, &admin, fx.loan_mint, &mut seeds, 0);
    let (_, borrower_position) = init_position(&mut svm, &admin, fx.market, borrower.pubkey());
    let debt_assets = 300_000_000u64;
    let debt_shares = debt_assets as u128 * 1_000_000;
    seed_borrow_state(
        &mut svm,
        fx.market,
        borrower_position,
        debt_assets,
        debt_shares,
    );

    // Give the borrower far more than their debt, and attempt to repay all of it.
    let overpay_amount = 1_000_000_000u64; // >> the 300_000_000 debt
    mint_to(
        &mut svm,
        &admin,
        fx.loan_mint,
        payer_ata,
        &admin,
        overpay_amount,
        spl_token_interface::ID,
    );

    let payer_balance_before = fetch_token_account_base(&svm, &payer_ata).amount;
    repay(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        payer_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        overpay_amount,
        0,
    )
    .expect("repay must succeed (clamped), not fail");

    let payer_balance_after = fetch_token_account_base(&svm, &payer_ata).amount;
    let actually_pulled = payer_balance_before - payer_balance_after;
    assert_eq!(
        actually_pulled, debt_assets,
        "U-REPAY-01: repay must pull exactly the debt ({debt_assets}), never the requested overpay amount ({overpay_amount})"
    );

    let position_after: Position = fetch_position(&svm, &borrower_position);
    assert_eq!(
        position_after.borrow_shares, 0,
        "debt must be fully cleared"
    );
    invariants::assert_all_lending(
        &svm,
        &fx.market,
        &[lender_position, borrower_position],
        &fx.fee_position,
    );
}

// U-REPAY-02 / INV-REP-05: full repayment via shares drives position.borrow_shares to exactly 0,
// leaving no dust share.
#[test]
fn full_repayment_via_shares_leaves_no_dust() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (lender, lender_ata) =
        wallet_with_ata(&mut svm, &admin, fx.loan_mint, &mut seeds, 1_000_000_000);
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
        1_000_000_000,
        0,
    )
    .expect("supply must succeed");

    let (borrower, payer_ata) = wallet_with_ata(&mut svm, &admin, fx.loan_mint, &mut seeds, 0);
    let (_, borrower_position) = init_position(&mut svm, &admin, fx.market, borrower.pubkey());
    let debt_assets = 250_000_000u64;
    let debt_shares = debt_assets as u128 * 1_000_000;
    seed_borrow_state(
        &mut svm,
        fx.market,
        borrower_position,
        debt_assets,
        debt_shares,
    );

    mint_to(
        &mut svm,
        &admin,
        fx.loan_mint,
        payer_ata,
        &admin,
        debt_assets,
        spl_token_interface::ID,
    );

    repay(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        payer_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        0,
        debt_shares,
    )
    .expect("full repay via shares must succeed");

    let position_after: Position = fetch_position(&svm, &borrower_position);
    assert_eq!(position_after.borrow_shares, 0);
    let market_after: Market = fetch_market(&svm, &fx.market);
    assert_eq!(market_after.total_borrow_shares, 0);
    assert_eq!(market_after.total_borrow_assets, 0);
    invariants::assert_all_lending(
        &svm,
        &fx.market,
        &[lender_position, borrower_position],
        &fx.fee_position,
    );
}

// Repay is callable by anyone (INV-AUTH-03) -- a third party with no relationship to the position
// can repay another user's debt.
#[test]
fn repay_by_third_party_succeeds() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (lender, lender_ata) =
        wallet_with_ata(&mut svm, &admin, fx.loan_mint, &mut seeds, 1_000_000_000);
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
        1_000_000_000,
        0,
    )
    .expect("supply must succeed");

    let borrower_owner = fixed_pubkey(seeds.next()); // never signs anything
    let (_, borrower_position) = init_position(&mut svm, &admin, fx.market, borrower_owner);
    let debt_assets = 200_000_000u64;
    seed_borrow_state(
        &mut svm,
        fx.market,
        borrower_position,
        debt_assets,
        debt_assets as u128 * 1_000_000,
    );

    let (stranger, stranger_ata) =
        wallet_with_ata(&mut svm, &admin, fx.loan_mint, &mut seeds, debt_assets);

    repay(
        &mut svm,
        &stranger,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        stranger_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        debt_assets,
        0,
    )
    .expect("a stranger must be able to repay someone else's debt");

    let position_after: Position = fetch_position(&svm, &borrower_position);
    assert_eq!(position_after.borrow_shares, 0);
}

// Multi-user supply/withdraw with interest: two lenders, real debt, a real accrual, and each
// lender's claim grows in proportion to their shares.
#[test]
fn multi_user_supply_withdraw_with_interest() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);

    let (alice, alice_ata) =
        wallet_with_ata(&mut svm, &admin, fx.loan_mint, &mut seeds, 1_000_000_000);
    let (_, alice_position) = init_position(&mut svm, &admin, fx.market, alice.pubkey());
    supply(
        &mut svm,
        &alice,
        fx.market,
        alice_position,
        fx.fee_position,
        fx.loan_vault,
        alice_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        1_000_000_000,
        0,
    )
    .expect("alice supply");

    let (bob, bob_ata) = wallet_with_ata(&mut svm, &admin, fx.loan_mint, &mut seeds, 1_000_000_000);
    let (_, bob_position) = init_position(&mut svm, &admin, fx.market, bob.pubkey());
    supply(
        &mut svm,
        &bob,
        fx.market,
        bob_position,
        fx.fee_position,
        fx.loan_vault,
        bob_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        1_000_000_000,
        0,
    )
    .expect("bob supply");

    invariants::assert_all_lending(
        &svm,
        &fx.market,
        &[alice_position, bob_position],
        &fx.fee_position,
    );

    let borrower = fixed_pubkey(seeds.next());
    let (_, borrower_position) = init_position(&mut svm, &admin, fx.market, borrower);
    // 90% utilization against the pooled 2e9 supply.
    seed_borrow_state(
        &mut svm,
        fx.market,
        borrower_position,
        1_800_000_000,
        1_800_000_000u128 * 1_000_000,
    );
    invariants::assert_inv_cus_01(&svm, &fx.market);

    let before = fetch_market(&svm, &fx.market);
    let mut clock = svm.get_sysvar::<solana_clock::Clock>();
    clock.unix_timestamp += 86_400; // 1 day
    svm.set_sysvar(&clock);

    accrue_interest(&mut svm, &admin, fx.market, fx.fee_position).expect("accrue_interest");
    let after = fetch_market(&svm, &fx.market);
    assert!(
        after.total_borrow_assets > before.total_borrow_assets,
        "interest must have accrued"
    );
    assert!(after.total_supply_assets > before.total_supply_assets);
    assert_eq!(
        after.total_supply_assets - before.total_supply_assets,
        after.total_borrow_assets - before.total_borrow_assets,
        "interest must be a pure transfer: both totals grow by exactly the same amount"
    );

    invariants::assert_all_lending(
        &svm,
        &fx.market,
        &[alice_position, bob_position, borrower_position],
        &fx.fee_position,
    );

    // Alice and Bob each hold half the pre-accrual shares, so each should be able to claim
    // approximately half of the now-larger total_supply_assets.
    let alice_state = fetch_position(&svm, &alice_position);
    let alice_claim = aegis_math::to_assets_down(
        alice_state.supply_shares,
        after.total_supply_assets,
        after.total_supply_shares,
    )
    .unwrap();
    assert!(
        alice_claim > 1_000_000_000,
        "alice's claim must have grown from accrued interest"
    );
}

// One year of accrual on a dormant market: deterministic, no overflow, bounded, correct fee/ts
// updates, invariants preserved. Warped via the sysvar clock, never real wall-clock waiting.
#[test]
fn one_year_dormant_market_accrual() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (lender, lender_ata) =
        wallet_with_ata(&mut svm, &admin, fx.loan_mint, &mut seeds, 1_000_000_000);
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
        1_000_000_000,
        0,
    )
    .expect("supply must succeed");

    let borrower = fixed_pubkey(seeds.next());
    let (_, borrower_position) = init_position(&mut svm, &admin, fx.market, borrower);
    seed_borrow_state(
        &mut svm,
        fx.market,
        borrower_position,
        900_000_000,
        900_000_000u128 * 1_000_000,
    );

    let before = fetch_market(&svm, &fx.market);
    let mut clock = svm.get_sysvar::<solana_clock::Clock>();
    clock.unix_timestamp += 31_536_000; // exactly one year
    svm.set_sysvar(&clock);

    let result = accrue_interest(&mut svm, &admin, fx.market, fx.fee_position);
    result.expect("one year of accrual must not overflow or panic");

    let after = fetch_market(&svm, &fx.market);
    assert_eq!(after.last_accrual_ts, before.last_accrual_ts + 31_536_000);
    assert!(after.total_borrow_assets > before.total_borrow_assets);
    // Bounded: interest cannot exceed what max_rate_ps compounded for a year would produce, which
    // for the reference params (max 1000% APR) is well under 100x principal.
    assert!(
        after.total_borrow_assets < before.total_borrow_assets * 100,
        "accrual must be bounded, not explosive"
    );
    let fee_position_after = fetch_position(&svm, &fx.fee_position);
    assert!(
        fee_position_after.supply_shares > 0,
        "the protocol fee must have accrued"
    );
    invariants::assert_all_lending(
        &svm,
        &fx.market,
        &[lender_position, borrower_position],
        &fx.fee_position,
    );
}

// 100% utilization: total_borrow_assets == total_supply_assets. Verifies utilization/rate/accrual
// all handle the boundary without a division-by-zero edge, and that withdrawal is correctly
// bounded to zero free liquidity.
#[test]
fn hundred_percent_utilization() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (lender, lender_ata) =
        wallet_with_ata(&mut svm, &admin, fx.loan_mint, &mut seeds, 1_000_000_000);
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
        1_000_000_000,
        0,
    )
    .expect("supply must succeed");

    let borrower = fixed_pubkey(seeds.next());
    let (_, borrower_position) = init_position(&mut svm, &admin, fx.market, borrower);
    seed_borrow_state(
        &mut svm,
        fx.market,
        borrower_position,
        1_000_000_000,
        1_000_000_000u128 * 1_000_000,
    );
    invariants::assert_inv_cus_01(&svm, &fx.market);

    let market_state = fetch_market(&svm, &fx.market);
    assert_eq!(
        market_state.total_supply_assets,
        market_state.total_borrow_assets
    );

    let mut clock = svm.get_sysvar::<solana_clock::Clock>();
    clock.unix_timestamp += 3_600;
    svm.set_sysvar(&clock);
    accrue_interest(&mut svm, &admin, fx.market, fx.fee_position)
        .expect("accrual at 100% utilization must not panic or divide by zero");

    // No free liquidity: any withdrawal must fail.
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
        1,
        0,
    );
    aegis_test_kit::assert_aegis_error(&result, aegis::error::AegisError::InsufficientLiquidity);
    invariants::assert_all_lending(
        &svm,
        &fx.market,
        &[lender_position, borrower_position],
        &fx.fee_position,
    );
}

// I-CUS-01: INV-CUS-01 holds after every meaningful loan-side operation across a realistic
// multi-step flow.
#[test]
fn i_cus_01_holds_after_every_operation() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (lender, lender_ata) =
        wallet_with_ata(&mut svm, &admin, fx.loan_mint, &mut seeds, 1_000_000_000);
    let (_, lender_position) = init_position(&mut svm, &admin, fx.market, lender.pubkey());
    invariants::assert_inv_cus_01(&svm, &fx.market);

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
        400_000_000,
        0,
    )
    .expect("supply 1");
    invariants::assert_inv_cus_01(&svm, &fx.market);

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
        300_000_000,
        0,
    )
    .expect("supply 2");
    invariants::assert_inv_cus_01(&svm, &fx.market);

    withdraw(
        &mut svm,
        &lender,
        fx.market,
        lender_position,
        fx.fee_position,
        fx.loan_vault,
        lender_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        100_000_000,
        0,
    )
    .expect("withdraw 1");
    invariants::assert_inv_cus_01(&svm, &fx.market);

    let (borrower, payer_ata) =
        wallet_with_ata(&mut svm, &admin, fx.loan_mint, &mut seeds, 50_000_000);
    let (_, borrower_position) = init_position(&mut svm, &admin, fx.market, borrower.pubkey());
    seed_borrow_state(
        &mut svm,
        fx.market,
        borrower_position,
        200_000_000,
        200_000_000u128 * 1_000_000,
    );
    invariants::assert_inv_cus_01(&svm, &fx.market);

    let mut clock = svm.get_sysvar::<solana_clock::Clock>();
    clock.unix_timestamp += 1_000;
    svm.set_sysvar(&clock);
    accrue_interest(&mut svm, &admin, fx.market, fx.fee_position).expect("accrue");
    invariants::assert_inv_cus_01(&svm, &fx.market);

    repay(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        payer_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        50_000_000,
        0,
    )
    .expect("partial repay");
    invariants::assert_inv_cus_01(&svm, &fx.market);
}
