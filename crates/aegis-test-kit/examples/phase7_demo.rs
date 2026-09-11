//! Phase 7 demo (`docs/phases/phase-07-token2022.md` "Demo"): two side-by-side markets running an
//! identical flow -- Market A with classic SPL Token collateral, Market B with a Token-2022
//! transfer-fee collateral mint -- printing `requested` vs. `credited` (and the fee, where
//! applicable) at every collateral-side transfer, so the accounting reconciliation is visible
//! rather than merely asserted. Ends with the verified extension-policy rejection table
//! (`docs/token-compatibility.md` §0/§2), generated from real `create_market` attempts against
//! real fixtures, not printed from memory.
//!
//! Zero-cost and local: an in-process LiteSVM instance loaded with the actual built `aegis.so`.
//! No devnet, no RPC, no API key (ADR-0008, `docs/zero-cost-demo.md`).
//!
//! Run with `make demo` or directly: `cargo run -p aegis-test-kit --example phase7_demo`.

#![allow(clippy::result_large_err)]

use aegis::error::AegisError;
use aegis_test_kit::{
    accrue_interest, borrow, create_market, create_spl_mint, create_token_2022_mint,
    create_token_account, deploy, deposit_collateral, fetch_market, fetch_position,
    fetch_token_account_base, init_position, initialize_protocol, invariants, liquidate, mint_to,
    reference_market_args, set_price, spl_token_2022_interface, spl_token_interface, supply,
    PriceFixture, Token2022Extension,
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

const COLLATERAL_FEED_ID: [u8; 32] = [0xAAu8; 32];
const LOAN_FEED_ID: [u8; 32] = [0xBBu8; 32];

struct MarketFixture {
    name: &'static str,
    collateral_mint: Pubkey,
    collateral_token_program: Pubkey,
    loan_mint: Pubkey,
    market: Pubkey,
    collateral_vault: Pubkey,
    loan_vault: Pubkey,
    fee_position: Pubkey,
    lender: Keypair,
    lender_position: Pubkey,
    lender_ata: Pubkey,
    borrower: Keypair,
    borrower_position: Pubkey,
    borrower_collateral_ata: Pubkey,
    borrower_loan_ata: Pubkey,
}

#[allow(clippy::too_many_arguments)]
fn setup_market(
    svm: &mut litesvm::LiteSVM,
    admin: &Keypair,
    name: &'static str,
    config_id: u16,
    fee_recipient: Pubkey,
    collateral_mint: Pubkey,
    collateral_token_program: Pubkey,
    collateral_mint_extensions: &[spl_token_2022_interface::extension::ExtensionType],
    loan_mint: Pubkey,
    seed_base: u8,
) -> MarketFixture {
    let args = reference_market_args(config_id, COLLATERAL_FEED_ID, LOAN_FEED_ID, false);
    let (result, market, collateral_vault, loan_vault, fee_position) = create_market(
        svm,
        admin,
        collateral_mint,
        loan_mint,
        collateral_token_program,
        spl_token_interface::ID,
        fee_recipient,
        args,
    );
    result.expect("create_market must succeed");

    let lender = Keypair::new_from_array([seed_base; 32]);
    svm.airdrop(&lender.pubkey(), 10_000_000_000).unwrap();
    let lender_ata = create_token_account(
        svm,
        admin,
        seed_base.wrapping_add(1),
        loan_mint,
        lender.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    let supply_amount = 1_200_000_000u64;
    mint_to(
        svm,
        admin,
        loan_mint,
        lender_ata,
        admin,
        supply_amount,
        spl_token_interface::ID,
    );
    let (_, lender_position) = init_position(svm, admin, market, lender.pubkey());
    supply(
        svm,
        &lender,
        market,
        lender_position,
        fee_position,
        loan_vault,
        lender_ata,
        loan_mint,
        spl_token_interface::ID,
        supply_amount,
        0,
    )
    .expect("supply must succeed");

    let borrower = Keypair::new_from_array([seed_base.wrapping_add(2); 32]);
    svm.airdrop(&borrower.pubkey(), 10_000_000_000).unwrap();
    let borrower_collateral_ata = create_token_account(
        svm,
        admin,
        seed_base.wrapping_add(3),
        collateral_mint,
        borrower.pubkey(),
        collateral_token_program,
        collateral_mint_extensions,
    );
    let borrower_loan_ata = create_token_account(
        svm,
        admin,
        seed_base.wrapping_add(4),
        loan_mint,
        borrower.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    let (_, borrower_position) = init_position(svm, admin, market, borrower.pubkey());

    MarketFixture {
        name,
        collateral_mint,
        collateral_token_program,
        loan_mint,
        market,
        collateral_vault,
        loan_vault,
        fee_position,
        lender,
        lender_position,
        lender_ata,
        borrower,
        borrower_position,
        borrower_collateral_ata,
        borrower_loan_ata,
    }
}

fn assert_custody(svm: &litesvm::LiteSVM, fixture: &MarketFixture) {
    invariants::assert_inv_cus_01(svm, &fixture.market);
    invariants::assert_inv_cus_02(
        svm,
        &fixture.market,
        &[fixture.lender_position, fixture.borrower_position],
    );
}

fn main() {
    println!("Aegis Protocol — Phase 7 demo (Token-2022 completion)");
    println!("Zero-cost, local, offline: in-process LiteSVM, no devnet, no RPC, no API key.\n");

    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    println!("Deployed program {} into LiteSVM.", aegis::id());

    // --- Setup: two mints per side, one plain, one Token-2022 with a 2% transfer fee ---
    section(
        "1. Two markets: A = classic SPL collateral, B = Token-2022 transfer-fee (2%) collateral",
    );
    let guardian = fixed_pubkey(2);
    let fee_recipient = fixed_pubkey(3);
    initialize_protocol(&mut svm, &admin, guardian, fee_recipient).expect("initialize_protocol");

    let sol_mint_a = create_spl_mint(&mut svm, &admin, 10, 9, admin.pubkey(), None);
    let usdc_mint_a = create_spl_mint(&mut svm, &admin, 11, 6, admin.pubkey(), None);
    let fee_extension_types =
        [spl_token_2022_interface::extension::ExtensionType::TransferFeeConfig];
    let sol_mint_b = create_token_2022_mint(
        &mut svm,
        &admin,
        12,
        9,
        admin.pubkey(),
        None,
        &[Token2022Extension::TransferFeeConfig {
            basis_points: 200,
            maximum_fee: u64::MAX,
        }],
    );
    let usdc_mint_b = create_spl_mint(&mut svm, &admin, 13, 6, admin.pubkey(), None);

    let market_a = setup_market(
        &mut svm,
        &admin,
        "A (classic SPL)",
        0,
        fee_recipient,
        sol_mint_a,
        spl_token_interface::ID,
        &[],
        usdc_mint_a,
        20,
    );
    let market_b = setup_market(
        &mut svm,
        &admin,
        "B (Token-2022, 2% transfer fee)",
        1,
        fee_recipient,
        sol_mint_b,
        spl_token_2022_interface::ID,
        &fee_extension_types,
        usdc_mint_b,
        40,
    );
    println!(
        "Market A: {} / market B: {}",
        market_a.market, market_b.market
    );

    // --- 2. Deposits on both markets. Market B requests slightly more (10.3 SOL, not 10) so that
    // its NET-of-fee collateral still comfortably exceeds the ~9.9703 SOL the later liquidation
    // will need to seize -- the exact same requirement Market A's own 10 SOL barely clears
    // (economic-model.md §7.5's own worked example has the identical ~0.3% margin). This keeps
    // both markets on the clean, non-clamped liquidation path so the side-by-side comparison
    // below is a genuine apples-to-apples reconciliation, not one market hitting the collateral
    // clamp (already covered by `A-TOK-10`) and the other not. ---
    section("2. Borrower deposits collateral on both markets (B requests slightly more to offset the fee)");
    for (fixture, deposit_amount) in [
        (&market_a, 10_000_000_000u64),
        (&market_b, 10_300_000_000u64),
    ] {
        mint_to(
            &mut svm,
            &admin,
            fixture.collateral_mint,
            fixture.borrower_collateral_ata,
            &admin,
            deposit_amount,
            fixture.collateral_token_program,
        );
        deposit_collateral(
            &mut svm,
            &fixture.borrower,
            fixture.market,
            fixture.borrower_position,
            fixture.collateral_vault,
            fixture.borrower_collateral_ata,
            fixture.collateral_mint,
            fixture.collateral_token_program,
            deposit_amount,
        )
        .expect("deposit_collateral must succeed");
        assert_custody(&svm, fixture);
        let credited = fetch_position(&svm, &fixture.borrower_position).collateral_amount;
        let fee = deposit_amount - credited;
        println!(
            "  Market {:32} requested {}  credited {}  fee {}",
            fixture.name,
            format_sol(deposit_amount),
            format_sol(credited),
            format_sol(fee),
        );
    }

    // --- 3. Identical borrows on both markets ---
    section("3. Borrower borrows 900 USDC on both markets at SOL=$150.00 / USDC=$1.00");
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
    let borrow_amount = 900_000_000u64;
    for fixture in [&market_a, &market_b] {
        borrow(
            &mut svm,
            &fixture.borrower,
            fixture.market,
            fixture.borrower_position,
            fixture.fee_position,
            fixture.loan_vault,
            fixture.borrower_loan_ata,
            fixture.loan_mint,
            spl_token_interface::ID,
            c0,
            l0,
            borrow_amount,
            0,
        )
        .expect("borrow must succeed");
        assert_custody(&svm, fixture);
        println!(
            "  Market {:32} borrowed {}",
            fixture.name,
            format_usdc(borrow_amount)
        );
    }

    section("4. Interest accrual (permissionless instruction) on both markets");
    for fixture in [&market_a, &market_b] {
        accrue_interest(&mut svm, &admin, fixture.market, fixture.fee_position)
            .expect("accrue_interest must succeed");
        assert_custody(&svm, fixture);
    }
    println!("  accrue_interest succeeded on both markets; custody invariants held.");

    // --- 5/6. Price crashes; liquidation on both markets ---
    section("5. SOL crashes to $95.00 -- both positions become liquidatable");
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

    section("6. Liquidation on both markets -- full debt repaid, collateral seized");
    for (idx, fixture) in [&market_a, &market_b].into_iter().enumerate() {
        let seed_base = 60 + (idx as u8) * 10;
        let liquidator = Keypair::new_from_array([seed_base; 32]);
        svm.airdrop(&liquidator.pubkey(), 10_000_000_000).unwrap();
        let liquidator_loan_ata = create_token_account(
            &mut svm,
            &admin,
            seed_base + 1,
            fixture.loan_mint,
            liquidator.pubkey(),
            spl_token_interface::ID,
            &[],
        );
        mint_to(
            &mut svm,
            &admin,
            fixture.loan_mint,
            liquidator_loan_ata,
            &admin,
            1_000_000_000,
            spl_token_interface::ID,
        );
        let liquidator_collateral_ata = create_token_account(
            &mut svm,
            &admin,
            seed_base + 2,
            fixture.collateral_mint,
            liquidator.pubkey(),
            fixture.collateral_token_program,
            if fixture.collateral_token_program == spl_token_2022_interface::ID {
                &fee_extension_types
            } else {
                &[]
            },
        );
        let debt_before = {
            let p = fetch_position(&svm, &fixture.borrower_position);
            let m = fetch_market(&svm, &fixture.market);
            aegis_math::to_assets_up(
                p.borrow_shares,
                m.total_borrow_assets,
                m.total_borrow_shares,
            )
            .unwrap()
        };
        let vault_before = fetch_token_account_base(&svm, &fixture.collateral_vault).amount;
        liquidate(
            &mut svm,
            &liquidator,
            fixture.market,
            fixture.borrower_position,
            fixture.fee_position,
            fixture.loan_vault,
            fixture.collateral_vault,
            liquidator_loan_ata,
            liquidator_collateral_ata,
            fixture.loan_mint,
            fixture.collateral_mint,
            spl_token_interface::ID,
            fixture.collateral_token_program,
            c1,
            l1,
            debt_before,
            0,
        )
        .expect("liquidation must succeed");
        assert_custody(&svm, fixture);
        let vault_after = fetch_token_account_base(&svm, &fixture.collateral_vault).amount;
        let liquidator_received = fetch_token_account_base(&svm, &liquidator_collateral_ata).amount;
        let protocol_cut = fetch_market(&svm, &fixture.market).collateral_fee_accrued;
        println!(
            "  Market {:32} repaid {}  vault decreased by {}  liquidator received {}  protocol_cut {}",
            fixture.name,
            format_usdc(debt_before),
            format_sol(vault_before - vault_after),
            format_sol(liquidator_received),
            format_sol(protocol_cut),
        );
        if fixture.collateral_token_program == spl_token_2022_interface::ID {
            println!(
                "    (Market {} bears the outbound transfer fee on the liquidator's leg -- \
                 the vault-side accounting above still reconciles exactly, per INV-CUS-02)",
                fixture.name
            );
        }
    }

    // --- 7. Cleanup: lenders withdraw on both markets ---
    section("7. Cleanup: lenders withdraw on both markets");
    for fixture in [&market_a, &market_b] {
        let lender_shares = fetch_position(&svm, &fixture.lender_position).supply_shares;
        let lender_before = fetch_token_account_base(&svm, &fixture.lender_ata).amount;
        aegis_test_kit::withdraw(
            &mut svm,
            &fixture.lender,
            fixture.market,
            fixture.lender_position,
            fixture.fee_position,
            fixture.loan_vault,
            fixture.lender_ata,
            fixture.loan_mint,
            spl_token_interface::ID,
            0,
            lender_shares,
        )
        .expect("lender withdrawal must succeed");
        assert_custody(&svm, fixture);
        let lender_after = fetch_token_account_base(&svm, &fixture.lender_ata).amount;
        println!(
            "  Market {:32} lender redeemed {}",
            fixture.name,
            format_usdc(lender_after - lender_before)
        );
    }

    // --- Rejection table: real create_market attempts against every Tier C fixture this
    // repository can construct, plus the two role-asymmetric cases. ---
    section("8. Verified extension-policy rejection table (real create_market attempts)");
    let usdc_for_rejections = create_spl_mint(&mut svm, &admin, 90, 6, admin.pubkey(), None);
    let mut rows: Vec<(&str, Result<(), AegisError>)> = Vec::new();

    let try_reject = |svm: &mut litesvm::LiteSVM,
                      admin: &Keypair,
                      collateral_mint: Pubkey,
                      collateral_program: Pubkey|
     -> Result<(), AegisError> {
        let args = reference_market_args(rand_config_id(), COLLATERAL_FEED_ID, LOAN_FEED_ID, false);
        let (result, ..) = create_market(
            svm,
            admin,
            collateral_mint,
            usdc_for_rejections,
            collateral_program,
            spl_token_interface::ID,
            fee_recipient,
            args,
        );
        match result {
            Ok(_) => Ok(()),
            Err(failed) => Err(decode_aegis_error(&failed)),
        }
    };

    let transfer_hook_mint = create_token_2022_mint(
        &mut svm,
        &admin,
        91,
        9,
        admin.pubkey(),
        None,
        &[Token2022Extension::TransferHook(fixed_pubkey(199))],
    );
    rows.push((
        "TransferHook",
        try_reject(
            &mut svm,
            &admin,
            transfer_hook_mint,
            spl_token_2022_interface::ID,
        ),
    ));

    let permanent_delegate_mint = create_token_2022_mint(
        &mut svm,
        &admin,
        92,
        9,
        admin.pubkey(),
        None,
        &[Token2022Extension::PermanentDelegate(fixed_pubkey(198))],
    );
    rows.push((
        "PermanentDelegate",
        try_reject(
            &mut svm,
            &admin,
            permanent_delegate_mint,
            spl_token_2022_interface::ID,
        ),
    ));

    let mint_close_authority_mint = create_token_2022_mint(
        &mut svm,
        &admin,
        93,
        9,
        admin.pubkey(),
        None,
        &[Token2022Extension::MintCloseAuthority(fixed_pubkey(197))],
    );
    rows.push((
        "MintCloseAuthority",
        try_reject(
            &mut svm,
            &admin,
            mint_close_authority_mint,
            spl_token_2022_interface::ID,
        ),
    ));

    let default_frozen_mint = create_token_2022_mint(
        &mut svm,
        &admin,
        94,
        9,
        admin.pubkey(),
        Some(fixed_pubkey(196)),
        &[Token2022Extension::DefaultAccountStateFrozen],
    );
    rows.push((
        "DefaultAccountState = Frozen",
        try_reject(
            &mut svm,
            &admin,
            default_frozen_mint,
            spl_token_2022_interface::ID,
        ),
    ));

    let pausable_mint = create_token_2022_mint(
        &mut svm,
        &admin,
        95,
        9,
        admin.pubkey(),
        None,
        &[Token2022Extension::Pausable(fixed_pubkey(195))],
    );
    rows.push((
        "Pausable (RV-5)",
        try_reject(
            &mut svm,
            &admin,
            pausable_mint,
            spl_token_2022_interface::ID,
        ),
    ));

    let non_transferable_mint = create_token_2022_mint(
        &mut svm,
        &admin,
        96,
        9,
        admin.pubkey(),
        None,
        &[Token2022Extension::NonTransferable],
    );
    rows.push((
        "NonTransferable (RV-5)",
        try_reject(
            &mut svm,
            &admin,
            non_transferable_mint,
            spl_token_2022_interface::ID,
        ),
    ));

    let unrecognized_mint = aegis_test_kit::create_token_2022_mint_with_unrecognized_extension(
        &mut svm,
        9,
        admin.pubkey(),
    );
    rows.push((
        "Unrecognized discriminant (positive allowlist)",
        try_reject(
            &mut svm,
            &admin,
            unrecognized_mint,
            spl_token_2022_interface::ID,
        ),
    ));

    // Role asymmetry: transfer-fee as the LOAN asset is rejected (as collateral it is
    // demonstrated as ACCEPTED by Market B above -- this row shows the other half).
    let fee_mint_for_loan_check = create_token_2022_mint(
        &mut svm,
        &admin,
        97,
        6,
        admin.pubkey(),
        None,
        &[Token2022Extension::TransferFeeConfig {
            basis_points: 50,
            maximum_fee: u64::MAX,
        }],
    );
    let plain_collateral_for_loan_check =
        create_spl_mint(&mut svm, &admin, 98, 9, admin.pubkey(), None);
    let args_loan_check =
        reference_market_args(rand_config_id(), COLLATERAL_FEED_ID, LOAN_FEED_ID, false);
    let (loan_check_result, ..) = create_market(
        &mut svm,
        &admin,
        plain_collateral_for_loan_check,
        fee_mint_for_loan_check,
        spl_token_interface::ID,
        spl_token_2022_interface::ID,
        fee_recipient,
        args_loan_check,
    );
    rows.push((
        "TransferFeeConfig as LOAN asset (accepted as collateral above)",
        match loan_check_result {
            Ok(_) => Ok(()),
            Err(failed) => Err(decode_aegis_error(&failed)),
        },
    ));

    println!("  {:52} {:32} Reason", "Extension", "Result");
    for (name, result) in &rows {
        match result {
            Ok(()) => println!("  {name:52} {:32} -", "ACCEPTED"),
            Err(err) => println!("  {name:52} {:32} {err:?}", "REJECTED"),
        }
    }

    println!("\nPhase 7 demo complete. INV-CUS-01/INV-CUS-02 held after every instruction on both markets.");
}

fn rand_config_id() -> u16 {
    use std::sync::atomic::{AtomicU16, Ordering};
    static NEXT: AtomicU16 = AtomicU16::new(500);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

fn decode_aegis_error(failed: &litesvm::types::FailedTransactionMetadata) -> AegisError {
    use solana_instruction_error::InstructionError;
    use solana_transaction_error::TransactionError;
    match &failed.err {
        TransactionError::InstructionError(_, InstructionError::Custom(code)) => {
            aegis_error_from_code(*code).expect("expected a recognized AegisError code")
        }
        other => panic!("expected a custom program error, got {other:?}"),
    }
}

fn aegis_error_from_code(code: u32) -> Option<AegisError> {
    // AegisError's discriminants start at 6000 (Anchor's custom-error base); enumerate the token
    // policy band this demo actually exercises rather than depending on a From<u32> impl the
    // error type does not provide.
    let candidates = [
        AegisError::TokenProgramMintMismatch,
        AegisError::UnsupportedTokenExtension,
        AegisError::TransferFeeNotAllowedForLoanAsset,
        AegisError::FreezeAuthorityNotAcknowledged,
        AegisError::InvalidMintAccountData,
    ];
    candidates
        .into_iter()
        .find(|candidate| u32::from(*candidate) == code)
}
