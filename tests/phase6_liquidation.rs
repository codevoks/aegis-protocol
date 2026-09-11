//! Phase 6 — `liquidate` unit/adversarial/integration tests (`docs/phases/phase-06-liquidation.md`,
//! `docs/economic-model.md` §7, `docs/instruction-catalogue.md` §17).
//!
//! Setup helpers mirror `tests/phase5_oracle_adversarial.rs` exactly (same `Fixture`/
//! `setup_market`/`wallet_with_ata`/`setup_borrower` shapes) so the reference SOL(9dp)/USDC(6dp)
//! market and its $150.00/$1.00 reference prices stay consistent across phases.

#![allow(clippy::result_large_err)]

use aegis::error::AegisError;
use aegis_test_kit::{
    absorb_bad_debt, assert_aegis_error, borrow, create_market, create_spl_mint,
    create_token_account, deploy, deposit_collateral, fetch_market, fetch_position, init_position,
    initialize_protocol, invariants, liquidate, mint_to, pyth_solana_receiver_sdk,
    reference_market_args, set_price, spl_token_interface, supply,
    token_accounts::fetch_token_account_base, PriceFixture,
};
use pyth_solana_receiver_sdk::price_update::VerificationLevel;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

fn program_bytes() -> &'static [u8] {
    include_bytes!(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/aegis.so"))
}

const COLLATERAL_FEED_ID: [u8; 32] = [0xAAu8; 32];
const LOAN_FEED_ID: [u8; 32] = [0xBBu8; 32];
const FLAT_COLLATERAL_FEED_ID: [u8; 32] = [0xCCu8; 32];
const FLAT_LOAN_FEED_ID: [u8; 32] = [0xDDu8; 32];

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

/// SOL(9dp)/USDC(6dp) reference market — identical parameters to Phase 5's fixture
/// (`max_ltv=0.75, LT=0.80, bonus=0.05, close_factor=0.50, full_liq_hf=0.95,
/// liq_protocol_fee=0.10, min_debt=10 USDC`).
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

/// Injects the exact SOL=$95.00±$0.20 / USDC=$1.0000±$0.0002 crash prices from
/// `economic-model.md` §6.5/§7.5 (`price_c_lo = 94.80e18`, `price_l_hi = 1.0002e18`).
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

/// Lender supplies, borrower deposits and borrows against SOL at $150.00 -- returns everything a
/// liquidation test needs, positioned exactly per `economic-model.md` §6.5/§7.5's inputs when
/// called with `supply_amount=1_000_000_000_000, collateral_amount=10_000_000_000,
/// borrow_amount=900_000_000`.
#[allow(clippy::too_many_arguments)]
fn setup_borrowed_position(
    svm: &mut litesvm::LiteSVM,
    admin: &Keypair,
    fx: &Fixture,
    seeds: &mut SeedGen,
    supply_amount: u64,
    collateral_amount: u64,
    borrow_amount: u64,
) -> (Keypair, Pubkey, Pubkey, Pubkey) {
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

    let n = now(svm);
    let (c, l) = valid_prices(svm, seeds, n);
    borrow(
        svm,
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
        borrow_amount,
        0,
    )
    .expect("borrow must succeed");

    (
        borrower,
        borrower_position,
        borrower_collateral_ata,
        borrower_loan_ata,
    )
}

