//! Phase 4 demo (`docs/phases/phase-04-lending.md` "Demo"): a lender supplies loan liquidity, a
//! borrow is attempted and correctly refused (`OracleNotYetAvailable`), a debt position is seeded
//! through TEST-KIT state injection (never a weakened `borrow`), time is warped 30 days, interest
//! accrues, utilization/borrow-APY/supply-APY are printed, the protocol fee accrues as supply
//! shares, and the lender withdraws principal plus earned interest.
//!
//! Zero-cost and local: an in-process LiteSVM instance loaded with the actual built `aegis.so` and
//! the real embedded SPL Token program bytecode LiteSVM ships. No devnet, no RPC, no API key
//! (`docs/zero-cost-demo.md`). Time is warped via the sysvar `Clock`, never real wall-clock
//! waiting.
//!
//! Run with `make demo` (which runs `anchor build` first) or directly:
//! `cargo run -p aegis-test-kit --example phase4_demo`.

#![allow(clippy::result_large_err)]

use aegis::instructions::admin::CreateMarketArgs;
use aegis_math::{
    borrow_rate, mul_div_floor, taylor3, taylor_x, utilization, SECONDS_PER_YEAR, WAD,
};
use aegis_test_kit::{
    accrue_interest, borrow_ix, create_market, create_spl_mint, create_token_account, deploy,
    fetch_market, fetch_position, fetch_token_account_base, init_position, initialize_protocol,
    invariants, mint_to, reference_market_args, seed_borrow_state, spl_token_interface, supply,
    withdraw,
};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_message::{Message, VersionedMessage};
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;

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

fn send(
    svm: &mut litesvm::LiteSVM,
    payer: &Keypair,
    ix: Instruction,
) -> litesvm::types::TransactionResult {
    let blockhash = svm.latest_blockhash();
    let message = Message::new_with_blockhash(&[ix], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(message), &[payer])
        .expect("failed to sign transaction");
    svm.send_transaction(tx)
}

/// Formats a WAD fraction as a percentage string with 4 decimal places, without floats: prints
/// the integer and fractional parts computed by plain integer division/remainder.
fn format_wad_pct(wad_value: u128) -> String {
    let scaled = wad_value * 1_000_000 / WAD; // basis points * 100, i.e. hundred-thousandths of 1%
    let whole = scaled / 10_000;
    let frac = scaled % 10_000;
    format!("{whole}.{frac:04}%")
}

