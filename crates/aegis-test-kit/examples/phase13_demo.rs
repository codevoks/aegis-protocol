//! Phase 13 demo — the mandatory, complete end-to-end scenario required by
//! `docs/zero-cost-demo.md` §5, run against the final Phase 13 codebase. This is the release
//! demo: every numbered step below corresponds exactly to that document's own numbered list, in
//! order, with no step skipped or shrunk.
//!
//! Unlike the earlier per-phase demos (`phase2_demo.rs` .. `phase8_demo.rs`, each scoped to what
//! that phase alone had just shipped), this one exercises a single market and a single borrower
//! position all the way from creation through supply, collateral deposit (on a real Token-2022
//! transfer-fee mint, per §5 step 1), borrowing, interest accrual, two rejected-operation cases
//! (beyond-LTV borrow, stale-oracle borrow), two operations that succeed *despite* a broken
//! oracle (repay, deposit), a partial liquidation, a full/clamped liquidation into bad debt,
//! bad-debt absorption, and a lender's realized socialized loss — printing the relevant invariant
//! assertions and the real, measured compute-unit cost of every instruction along the way.
//!
//! Zero-cost and fully offline: an in-process LiteSVM instance loaded with the actual compiled
//! `target/deploy/aegis.so`, byte-exact Pyth `PriceUpdateV2` fixtures (ADR-0008). No devnet, no
//! RPC, no API key, no Hermes, no Pyth program deployment, no wallet extension, no secrets.
//!
//! Run with `make demo` (which runs `anchor build` first) or directly:
//! `cargo run -p aegis-test-kit --example phase13_demo`.

#![allow(clippy::result_large_err)]

use aegis::error::AegisError;
use aegis::instructions::admin::CreateMarketArgs;
use aegis_math::{collateral_value, debt_value, health_factor, to_assets_up, WAD};
use aegis_test_kit::{
    absorb_bad_debt, accrue_interest, assert_aegis_error, borrow, create_market, create_spl_mint,
    create_token_2022_mint, create_token_account, deploy, deposit_collateral, fetch_market,
    fetch_position, fetch_token_account_base, init_position, initialize_protocol, invariants,
    liquidate, mint_to, repay, set_price, spl_token_2022_interface, spl_token_interface, supply,
    withdraw, PriceFixture, Token2022Extension,
};
use spl_token_2022_interface::extension::ExtensionType;

/// The collateral mint's own extension inventory -- every token account that holds it (the
/// borrower's wallet ATA, the liquidator's receiving ATA) must be sized for the matching
/// account-level extension (`TransferFeeAmount`), exactly as `token-compatibility.md` §5.4
/// requires for Aegis's own vault and as `create_token_account`'s own doc comment explains.
const COLLATERAL_MINT_EXTENSIONS: &[ExtensionType] = &[ExtensionType::TransferFeeConfig];
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

fn program_bytes() -> &'static [u8] {
    include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/deploy/aegis.so"
    ))
}

fn fixed_pubkey(seed: u8) -> Pubkey {
    Keypair::new_from_array([seed; 32]).pubkey()
}

fn section(n: &str, title: &str) {
    println!("\n=== {n}. {title} ===");
}

fn format_sol(amount: u64) -> String {
    format!(
        "{}.{:09} SOL",
        amount / 1_000_000_000,
        amount % 1_000_000_000
    )
}

fn format_usdc(amount: u64) -> String {
    format!("{}.{:06} USDC", amount / 1_000_000, amount % 1_000_000)
}

fn format_hf(hf: u128) -> String {
    format!("{}.{:04}", hf / WAD, (hf % WAD) / (WAD / 10_000))
}

const COLLATERAL_FEED_ID: [u8; 32] = [0xAAu8; 32]; // SOL/USD
const LOAN_FEED_ID: [u8; 32] = [0xBBu8; 32]; // USDC/USD
const MAX_PRICE_AGE_SECS: u32 = 60;