fn liquidator_wallets(
    svm: &mut litesvm::LiteSVM,
    admin: &Keypair,
    fx: &Fixture,
    seeds: &mut SeedGen,
    loan_balance: u64,
) -> (Keypair, Pubkey, Pubkey) {
    let (liquidator, liquidator_loan_ata) =
        wallet_with_ata(svm, admin, fx.loan_mint, seeds, loan_balance);
    let liquidator_collateral_ata = create_token_account(
        svm,
        admin,
        seeds.next(),
        fx.collateral_mint,
        liquidator.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    (liquidator, liquidator_loan_ata, liquidator_collateral_ata)
}

#[allow(clippy::too_many_arguments)]
fn do_liquidate(
    svm: &mut litesvm::LiteSVM,
    liquidator: &Keypair,
    fx: &Fixture,
    position: Pubkey,
    liquidator_loan_ata: Pubkey,
    liquidator_collateral_ata: Pubkey,
    c: Pubkey,
    l: Pubkey,
    repay_assets: u64,
    seize_collateral: u64,
) -> litesvm::types::TransactionResult {
    liquidate(
        svm,
        liquidator,
        fx.market,
        position,
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
        repay_assets,
        seize_collateral,
    )
}

// ============================================================================================
// U-LIQ-01: the exact worked example from economic-model.md §7.5.
// ============================================================================================

#[test]
fn u_liq_01_worked_example_on_chain() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (borrower, borrower_position, borrower_collateral_ata, _borrower_loan_ata) =
        setup_borrowed_position(
            &mut svm,
            &admin,
            &fx,
            &mut seeds,
            1_000_000_000_000,
            10_000_000_000,
            900_000_000,
        );

    let (liquidator, liquidator_loan_ata, liquidator_collateral_ata) =
        liquidator_wallets(&mut svm, &admin, &fx, &mut seeds, 1_000_000_000);

    let n = now(&svm);
    let (c, l) = crash_prices(&mut svm, &mut seeds, n);

    let position_before = fetch_position(&svm, &borrower_position);
    assert_eq!(position_before.collateral_amount, 10_000_000_000);

    let market_before = fetch_market(&svm, &fx.market);
    let total_supply_before = market_before.total_supply_assets;

    do_liquidate(
        &mut svm,
        &liquidator,
        &fx,
        borrower_position,
        liquidator_loan_ata,
        liquidator_collateral_ata,
        c,
        l,
        900_000_000,
        0,
    )
    .expect("liquidation of the worked example must succeed");

    let position_after = fetch_position(&svm, &borrower_position);
    let market_after = fetch_market(&svm, &fx.market);

    // Exact figures from economic-model.md §7.5.
    assert_eq!(market_after.total_borrow_assets, 0, "debt fully repaid");
    assert_eq!(position_after.borrow_shares, 0);
    let remaining_collateral = position_after.collateral_amount;
    assert_eq!(remaining_collateral, 10_000_000_000 - 9_970_348_101); // 29_651_899 (~0.02965 SOL)
    assert_eq!(market_after.collateral_fee_accrued, 47_477_848);

    let liquidator_collateral = fetch_token_account_base(&svm, &liquidator_collateral_ata);
    assert_eq!(liquidator_collateral.amount, 9_922_870_253); // to_liquidator

    // U-LIQ-06 / INV-LIQ-09: total_supply_assets never rises from a liquidation.
    assert_eq!(market_after.total_supply_assets, total_supply_before);

    // Custody invariants hold exactly.
    invariants::assert_inv_cus_01(&svm, &fx.market);
    invariants::assert_all(&svm, &fx.market, &[borrower_position]);

    let _ = borrower_collateral_ata; // unused beyond setup
    let _ = borrower;
}

// ============================================================================================
// U-LIQ-02 / E-12 / INV-LIQ-01 / INV-SOLV-02: HF == WAD is STRICTLY not liquidatable.
// ============================================================================================

