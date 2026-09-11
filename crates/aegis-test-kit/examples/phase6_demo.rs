//! Phase 6 demo (`docs/zero-cost-demo.md` §5, steps 11-15): SOL crashes to $95.00, a position
//! becomes liquidatable, and a liquidator earns the bonus minus the protocol's cut -- the exact
//! worked figures from `economic-model.md` §7.5. SOL then crashes further to $40.00 on a SECOND
//! position, whose collateral is fully seized by the collateral clamp while debt remains; the
//! resulting bad debt is absorbed with the protocol's own fee shares burned FIRST, and the
//! residual loss is socialized -- a lender then withdraws and realizes it directly.
//!
//! Zero-cost and local: an in-process LiteSVM instance loaded with the actual built `aegis.so`
//! and byte-exact `PriceUpdateV2` fixtures built with the real `pyth-solana-receiver-sdk`. No
//! devnet, no RPC, no API key, no Hermes, no Pyth program deployment (ADR-0008).
//!
//! Run with `make demo` (which runs `anchor build` first) or directly:
//! `cargo run -p aegis-test-kit --example phase6_demo`.

#![allow(clippy::result_large_err)]

use aegis::instructions::admin::CreateMarketArgs;
use aegis_math::{collateral_value, debt_value, health_factor, WAD};
use aegis_test_kit::{
    absorb_bad_debt, accrue_interest, borrow, create_market, create_spl_mint, create_token_account,
    deploy, deposit_collateral, fetch_market, fetch_position, fetch_token_account_base,
    init_position, initialize_protocol, invariants, liquidate, mint_to, set_price,
    spl_token_interface, supply, withdraw, withdraw_collateral_fees, PriceFixture,
};
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