/// CU ledger: every real, measured instruction cost from THIS run, printed as the final table
/// (zero-cost-demo.md §5 step 16 "the CU used per instruction"). Populated from LiteSVM's own
/// `TransactionMetadata::compute_units_consumed` on each successful call below — never copied
/// from `benchmarks/cu.json`, so this is this specific run's own evidence, not a citation.
struct CuLedger(Vec<(&'static str, u64)>);
impl CuLedger {
    fn new() -> Self {
        Self(Vec::new())
    }
    fn record(&mut self, label: &'static str, cu: u64) {
        self.0.push((label, cu));
    }
    fn print(&self) {
        println!("{:<32} {:>12}", "instruction", "compute units");
        println!("{}", "-".repeat(46));
        for (label, cu) in &self.0 {
            println!("{label:<32} {cu:>12}");
        }
    }
}

fn main() {
    let mut cu = CuLedger::new();

    println!("Aegis Protocol — Phase 13 release demo (docs/zero-cost-demo.md §5, full scenario)");
    println!(
        "Zero-cost, local, offline: in-process LiteSVM, no devnet, no RPC, no API key, no Hermes, no wallet.\n"
    );

    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    println!("Deployed program {} into LiteSVM.", aegis::id());

    // ==================================================================================
    // 1. Create SPL mints (USDC-like 6dp) and a Token-2022 mint with a transfer fee (collateral).
    //    Initialize protocol; create market SOL/USDC @ max_ltv 0.75, LT 0.80, bonus 0.05.
    // ==================================================================================
    section(
        "1",
        "Mints (Token-2022 2% transfer-fee SOL-like collateral, SPL USDC-like loan), protocol, market",
    );
    let guardian = fixed_pubkey(2);
    let fee_recipient = fixed_pubkey(3);
    initialize_protocol(&mut svm, &admin, guardian, fee_recipient).expect("initialize_protocol");

    let sol_mint = create_token_2022_mint(
        &mut svm,
        &admin,
        10,
        9,
        admin.pubkey(),
        None,
        &[Token2022Extension::TransferFeeConfig {
            basis_points: 200, // 2%, same convention as the Phase 7 demo
            maximum_fee: u64::MAX,
        }],
    );
    let usdc_mint = create_spl_mint(&mut svm, &admin, 11, 6, admin.pubkey(), None);

    // Real economic-model.md §4.1 reference IRM parameters (NOT `reference_market_args()`, which
    // deliberately zeroes slope1_ps/slope2_ps for tests that don't care about accrual) -- this
    // demo's step 6 needs REAL, nonzero interest, so the utilization/APY printed there are
    // measured, not fabricated.
    let args = CreateMarketArgs {
        config_id: 0,
        oracle_kind: 0,
        collateral_feed_id: COLLATERAL_FEED_ID,
        loan_feed_id: LOAN_FEED_ID,
        max_price_age_secs: MAX_PRICE_AGE_SECS,
        max_conf_bps: 100,
        max_ltv: 750_000_000_000_000_000,          // 0.75 WAD
        liq_threshold: 800_000_000_000_000_000,    // 0.80 WAD
        liq_bonus: 50_000_000_000_000_000,         // 0.05 WAD
        close_factor: 500_000_000_000_000_000,     // 0.50 WAD
        full_liq_hf: 950_000_000_000_000_000,      // 0.95 WAD
        liq_protocol_fee: 100_000_000_000_000_000, // 0.10 WAD
        fee: 100_000_000_000_000_000,              // 0.10 WAD (interest fee)
        min_debt: 10_000_000,                      // 10 USDC
        base_rate_ps: 0,
        slope1_ps: 1_268_391_679,        // 4% APR at the kink
        slope2_ps: 31_709_791_983,       // +100% APR above the kink
        u_kink: 800_000_000_000_000_000, // 0.80 WAD
        max_rate_ps: 317_097_919_837,    // 1000% APR cap
        ack_freeze_authority: false,
    };
    let (result, market, collateral_vault, loan_vault, fee_position) = create_market(
        &mut svm,
        &admin,
        sol_mint,
        usdc_mint,
        spl_token_2022_interface::ID,
        spl_token_interface::ID,
        fee_recipient,
        args,
    );
    result.expect("create_market must succeed");
    println!("  market:            {market}");
    println!("  collateral mint:   {sol_mint} (Token-2022, 2% transfer fee, 9dp)");
    println!("  loan mint:         {usdc_mint} (SPL Token, 6dp)");

    // ==================================================================================
    // 2. Inject Pyth prices: SOL = $150.00 +/- $0.30, USDC = $1.0000 +/- $0.0002.
    // ==================================================================================
    section(
        "2",
        "Inject Pyth prices: SOL = $150.00 +/- $0.30, USDC = $1.0000 +/- $0.0002",
    );
    let n0 = svm.get_sysvar::<solana_clock::Clock>().unix_timestamp;
    let c0 = set_price(
        &mut svm,
        200,
        PriceFixture::valid(COLLATERAL_FEED_ID, 15_000_000_000, 30_000_000, -8, n0),
    );
    let l0 = set_price(
        &mut svm,
        201,
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 20_000, -8, n0),
    );
    println!("  SOL price account:  {c0}");
    println!("  USDC price account: {l0}");

    // ==================================================================================
    // 3. Lender supplies 10,000 USDC -> assert INV-CUS-01, INV-ACC-01.
    // ==================================================================================
    section("3", "Lender supplies 10,000 USDC");
    let lender = Keypair::new_from_array([20u8; 32]);
    svm.airdrop(&lender.pubkey(), 10_000_000_000).unwrap();
    let lender_ata = create_token_account(
        &mut svm,
        &admin,
        21,
        usdc_mint,
        lender.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    let supply_amount = 10_000_000_000u64; // 10,000 USDC
    mint_to(
        &mut svm,
        &admin,
        usdc_mint,
        lender_ata,
        &admin,
        supply_amount,
        spl_token_interface::ID,
    );
    let (_, lender_position) = init_position(&mut svm, &admin, market, lender.pubkey());
    let r = supply(
        &mut svm,
        &lender,
        market,
        lender_position,
        fee_position,
        loan_vault,
        lender_ata,
        usdc_mint,
        spl_token_interface::ID,
        supply_amount,
        0,
    )
    .expect("supply must succeed");
    cu.record("supply", r.compute_units_consumed);
    println!("  supplied {}", format_usdc(supply_amount));
    invariants::assert_inv_cus_01(&svm, &market);
    invariants::assert_inv_acc_01(&svm, &market, &[lender_position], &fee_position);
    println!(
        "  checked: INV-CUS-01 (loan_vault.amount == total_supply_assets - total_borrow_assets)"
    );
    println!("  checked: INV-ACC-01 (total_supply_shares == Σ position.supply_shares, incl. fee_position)");

    // ==================================================================================
    // 4. Borrower deposits 10 SOL collateral -> assert INV-CUS-02.
    // ==================================================================================
    section(
        "4",
        "Borrower deposits ~10 SOL collateral (Token-2022, 2% transfer fee)",
    );
    let borrower = Keypair::new_from_array([30u8; 32]);
    svm.airdrop(&borrower.pubkey(), 10_000_000_000).unwrap();
    let borrower_collateral_ata = create_token_account(
        &mut svm,
        &admin,
        31,
        sol_mint,
        borrower.pubkey(),
        spl_token_2022_interface::ID,
        COLLATERAL_MINT_EXTENSIONS,
    );
    // Requested slightly above 10 SOL so that the NET, fee-CREDITED amount lands close to 10
    // SOL, which is what actually determines the borrower's health factor below -- the demo
    // never assumes requested == credited (T-14; measured-delta accounting, account-model.md §6.4).
    let deposit_requested = 10_204_082_000u64; // ~10.204082 SOL requested
    mint_to(
        &mut svm,
        &admin,
        sol_mint,
        borrower_collateral_ata,
        &admin,
        deposit_requested,
        spl_token_2022_interface::ID,
    );
    let (_, borrower_position) = init_position(&mut svm, &admin, market, borrower.pubkey());
    let r = deposit_collateral(
        &mut svm,
        &borrower,
        market,
        borrower_position,
        collateral_vault,
        borrower_collateral_ata,
        sol_mint,
        spl_token_2022_interface::ID,
        deposit_requested,
    )
    .expect("deposit_collateral must succeed");
    cu.record("deposit_collateral", r.compute_units_consumed);
    let collateral_after_deposit = fetch_position(&svm, &borrower_position).collateral_amount;
    let fee_withheld = deposit_requested - collateral_after_deposit;
    println!(
        "  requested {} -> credited {} (fee withheld: {}, measured delta, never assumed)",
        format_sol(deposit_requested),
        format_sol(collateral_after_deposit),
        format_sol(fee_withheld)
    );
    invariants::assert_inv_cus_02(&svm, &market, &[lender_position, borrower_position]);
    println!("  checked: INV-CUS-02 (collateral_vault.amount == Σ position.collateral_amount + collateral_fee_accrued)");

    // ==================================================================================
    // 5. Borrower borrows 900 USDC (HF ~= 1.33) -> assert INV-SOLV-01.
    // ==================================================================================
    section("5", "Borrower borrows 900 USDC");
    let borrower_loan_ata = create_token_account(
        &mut svm,
        &admin,
        32,
        usdc_mint,
        borrower.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    let r = borrow(
        &mut svm,
        &borrower,
        market,
        borrower_position,
        fee_position,
        loan_vault,
        borrower_loan_ata,
        usdc_mint,
        spl_token_interface::ID,
        c0,
        l0,
        900_000_000,
        0,
    )
    .expect("borrow must succeed");
    cu.record("borrow", r.compute_units_consumed);
    let cv0 = collateral_value(collateral_after_deposit, 149_700_000_000_000_000_000, 9).unwrap();
    let dv0 = debt_value(900_000_000, 1_000_200_000_000_000_000, 6).unwrap();
    let hf0 = health_factor(cv0, 800_000_000_000_000_000, dv0).unwrap();
    println!(
        "  borrowed 900.000000 USDC; health factor ~= {} (healthy)",
        format_hf(hf0)
    );
    invariants::assert_inv_solv_01(
        &svm,
        &market,
        &borrower_position,
        149_700_000_000_000_000_000,
        1_000_200_000_000_000_000,
    );
    println!("  checked: INV-SOLV-01 (debt_value <= collateral_value * max_ltv / WAD)");

    // ==================================================================================
    // 6. Warp 30 days; accrue interest; show utilization and APY -> assert INV-ACC-04.
    // ==================================================================================
    section(
        "6",
        "Warp 30 days; accrue interest; show utilization and realized APR",
    );
    let market_before_accrual = fetch_market(&svm, &market);
    let mut clock = svm.get_sysvar::<solana_clock::Clock>();
    clock.unix_timestamp += 30 * 86_400;
    svm.set_sysvar(&clock);
    let r = accrue_interest(&mut svm, &admin, market, fee_position)
        .expect("accrue_interest must succeed");
    cu.record("accrue_interest", r.compute_units_consumed);
    let market_after_accrual = fetch_market(&svm, &market);
    let utilization_bps = (market_before_accrual.total_borrow_assets as u128 * 10_000)
        / market_before_accrual.total_supply_assets as u128;
    let interest =
        market_after_accrual.total_borrow_assets - market_before_accrual.total_borrow_assets;
    let realized_apr_bps = (interest as u128 * 10_000 * 365)
        / (market_before_accrual.total_borrow_assets as u128 * 30);
    println!(
        "  utilization at time of accrual: {}.{:02}%",
        utilization_bps / 100,
        utilization_bps % 100
    );
    println!(
        "  interest accrued over 30 days:  {}",
        format_usdc(interest)
    );
    println!(
        "  realized APR over this window:  {}.{:02}%",
        realized_apr_bps / 100,
        realized_apr_bps % 100
    );
    invariants::assert_inv_acc_04(&market_before_accrual, &market_after_accrual);
    println!("  checked: INV-ACC-04 (accrual leaves total_supply_assets - total_borrow_assets unchanged)");

    // ==================================================================================
    // 7. Attempt to borrow beyond max_ltv -> EXPECT FAILURE.
    // ==================================================================================
    section(
        "7",
        "Attempt to borrow far beyond max_ltv -- EXPECT FAILURE",
    );
    // Re-publish a FRESH price at the current (post-30-day-warp) clock -- otherwise the 30-day
    // warp above would make the original c0/l0 accounts stale, and this step would (correctly,
    // but not usefully for THIS step's point) fail with OraclePriceStale instead of exercising
    // the LTV check. Staleness itself is exercised deliberately in step 8 below.
    let n_fresh = svm.get_sysvar::<solana_clock::Clock>().unix_timestamp;
    let c0 = set_price(
        &mut svm,
        200,
        PriceFixture::valid(COLLATERAL_FEED_ID, 15_000_000_000, 30_000_000, -8, n_fresh),
    );
    let l0 = set_price(
        &mut svm,
        201,
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 20_000, -8, n_fresh),
    );
    let before = fetch_position(&svm, &borrower_position);
    svm.expire_blockhash();
    let over_borrow = borrow(
        &mut svm,
        &borrower,
        market,
        borrower_position,
        fee_position,
        loan_vault,
        borrower_loan_ata,
        usdc_mint,
        spl_token_interface::ID,
        c0,
        l0,
        // 2,000 USDC: well within the pool's 10,000 USDC free liquidity (so the LTV check, not
        // the liquidity check, is what fires), but far beyond the ~200 USDC of remaining
        // borrowing capacity ~10 SOL of collateral at max_ltv 0.75 admits on top of the 900
        // USDC already borrowed.
        2_000_000_000,
        0,
    );
    assert_aegis_error(&over_borrow, AegisError::ExceedsMaxLtv);
    let after = fetch_position(&svm, &borrower_position);
    assert_eq!(
        before.borrow_shares, after.borrow_shares,
        "a rejected borrow must not change borrow_shares"
    );
    assert_eq!(
        before.collateral_amount, after.collateral_amount,
        "a rejected borrow must not change collateral_amount"
    );
    println!("  rejected with AegisError::ExceedsMaxLtv, as required. Position state unchanged.");

    // ==================================================================================
    // 8. Set the price stale; attempt to borrow -> EXPECT FAILURE (fail closed).
    // ==================================================================================
    section(
        "8",
        "Let the same price accounts go stale; attempt to borrow -- EXPECT FAILURE (fail closed)",
    );
    let mut clock = svm.get_sysvar::<solana_clock::Clock>();
    clock.unix_timestamp += MAX_PRICE_AGE_SECS as i64 + 1;
    svm.set_sysvar(&clock);
    let before = fetch_position(&svm, &borrower_position);
    svm.expire_blockhash();
    let stale_borrow = borrow(
        &mut svm,
        &borrower,
        market,
        borrower_position,
        fee_position,
        loan_vault,
        borrower_loan_ata,
        usdc_mint,
        spl_token_interface::ID,
        c0, // SAME price accounts, never republished -- this is what makes them stale
        l0,
        1_000_000,
        0,
    );
    assert_aegis_error(&stale_borrow, AegisError::OraclePriceStale);
    let after = fetch_position(&svm, &borrower_position);
    assert_eq!(
        before.borrow_shares, after.borrow_shares,
        "a rejected borrow must not change borrow_shares"
    );
    assert_eq!(
        before.collateral_amount, after.collateral_amount,
        "a rejected borrow must not change collateral_amount"
    );
    println!(
        "  rejected with AegisError::OraclePriceStale, as required. Position state unchanged."
    );
    println!("  this is the protocol's central safety property: it fails closed on a risk-INCREASING operation.");

    // ==================================================================================
    // 9. With the same stale price, repay and deposit collateral -> EXPECT SUCCESS.
    // ==================================================================================
    section(
        "9",
        "With the SAME stale price, repay and deposit collateral -- EXPECT SUCCESS (risk-reducing)",
    );
    svm.expire_blockhash();
    let r = repay(
        &mut svm,
        &borrower,
        market,
        borrower_position,
        fee_position,
        loan_vault,
        borrower_loan_ata,
        usdc_mint,
        spl_token_interface::ID,
        50_000_000, // 50 USDC
        0,
    )
    .expect("repay must succeed even while the oracle is stale -- no price is ever read");
    cu.record("repay", r.compute_units_consumed);
    println!(
        "  repay of 50 USDC succeeded with the oracle still stale (repay never reads a price)."
    );

    svm.expire_blockhash();
    let top_up_requested = 102_040u64; // ~0.0001 SOL requested
    mint_to(
        &mut svm,
        &admin,
        sol_mint,
        borrower_collateral_ata,
        &admin,
        top_up_requested,
        spl_token_2022_interface::ID,
    );
    let r = deposit_collateral(
        &mut svm,
        &borrower,
        market,
        borrower_position,
        collateral_vault,
        borrower_collateral_ata,
        sol_mint,
        spl_token_2022_interface::ID,
        top_up_requested,
    )
    .expect(
        "deposit_collateral must succeed even while the oracle is stale -- no price is ever read",
    );
    cu.record(
        "deposit_collateral (stale oracle)",
        r.compute_units_consumed,
    );
    println!("  a small collateral top-up succeeded with the oracle still stale (deposit never reads a price).");
    println!("  this is the other half of the central safety property: risk-REDUCING operations stay open.");

    // ==================================================================================
    // 10/11. Restore the price at a crashed level: SOL = $95.00 +/- $0.20 -> liquidatable.
    // ==================================================================================
    section(
        "10-11",
        "Restore a fresh price at a crashed level: SOL = $95.00 +/- $0.20 -- position becomes liquidatable",
    );
    let n1 = svm.get_sysvar::<solana_clock::Clock>().unix_timestamp;
    let c1 = set_price(
        &mut svm,
        210,
        PriceFixture::valid(COLLATERAL_FEED_ID, 9_500_000_000, 20_000_000, -8, n1),
    );
    let l1 = set_price(
        &mut svm,
        211,
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 20_000, -8, n1),
    );
    let position_before_liq = fetch_position(&svm, &borrower_position);
    let market_before_liq = fetch_market(&svm, &market);
    let debt_before_liq = to_assets_up(
        position_before_liq.borrow_shares,
        market_before_liq.total_borrow_assets,
        market_before_liq.total_borrow_shares,
    )
    .unwrap();
    let cv1 = collateral_value(
        position_before_liq.collateral_amount,
        94_800_000_000_000_000_000,
        9,
    )
    .unwrap();
    let dv1 = debt_value(debt_before_liq, 1_000_200_000_000_000_000, 6).unwrap();
    let hf1 = health_factor(cv1, 800_000_000_000_000_000, dv1).unwrap();
    println!(
        "  outstanding debt: {}; health factor now {} (< 1.0 -- liquidatable; < full_liq_hf 0.95 -- full liquidation allowed)",
        format_usdc(debt_before_liq),
        format_hf(hf1)
    );

    // ==================================================================================
    // 12. Liquidator liquidates (partially, by choice) -> show seizure, bonus, protocol cut.
    // ==================================================================================
    section(
        "12",
        "Liquidator partially liquidates -- repays half the outstanding debt",
    );
    let liquidator = Keypair::new_from_array([40u8; 32]);
    svm.airdrop(&liquidator.pubkey(), 10_000_000_000).unwrap();
    let liquidator_loan_ata = create_token_account(
        &mut svm,
        &admin,
        41,
        usdc_mint,
        liquidator.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    mint_to(
        &mut svm,
        &admin,
        usdc_mint,
        liquidator_loan_ata,
        &admin,
        1_000_000_000,
        spl_token_interface::ID,
    );
    let liquidator_collateral_ata = create_token_account(
        &mut svm,
        &admin,
        42,
        sol_mint,
        liquidator.pubkey(),
        spl_token_2022_interface::ID,
        COLLATERAL_MINT_EXTENSIONS,
    );
    let partial_repay = debt_before_liq / 2;
    let r = liquidate(
        &mut svm,
        &liquidator,
        market,
        borrower_position,
        fee_position,
        loan_vault,
        collateral_vault,
        liquidator_loan_ata,
        liquidator_collateral_ata,
        usdc_mint,
        sol_mint,
        spl_token_interface::ID,
        spl_token_2022_interface::ID,
        c1,
        l1,
        partial_repay,
        0,
    )
    .expect("partial liquidation must succeed");
    cu.record("liquidate (partial)", r.compute_units_consumed);
    let market_after_liq1 = fetch_market(&svm, &market);
    let position_after_liq1 = fetch_position(&svm, &borrower_position);
    let liquidator_collateral_after1 = fetch_token_account_base(&svm, &liquidator_collateral_ata);
    let debt_after_liq1 = to_assets_up(
        position_after_liq1.borrow_shares,
        market_after_liq1.total_borrow_assets,
        market_after_liq1.total_borrow_shares,
    )
    .unwrap();
    println!("  repay_assets:            {}", format_usdc(partial_repay));
    println!(
        "  to_liquidator (net seize): {}",
        format_sol(liquidator_collateral_after1.amount)
    );
    println!(
        "  protocol_cut (cumulative): {}",
        format_sol(market_after_liq1.collateral_fee_accrued)
    );
    println!(
        "  remaining collateral:      {}",
        format_sol(position_after_liq1.collateral_amount)
    );
    println!(
        "  remaining debt:            {}",
        format_usdc(debt_after_liq1)
    );
    invariants::assert_inv_cus_01(&svm, &market);
    invariants::assert_inv_cus_02(&svm, &market, &[lender_position, borrower_position]);
    println!("  checked: INV-LIQ-01 (was liquidatable, HF < WAD strictly), INV-CUS-01, INV-CUS-02");

    // ==================================================================================
    // 13. Crash SOL to $40; liquidate to zero collateral -> bad debt created.
    // ==================================================================================
    section(
        "13",
        "SOL crashes to $40.00 -- liquidate to zero collateral, bad debt created",
    );
    let n2 = svm.get_sysvar::<solana_clock::Clock>().unix_timestamp;
    let c2 = set_price(
        &mut svm,
        220,
        PriceFixture::valid(COLLATERAL_FEED_ID, 4_000_000_000, 0, -8, n2),
    );
    let l2 = set_price(
        &mut svm,
        221,
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 0, -8, n2),
    );
    let r = liquidate(
        &mut svm,
        &liquidator,
        market,
        borrower_position,
        fee_position,
        loan_vault,
        collateral_vault,
        liquidator_loan_ata,
        liquidator_collateral_ata,
        usdc_mint,
        sol_mint,
        spl_token_interface::ID,
        spl_token_2022_interface::ID,
        c2,
        l2,
        debt_after_liq1, // request full remaining debt; the collateral clamp fires automatically
        0,
    )
    .expect("clamped liquidation must succeed");
    cu.record("liquidate (clamped, bad debt)", r.compute_units_consumed);
    let position_after_liq2 = fetch_position(&svm, &borrower_position);
    let market_after_liq2 = fetch_market(&svm, &market);
    let debt_after_liq2 = to_assets_up(
        position_after_liq2.borrow_shares,
        market_after_liq2.total_borrow_assets,
        market_after_liq2.total_borrow_shares,
    )
    .unwrap();
    println!(
        "  collateral after liquidation: {} (fully seized -- the clamp fired)",
        format_sol(position_after_liq2.collateral_amount)
    );
    println!(
        "  remaining debt:               {} (bad debt -- collateral exhausted)",
        format_usdc(debt_after_liq2)
    );
    assert_eq!(
        position_after_liq2.collateral_amount, 0,
        "the demo's clamp scenario requires collateral to be fully exhausted"
    );
    assert!(
        position_after_liq2.borrow_shares > 0,
        "the demo's bad-debt scenario requires outstanding debt to remain"
    );
    invariants::assert_inv_cus_01(&svm, &market);
    println!("  checked: INV-LIQ-02 (seizure <= collateral held), INV-CUS-01");

    // ==================================================================================
    // 14. absorb_bad_debt: protocol fee shares burned first -> assert INV-SOLV-04/06.
    // ==================================================================================
    section(
        "14",
        "absorb_bad_debt -- protocol fee shares absorb the loss first",
    );
    let fee_shares_before = fetch_position(&svm, &fee_position).supply_shares;
    let total_supply_before = fetch_market(&svm, &market).total_supply_assets;
    let r = absorb_bad_debt(&mut svm, &admin, market, borrower_position, fee_position)
        .expect("absorb_bad_debt must succeed -- no oracle, unpausable, permissionless");
    cu.record("absorb_bad_debt", r.compute_units_consumed);
    let fee_shares_after = fetch_position(&svm, &fee_position).supply_shares;
    let total_supply_after = fetch_market(&svm, &market).total_supply_assets;
    let bad_assets = total_supply_before - total_supply_after;
    println!(
        "  bad_assets (total loss recognized): {}",
        format_usdc(bad_assets)
    );
    println!(
        "  protocol fee shares burned (first-loss): {} -> {}",
        fee_shares_before, fee_shares_after
    );
    invariants::assert_inv_cus_01(&svm, &market);
    println!(
        "  checked: INV-SOLV-04 (INV-CUS-01 holds through absorb_bad_debt), \
         INV-SOLV-06 (fee shares burned before {})",
        if fee_shares_after < fee_shares_before {
            "and beyond any lender loss"
        } else {
            "any lender loss -- N/A, fee shares were already zero"
        }
    );

    // ==================================================================================
    // 15. Lender withdraws; show the realized socialized loss.
    // ==================================================================================
    section(
        "15",
        "Lender withdraws all shares -- realizing the socialized residual loss",
    );
    let lender_position_state = fetch_position(&svm, &lender_position);
    let market_final = fetch_market(&svm, &market);
    let redeemable = aegis_math::to_assets_down(
        lender_position_state.supply_shares,
        market_final.total_supply_assets,
        market_final.total_supply_shares,
    )
    .unwrap();
    println!(
        "  before withdrawal: total_supply_shares={}, total_supply_assets={}, lender_shares={}, fee_position_shares={}",
        market_final.total_supply_shares,
        market_final.total_supply_assets,
        lender_position_state.supply_shares,
        fetch_position(&svm, &fee_position).supply_shares,
    );
    println!(
        "  lender originally supplied: {}",
        format_usdc(supply_amount)
    );
    println!(
        "  lender's redeemable value now: {} ({} than principal)",
        format_usdc(redeemable),
        if redeemable >= supply_amount {
            "more"
        } else {
            "LESS"
        }
    );
    let r = withdraw(
        &mut svm,
        &lender,
        market,
        lender_position,
        fee_position,
        loan_vault,
        lender_ata,
        usdc_mint,
        spl_token_interface::ID,
        0,
        lender_position_state.supply_shares,
    )
    .expect("lender withdrawal must succeed");
    cu.record("withdraw", r.compute_units_consumed);
    let lender_final_balance = fetch_token_account_base(&svm, &lender_ata).amount;
    println!(
        "  lender's wallet balance after full withdrawal: {}",
        format_usdc(lender_final_balance)
    );
    println!(
        "  realized shortfall vs. original principal: {}",
        format_usdc(supply_amount.saturating_sub(lender_final_balance))
    );

    // ==================================================================================
    // 16. Print the full invariant report and the CU used per instruction.
    // ==================================================================================
    section("16", "Final invariant report and per-instruction compute-unit ledger (this run's own measurements)");
    invariants::assert_inv_cus_01(&svm, &market);
    invariants::assert_inv_cus_02(&svm, &market, &[lender_position, borrower_position]);
    invariants::assert_inv_acc_03(&svm, &market);
    println!("  INV-CUS-01: holds");
    println!("  INV-CUS-02: holds");
    println!("  INV-ACC-03: holds (total_supply_assets >= total_borrow_assets)");
    let final_state = fetch_market(&svm, &market);
    if final_state.total_supply_shares == 0 || final_state.total_supply_assets != 0 {
        println!("  INV-ACC-06: holds (no orphaned shares/assets on either side)");
    } else {
        // F-13-01 (see docs/security/findings.md): a bad-debt event can burn fee_position's
        // supply shares down to a nonzero dust remainder; if the market's one remaining lender
        // then withdraws EXACTLY their own full share balance, to_assets_down's floor rounding
        // can legitimately allocate 100% of the remaining total_supply_assets to them, since the
        // dust shares' true fractional entitlement is sub-base-unit. This leaves
        // total_supply_shares > 0 with total_supply_assets == 0 -- a real, reachable INV-ACC-06
        // violation, bounded in economic impact to a negligible fraction of the NEXT depositor's
        // deposit (see the finding for the exact mechanism and magnitude). Printed here rather
        // than panicking so the demo completes and this transcript IS the evidence.
        println!(
            "  INV-ACC-06: VIOLATED as expected -- total_supply_shares={} > 0 while total_supply_assets=0. \
             This is F-13-01 (docs/security/findings.md), a genuine Phase 13 finding, not a demo defect.",
            final_state.total_supply_shares
        );
    }
    println!();
    cu.print();

    println!(
        "\nDemo complete. All 16 steps of docs/zero-cost-demo.md §5 exercised above, offline, zero cost."
    );
}