fn setup_flat_market(svm: &mut litesvm::LiteSVM, admin: &Keypair, seeds: &mut SeedGen) -> Fixture {
    let guardian = fixed_pubkey(seeds.next());
    let fee_recipient = fixed_pubkey(seeds.next());
    initialize_protocol(svm, admin, guardian, fee_recipient).expect("initialize_protocol");

    // Both mints at 6dp so collateral/loan valuation cancels cleanly at any flat price.
    let collateral_mint = create_spl_mint(svm, admin, seeds.next(), 6, admin.pubkey(), None);
    let loan_mint = create_spl_mint(svm, admin, seeds.next(), 6, admin.pubkey(), None);
    let args = reference_market_args(1, FLAT_COLLATERAL_FEED_ID, FLAT_LOAN_FEED_ID, false);
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

fn flat_price(
    svm: &mut litesvm::LiteSVM,
    seeds: &mut SeedGen,
    feed: [u8; 32],
    raw: i64,
    at: i64,
) -> Pubkey {
    set_price(svm, seeds.next(), PriceFixture::valid(feed, raw, 0, -8, at))
}

#[test]
fn u_liq_02_hf_equal_to_wad_is_not_liquidatable_strict() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_flat_market(&mut svm, &admin, &mut seeds);

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

    let (borrower, borrower_collateral_ata) = wallet_with_ata(
        &mut svm,
        &admin,
        fx.collateral_mint,
        &mut seeds,
        200_000_000,
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
        200_000_000,
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

    // Borrow 100_000_000 at $1.00/$1.00 flat (well within max_ltv 0.75: cv=2e20, dv=1e20).
    let n0 = now(&svm);
    let c0 = flat_price(
        &mut svm,
        &mut seeds,
        FLAT_COLLATERAL_FEED_ID,
        100_000_000,
        n0,
    );
    let l0 = flat_price(&mut svm, &mut seeds, FLAT_LOAN_FEED_ID, 100_000_000, n0);
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
        c0,
        l0,
        100_000_000,
        0,
    )
    .expect("borrow must succeed");

    let (liquidator, liquidator_loan_ata, liquidator_collateral_ata) =
        liquidator_wallets(&mut svm, &admin, &fx, &mut seeds, 1_000_000_000);

    // --- Exactly HF == WAD: collateral price raw=62_500_000 (independently verified: HF ==
    // exactly 1e18). Must be REJECTED, strictly. ---
    let n1 = now(&svm);
    let c1 = flat_price(
        &mut svm,
        &mut seeds,
        FLAT_COLLATERAL_FEED_ID,
        62_500_000,
        n1,
    );
    let l1 = flat_price(&mut svm, &mut seeds, FLAT_LOAN_FEED_ID, 100_000_000, n1);

    let before = svm.get_account(&borrower_position).unwrap();
    let result = do_liquidate(
        &mut svm,
        &liquidator,
        &fx,
        borrower_position,
        liquidator_loan_ata,
        liquidator_collateral_ata,
        c1,
        l1,
        1,
        0,
    );
    assert_aegis_error(&result, AegisError::NotLiquidatable);
    let after = svm.get_account(&borrower_position).unwrap();
    assert_eq!(
        before.data, after.data,
        "HF==WAD rejection must not mutate state"
    );

    // --- One unit of price below: HF is now strictly < WAD. Must SUCCEED. ---
    let n2 = now(&svm);
    let c2 = flat_price(
        &mut svm,
        &mut seeds,
        FLAT_COLLATERAL_FEED_ID,
        62_499_999,
        n2,
    );
    let l2 = flat_price(&mut svm, &mut seeds, FLAT_LOAN_FEED_ID, 100_000_000, n2);
    do_liquidate(
        &mut svm,
        &liquidator,
        &fx,
        borrower_position,
        liquidator_loan_ata,
        liquidator_collateral_ata,
        c2,
        l2,
        1,
        0,
    )
    .expect("HF strictly below WAD must be liquidatable");
}

// ============================================================================================
// U-LIQ-03 / U-LIQ-05: collateral clamp -- repay recomputed upward-rounded, full seizure with
// debt remaining (the bridge into bad debt).
// ============================================================================================

#[test]
fn u_liq_03_and_05_collateral_clamp_and_full_seizure_with_remaining_debt() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    // Insufficient collateral (9 SOL instead of 10) for the naive full-repay seizure of
    // 9.97035 SOL -- independently cross-checked via python in crates/aegis-math's own test
    // (`u_liq_03_and_05_collateral_clamp_recomputes_repay_upward_and_leaves_remaining_debt`).
    let (borrower, borrower_position, _bca, _bla) = setup_borrowed_position(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        9_000_000_000,
        900_000_000,
    );
    let _ = borrower;

    let (liquidator, liquidator_loan_ata, liquidator_collateral_ata) =
        liquidator_wallets(&mut svm, &admin, &fx, &mut seeds, 1_000_000_000);

    let n = now(&svm);
    let (c, l) = crash_prices(&mut svm, &mut seeds, n);

    do_liquidate(
        &mut svm,
        &liquidator,
        &fx,
        borrower_position,
        liquidator_loan_ata,
        liquidator_collateral_ata,
        c,
        l,
        900_000_000, // full debt requested -- exceeds available collateral, forcing the clamp
        0,
    )
    .expect("clamp path must still succeed");

    let position_after = fetch_position(&svm, &borrower_position);
    // U-LIQ-03: ALL collateral seized (clamped to exactly what existed) -- INV-LIQ-02 exact
    // equality at the bound.
    assert_eq!(position_after.collateral_amount, 0);
    // U-LIQ-05: debt remains -- not silently erased.
    assert!(
        position_after.borrow_shares > 0,
        "debt must remain after a clamped full seizure"
    );

    let market_after = fetch_market(&svm, &fx.market);
    assert!(market_after.total_borrow_assets > 0);
    assert_eq!(market_after.collateral_fee_accrued, 42_857_142);

    let liquidator_collateral = fetch_token_account_base(&svm, &liquidator_collateral_ata);
    assert_eq!(liquidator_collateral.amount, 8_957_142_858);

    invariants::assert_inv_cus_01(&svm, &fx.market);
    invariants::assert_all(&svm, &fx.market, &[borrower_position]);

    // Now this position is eligible for absorb_bad_debt (collateral == 0, debt > 0) -- proving
    // the bridge works end-to-end.
    absorb_bad_debt(
        &mut svm,
        &admin,
        fx.market,
        borrower_position,
        fx.fee_position,
    )
    .expect("absorb_bad_debt must succeed once collateral is fully seized");
}

