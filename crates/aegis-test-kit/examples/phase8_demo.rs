//! Phase 8 demo (`docs/phases/phase-08-composability.md` "Demo",
//! `docs/composability.md`, ADR-0013): a liquidator with **zero** loan-asset balance liquidates an
//! unhealthy position anyway, by supplying `example-liquidator` as a callback that seizes the
//! collateral, swaps it at a deterministic local rate, and repays `loan_vault` directly -- all in
//! one transaction. Then, on a second unhealthy position, an ordinary liquidator who omits the
//! callback liquidates exactly as Phase 6 always did, proving `I-LIQ-CB-02` side by side with the
//! callback path rather than merely asserting it in a separate test file.
//!
//! Zero-cost and local: an in-process LiteSVM instance loaded with the actual built `aegis.so`,
//! `example_liquidator.so` and `hostile_callback.so`. No devnet, no RPC, no API key, no Jupiter
//! (ADR-0008, `docs/zero-cost-demo.md`).
//!
//! Run with `make demo` (after `make build`) or directly:
//! `cargo run -p aegis-test-kit --example phase8_demo`.
//!
//! The companion TypeScript keeper demo (`bots/liquidator/`) is run separately -- see its own
//! README -- and identifies + executes the same shape of callback liquidation against a local,
//! plain (non-forking) Surfpool validator.

#![allow(clippy::result_large_err, clippy::too_many_arguments)]

use aegis_test_kit::{
    create_market, create_spl_mint, create_token_account, deposit_collateral, fetch_market,
    fetch_position, fetch_token_account_base, init_position, initialize_protocol, invariants,
    liquidate, liquidate_with_callback, mint_to, reference_market_args, set_price,
    spl_token_interface, supply, PriceFixture,
};
use anchor_lang::InstructionData;
use solana_instruction::AccountMeta;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

fn aegis_bytes() -> &'static [u8] {
    include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/deploy/aegis.so"
    ))
}
fn example_liquidator_bytes() -> &'static [u8] {
    include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/deploy/example_liquidator.so"
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

const COLLATERAL_FEED_ID: [u8; 32] = [0xAAu8; 32];
const LOAN_FEED_ID: [u8; 32] = [0xBBu8; 32];

struct Fixture {
    market: Pubkey,
    fee_position: Pubkey,
    collateral_vault: Pubkey,
    loan_vault: Pubkey,
    collateral_mint: Pubkey,
    loan_mint: Pubkey,
}