fn main() {
    println!("Aegis Protocol — Phase 4 demo (lending, borrowing and interest)");
    println!("Zero-cost, local, offline: in-process LiteSVM, no devnet, no RPC, no API key.\n");

    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    println!("Deployed program {} into LiteSVM.", aegis::id());
    println!("Admin/deployer:  {}", admin.pubkey());

    // --- 1. Protocol and market ---
    section("1. Protocol and market");
    let guardian = fixed_pubkey(2);
    let fee_recipient = fixed_pubkey(3);
    initialize_protocol(&mut svm, &admin, guardian, fee_recipient).expect("initialize_protocol");

    let sol_mint = create_spl_mint(&mut svm, &admin, 10, 9, admin.pubkey(), None);
    let usdc_mint = create_spl_mint(&mut svm, &admin, 11, 6, admin.pubkey(), None);
    // Real reference IRM params from economic-model.md §4.1 (reference_market_args' own copy sets
    // every slope to zero, since Phase 2/3 never accrue interest).
    let args = CreateMarketArgs {
        base_rate_ps: 0,
        slope1_ps: 1_268_391_679,
        slope2_ps: 31_709_791_983,
        u_kink: 800_000_000_000_000_000,
        max_rate_ps: 317_097_919_837,
        fee: 100_000_000_000_000_000, // 0.10 WAD
        ..reference_market_args(0, [0xAA; 32], [0xBB; 32], false)
    };
    let (result, market, _collateral_vault, loan_vault, fee_position) = create_market(
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
    println!("Market:      {market}");
    println!("loan_vault:  {loan_vault}");
    println!("fee_position: {fee_position} (owner {fee_recipient})");

    // --- 2. Lender supplies loan liquidity ---
    section("2. Lender supplies loan liquidity");
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
    let lender_state = fetch_position(&svm, &lender_position);
    println!("  supplied:       {supply_amount} (1,000,000.000000 USDC)");
    println!("  supply_shares:  {}", lender_state.supply_shares);
    invariants::assert_all_lending(&svm, &market, &[lender_position], &fee_position);
    println!("  INV-CUS-01 / INV-ACC-01/02/03/06: all hold");

    // --- 3. Borrow is attempted and correctly refused ---
    section("3. Borrow is attempted -- and correctly refused");
    let borrower = Keypair::new_from_array([30u8; 32]);
    svm.airdrop(&borrower.pubkey(), 10_000_000_000)
        .expect("airdrop to borrower");
    let borrower_ata = create_token_account(
        &mut svm,
        &admin,
        31,
        usdc_mint,
        borrower.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    let (r, borrower_position) = init_position(&mut svm, &admin, market, borrower.pubkey());
    r.expect("init_position (borrower)");
    let ix = borrow_ix(
        &borrower.pubkey(),
        market,
        borrower_position,
        fee_position,
        loan_vault,
        borrower_ata,
        usdc_mint,
        spl_token_interface::ID,
        500_000_000_000,
        0,
    );
    let borrow_result = send(&mut svm, &borrower, ix);
    match &borrow_result {
        Err(failed) => println!("  borrow(500,000 USDC) -> REJECTED: {:?}", failed.err),
        Ok(_) => panic!("borrow must fail in Phase 4 -- it did not"),
    }
    let borrower_state = fetch_position(&svm, &borrower_position);
    assert_eq!(
        borrower_state.borrow_shares, 0,
        "the refused borrow must not have moved any state"
    );
    println!(
        "  position.borrow_shares after refusal: {} (unchanged)",
        borrower_state.borrow_shares
    );

    // --- 4. Seed debt via TEST-KIT state injection (not a weakened borrow) ---
    section("4. Seed debt via TEST-KIT state injection");
    let seeded_debt = 900_000_000_000u64; // 90% utilization against the 1,000,000 USDC supplied
    let seeded_shares = seeded_debt as u128 * 1_000_000;
    seed_borrow_state(
        &mut svm,
        market,
        borrower_position,
        seeded_debt,
        seeded_shares,
    );
    println!("  seeded total_borrow_assets += {seeded_debt} (900,000.000000 USDC)");
    println!("  (this is a test fixture, not a real instruction -- borrow remains hard-gated)");
    invariants::assert_inv_cus_01(&svm, &market);
    println!("  INV-CUS-01: holds immediately after injection");

    // --- 5. Time warped 30 days ---
    section("5. Time warped 30 days (sysvar Clock, no real wall-clock waiting)");
    let before = fetch_market(&svm, &market);
    let mut clock = svm.get_sysvar::<solana_clock::Clock>();
    let dt = 30 * 86_400i64;
    clock.unix_timestamp += dt;
    svm.set_sysvar(&clock);
    println!("  last_accrual_ts before: {}", before.last_accrual_ts);
    println!("  warped forward by:      {dt} seconds (30 days)");

    // --- 6. Utilization and projected APYs, computed from current state before accrual ---
    section("6. Utilization and projected APYs (current-rate projection)");
    let u = utilization(before.total_borrow_assets, before.total_supply_assets).unwrap();
    println!("  utilization: {}", format_wad_pct(u));
    let r_ps = borrow_rate(
        u,
        before.base_rate_ps,
        before.slope1_ps,
        before.slope2_ps,
        before.u_kink,
        before.max_rate_ps,
    )
    .unwrap();
    let borrow_x_year = taylor_x(r_ps, SECONDS_PER_YEAR as u64).unwrap();
    let borrow_apy = taylor3(borrow_x_year).unwrap();
    println!("  borrow APY (projected): {}", format_wad_pct(borrow_apy));

    let fee = before.fee;
    let one_minus_fee = WAD - fee;
    let supply_r_ps = mul_div_floor(r_ps, u, WAD).unwrap();
    let supply_r_ps_net = mul_div_floor(supply_r_ps, one_minus_fee, WAD).unwrap();
    let supply_x_year = taylor_x(supply_r_ps_net, SECONDS_PER_YEAR as u64).unwrap();
    let supply_apy = taylor3(supply_x_year).unwrap();
    println!(
        "  supply APY (projected, net of {} protocol fee): {}",
        format_wad_pct(fee),
        format_wad_pct(supply_apy)
    );

    // --- 7. Interest accrues, permissionlessly ---
    section("7. accrue_interest (permissionless)");
    let keeper = Keypair::new_from_array([40u8; 32]);
    svm.airdrop(&keeper.pubkey(), 10_000_000_000)
        .expect("airdrop to keeper");
    accrue_interest(&mut svm, &keeper, market, fee_position).expect("accrue_interest must succeed");
    let after = fetch_market(&svm, &market);
    println!(
        "  called by: {} (an unrelated keeper, not the admin)",
        keeper.pubkey()
    );
    println!(
        "  total_borrow_assets: {} -> {}",
        before.total_borrow_assets, after.total_borrow_assets
    );
    println!(
        "  total_supply_assets: {} -> {}",
        before.total_supply_assets, after.total_supply_assets
    );
    println!(
        "  last_accrual_ts:     {} -> {}",
        before.last_accrual_ts, after.last_accrual_ts
    );
    let interest = after.total_borrow_assets - before.total_borrow_assets;
    println!(
        "  interest accrued over 30 days: {interest} base units ({}.{:06} USDC)",
        interest / 1_000_000,
        interest % 1_000_000
    );
    invariants::assert_all_lending(
        &svm,
        &market,
        &[lender_position, borrower_position],
        &fee_position,
    );
    println!("  INV-CUS-01 / INV-ACC-01/02/03/06: all hold after accrual");

    // --- 8. Protocol fee shares accrued ---
    section("8. Protocol fee shares accrued");
    let fee_position_state = fetch_position(&svm, &fee_position);
    println!(
        "  fee_position.supply_shares: {}",
        fee_position_state.supply_shares
    );
    let fee_claim = aegis_math::to_assets_down(
        fee_position_state.supply_shares,
        after.total_supply_assets,
        after.total_supply_shares,
    )
    .unwrap();
    println!("  fee_position's claimable assets: {fee_claim} base units");

    // --- 9. Lender withdraws principal plus earned interest ---
    section("9. Lender withdraws principal plus earned interest");
    let lender_state = fetch_position(&svm, &lender_position);
    let lender_claim = aegis_math::to_assets_down(
        lender_state.supply_shares,
        after.total_supply_assets,
        after.total_supply_shares,
    )
    .unwrap();
    let free_liquidity = after.total_supply_assets - after.total_borrow_assets;
    let withdraw_amount = lender_claim.min(free_liquidity);
    println!(
        "  lender's full claim: {lender_claim} (principal {supply_amount} + interest {})",
        lender_claim - supply_amount
    );
    println!("  free liquidity available: {free_liquidity}");
    println!("  withdrawing: {withdraw_amount} (bounded by free liquidity: most of the pool is lent out to the borrower)");
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
        withdraw_amount,
        0,
    )
    .expect("withdrawal must succeed");
    let ata_after = fetch_token_account_base(&svm, &lender_ata);
    println!(
        "  lender_ata balance after withdrawal: {}",
        ata_after.amount
    );
    assert_eq!(
        ata_after.amount, withdraw_amount,
        "sanity: the lender's wallet balance must reflect exactly the withdrawn amount"
    );

    invariants::assert_all_lending(
        &svm,
        &market,
        &[lender_position, borrower_position],
        &fee_position,
    );
    println!("\n  INV-CUS-01 / INV-ACC-01/02/03/06: all hold after the full flow");

    println!("\nDemo complete. All Phase 4 acceptance criteria exercised above.");
}