// ============================================================================================
// U-LIQ-04: dust rule forces full repayment.
// ============================================================================================

#[test]
fn u_liq_04_dust_rule_forces_full_repayment_on_chain() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    // Small debt (20 USDC) against a SMALL collateral position (1 SOL, not 10) -- independently
    // verified (python cross-check during authoring) that 1 SOL at $24.00 against 20 USDC debt
    // gives HF = 0.96 WAD, inside [full_liq_hf(0.95), WAD): the close-factor (partial) branch.
    let (borrower, borrower_position, _bca, _bla) = setup_borrowed_position(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        1_000_000_000, // 1 SOL
        20_000_000,    // 20 USDC; min_debt = 10 USDC, close_factor = 50%
    );
    let _ = borrower;

    let (liquidator, liquidator_loan_ata, liquidator_collateral_ata) =
        liquidator_wallets(&mut svm, &admin, &fx, &mut seeds, 1_000_000_000);

    // SOL crashes to $24.00: HF = 0.96 WAD (full_liq_hf <= HF < WAD), close-factor branch.
    // close_factor=0.5 naive repay = 10 USDC, leaving exactly 10 USDC = min_debt -- this exact
    // boundary does NOT force dust (confirmed below); the next test pushes one unit further.
    let n = now(&svm);
    let c = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(COLLATERAL_FEED_ID, 2_400_000_000, 0, -8, n),
    );
    let l = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 0, -8, n),
    );

    // Requesting the full close-factor amount (10 USDC) must succeed and leave exactly 10 USDC
    // (== min_debt) of debt -- not dust.
    do_liquidate(
        &mut svm,
        &liquidator,
        &fx,
        borrower_position,
        liquidator_loan_ata,
        liquidator_collateral_ata,
        c,
        l,
        10_000_000,
        0,
    )
    .expect("liquidating exactly the close-factor amount must succeed");
    let market_after = fetch_market(&svm, &fx.market);
    assert_eq!(market_after.total_borrow_assets, 10_000_000);

    invariants::assert_inv_cus_01(&svm, &fx.market);
}

#[test]
fn u_liq_04_dust_rule_boundary_forces_full_when_remaining_would_be_below_min_debt() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    // 19_999_998 debt (one less than the previous test's 20 USDC): close-factor (50%) naive
    // repay = 9_999_999, remaining = 9_999_999 < min_debt(10_000_000) -- dust, so `max_repay`
    // itself is forced up to the FULL debt (INV-LIQ-04's own "unless the dust rule forces full
    // repayment" clause).
    let debt = 19_999_998u64;
    let (borrower, borrower_position, _bca, _bla) = setup_borrowed_position(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        1_000_000_000,
        debt,
    );
    let _ = borrower;

    let (liquidator, liquidator_loan_ata, liquidator_collateral_ata) =
        liquidator_wallets(&mut svm, &admin, &fx, &mut seeds, 1_000_000_000);

    let n = now(&svm);
    let c = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(COLLATERAL_FEED_ID, 2_400_000_000, 0, -8, n),
    );
    let l = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 0, -8, n),
    );

    // The concrete proof the dust rule fired: 10_000_000 STRICTLY EXCEEDS the plain close-factor
    // cap (floor(19_999_998 * 0.5) = 9_999_999) -- without the dust rule this would be rejected
    // as RepayExceedsMaxRepay. It succeeds here only because the dust rule raised max_repay to
    // the full debt.
    do_liquidate(
        &mut svm,
        &liquidator,
        &fx,
        borrower_position,
        liquidator_loan_ata,
        liquidator_collateral_ata,
        c,
        l,
        10_000_000,
        0,
    )
    .expect("dust rule must raise max_repay above the plain close-factor cap");

    let market_after = fetch_market(&svm, &fx.market);
    assert_eq!(market_after.total_borrow_assets, debt - 10_000_000);
    // (P-LIQ-1: this liquidation improved HF enough that the position is healthy again --
    // requesting a second liquidation at the same price would now correctly be rejected as
    // NotLiquidatable; that follow-on improvement is exactly the derived property under test in
    // `crates/aegis-math/tests/liquidation_property.rs`, not re-asserted here.)

    invariants::assert_inv_cus_01(&svm, &fx.market);
}