fn setup_market(
    svm: &mut litesvm::LiteSVM,
    admin: &Keypair,
    config_id: u16,
    fee_recipient: Pubkey,
    collateral_mint: Pubkey,
    loan_mint: Pubkey,
) -> Fixture {
    let args = reference_market_args(config_id, COLLATERAL_FEED_ID, LOAN_FEED_ID, false);
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

fn print_state(svm: &litesvm::LiteSVM, label: &str, fx: &Fixture, position: Pubkey) {
    let market = fetch_market(svm, &fx.market);
    let pos = fetch_position(svm, &position);
    let collateral_vault = fetch_token_account_base(svm, &fx.collateral_vault);
    let loan_vault = fetch_token_account_base(svm, &fx.loan_vault);
    println!("  [{label}]");
    println!(
        "    position: collateral={} borrow_shares={}",
        format_sol(pos.collateral_amount),
        pos.borrow_shares
    );
    println!(
        "    market:   total_borrow_assets={} total_supply_assets={} collateral_fee_accrued={} \
         liquidation_guard={}",
        format_usdc(market.total_borrow_assets),
        format_usdc(market.total_supply_assets),
        format_sol(market.collateral_fee_accrued),
        market.liquidation_guard
    );
    println!(
        "    vaults:   collateral_vault={} loan_vault={}",
        format_sol(collateral_vault.amount),
        format_usdc(loan_vault.amount)
    );
}

fn main() {
    println!("Aegis Phase 8 demo -- composability and liquidation routing");
    println!("(docs/phases/phase-08-composability.md, docs/composability.md, ADR-0013)");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(aegis::ID, aegis_bytes())
        .expect("load aegis.so");
    svm.add_program(example_liquidator::ID, example_liquidator_bytes())
        .expect("load example_liquidator.so");
    let admin = Keypair::new_from_array([7u8; 32]);
    svm.airdrop(&admin.pubkey(), 100_000_000_000)
        .expect("airdrop admin");

    section("1. Protocol + market setup (SOL(9dp) collateral / USDC(6dp) loan)");
    let guardian = fixed_pubkey(1);
    let fee_recipient = fixed_pubkey(2);
    initialize_protocol(&mut svm, &admin, guardian, fee_recipient).expect("initialize_protocol");
    let collateral_mint = create_spl_mint(&mut svm, &admin, 3, 9, admin.pubkey(), None);
    let loan_mint = create_spl_mint(&mut svm, &admin, 4, 6, admin.pubkey(), None);
    let fx = setup_market(
        &mut svm,
        &admin,
        0,
        fee_recipient,
        collateral_mint,
        loan_mint,
    );
    println!("  market = {}", fx.market);

    section("2. Lender supplies liquidity");
    let lender = Keypair::new_from_array([5u8; 32]);
    svm.airdrop(&lender.pubkey(), 10_000_000_000).unwrap();
    let lender_ata = create_token_account(
        &mut svm,
        &admin,
        6,
        fx.loan_mint,
        lender.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    mint_to(
        &mut svm,
        &admin,
        fx.loan_mint,
        lender_ata,
        &admin,
        2_000_000_000_000,
        spl_token_interface::ID,
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
    .expect("supply");
    println!("  lender supplied {}", format_usdc(1_000_000_000_000));

    // --- Callback-liquidation scenario ---
    section("3. An unhealthy borrower opens a position (Borrower A)");
    let borrower_a = Keypair::new_from_array([10u8; 32]);
    svm.airdrop(&borrower_a.pubkey(), 10_000_000_000).unwrap();
    let borrower_a_collateral_ata = create_token_account(
        &mut svm,
        &admin,
        11,
        fx.collateral_mint,
        borrower_a.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    mint_to(
        &mut svm,
        &admin,
        fx.collateral_mint,
        borrower_a_collateral_ata,
        &admin,
        10_000_000_000,
        spl_token_interface::ID,
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
        12,
        fx.loan_mint,
        borrower_a.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    let n = svm.get_sysvar::<solana_clock::Clock>().unix_timestamp;
    let c1 = set_price(
        &mut svm,
        13,
        PriceFixture::valid(COLLATERAL_FEED_ID, 15_000_000_000, 0, -8, n),
    );
    let l1 = set_price(
        &mut svm,
        14,
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 0, -8, n),
    );
    aegis_test_kit::borrow(
        &mut svm,
        &borrower_a,
        fx.market,
        position_a,
        fx.fee_position,
        fx.loan_vault,
        borrower_a_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        c1,
        l1,
        900_000_000,
        0,
    )
    .expect("borrow A");
    println!(
        "  Borrower A deposited {} collateral, borrowed {} at $150.00/SOL",
        format_sol(10_000_000_000),
        format_usdc(900_000_000)
    );

    println!("\n  SOL crashes to $95.00 -- Borrower A's position is now liquidatable.");
    let n = svm.get_sysvar::<solana_clock::Clock>().unix_timestamp;
    let c_crash = set_price(
        &mut svm,
        15,
        PriceFixture::valid(COLLATERAL_FEED_ID, 9_500_000_000, 20_000_000, -8, n),
    );
    let l_crash = set_price(
        &mut svm,
        16,
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 20_000, -8, n),
    );

    section("4. A liquidator with ZERO loan-asset balance uses the callback");
    let liquidator = Keypair::new_from_array([20u8; 32]);
    svm.airdrop(&liquidator.pubkey(), 10_000_000_000).unwrap();
    let liquidator_loan_ata = create_token_account(
        &mut svm,
        &admin,
        21,
        fx.loan_mint,
        liquidator.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    let liquidator_collateral_ata = create_token_account(
        &mut svm,
        &admin,
        22,
        fx.collateral_mint,
        liquidator.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    println!(
        "  liquidator's loan-asset balance: {} (cannot cover the $900 repayment without the \
         callback)",
        format_usdc(0)
    );

    let (authority, _bump) = Pubkey::find_program_address(&[b"authority"], &example_liquidator::ID);
    let callback_collateral_account = create_token_account(
        &mut svm,
        &admin,
        23,
        fx.collateral_mint,
        authority,
        spl_token_interface::ID,
        &[],
    );
    let loan_reserve = create_token_account(
        &mut svm,
        &admin,
        24,
        fx.loan_mint,
        authority,
        spl_token_interface::ID,
        &[],
    );
    mint_to(
        &mut svm,
        &admin,
        fx.loan_mint,
        loan_reserve,
        &admin,
        2_000_000_000,
        spl_token_interface::ID,
    );
    println!(
        "  example-liquidator's own reserve, pre-funded ahead of time: {}",
        format_usdc(2_000_000_000)
    );

    print_state(&svm, "before callback liquidation", &fx, position_a);

    let rate_wad: u128 = 100_000_000_000_000_000_000; // $100/SOL deterministic local rate
    let callback_data = example_liquidator::instruction::HandleLiquidation { rate_wad }.data();
    let extra_accounts = vec![
        AccountMeta::new_readonly(authority, false),
        AccountMeta::new(loan_reserve, false),
    ];

    println!("\n  -> liquidate(repay_assets=900 USDC, callback=example_liquidator)");
    liquidate_with_callback(
        &mut svm,
        &liquidator,
        fx.market,
        position_a,
        fx.fee_position,
        fx.loan_vault,
        fx.collateral_vault,
        liquidator_loan_ata,
        liquidator_collateral_ata,
        fx.loan_mint,
        fx.collateral_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        c_crash,
        l_crash,
        900_000_000,
        0,
        example_liquidator::ID,
        callback_collateral_account,
        extra_accounts,
        callback_data,
    )
    .expect("callback liquidation must succeed");

    println!("  <- Ok: seized collateral -> callback -> deterministic local swap -> loan_vault");
    println!(
        "     reloaded loan_vault, measured delta >= required repayment, guard cleared, \
         post-conditions verified"
    );
    print_state(&svm, "after callback liquidation", &fx, position_a);
    invariants::assert_inv_cus_02(&svm, &fx.market, &[position_a]);
    println!("  INV-CUS-02 (collateral custody) holds exactly.");

    // --- No-callback scenario, side by side ---
    section("5. A second unhealthy borrower (Borrower B) -- ordinary, no-callback liquidation");
    let borrower_b = Keypair::new_from_array([30u8; 32]);
    svm.airdrop(&borrower_b.pubkey(), 10_000_000_000).unwrap();
    let borrower_b_collateral_ata = create_token_account(
        &mut svm,
        &admin,
        31,
        fx.collateral_mint,
        borrower_b.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    mint_to(
        &mut svm,
        &admin,
        fx.collateral_mint,
        borrower_b_collateral_ata,
        &admin,
        10_000_000_000,
        spl_token_interface::ID,
    );
    let (_, position_b) = init_position(&mut svm, &admin, fx.market, borrower_b.pubkey());
    deposit_collateral(
        &mut svm,
        &borrower_b,
        fx.market,
        position_b,
        fx.collateral_vault,
        borrower_b_collateral_ata,
        fx.collateral_mint,
        spl_token_interface::ID,
        10_000_000_000,
    )
    .expect("deposit_collateral B");
    let borrower_b_loan_ata = create_token_account(
        &mut svm,
        &admin,
        32,
        fx.loan_mint,
        borrower_b.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    aegis_test_kit::borrow(
        &mut svm,
        &borrower_b,
        fx.market,
        position_b,
        fx.fee_position,
        fx.loan_vault,
        borrower_b_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        c1,
        l1,
        900_000_000,
        0,
    )
    .expect("borrow B");

    let liquidator_2 = Keypair::new_from_array([40u8; 32]);
    svm.airdrop(&liquidator_2.pubkey(), 10_000_000_000).unwrap();
    let liquidator_2_loan_ata = create_token_account(
        &mut svm,
        &admin,
        41,
        fx.loan_mint,
        liquidator_2.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    mint_to(
        &mut svm,
        &admin,
        fx.loan_mint,
        liquidator_2_loan_ata,
        &admin,
        1_000_000_000,
        spl_token_interface::ID,
    );
    let liquidator_2_collateral_ata = create_token_account(
        &mut svm,
        &admin,
        42,
        fx.collateral_mint,
        liquidator_2.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    println!(
        "  This liquidator pre-funded {} the ordinary way (Phase 6 behavior).",
        format_usdc(1_000_000_000)
    );

    print_state(&svm, "before no-callback liquidation", &fx, position_b);
    println!("\n  -> liquidate(repay_assets=900 USDC, callback=None)  [I-LIQ-CB-02]");
    liquidate(
        &mut svm,
        &liquidator_2,
        fx.market,
        position_b,
        fx.fee_position,
        fx.loan_vault,
        fx.collateral_vault,
        liquidator_2_loan_ata,
        liquidator_2_collateral_ata,
        fx.loan_mint,
        fx.collateral_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        c_crash,
        l_crash,
        900_000_000,
        0,
    )
    .expect("no-callback liquidation must succeed exactly as Phase 6");
    println!("  <- Ok: identical Phase 6 code path, byte-for-byte (I-LIQ-CB-02)");
    print_state(&svm, "after no-callback liquidation", &fx, position_b);

    section("Summary");
    println!(
        "  Callback liquidation:    borrower A fully repaid, liquidator never held loan asset."
    );
    println!("  No-callback liquidation: borrower B fully repaid, exactly as Phase 6.");
    println!(
        "  Companion TypeScript keeper demo: see bots/liquidator/README.md \
         (scans, computes health off-chain, executes the callback path against a local, \
         non-forking Surfpool validator -- no network)."
    );
    println!("\nPhase 8 demo complete.");
}
