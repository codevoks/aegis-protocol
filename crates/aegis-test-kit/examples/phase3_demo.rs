//! Phase 3 demo (`docs/phases/phase-03-collateral.md` "Demo"): SPL and Token-2022 transfer-fee
//! collateral deposits with measured-delta accounting, the INV-CUS-02 custody invariant checked
//! after every step, a zero-debt withdrawal, and closing a position with rent reclaimed.
//!
//! Zero-cost and local: an in-process LiteSVM instance loaded with the actual built `aegis.so`
//! and the real embedded SPL Token / Token-2022 program bytecode LiteSVM ships. No devnet, no
//! RPC, no API key (`docs/zero-cost-demo.md`).
//!
//! Run with `make demo` (which runs `anchor build` first) or directly:
//! `cargo run -p aegis-test-kit --example phase3_demo`.

#![allow(clippy::result_large_err)]

use aegis_test_kit::{
    close_position, create_market, create_spl_mint, create_token_2022_mint, create_token_account,
    deploy, deposit_collateral, fetch_position, init_position, initialize_protocol, invariants,
    mint_to, reference_market_args, spl_token_2022_interface, spl_token_interface,
    withdraw_collateral, Token2022Extension,
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

fn main() {
    println!("Aegis Protocol — Phase 3 demo (collateral flows)");
    println!("Zero-cost, local, offline: in-process LiteSVM, no devnet, no RPC, no API key.\n");

    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    println!("Deployed program {} into LiteSVM.", aegis::id());
    println!("Admin/deployer:  {}", admin.pubkey());

    // --- 1. Protocol, markets and positions ---
    section("1. Protocol, markets and positions");
    let guardian = fixed_pubkey(2);
    let fee_recipient = fixed_pubkey(3);
    initialize_protocol(&mut svm, &admin, guardian, fee_recipient).expect("initialize_protocol");
    println!(
        "Protocol initialized. admin={} guardian={guardian}",
        admin.pubkey()
    );

    let sol_mint = create_spl_mint(&mut svm, &admin, 10, 9, admin.pubkey(), None);
    let usdc_mint = create_spl_mint(&mut svm, &admin, 11, 6, admin.pubkey(), None);
    let spl_args = reference_market_args(0, [0xAA; 32], [0xBB; 32], false);
    let (result, spl_market, spl_cvault, ..) = create_market(
        &mut svm,
        &admin,
        sol_mint,
        usdc_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        fee_recipient,
        spl_args,
    );
    result.expect("SPL market creation must succeed");
    println!("SPL market:         {spl_market}");
    println!("  collateral_vault: {spl_cvault}");

    let basis_points = 500u16; // 5%
    let maximum_fee = 10_000_000_000u64;
    let fee_mint = create_token_2022_mint(
        &mut svm,
        &admin,
        20,
        9,
        admin.pubkey(),
        None,
        &[Token2022Extension::TransferFeeConfig {
            basis_points,
            maximum_fee,
        }],
    );
    let t22_args = reference_market_args(0, [0xCC; 32], [0xDD; 32], false);
    let (result, t22_market, t22_cvault, ..) = create_market(
        &mut svm,
        &admin,
        fee_mint,
        usdc_mint,
        spl_token_2022_interface::ID,
        spl_token_interface::ID,
        fee_recipient,
        t22_args,
    );
    result.expect("Token-2022 market creation must succeed");
    println!("Token-2022 market:  {t22_market}  (5% transfer fee on collateral)");
    println!("  collateral_vault: {t22_cvault}");

    let spl_owner = Keypair::new_from_array([40u8; 32]);
    svm.airdrop(&spl_owner.pubkey(), 10_000_000_000)
        .expect("airdrop to SPL position owner");
    let (r, spl_position) = init_position(&mut svm, &admin, spl_market, spl_owner.pubkey());
    r.expect("init_position (SPL market)");
    let t22_owner = fixed_pubkey(41);
    let (r, t22_position) = init_position(&mut svm, &admin, t22_market, t22_owner);
    r.expect("init_position (Token-2022 market)");
    println!(
        "SPL market position:        {spl_position} (owner {})",
        spl_owner.pubkey()
    );
    println!("Token-2022 market position: {t22_position} (owner {t22_owner})");

    // --- 2. SPL collateral deposit ---
    section("2. SPL collateral deposit (no fee)");
    let depositor_ata = create_token_account(
        &mut svm,
        &admin,
        50,
        sol_mint,
        admin.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    let spl_amount = 5_000_000_000u64; // 5.0 SOL @ 9dp
    mint_to(
        &mut svm,
        &admin,
        sol_mint,
        depositor_ata,
        &admin,
        spl_amount,
        spl_token_interface::ID,
    );
    deposit_collateral(
        &mut svm,
        &admin,
        spl_market,
        spl_position,
        spl_cvault,
        depositor_ata,
        sol_mint,
        spl_token_interface::ID,
        spl_amount,
    )
    .expect("SPL deposit must succeed");
    let spl_position_state = fetch_position(&svm, &spl_position);
    println!("  requested: {spl_amount}");
    println!("  credited:  {}", spl_position_state.collateral_amount);
    assert_eq!(spl_amount, spl_position_state.collateral_amount);
    invariants::assert_inv_cus_02(&svm, &spl_market, &[spl_position]);
    println!("  INV-CUS-02: holds exactly (vault == Σ positions + fee_accrued)");

    // --- 3. Token-2022 transfer-fee collateral deposit ---
    section("3. Token-2022 transfer-fee collateral deposit");
    let t22_extensions = &[spl_token_2022_interface::extension::ExtensionType::TransferFeeConfig];
    let t22_depositor_ata = create_token_account(
        &mut svm,
        &admin,
        51,
        fee_mint,
        admin.pubkey(),
        spl_token_2022_interface::ID,
        t22_extensions,
    );
    let t22_requested = 1_000_000_000u64; // 1.0 token @ 9dp
    mint_to(
        &mut svm,
        &admin,
        fee_mint,
        t22_depositor_ata,
        &admin,
        t22_requested,
        spl_token_2022_interface::ID,
    );
    deposit_collateral(
        &mut svm,
        &admin,
        t22_market,
        t22_position,
        t22_cvault,
        t22_depositor_ata,
        fee_mint,
        spl_token_2022_interface::ID,
        t22_requested,
    )
    .expect("Token-2022 deposit must succeed");
    let t22_position_state = fetch_position(&svm, &t22_position);
    let credited = t22_position_state.collateral_amount;
    println!("  requested: {t22_requested}");
    println!(
        "  credited:  {credited}  (fee = {})",
        t22_requested - credited
    );
    assert_ne!(
        t22_requested, credited,
        "requested and credited must differ on a transfer-fee mint"
    );
    invariants::assert_inv_cus_02(&svm, &t22_market, &[t22_position]);
    println!("  INV-CUS-02: holds exactly against the credited (not requested) amount");

    // --- 4. Zero-debt withdrawal ---
    section("4. Zero-debt withdrawal (SPL market)");
    let spl_owner_ata = create_token_account(
        &mut svm,
        &admin,
        52,
        sol_mint,
        spl_owner.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    withdraw_collateral(
        &mut svm,
        &spl_owner,
        spl_market,
        spl_position,
        spl_cvault,
        spl_owner_ata,
        sol_mint,
        spl_token_interface::ID,
        // Debt-free withdrawal reads no oracle at all (E-08, Phase 5) -- placeholders unused.
        Pubkey::default(),
        Pubkey::default(),
        spl_amount,
    )
    .expect("zero-debt withdrawal must succeed");
    let spl_position_state = fetch_position(&svm, &spl_position);
    println!("  withdrawn: {spl_amount}");
    println!(
        "  position.collateral_amount now: {}",
        spl_position_state.collateral_amount
    );
    assert_eq!(spl_position_state.collateral_amount, 0);
    invariants::assert_inv_cus_02(&svm, &spl_market, &[spl_position]);
    println!("  INV-CUS-02: holds exactly");

    // --- 5. Close the now-empty position ---
    section("5. close_position — rent reclaimed");
    let lamports_before = svm.get_account(&spl_owner.pubkey()).unwrap().lamports;
    let position_rent = svm.get_account(&spl_position).unwrap().lamports;
    close_position(&mut svm, &spl_owner, spl_market, spl_position)
        .expect("close_position must succeed on an exactly-empty position");
    let lamports_after = svm.get_account(&spl_owner.pubkey()).unwrap().lamports;
    println!("  position rent (lamports):        {position_rent}");
    println!("  owner balance before close:       {lamports_before}");
    println!("  owner balance after close:        {lamports_after}");
    println!(
        "  position account after close:     {}",
        match svm.get_account(&spl_position) {
            None => "purged".to_string(),
            Some(a) => format!(
                "owner={} data_len={} lamports={}",
                a.owner,
                a.data.len(),
                a.lamports
            ),
        }
    );

    println!("\nDemo complete. All Phase 3 acceptance criteria exercised above.");
}