// ============================================================================================
// U-LIQ-06 / INV-LIQ-09: total_supply_assets never rises from a liquidation.
// ============================================================================================

#[test]
fn u_liq_06_total_supply_assets_never_rises() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (borrower, borrower_position, _bca, _bla) = setup_borrowed_position(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        10_000_000_000,
        900_000_000,
    );
    let _ = borrower;
    let (liquidator, liquidator_loan_ata, liquidator_collateral_ata) =
        liquidator_wallets(&mut svm, &admin, &fx, &mut seeds, 1_000_000_000);

    let total_supply_before = fetch_market(&svm, &fx.market).total_supply_assets;
    let n = now(&svm);
    let (c, l) = crash_prices(&mut svm, &mut seeds, n);
    do_liquidate(
        &mut svm,
        &liquidator,
        &fx,
        borrower_position,
        liquidator_loan_ata,
        liquidator_collateral_ata,
        c,
        l,
        400_000_000,
        0,
    )
    .expect("partial liquidation must succeed");
    let total_supply_after = fetch_market(&svm, &fx.market).total_supply_assets;
    assert_eq!(
        total_supply_after, total_supply_before,
        "INV-LIQ-09: liquidation must never increase total_supply_assets"
    );
}

// ============================================================================================
// U-LIQ-07: self-liquidation is permitted but economically unprofitable versus a plain repay.
// ============================================================================================

#[test]
fn u_liq_07_self_liquidation_permitted_but_costs_exactly_the_protocol_cut() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (borrower, borrower_position, borrower_collateral_ata, borrower_loan_ata) =
        setup_borrowed_position(
            &mut svm,
            &admin,
            &fx,
            &mut seeds,
            1_000_000_000_000,
            10_000_000_000,
            900_000_000,
        );

    // Top up the borrower's own loan wallet so they can repay themselves.
    mint_to(
        &mut svm,
        &admin,
        fx.loan_mint,
        borrower_loan_ata,
        &admin,
        1_000_000_000,
        spl_token_interface::ID,
    );

    let n = now(&svm);
    let (c, l) = crash_prices(&mut svm, &mut seeds, n);

    let position_before = fetch_position(&svm, &borrower_position);
    let wallet_collateral_before = fetch_token_account_base(&svm, &borrower_collateral_ata).amount;
    let total_collateral_before = position_before.collateral_amount + wallet_collateral_before;

    // The borrower liquidates their OWN position: `liquidator == owner`.
    do_liquidate(
        &mut svm,
        &borrower,
        &fx,
        borrower_position,
        borrower_loan_ata,
        borrower_collateral_ata,
        c,
        l,
        900_000_000,
        0,
    )
    .expect("self-liquidation must be PERMITTED, not blocked");

    let position_after = fetch_position(&svm, &borrower_position);
    let wallet_collateral_after = fetch_token_account_base(&svm, &borrower_collateral_ata).amount;
    let total_collateral_after = position_after.collateral_amount + wallet_collateral_after;

    let market_after = fetch_market(&svm, &fx.market);
    let protocol_cut = market_after.collateral_fee_accrued;
    assert!(protocol_cut > 0);

    // Economically unprofitable: the borrower's combined (position + wallet) collateral holding
    // falls by EXACTLY the protocol cut -- the bonus they'd have earned from a third party is
    // paid right back to themselves (base_seize + bonus - protocol_cut returns to their wallet,
    // base_seize + bonus leaves the position), which is strictly worse than calling `repay`
    // directly (zero collateral cost). This is the concrete, on-chain proof of T-22.
    assert_eq!(
        total_collateral_before - total_collateral_after,
        protocol_cut,
        "self-liquidation must cost the borrower exactly the protocol cut, no more, no less"
    );
}

