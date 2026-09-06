//! Phase 2 demo (`docs/phases/phase-02-state.md` "Demo"): protocol initialization, one standard
//! SPL market, one Token-2022-compatible market, position initialization, vault custody evidence,
//! and a rejection table for incompatible mints/extensions with the specific reason for each.
//!
//! Zero-cost and local: an in-process LiteSVM instance loaded with the actual built `aegis.so`
//! and the real embedded SPL Token / Token-2022 program bytecode LiteSVM ships. No devnet, no
//! RPC, no API key (`docs/zero-cost-demo.md`).
//!
//! Run with `make demo` (which runs `anchor build` first) or directly:
//! `cargo run -p aegis-test-kit --example phase2_demo`.

// `litesvm::types::TransactionResult`'s `Err` variant is a third-party type this crate does not
// control.
#![allow(clippy::result_large_err)]

use aegis::error::AegisError;
use aegis_test_kit::{
    collateral_vault_pda, create_market, create_spl_mint, create_token_2022_mint,
    create_token_2022_mint_with_unrecognized_extension, deploy, fetch_market, fetch_protocol,
    fetch_token_account_base, init_position, initialize_protocol, loan_vault_pda,
    reference_market_args, spl_token_2022_interface, spl_token_interface, Token2022Extension,
};
use litesvm::LiteSVM;
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
    println!("Aegis Protocol — Phase 2 demo (state, PDAs, custody primitives)");
    println!("Zero-cost, local, offline: in-process LiteSVM, no devnet, no RPC, no API key.\n");

    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    println!("Deployed program {} into LiteSVM.", aegis::id());
    println!("Admin/deployer:  {}", admin.pubkey());

    // --- 1. Initialize protocol ---
    section("1. Protocol initialization");
    let guardian = fixed_pubkey(2);
    let fee_recipient = fixed_pubkey(3);
    initialize_protocol(&mut svm, &admin, guardian, fee_recipient).expect("initialize_protocol");
    let (protocol_pubkey, _) = aegis_test_kit::protocol_pda();
    let protocol = fetch_protocol(&svm, &protocol_pubkey);
    println!("Protocol account: {protocol_pubkey}");
    println!("  admin:         {}", protocol.admin);
    println!("  guardian:      {}", protocol.guardian);
    println!("  fee_recipient: {}", protocol.fee_recipient);
    println!("  paused:        {:#04b}", protocol.paused);

    // --- 2. Standard SPL market ---
    section("2. Standard SPL market (SOL-like collateral / USDC-like loan)");
    let sol_mint = create_spl_mint(&mut svm, &admin, 10, 9, admin.pubkey(), None);
    let usdc_mint = create_spl_mint(&mut svm, &admin, 11, 6, admin.pubkey(), None);
    let spl_args = reference_market_args(0, [0xAA; 32], [0xBB; 32], false);
    let (result, spl_market, spl_cvault, spl_lvault, spl_fee_position) = create_market(
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
    print_market_snapshot(&svm, &spl_market);
    print_vault_evidence(&svm, "collateral", &spl_cvault, &spl_market);
    print_vault_evidence(&svm, "loan", &spl_lvault, &spl_market);
    println!("Fee position:      {spl_fee_position}");

    // --- 3. Token-2022-compatible market (transfer-fee collateral) ---
    section("3. Token-2022 market (transfer-fee collateral / plain SPL loan)");
    let fee_mint = create_token_2022_mint(
        &mut svm,
        &admin,
        20,
        9,
        admin.pubkey(),
        None,
        &[Token2022Extension::TransferFeeConfig {
            basis_points: 50, // 0.50%
            maximum_fee: 1_000_000,
        }],
    );
    let t22_args = reference_market_args(0, [0xCC; 32], [0xDD; 32], false);
    let (result, t22_market, t22_cvault, t22_lvault, t22_fee_position) = create_market(
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
    print_market_snapshot(&svm, &t22_market);
    let t22_market_state = fetch_market(&svm, &t22_market);
    println!(
        "  collateral_has_transfer_fee flag set: {}",
        t22_market_state.flags & aegis::constants::FLAG_COLLATERAL_HAS_TRANSFER_FEE != 0
    );
    print_vault_evidence(&svm, "collateral", &t22_cvault, &t22_market);
    print_vault_evidence(&svm, "loan", &t22_lvault, &t22_market);
    println!("Fee position:      {t22_fee_position}");
    let cvault_account = svm.get_account(&t22_cvault).expect("vault exists");
    println!(
        "  Token-2022 vault size: {} bytes (never hardcoded to 165)",
        cvault_account.data.len()
    );

    // --- 4. Position initialization ---
    section("4. Position initialization");
    let lender = fixed_pubkey(40);
    let borrower = fixed_pubkey(41);
    let (r1, lender_position) = init_position(&mut svm, &admin, spl_market, lender);
    r1.expect("init_position (lender, SPL market)");
    let (r2, borrower_position) = init_position(&mut svm, &admin, spl_market, borrower);
    r2.expect("init_position (borrower, SPL market)");
    let (r3, t22_position) = init_position(&mut svm, &admin, t22_market, borrower);
    r3.expect("init_position (borrower, Token-2022 market)");
    println!("SPL market lender position:      {lender_position}");
    println!("SPL market borrower position:    {borrower_position}");
    println!("Token-2022 market borrower position: {t22_position}");

    // --- 5. Rejection table ---
    section("5. Rejection table — incompatible mints and parameters");
    println!("{:<45} {:<40}", "Attempt", "Rejection reason");
    println!("{}", "-".repeat(90));

    let mut config_id = 10u16;

    let hook_mint = create_token_2022_mint(
        &mut svm,
        &admin,
        50,
        9,
        admin.pubkey(),
        None,
        &[Token2022Extension::TransferHook(fixed_pubkey(60))],
    );
    config_id += 1;
    report_attempt(
        "TransferHook collateral",
        try_create_market(
            &mut svm,
            &admin,
            hook_mint,
            spl_token_2022_interface::ID,
            usdc_mint,
            fee_recipient,
            config_id,
            false,
        ),
    );

    let delegate_mint = create_token_2022_mint(
        &mut svm,
        &admin,
        51,
        9,
        admin.pubkey(),
        None,
        &[Token2022Extension::PermanentDelegate(fixed_pubkey(61))],
    );
    config_id += 1;
    report_attempt(
        "PermanentDelegate collateral",
        try_create_market(
            &mut svm,
            &admin,
            delegate_mint,
            spl_token_2022_interface::ID,
            usdc_mint,
            fee_recipient,
            config_id,
            false,
        ),
    );

    let close_auth_mint = create_token_2022_mint(
        &mut svm,
        &admin,
        52,
        9,
        admin.pubkey(),
        None,
        &[Token2022Extension::MintCloseAuthority(fixed_pubkey(62))],
    );
    config_id += 1;
    report_attempt(
        "MintCloseAuthority collateral",
        try_create_market(
            &mut svm,
            &admin,
            close_auth_mint,
            spl_token_2022_interface::ID,
            usdc_mint,
            fee_recipient,
            config_id,
            false,
        ),
    );

    let frozen_default_mint = create_token_2022_mint(
        &mut svm,
        &admin,
        53,
        9,
        admin.pubkey(),
        Some(fixed_pubkey(63)),
        &[Token2022Extension::DefaultAccountStateFrozen],
    );
    config_id += 1;
    report_attempt(
        "DefaultAccountState=Frozen collateral",
        try_create_market(
            &mut svm,
            &admin,
            frozen_default_mint,
            spl_token_2022_interface::ID,
            usdc_mint,
            fee_recipient,
            config_id,
            false,
        ),
    );

    let unrecognized_mint =
        create_token_2022_mint_with_unrecognized_extension(&mut svm, 9, admin.pubkey());
    config_id += 1;
    report_attempt(
        "Unrecognized extension collateral",
        try_create_market(
            &mut svm,
            &admin,
            unrecognized_mint,
            spl_token_2022_interface::ID,
            usdc_mint,
            fee_recipient,
            config_id,
            false,
        ),
    );

    // Transfer-fee mint as the *loan* asset — rejected (accepted as collateral above, §3).
    config_id += 1;
    let args = reference_market_args(config_id, [1u8; 32], [2u8; 32], false);
    let (result, ..) = create_market(
        &mut svm,
        &admin,
        usdc_mint,
        fee_mint,
        spl_token_interface::ID,
        spl_token_2022_interface::ID,
        fee_recipient,
        args,
    );
    report_attempt("Transfer-fee mint as LOAN asset", result);

    let freeze_mint = create_spl_mint(
        &mut svm,
        &admin,
        54,
        9,
        admin.pubkey(),
        Some(fixed_pubkey(64)),
    );
    config_id += 1;
    report_attempt(
        "Freeze-authority collateral, unacknowledged",
        try_create_market(
            &mut svm,
            &admin,
            freeze_mint,
            spl_token_interface::ID,
            usdc_mint,
            fee_recipient,
            config_id,
            false,
        ),
    );

    // Same mint for both legs.
    config_id += 1;
    let args = reference_market_args(config_id, [1u8; 32], [2u8; 32], false);
    let (result, ..) = create_market(
        &mut svm,
        &admin,
        usdc_mint,
        usdc_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        fee_recipient,
        args,
    );
    report_attempt("collateral_mint == loan_mint", result);

    // Derived liquidation bound violation: plausible bonus, high threshold.
    config_id += 1;
    let mut args = reference_market_args(config_id, [1u8; 32], [2u8; 32], false);
    args.liq_threshold = 850_000_000_000_000_000;
    args.liq_bonus = 240_000_000_000_000_000;
    args.max_ltv = 700_000_000_000_000_000;
    let (result, ..) = create_market(
        &mut svm,
        &admin,
        sol_mint,
        usdc_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        fee_recipient,
        args,
    );
    report_attempt("LT=0.85, bonus=0.24 (derived bound INV-LIQ-06)", result);

    println!("\nDemo complete. All Phase 2 acceptance criteria exercised above.");
}

#[allow(clippy::too_many_arguments)]
fn try_create_market(
    svm: &mut LiteSVM,
    admin: &Keypair,
    collateral: Pubkey,
    collateral_program: Pubkey,
    loan: Pubkey,
    fee_recipient: Pubkey,
    config_id: u16,
    ack_freeze_authority: bool,
) -> litesvm::types::TransactionResult {
    let args = reference_market_args(config_id, [1u8; 32], [2u8; 32], ack_freeze_authority);
    let (result, ..) = create_market(
        svm,
        admin,
        collateral,
        loan,
        collateral_program,
        spl_token_interface::ID,
        fee_recipient,
        args,
    );
    result
}

fn report_attempt(label: &str, result: litesvm::types::TransactionResult) {
    let reason = match result {
        Ok(_) => "ACCEPTED (unexpected)".to_string(),
        Err(failed) => describe_rejection(&failed),
    };
    println!("{label:<45} {reason:<40}");
}

fn print_market_snapshot(svm: &LiteSVM, market_pubkey: &Pubkey) {
    let market = fetch_market(svm, market_pubkey);
    println!("Market account:    {market_pubkey}");
    println!(
        "  collateral_mint: {}  (decimals {})",
        market.collateral_mint, market.collateral_decimals
    );
    println!(
        "  loan_mint:       {}  (decimals {})",
        market.loan_mint, market.loan_decimals
    );
    println!("  config_id:       {}", market.config_id);
    println!(
        "  max_ltv={:.2}  liq_threshold={:.2}  liq_bonus={:.2}  close_factor={:.2}",
        wad_to_f64(market.max_ltv),
        wad_to_f64(market.liq_threshold),
        wad_to_f64(market.liq_bonus),
        wad_to_f64(market.close_factor)
    );
    println!(
        "  full_liq_hf={:.2}  liq_protocol_fee={:.2}  fee={:.2}  min_debt={}",
        wad_to_f64(market.full_liq_hf),
        wad_to_f64(market.liq_protocol_fee),
        wad_to_f64(market.fee),
        market.min_debt
    );
    println!(
        "  total_supply_assets={} total_borrow_assets={} (Phase 2: always zero)",
        market.total_supply_assets, market.total_borrow_assets
    );
}

fn print_vault_evidence(svm: &LiteSVM, role: &str, vault: &Pubkey, market: &Pubkey) {
    let base = fetch_token_account_base(svm, vault);
    let (expected, _) = if role == "collateral" {
        collateral_vault_pda(market)
    } else {
        loan_vault_pda(market)
    };
    println!(
        "  {role} vault: {vault} (canonical: {}) authority={} mint={}",
        vault == &expected,
        base.owner,
        base.mint
    );
}

fn wad_to_f64(v: u128) -> f64 {
    v as f64 / 1_000_000_000_000_000_000.0
}

fn describe_rejection(failed: &litesvm::types::FailedTransactionMetadata) -> String {
    use solana_instruction_error::InstructionError;
    use solana_transaction_error::TransactionError;
    match &failed.err {
        TransactionError::InstructionError(_, InstructionError::Custom(code)) => {
            describe_custom_code(*code)
        }
        other => format!("{other:?}"),
    }
}

fn describe_custom_code(code: u32) -> String {
    let known: &[(AegisError, &str)] = &[
        (
            AegisError::UnsupportedTokenExtension,
            "UnsupportedTokenExtension",
        ),
        (
            AegisError::TransferFeeNotAllowedForLoanAsset,
            "TransferFeeNotAllowedForLoanAsset",
        ),
        (
            AegisError::FreezeAuthorityNotAcknowledged,
            "FreezeAuthorityNotAcknowledged",
        ),
        (AegisError::InvalidMintAccountData, "InvalidMintAccountData"),
        (
            AegisError::SameCollateralAndLoanMint,
            "SameCollateralAndLoanMint",
        ),
        (
            AegisError::LiquidationBonusExceedsThresholdBound,
            "LiquidationBonusExceedsThresholdBound (INV-LIQ-06)",
        ),
        (AegisError::NotProtocolAdmin, "NotProtocolAdmin"),
    ];
    for (err, name) in known {
        if u32::from(*err) == code {
            return name.to_string();
        }
    }
    format!("custom error code {code}")
}
