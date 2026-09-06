//! Phase 5 demo (`docs/phases/phase-05-oracle.md` "Demo"): real, oracle-validated `borrow`
//! succeeds; the oracle is made stale and `borrow` fails closed while a risky debt-bearing
//! withdrawal also fails, but `repay` and `deposit_collateral` keep working; the oracle recovers
//! at a new price and the recomputed health factor is printed.
//!
//! Zero-cost and local: an in-process LiteSVM instance loaded with the actual built `aegis.so`
//! and the real embedded SPL Token program bytecode LiteSVM ships, plus byte-exact `PriceUpdateV2`
//! fixtures built with the real `pyth-solana-receiver-sdk` and injected directly via
//! `LiteSVM::set_account`. No devnet, no RPC, no API key, no Hermes, no Pyth program deployment
//! (`docs/zero-cost-demo.md`, ADR-0008).
//!
//! Run with `make demo` (which runs `anchor build` first) or directly:
//! `cargo run -p aegis-test-kit --example phase5_demo`.

#![allow(clippy::result_large_err)]

use aegis::instructions::admin::CreateMarketArgs;
use aegis_math::{collateral_value, debt_value, health_factor, WAD};
use aegis_test_kit::create_spl_mint;
use aegis_test_kit::{
    borrow, create_market, create_token_account, deploy, deposit_collateral, fetch_market,
    fetch_position, fetch_token_account_base, init_position, initialize_protocol,
    inject_price_update, invariants, mint_to, reference_market_args, repay, set_price,
    spl_token_interface, supply, withdraw_collateral, PriceFixture,
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

fn format_wad_usd(wad_value: u128) -> String {
    let dollars = wad_value / WAD;
    let cents = (wad_value % WAD) / (WAD / 100);
    format!("${dollars}.{cents:02}")
}

const COLLATERAL_FEED_ID: [u8; 32] = [0xAAu8; 32]; // SOL/USD
const LOAN_FEED_ID: [u8; 32] = [0xBBu8; 32]; // USDC/USD

fn main() {
    println!("Aegis Protocol — Phase 5 demo (oracle-backed borrowing)");
    println!("Zero-cost, local, offline: in-process LiteSVM, no devnet, no RPC, no API key, no Hermes.\n");

    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    println!("Deployed program {} into LiteSVM.", aegis::id());
    println!("Admin/deployer:  {}", admin.pubkey());

    // --- 1. Protocol, market and positions ---
    section("1. Protocol, market and positions");
    let guardian = fixed_pubkey(2);
    let fee_recipient = fixed_pubkey(3);
    initialize_protocol(&mut svm, &admin, guardian, fee_recipient).expect("initialize_protocol");

    let sol_mint = create_spl_mint(&mut svm, &admin, 10, 9, admin.pubkey(), None);
    let usdc_mint = create_spl_mint(&mut svm, &admin, 11, 6, admin.pubkey(), None);
    let args: CreateMarketArgs = reference_market_args(0, COLLATERAL_FEED_ID, LOAN_FEED_ID, false);
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
    println!("collateral_vault:  {collateral_vault}");
    println!("loan_vault:        {loan_vault}");

    let lender = Keypair::new_from_array([20u8; 32]);
    svm.airdrop(&lender.pubkey(), 10_000_000_000)
        .expect("airdrop to lender");
    let lender_ata = create_token_account(
        &mut svm,
        &admin,
        21,
        usdc_mint,
        lender.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    let supply_amount = 1_000_000_000_000u64; // 1,000,000 USDC @ 6dp
    mint_to(
        &mut svm,
        &admin,
        usdc_mint,
        lender_ata,
        &admin,
        supply_amount,
        spl_token_interface::ID,
    );
    let (r, lender_position) = init_position(&mut svm, &admin, market, lender.pubkey());
    r.expect("init_position (lender)");
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
    println!("Lender supplied {supply_amount} (1,000,000.000000 USDC)");

    let borrower = Keypair::new_from_array([30u8; 32]);
    svm.airdrop(&borrower.pubkey(), 10_000_000_000)
        .expect("airdrop to borrower");
    let borrower_collateral_ata = create_token_account(
        &mut svm,
        &admin,
        31,
        sol_mint,
        borrower.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    let collateral_amount = 10_000_000_000u64; // 10 SOL
    mint_to(
        &mut svm,
        &admin,
        sol_mint,
        borrower_collateral_ata,
        &admin,
        collateral_amount,
        spl_token_interface::ID,
    );
    let (r, borrower_position) = init_position(&mut svm, &admin, market, borrower.pubkey());
    r.expect("init_position (borrower)");
    deposit_collateral(
        &mut svm,
        &borrower,
        market,
        borrower_position,
        collateral_vault,
        borrower_collateral_ata,
        sol_mint,
        spl_token_interface::ID,
        collateral_amount,
    )
    .expect("deposit_collateral must succeed");
    println!("Borrower deposited {collateral_amount} (10.000000000 SOL) collateral");
    let borrower_loan_ata = create_token_account(
        &mut svm,
        &admin,
        32,
        usdc_mint,
        borrower.pubkey(),
        spl_token_interface::ID,
        &[],
    );

    // --- 2. Set deterministic valid prices ---
    section("2. Deterministic valid Pyth price updates (SOL $150.00, USDC $1.00, zero conf)");
    let now = svm.get_sysvar::<solana_clock::Clock>().unix_timestamp;
    let collateral_price = set_price(
        &mut svm,
        200,
        PriceFixture::valid(COLLATERAL_FEED_ID, 15_000_000_000, 0, -8, now),
    );
    let loan_price = set_price(
        &mut svm,
        201,
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 0, -8, now),
    );
    println!("collateral_price_update: {collateral_price} (real PriceUpdateV2, owner = pyth receiver program)");
    println!("loan_price_update:       {loan_price}");

    // --- 3. Real, oracle-validated borrow succeeds ---
    section("3. borrow succeeds (real oracle validation, real LTV check)");
    let borrow_amount = 900_000_000u64; // 900 USDC
    borrow(
        &mut svm,
        &borrower,
        market,
        borrower_position,
        fee_position,
        loan_vault,
        borrower_loan_ata,
        usdc_mint,
        spl_token_interface::ID,
        collateral_price,
        loan_price,
        borrow_amount,
        0,
    )
    .expect("borrow must succeed against a valid oracle and within max_ltv");
    let position_state = fetch_position(&svm, &borrower_position);
    let market_state = fetch_market(&svm, &market);
    println!("  borrowed:                  {borrow_amount} (900.000000 USDC)");
    println!(
        "  position.borrow_shares:    {}",
        position_state.borrow_shares
    );
    let debt_value_now = debt_value(borrow_amount, 1_000_000_000_000_000_000u128, 6).unwrap();
    let collateral_value_now =
        collateral_value(collateral_amount, 150_000_000_000_000_000_000u128, 9).unwrap();
    let hf = health_factor(
        collateral_value_now,
        market_state.liq_threshold,
        debt_value_now,
    )
    .unwrap();
    println!(
        "  collateral value: {}  debt value: {}",
        format_wad_usd(collateral_value_now),
        format_wad_usd(debt_value_now)
    );
    println!(
        "  health factor: {}.{:04}",
        hf / WAD,
        (hf % WAD) / (WAD / 10_000)
    );
    invariants::assert_inv_cus_01(&svm, &market);
    println!("  INV-CUS-01: holds");

    // --- 4. Oracle becomes stale ---
    section("4. Oracle becomes stale (30 days pass, no new price posted)");
    let mut clock = svm.get_sysvar::<solana_clock::Clock>();
    clock.unix_timestamp += 30 * 86_400;
    svm.set_sysvar(&clock);
    println!("  warped forward 30 days; the SAME price accounts are now far past max_price_age_secs (60s)");

    // --- 5. borrow fails closed ---
    section("5. borrow fails closed against the stale oracle");
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
        collateral_price,
        loan_price,
        1_000_000,
        0,
    );
    match &stale_borrow {
        Err(failed) => println!("  borrow(1 USDC) -> REJECTED: {:?}", failed.err),
        Ok(_) => panic!("borrow must fail against a stale oracle -- it did not"),
    }

    // --- 6. Risky debt-bearing withdrawal also fails closed ---
    section("6. debt-bearing withdraw_collateral also fails closed against the stale oracle");
    let stale_withdraw = withdraw_collateral(
        &mut svm,
        &borrower,
        market,
        borrower_position,
        collateral_vault,
        borrower_collateral_ata,
        sol_mint,
        spl_token_interface::ID,
        collateral_price,
        loan_price,
        1_000_000_000,
    );
    match &stale_withdraw {
        Err(failed) => println!("  withdraw_collateral(1 SOL) -> REJECTED: {:?}", failed.err),
        Ok(_) => panic!("debt-bearing withdraw must fail against a stale oracle -- it did not"),
    }

    // --- 7. repay still succeeds (no oracle needed) ---
    section("7. repay still succeeds -- no oracle required (INV-REP-01)");
    repay(
        &mut svm,
        &borrower,
        market,
        borrower_position,
        fee_position,
        loan_vault,
        borrower_loan_ata,
        usdc_mint,
        spl_token_interface::ID,
        100_000_000,
        0,
    )
    .expect("repay must succeed while the oracle is stale");
    println!("  repaid 100.000000 USDC while the oracle was stale");

    // --- 8. deposit_collateral still succeeds (no oracle needed) ---
    section("8. deposit_collateral still succeeds -- no oracle required (INV-ORA-02)");
    mint_to(
        &mut svm,
        &admin,
        sol_mint,
        borrower_collateral_ata,
        &admin,
        1_000_000_000,
        spl_token_interface::ID,
    );
    deposit_collateral(
        &mut svm,
        &borrower,
        market,
        borrower_position,
        collateral_vault,
        borrower_collateral_ata,
        sol_mint,
        spl_token_interface::ID,
        1_000_000_000,
    )
    .expect("deposit_collateral must succeed while the oracle is stale");
    println!("  deposited 1 more SOL while the oracle was stale");
    invariants::assert_inv_cus_01(&svm, &market);
    println!("  INV-CUS-01: still holds through the whole outage episode");

    // --- 9. Oracle recovers at a new price ---
    section("9. Oracle recovers -- SOL now $120.00");
    let recovery_time = svm.get_sysvar::<solana_clock::Clock>().unix_timestamp;
    inject_price_update(
        &mut svm,
        collateral_price,
        PriceFixture::valid(COLLATERAL_FEED_ID, 12_000_000_000, 0, -8, recovery_time),
    );
    inject_price_update(
        &mut svm,
        loan_price,
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 0, -8, recovery_time),
    );

    let final_position = fetch_position(&svm, &borrower_position);
    let final_market = fetch_market(&svm, &market);
    let debt_assets_final = aegis_math::to_assets_up(
        final_position.borrow_shares,
        final_market.total_borrow_assets,
        final_market.total_borrow_shares,
    )
    .unwrap();
    let debt_value_final = debt_value(debt_assets_final, 1_000_000_000_000_000_000u128, 6).unwrap();
    let collateral_value_final = collateral_value(
        final_position.collateral_amount,
        120_000_000_000_000_000_000u128,
        9,
    )
    .unwrap();
    let hf_final = health_factor(
        collateral_value_final,
        final_market.liq_threshold,
        debt_value_final,
    )
    .unwrap();
    println!(
        "  position.collateral_amount: {} ({}.{:09} SOL)",
        final_position.collateral_amount,
        final_position.collateral_amount / 1_000_000_000,
        final_position.collateral_amount % 1_000_000_000
    );
    println!(
        "  debt_assets: {debt_assets_final}  debt_value: {}",
        format_wad_usd(debt_value_final)
    );
    println!(
        "  collateral_value at the new price: {}",
        format_wad_usd(collateral_value_final)
    );
    println!(
        "  new health factor: {}.{:04}",
        hf_final / WAD,
        (hf_final % WAD) / (WAD / 10_000)
    );

    // A fresh borrow now succeeds again, proving the position's own accounting survived the
    // whole outage-and-recovery episode intact.
    svm.expire_blockhash();
    borrow(
        &mut svm,
        &borrower,
        market,
        borrower_position,
        fee_position,
        loan_vault,
        borrower_loan_ata,
        usdc_mint,
        spl_token_interface::ID,
        collateral_price,
        loan_price,
        1_000_000,
        0,
    )
    .expect("borrow must succeed again once the oracle has recovered");
    invariants::assert_inv_cus_01(&svm, &market);
    println!("\n  INV-CUS-01: holds after the full outage-and-recovery episode");

    let loan_vault_final = fetch_token_account_base(&svm, &loan_vault);
    println!("  loan_vault.amount: {}", loan_vault_final.amount);

    println!("\nDemo complete. All Phase 5 acceptance criteria exercised above.");
}