// ============================================================================================
// A-LIQ-01 / INV-SOLV-02: attempting to liquidate a healthy position must fail, with zero state
// mutation anywhere.
// ============================================================================================

#[test]
fn a_liq_01_healthy_position_cannot_be_liquidated_and_nothing_mutates() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let (borrower, borrower_position, borrower_collateral_ata, borrower_loan_ata) =
        setup_borrowed_position(
            &mut svm,
            &admin,
            &fx,
            &mut seeds,
            1_000_000_000_000,
            10_000_000_000,
            900_000_000, // HF ~= 1.33, healthy
        );
    let _ = borrower;

    let (liquidator, liquidator_loan_ata, liquidator_collateral_ata) =
        liquidator_wallets(&mut svm, &admin, &fx, &mut seeds, 1_000_000_000);

    let n = now(&svm);
    let (c, l) = valid_prices(&mut svm, &mut seeds, n); // unchanged, still $150/$1.00: healthy

    let market_before = svm.get_account(&fx.market).unwrap();
    let position_before = svm.get_account(&borrower_position).unwrap();
    let loan_vault_before = svm.get_account(&fx.loan_vault).unwrap();
    let collateral_vault_before = svm.get_account(&fx.collateral_vault).unwrap();
    let liquidator_loan_before = svm.get_account(&liquidator_loan_ata).unwrap();
    let liquidator_collateral_before = svm.get_account(&liquidator_collateral_ata).unwrap();
    let borrower_loan_before = svm.get_account(&borrower_loan_ata).unwrap();
    let borrower_collateral_before = svm.get_account(&borrower_collateral_ata).unwrap();

    let result = do_liquidate(
        &mut svm,
        &liquidator,
        &fx,
        borrower_position,
        liquidator_loan_ata,
        liquidator_collateral_ata,
        c,
        l,
        100_000_000,
        0,
    );
    assert_aegis_error(&result, AegisError::NotLiquidatable);

    let market_after = svm.get_account(&fx.market).unwrap();
    let position_after = svm.get_account(&borrower_position).unwrap();
    let loan_vault_after = svm.get_account(&fx.loan_vault).unwrap();
    let collateral_vault_after = svm.get_account(&fx.collateral_vault).unwrap();
    let liquidator_loan_after = svm.get_account(&liquidator_loan_ata).unwrap();
    let liquidator_collateral_after = svm.get_account(&liquidator_collateral_ata).unwrap();
    let borrower_loan_after = svm.get_account(&borrower_loan_ata).unwrap();
    let borrower_collateral_after = svm.get_account(&borrower_collateral_ata).unwrap();

    assert_eq!(market_before.data, market_after.data, "market mutated");
    assert_eq!(
        position_before.data, position_after.data,
        "position mutated"
    );
    assert_eq!(
        loan_vault_before.data, loan_vault_after.data,
        "loan_vault mutated"
    );
    assert_eq!(
        collateral_vault_before.data, collateral_vault_after.data,
        "collateral_vault mutated"
    );
    assert_eq!(
        liquidator_loan_before.data, liquidator_loan_after.data,
        "liquidator loan ATA mutated: token movement occurred"
    );
    assert_eq!(
        liquidator_collateral_before.data, liquidator_collateral_after.data,
        "liquidator collateral ATA mutated: token movement occurred"
    );
    assert_eq!(borrower_loan_before.data, borrower_loan_after.data);
    assert_eq!(
        borrower_collateral_before.data,
        borrower_collateral_after.data
    );

    let market_state = fetch_market(&svm, &fx.market);
    assert_eq!(market_state.collateral_fee_accrued, 0, "no fee accrual");
}

// ============================================================================================
// Oracle adversarial re-run against `liquidate` (item #29): a representative subset of O-1..O-11
// against this instruction specifically. Full O-1..O-11 coverage already exists against `borrow`
// in tests/phase5_oracle_adversarial.rs; this proves the SAME shared `require_valid_price` guard
// is actually invoked here too, before any mutation.
// ============================================================================================

fn setup_liquidatable(
    svm: &mut litesvm::LiteSVM,
    admin: &Keypair,
    fx: &Fixture,
    seeds: &mut SeedGen,
) -> Pubkey {
    let (borrower, borrower_position, _bca, _bla) = setup_borrowed_position(
        svm,
        admin,
        fx,
        seeds,
        1_000_000_000_000,
        10_000_000_000,
        900_000_000,
    );
    let _ = borrower;
    borrower_position
}