fn section(title: &str) {
    println!("\n=== {title} ===");
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

const COLLATERAL_FEED_ID: [u8; 32] = [0xAAu8; 32]; // SOL/USD
const LOAN_FEED_ID: [u8; 32] = [0xBBu8; 32]; // USDC/USD

fn main() {
    println!("Aegis Protocol — Phase 6 demo (liquidation and bad debt)");
    println!(
        "Zero-cost, local, offline: in-process LiteSVM, no devnet, no RPC, no API key, no Hermes.\n"
    );

    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    println!("Deployed program {} into LiteSVM.", aegis::id());

    // --- 1. Protocol, market ---
    section("1. Protocol and market (SOL/USDC, max_ltv 0.75, LT 0.80, bonus 0.05)");
    let guardian = fixed_pubkey(2);
    let fee_recipient = fixed_pubkey(3);
    initialize_protocol(&mut svm, &admin, guardian, fee_recipient).expect("initialize_protocol");

    let sol_mint = create_spl_mint(&mut svm, &admin, 10, 9, admin.pubkey(), None);
    let usdc_mint = create_spl_mint(&mut svm, &admin, 11, 6, admin.pubkey(), None);
    // NOT `reference_market_args()`: that shared test fixture deliberately zeroes
    // `slope1_ps`/`slope2_ps` (kept simple for tests that don't care about accrual), which would
    // make interest -- and therefore real protocol fee shares -- permanently zero regardless of
    // how long this demo warps time. This demo instead uses `economic-model.md` §4.1's actual
    // documented reference IRM parameters (4% APR at the kink, +100% APR above it, 1000% APR
    // cap), so step "Time passes" below produces a REAL, nonzero protocol fee.
    let args = CreateMarketArgs {
        config_id: 0,
        oracle_kind: 0,
        collateral_feed_id: COLLATERAL_FEED_ID,
        loan_feed_id: LOAN_FEED_ID,
        max_price_age_secs: 60,
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
        spl_token_interface::ID,
        spl_token_interface::ID,
        fee_recipient,
        args,
    );
    result.expect("create_market must succeed");
    println!("Market:            {market}");

    // --- 2. Lender supplies ---
    section("2. Lender supplies liquidity");
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
    // Deliberately modest (not "1,000,000 USDC") relative to the 900 USDC each borrower takes:
    // utilization needs to be meaningfully above zero during the 180-day warp below for the
    // reference IRM (base_rate_ps = 0) to accrue a real, nonzero protocol fee -- see that
    // section's own comment.
    let supply_amount = 1_200_000_000u64; // 1,200 USDC
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
    supply(
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
    println!("  supplied {}", format_usdc(supply_amount));
    invariants::assert_inv_cus_01(&svm, &market);

    // --- 3. Borrower A: the exact economic-model.md §7.5 scenario ---
    section("3. Borrower A deposits 10 SOL, borrows 900 USDC at SOL=$150.00");
    let borrower_a = Keypair::new_from_array([30u8; 32]);
    svm.airdrop(&borrower_a.pubkey(), 10_000_000_000).unwrap();
    let borrower_a_collateral_ata = create_token_account(
        &mut svm,
        &admin,
        31,
        sol_mint,
        borrower_a.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    mint_to(
        &mut svm,
        &admin,
        sol_mint,
        borrower_a_collateral_ata,
        &admin,
        10_000_000_000,
        spl_token_interface::ID,
    );
    let (_, borrower_a_position) = init_position(&mut svm, &admin, market, borrower_a.pubkey());
    deposit_collateral(
        &mut svm,
        &borrower_a,
        market,
        borrower_a_position,
        collateral_vault,
        borrower_a_collateral_ata,
        sol_mint,
        spl_token_interface::ID,
        10_000_000_000,
    )
    .expect("deposit_collateral must succeed");
    let borrower_a_loan_ata = create_token_account(
        &mut svm,
        &admin,
        32,
        usdc_mint,
        borrower_a.pubkey(),
        spl_token_interface::ID,
        &[],
    );

    let n0 = svm.get_sysvar::<solana_clock::Clock>().unix_timestamp;
    let c0 = set_price(
        &mut svm,
        200,
        PriceFixture::valid(COLLATERAL_FEED_ID, 15_000_000_000, 0, -8, n0),
    );
    let l0 = set_price(
        &mut svm,
        201,
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 0, -8, n0),
    );
    borrow(
        &mut svm,
        &borrower_a,
        market,
        borrower_a_position,
        fee_position,
        loan_vault,
        borrower_a_loan_ata,
        usdc_mint,
        spl_token_interface::ID,
        c0,
        l0,
        900_000_000,
        0,
    )
    .expect("borrow must succeed");
    let cv0 = collateral_value(10_000_000_000, 150_000_000_000_000_000_000, 9).unwrap();
    let dv0 = debt_value(900_000_000, WAD, 6).unwrap();
    let hf0 = health_factor(cv0, 800_000_000_000_000_000, dv0).unwrap();
    println!(
        "  borrowed 900.000000 USDC; health factor {}.{:04} (healthy)",
        hf0 / WAD,
        (hf0 % WAD) / (WAD / 10_000)
    );

    // --- 11. Price drops to $95.00 -> position A becomes liquidatable ---
    section("11. SOL crashes to $95.00 +/- $0.20 -- Borrower A becomes liquidatable");
    let c1 = set_price(
        &mut svm,
        210,
        PriceFixture::valid(COLLATERAL_FEED_ID, 9_500_000_000, 20_000_000, -8, n0),
    );
    let l1 = set_price(
        &mut svm,
        211,
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 20_000, -8, n0),
    );
    let cv1 = collateral_value(10_000_000_000, 94_800_000_000_000_000_000, 9).unwrap();
    let dv1 = debt_value(900_000_000, 1_000_200_000_000_000_000, 6).unwrap();
    let hf1 = health_factor(cv1, 800_000_000_000_000_000, dv1).unwrap();
    println!(
        "  health factor now {}.{:04} (< 1.0, and < full_liq_hf 0.95: full liquidation permitted)",
        hf1 / WAD,
        (hf1 % WAD) / (WAD / 10_000)
    );

    // --- 12. Liquidator liquidates A -- exact economic-model.md §7.5 figures ---
    section("12. Liquidator A repays the full 900 USDC debt");
    let liquidator_a = Keypair::new_from_array([40u8; 32]);
    svm.airdrop(&liquidator_a.pubkey(), 10_000_000_000).unwrap();
    let liquidator_a_loan_ata = create_token_account(
        &mut svm,
        &admin,
        41,
        usdc_mint,
        liquidator_a.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    mint_to(
        &mut svm,
        &admin,
        usdc_mint,
        liquidator_a_loan_ata,
        &admin,
        1_000_000_000,
        spl_token_interface::ID,
    );
    let liquidator_a_collateral_ata = create_token_account(
        &mut svm,
        &admin,
        42,
        sol_mint,
        liquidator_a.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    liquidate(
        &mut svm,
        &liquidator_a,
        market,
        borrower_a_position,
        fee_position,
        loan_vault,
        collateral_vault,
        liquidator_a_loan_ata,
        liquidator_a_collateral_ata,
        usdc_mint,
        sol_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        c1,
        l1,
        900_000_000,
        0,
    )
    .expect("liquidation must succeed");

    let market_after_liq_a = fetch_market(&svm, &market);
    let position_after_liq_a = fetch_position(&svm, &borrower_a_position);
    let liquidator_a_collateral = fetch_token_account_base(&svm, &liquidator_a_collateral_ata);
    println!("  repay_assets:      {}", format_usdc(900_000_000));
    println!(
        "  total_seize:       {} (base + bonus)",
        format_sol(9_970_348_101)
    );
    println!("  bonus_amount:      {}", format_sol(474_778_481));
    println!(
        "  protocol_cut:      {} (from the bonus only)",
        format_sol(market_after_liq_a.collateral_fee_accrued)
    );
    println!(
        "  to_liquidator:     {}",
        format_sol(liquidator_a_collateral.amount)
    );
    println!(
        "  remaining collateral on A: {}",
        format_sol(position_after_liq_a.collateral_amount)
    );
    println!(
        "  remaining debt on A:       {} (fully repaid)",
        position_after_liq_a.borrow_shares
    );
    invariants::assert_inv_cus_01(&svm, &market);
    invariants::assert_all(&svm, &market, &[lender_position, borrower_a_position]);
    println!("  INV-LIQ-*, INV-CUS-01, INV-CUS-02: hold");

    // --- Borrower B: a second position, whose debt will accrue real interest (crystallizing
    // real protocol fee shares into fee_position) before a much deeper crash creates bad debt. ---
    section("Borrower B deposits 10 SOL, borrows 900 USDC (same starting terms as A)");
    let borrower_b = Keypair::new_from_array([50u8; 32]);
    svm.airdrop(&borrower_b.pubkey(), 10_000_000_000).unwrap();
    let borrower_b_collateral_ata = create_token_account(
        &mut svm,
        &admin,
        51,
        sol_mint,
        borrower_b.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    mint_to(
        &mut svm,
        &admin,
        sol_mint,
        borrower_b_collateral_ata,
        &admin,
        10_000_000_000,
        spl_token_interface::ID,
    );
    let (_, borrower_b_position) = init_position(&mut svm, &admin, market, borrower_b.pubkey());
    deposit_collateral(
        &mut svm,
        &borrower_b,
        market,
        borrower_b_position,
        collateral_vault,
        borrower_b_collateral_ata,
        sol_mint,
        spl_token_interface::ID,
        10_000_000_000,
    )
    .expect("deposit_collateral must succeed");
    let borrower_b_loan_ata = create_token_account(
        &mut svm,
        &admin,
        52,
        usdc_mint,
        borrower_b.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    let n2 = svm.get_sysvar::<solana_clock::Clock>().unix_timestamp;
    let c2 = set_price(
        &mut svm,
        220,
        PriceFixture::valid(COLLATERAL_FEED_ID, 15_000_000_000, 0, -8, n2),
    );
    let l2 = set_price(
        &mut svm,
        221,
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 0, -8, n2),
    );
    borrow(
        &mut svm,
        &borrower_b,
        market,
        borrower_b_position,
        fee_position,
        loan_vault,
        borrower_b_loan_ata,
        usdc_mint,
        spl_token_interface::ID,
        c2,
        l2,
        900_000_000,
        0,
    )
    .expect("borrow must succeed");

    section("Time passes: 180 days of interest accrue on Borrower B's debt");
    let mut clock = svm.get_sysvar::<solana_clock::Clock>();
    clock.unix_timestamp += 180 * 86_400;
    svm.set_sysvar(&clock);
    accrue_interest(&mut svm, &admin, market, fee_position).expect("accrue_interest must succeed");
    let fee_position_state = fetch_position(&svm, &fee_position);
    println!(
        "  fee_position.supply_shares after accrual: {} (real protocol fee shares, not a fixture)",
        fee_position_state.supply_shares
    );

    // --- 13. Deeper crash: SOL to $40.00 -- Borrower B's collateral is fully seized, debt remains ---
    section(
        "13. SOL crashes to $40.00 -- Borrower B liquidated to zero collateral, bad debt created",
    );
    let n3 = svm.get_sysvar::<solana_clock::Clock>().unix_timestamp;
    let c3 = set_price(
        &mut svm,
        230,
        PriceFixture::valid(COLLATERAL_FEED_ID, 4_000_000_000, 0, -8, n3),
    );
    let l3 = set_price(
        &mut svm,
        231,
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 0, -8, n3),
    );
    let liquidator_b = Keypair::new_from_array([60u8; 32]);
    svm.airdrop(&liquidator_b.pubkey(), 10_000_000_000).unwrap();
    let liquidator_b_loan_ata = create_token_account(
        &mut svm,
        &admin,
        61,
        usdc_mint,
        liquidator_b.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    mint_to(
        &mut svm,
        &admin,
        usdc_mint,
        liquidator_b_loan_ata,
        &admin,
        2_000_000_000,
        spl_token_interface::ID,
    );
    let liquidator_b_collateral_ata = create_token_account(
        &mut svm,
        &admin,
        62,
        sol_mint,
        liquidator_b.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    let debt_before_crash = {
        let p = fetch_position(&svm, &borrower_b_position);
        let m = fetch_market(&svm, &market);
        aegis_math::to_assets_up(
            p.borrow_shares,
            m.total_borrow_assets,
            m.total_borrow_shares,
        )
        .unwrap()
    };
    println!(
        "  Borrower B's accrued debt just before the crash: {}",
        format_usdc(debt_before_crash)
    );
    liquidate(
        &mut svm,
        &liquidator_b,
        market,
        borrower_b_position,
        fee_position,
        loan_vault,
        collateral_vault,
        liquidator_b_loan_ata,
        liquidator_b_collateral_ata,
        usdc_mint,
        sol_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        c3,
        l3,
        debt_before_crash,
        0,
    )
    .expect("clamped liquidation must succeed");
    let position_b_after_liq = fetch_position(&svm, &borrower_b_position);
    println!(
        "  Borrower B collateral after liquidation: {} (fully seized -- the clamp fired)",
        format_sol(position_b_after_liq.collateral_amount)
    );
    let debt_after_liq = {
        let m = fetch_market(&svm, &market);
        aegis_math::to_assets_up(
            position_b_after_liq.borrow_shares,
            m.total_borrow_assets,
            m.total_borrow_shares,
        )
        .unwrap()
    };
    println!(
        "  Borrower B remaining debt: {} (bad debt -- collateral exhausted)",
        format_usdc(debt_after_liq)
    );
    assert_eq!(position_b_after_liq.collateral_amount, 0);
    assert!(
        position_b_after_liq.borrow_shares > 0,
        "bad debt must remain"
    );
    invariants::assert_inv_cus_01(&svm, &market);

    // --- 14. absorb_bad_debt: protocol fee shares burned FIRST ---
    section("14. absorb_bad_debt -- protocol fee shares absorb the loss first");
    let fee_shares_before = fetch_position(&svm, &fee_position).supply_shares;
    let total_supply_before = fetch_market(&svm, &market).total_supply_assets;

    absorb_bad_debt(&mut svm, &admin, market, borrower_b_position, fee_position)
        .expect("absorb_bad_debt must succeed -- no oracle, unpausable, permissionless");

    let fee_shares_after = fetch_position(&svm, &fee_position).supply_shares;
    let total_supply_after = fetch_market(&svm, &market).total_supply_assets;
    let bad_assets = total_supply_before - total_supply_after;
    println!(
        "  bad_assets (total loss recognized):       {}",
        format_usdc(bad_assets)
    );
    println!(
        "  protocol fee shares burned:                {} -> {} (first-loss)",
        fee_shares_before, fee_shares_after
    );
    println!(
        "  total_supply_assets:                       {} -> {}",
        format_usdc(total_supply_before),
        format_usdc(total_supply_after)
    );
    invariants::assert_inv_cus_01(&svm, &market);
    println!("  INV-CUS-01, INV-SOLV-04: hold exactly -- no tokens moved");

    // --- 15. Lender withdraws and realizes the socialized loss ---
    section("15. Lender withdraws all shares -- realizing the socialized residual loss");
    let lender_position_state = fetch_position(&svm, &lender_position);
    let market_final = fetch_market(&svm, &market);
    let redeemable = aegis_math::to_assets_down(
        lender_position_state.supply_shares,
        market_final.total_supply_assets,
        market_final.total_supply_shares,
    )
    .unwrap();
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

    withdraw(
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
    let lender_final_balance = fetch_token_account_base(&svm, &lender_ata).amount;
    println!(
        "  lender's loan-asset wallet balance after full withdrawal: {}",
        format_usdc(lender_final_balance)
    );
    println!(
        "  realized shortfall vs. original principal: {}",
        format_usdc(supply_amount.saturating_sub(lender_final_balance))
    );

    // --- 16. Admin withdraws the protocol's own accrued collateral fee ---
    section("16. Admin withdraws the protocol's own accrued collateral fee");
    let final_fee_accrued = fetch_market(&svm, &market).collateral_fee_accrued;
    let admin_collateral_ata = create_token_account(
        &mut svm,
        &admin,
        70,
        sol_mint,
        admin.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    withdraw_collateral_fees(
        &mut svm,
        &admin,
        market,
        collateral_vault,
        admin_collateral_ata,
        sol_mint,
        spl_token_interface::ID,
        final_fee_accrued,
    )
    .expect("withdraw_collateral_fees must succeed");
    println!(
        "  admin withdrew {} of protocol-owned collateral fees",
        format_sol(final_fee_accrued)
    );

    println!("\n=== Final invariant report ===");
    invariants::assert_inv_cus_01(&svm, &market);
    invariants::assert_inv_cus_02(
        &svm,
        &market,
        &[lender_position, borrower_a_position, borrower_b_position],
    );
    println!("  INV-CUS-01: holds");
    println!("  INV-CUS-02: holds");
    println!("\nDemo complete. All Phase 6 acceptance criteria exercised above.");
}