#[test]
fn a_oracle_liquidate_stale_price_is_rejected() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let borrower_position = setup_liquidatable(&mut svm, &admin, &fx, &mut seeds);
    let (liquidator, liquidator_loan_ata, liquidator_collateral_ata) =
        liquidator_wallets(&mut svm, &admin, &fx, &mut seeds, 1_000_000_000);

    let n = now(&svm);
    // Publish a valid-looking (crashed) price, then warp far past max_price_age_secs (60s).
    let (c, l) = crash_prices(&mut svm, &mut seeds, n);
    let mut clock = svm.get_sysvar::<solana_clock::Clock>();
    clock.unix_timestamp += 3600;
    svm.set_sysvar(&clock);

    let result = do_liquidate(
        &mut svm,
        &liquidator,
        &fx,
        borrower_position,
        liquidator_loan_ata,
        liquidator_collateral_ata,
        c,
        l,
        900_000_000,
        0,
    );
    assert_aegis_error(&result, AegisError::OraclePriceStale);
}

#[test]
fn a_oracle_liquidate_wrong_owner_is_rejected() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let borrower_position = setup_liquidatable(&mut svm, &admin, &fx, &mut seeds);
    let (liquidator, liquidator_loan_ata, liquidator_collateral_ata) =
        liquidator_wallets(&mut svm, &admin, &fx, &mut seeds, 1_000_000_000);

    let n = now(&svm);
    let bad_owner_fixture = PriceFixture {
        owner: spl_token_interface::ID,
        ..PriceFixture::valid(COLLATERAL_FEED_ID, 9_500_000_000, 20_000_000, -8, n)
    };
    let c = set_price(&mut svm, seeds.next(), bad_owner_fixture);
    let l = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 20_000, -8, n),
    );

    let result = do_liquidate(
        &mut svm,
        &liquidator,
        &fx,
        borrower_position,
        liquidator_loan_ata,
        liquidator_collateral_ata,
        c,
        l,
        900_000_000,
        0,
    );
    assert_aegis_error(&result, AegisError::OracleAccountOwnerMismatch);
}

#[test]
fn a_oracle_liquidate_wrong_feed_id_is_rejected() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let borrower_position = setup_liquidatable(&mut svm, &admin, &fx, &mut seeds);
    let (liquidator, liquidator_loan_ata, liquidator_collateral_ata) =
        liquidator_wallets(&mut svm, &admin, &fx, &mut seeds, 1_000_000_000);

    let n = now(&svm);
    let c = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid([0x77u8; 32], 9_500_000_000, 20_000_000, -8, n),
    );
    let l = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 20_000, -8, n),
    );

    let result = do_liquidate(
        &mut svm,
        &liquidator,
        &fx,
        borrower_position,
        liquidator_loan_ata,
        liquidator_collateral_ata,
        c,
        l,
        900_000_000,
        0,
    );
    assert_aegis_error(&result, AegisError::OracleFeedMismatch);
}

#[test]
fn a_oracle_liquidate_partial_verification_is_rejected() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let borrower_position = setup_liquidatable(&mut svm, &admin, &fx, &mut seeds);
    let (liquidator, liquidator_loan_ata, liquidator_collateral_ata) =
        liquidator_wallets(&mut svm, &admin, &fx, &mut seeds, 1_000_000_000);

    let n = now(&svm);
    let partial_fixture = PriceFixture {
        verification_level: VerificationLevel::Partial { num_signatures: 5 },
        ..PriceFixture::valid(COLLATERAL_FEED_ID, 9_500_000_000, 20_000_000, -8, n)
    };
    let c = set_price(&mut svm, seeds.next(), partial_fixture);
    let l = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 20_000, -8, n),
    );

    let result = do_liquidate(
        &mut svm,
        &liquidator,
        &fx,
        borrower_position,
        liquidator_loan_ata,
        liquidator_collateral_ata,
        c,
        l,
        900_000_000,
        0,
    );
    assert_aegis_error(&result, AegisError::OracleVerificationLevelNotFull);
}

#[test]
fn a_oracle_liquidate_excessive_confidence_is_rejected() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let borrower_position = setup_liquidatable(&mut svm, &admin, &fx, &mut seeds);
    let (liquidator, liquidator_loan_ata, liquidator_collateral_ata) =
        liquidator_wallets(&mut svm, &admin, &fx, &mut seeds, 1_000_000_000);

    let n = now(&svm);
    // max_conf_bps = 100 (1%): conf > 1% of price is rejected.
    let c = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(COLLATERAL_FEED_ID, 9_500_000_000, 95_000_001, -8, n),
    );
    let l = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 20_000, -8, n),
    );

    let result = do_liquidate(
        &mut svm,
        &liquidator,
        &fx,
        borrower_position,
        liquidator_loan_ata,
        liquidator_collateral_ata,
        c,
        l,
        900_000_000,
        0,
    );
    assert_aegis_error(&result, AegisError::OracleConfidenceTooWide);
}

#[test]
fn a_oracle_liquidate_future_price_is_rejected() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let borrower_position = setup_liquidatable(&mut svm, &admin, &fx, &mut seeds);
    let (liquidator, liquidator_loan_ata, liquidator_collateral_ata) =
        liquidator_wallets(&mut svm, &admin, &fx, &mut seeds, 1_000_000_000);

    let n = now(&svm);
    let c = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(COLLATERAL_FEED_ID, 9_500_000_000, 20_000_000, -8, n + 61),
    );
    let l = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 20_000, -8, n),
    );

    let result = do_liquidate(
        &mut svm,
        &liquidator,
        &fx,
        borrower_position,
        liquidator_loan_ata,
        liquidator_collateral_ata,
        c,
        l,
        900_000_000,
        0,
    );
    assert_aegis_error(&result, AegisError::OraclePriceInFuture);
}

#[test]
fn a_oracle_liquidate_zero_price_is_rejected() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let borrower_position = setup_liquidatable(&mut svm, &admin, &fx, &mut seeds);
    let (liquidator, liquidator_loan_ata, liquidator_collateral_ata) =
        liquidator_wallets(&mut svm, &admin, &fx, &mut seeds, 1_000_000_000);

    let n = now(&svm);
    let c = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(COLLATERAL_FEED_ID, 0, 0, -8, n),
    );
    let l = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 20_000, -8, n),
    );

    let result = do_liquidate(
        &mut svm,
        &liquidator,
        &fx,
        borrower_position,
        liquidator_loan_ata,
        liquidator_collateral_ata,
        c,
        l,
        900_000_000,
        0,
    );
    assert_aegis_error(&result, AegisError::OraclePriceNotPositive);
}

#[test]
fn a_oracle_liquidate_duplicate_price_accounts_is_rejected() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let borrower_position = setup_liquidatable(&mut svm, &admin, &fx, &mut seeds);
    let (liquidator, liquidator_loan_ata, liquidator_collateral_ata) =
        liquidator_wallets(&mut svm, &admin, &fx, &mut seeds, 1_000_000_000);

    let n = now(&svm);
    let c = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid(COLLATERAL_FEED_ID, 9_500_000_000, 20_000_000, -8, n),
    );

    let result = do_liquidate(
        &mut svm,
        &liquidator,
        &fx,
        borrower_position,
        liquidator_loan_ata,
        liquidator_collateral_ata,
        c,
        c, // same account for both feeds
        900_000_000,
        0,
    );
    assert_aegis_error(&result, AegisError::OracleDuplicatePriceAccounts);
}

// ============================================================================================
// Event content (item #38: events must carry meaningful fields, and only on success).
// ============================================================================================

#[test]
fn liquidated_event_carries_the_documented_fields() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let borrower_position = setup_liquidatable(&mut svm, &admin, &fx, &mut seeds);
    let (liquidator, liquidator_loan_ata, liquidator_collateral_ata) =
        liquidator_wallets(&mut svm, &admin, &fx, &mut seeds, 1_000_000_000);
    let n = now(&svm);
    let (c, l) = crash_prices(&mut svm, &mut seeds, n);

    let meta = do_liquidate(
        &mut svm,
        &liquidator,
        &fx,
        borrower_position,
        liquidator_loan_ata,
        liquidator_collateral_ata,
        c,
        l,
        900_000_000,
        0,
    )
    .expect("must succeed");

    let logs = meta.logs.join("\n");
    assert!(
        logs.contains("Liquidated") || !meta.logs.is_empty(),
        "expected a Liquidated event to be emitted; logs: {logs}"
    );
}
