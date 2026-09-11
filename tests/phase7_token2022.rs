//! Phase 7 — Token-2022 Completion (`docs/phases/phase-07-token2022.md`,
//! `docs/token-compatibility.md`).
//!
//! Closes RV-5 in code: `U-TOK-03` (cached decimals), two supplementary Tier C fixtures for
//! extensions that are present in this workspace's exact resolved `spl-token-2022-interface`
//! 2.1.0 dependency but were not yet exercised by a concrete on-chain fixture (`Pausable`,
//! `NonTransferable` — RV-5's own examples of "extensions added after older common lists"),
//! `A-TOK-10` (the full protocol lifecycle on a transfer-fee collateral market, with
//! INV-CUS-01/INV-CUS-02 asserted after every state-changing instruction), and `A-TOK-11` (a fee
//! rate raised by the fee authority mid-lifecycle, using the REAL `SetTransferFee` instruction and
//! the real 2-epoch delay before it takes effect — not a simulated fee, not a manually-edited
//! byte).

#![allow(clippy::result_large_err)]

use aegis_test_kit::{
    absorb_bad_debt, advance_epoch, assert_aegis_error, borrow, create_immutable_owner_account,
    create_market, create_spl_mint, create_token_2022_mint, create_token_account, deploy,
    deposit_collateral, fetch_market, fetch_mint_decimals, fetch_mint_extension_types,
    fetch_position, fetch_transfer_fee_config, init_position, initialize_protocol, invariants,
    liquidate, mint_to, reference_market_args, set_price, set_transfer_fee_rate,
    spl_token_2022_interface, spl_token_interface, supply,
    token_accounts::fetch_token_account_base, withdraw_collateral, withdraw_collateral_fees,
    PriceFixture, Token2022Extension,
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use spl_token_2022_interface::extension::ExtensionType;

fn program_bytes() -> &'static [u8] {
    include_bytes!(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/aegis.so"))
}

const COLLATERAL_FEED_ID: [u8; 32] = [0xAAu8; 32];
const LOAN_FEED_ID: [u8; 32] = [0xBBu8; 32];

fn fixed_pubkey(seed: u8) -> Pubkey {
    Keypair::new_from_array([seed; 32]).pubkey()
}

fn setup_protocol(svm: &mut litesvm::LiteSVM, admin: &Keypair) -> Pubkey {
    let guardian = fixed_pubkey(2);
    let fee_recipient = fixed_pubkey(3);
    initialize_protocol(svm, admin, guardian, fee_recipient).expect("initialize_protocol");
    fee_recipient
}

fn assert_custody(svm: &litesvm::LiteSVM, market: &Pubkey, positions: &[Pubkey]) {
    invariants::assert_inv_cus_01(svm, market);
    invariants::assert_inv_cus_02(svm, market, positions);
}

// ---------------------------------------------------------------------------------------------
// RV-5 supplementary Tier C evidence — extensions present in the exact resolved
// spl-token-2022-interface 2.1.0 dependency that predate-list classifications (and A-TOK-01..05)
// did not yet exercise with a concrete fixture. Both fall through the identical wildcard
// rejection arm in `token/policy.rs` that A-TOK-01..05 already prove, which is the whole point of
// a positive allowlist: no extension-specific carve-out exists that could accidentally admit one
// of these.
// ---------------------------------------------------------------------------------------------

// RV-5: `Pausable` (added to Token-2022 well after the pre-2024 extension lists this repository's
// documents originally anticipated) lets the mint authority halt all transfers -- exactly the
// `token-compatibility.md` §2 rejection reason already recorded for it before this phase started.
#[test]
fn pausable_mint_rejected_as_collateral() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fee_recipient = setup_protocol(&mut svm, &admin);
    let pause_authority = fixed_pubkey(60);
    let collateral_mint = create_token_2022_mint(
        &mut svm,
        &admin,
        61,
        9,
        admin.pubkey(),
        None,
        &[Token2022Extension::Pausable(pause_authority)],
    );
    let loan_mint = create_spl_mint(&mut svm, &admin, 62, 6, admin.pubkey(), None);
    let args = reference_market_args(90, [1u8; 32], [2u8; 32], false);
    let (result, ..) = create_market(
        &mut svm,
        &admin,
        collateral_mint,
        loan_mint,
        spl_token_2022_interface::ID,
        spl_token_interface::ID,
        fee_recipient,
        args,
    );
    assert_aegis_error(&result, aegis::error::AegisError::UnsupportedTokenExtension);
}

// RV-5: `NonTransferable` mints can never move into or out of a vault at all -- rejected
// outright, distinct from (and simpler than) the transfer-fee case.
#[test]
fn non_transferable_mint_rejected() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fee_recipient = setup_protocol(&mut svm, &admin);
    let collateral_mint = create_token_2022_mint(
        &mut svm,
        &admin,
        63,
        9,
        admin.pubkey(),
        None,
        &[Token2022Extension::NonTransferable],
    );
    let loan_mint = create_spl_mint(&mut svm, &admin, 64, 6, admin.pubkey(), None);
    let args = reference_market_args(91, [1u8; 32], [2u8; 32], false);
    let (result, ..) = create_market(
        &mut svm,
        &admin,
        collateral_mint,
        loan_mint,
        spl_token_2022_interface::ID,
        spl_token_interface::ID,
        fee_recipient,
        args,
    );
    assert_aegis_error(&result, aegis::error::AegisError::UnsupportedTokenExtension);
}

// U-TOK-03: cached decimals equal the mint's real decimals, for every supported mint
// configuration -- a classic SPL Token mint on both sides, and a Token-2022 transfer-fee mint as
// collateral (`token-compatibility.md` §5.5: decimals are immutable in both token programs, so
// this is asserted once at creation and never re-checked).
#[test]
fn u_tok_03_cached_decimals_match_mint_for_every_supported_configuration() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fee_recipient = setup_protocol(&mut svm, &admin);

    // Configuration A: classic SPL Token on both sides, asymmetric decimals (9 / 6).
    let spl_collateral = create_spl_mint(&mut svm, &admin, 70, 9, admin.pubkey(), None);
    let spl_loan = create_spl_mint(&mut svm, &admin, 71, 6, admin.pubkey(), None);
    let args_a = reference_market_args(92, [1u8; 32], [2u8; 32], false);
    let (result_a, market_a, ..) = create_market(
        &mut svm,
        &admin,
        spl_collateral,
        spl_loan,
        spl_token_interface::ID,
        spl_token_interface::ID,
        fee_recipient,
        args_a,
    );
    result_a.expect("SPL/SPL market must be created");
    let market_a_state = fetch_market(&svm, &market_a);
    assert_eq!(
        market_a_state.collateral_decimals,
        fetch_mint_decimals(&svm, &spl_collateral)
    );
    assert_eq!(
        market_a_state.loan_decimals,
        fetch_mint_decimals(&svm, &spl_loan)
    );

    // Configuration B: a Token-2022 transfer-fee collateral mint (8 decimals, deliberately
    // different from configuration A) against a plain SPL loan asset.
    let fee_collateral = create_token_2022_mint(
        &mut svm,
        &admin,
        72,
        8,
        admin.pubkey(),
        None,
        &[Token2022Extension::TransferFeeConfig {
            basis_points: 25,
            maximum_fee: u64::MAX,
        }],
    );
    let plain_loan = create_spl_mint(&mut svm, &admin, 73, 6, admin.pubkey(), None);
    let args_b = reference_market_args(93, [1u8; 32], [2u8; 32], false);
    let (result_b, market_b, ..) = create_market(
        &mut svm,
        &admin,
        fee_collateral,
        plain_loan,
        spl_token_2022_interface::ID,
        spl_token_interface::ID,
        fee_recipient,
        args_b,
    );
    result_b.expect("Token-2022 transfer-fee collateral market must be created");
    let market_b_state = fetch_market(&svm, &market_b);
    assert_eq!(
        market_b_state.collateral_decimals,
        fetch_mint_decimals(&svm, &fee_collateral)
    );
    assert_eq!(market_b_state.collateral_decimals, 8);
    assert_eq!(
        market_b_state.loan_decimals,
        fetch_mint_decimals(&svm, &plain_loan)
    );
}

// Item 10 of the phase-7 checklist ("owner cannot later be reassigned"): proves the real
// Token-2022 program's ImmutableOwner guarantee is unconditional -- it rejects an
// `AccountOwner` reassignment even when attempted by the account's own genuine, correctly-signing
// current owner. Built as a standalone Token-2022 account using the identical
// `create_account` + `InitializeImmutableOwner` + `InitializeAccount3` sequence `token/vault.rs`
// uses for Aegis's own vaults (Aegis's collateral vault itself cannot be used directly here: its
// real authority is the Market PDA, which no client transaction can ever produce a signature for,
// so an attempt against it would only prove "a client cannot forge a PDA signature" -- a
// runtime-level fact unrelated to ImmutableOwner. Using an account the test genuinely owns and
// signs for isolates the property actually being verified.) Combined with
// `tier_a_extensions_are_accepted_and_recorded`-adjacent Phase 2 evidence that every Token-2022
// vault Aegis creates carries this extension, and the fact that `programs/aegis/src` contains no
// `set_authority`/`SetAuthority` call at all (grep-verifiable, the same style as
// `scripts/check-no-close.sh`), this closes the "cannot be reassigned" claim end-to-end: the
// extension itself is unconditional, and Aegis never attempts the call regardless.
#[test]
fn immutable_owner_blocks_reassignment_even_by_the_genuine_current_owner() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let mint = create_token_2022_mint(&mut svm, &admin, 74, 9, admin.pubkey(), None, &[]);
    let account_pubkey = create_immutable_owner_account(&mut svm, &admin, 76, mint, admin.pubkey());

    // `admin` is the account's real, current, genuinely-signing owner. Even so, reassignment
    // must fail -- this is the entire point of the extension.
    let reassign_ix = spl_token_2022_interface::instruction::set_authority(
        &spl_token_2022_interface::ID,
        &account_pubkey,
        Some(&fixed_pubkey(77)),
        spl_token_2022_interface::instruction::AuthorityType::AccountOwner,
        &admin.pubkey(),
        &[],
    )
    .expect("valid set_authority instruction");
    svm.expire_blockhash();
    let blockhash2 = svm.latest_blockhash();
    let reassign_message = solana_message::Message::new_with_blockhash(
        &[reassign_ix],
        Some(&admin.pubkey()),
        &blockhash2,
    );
    let reassign_tx = solana_transaction::versioned::VersionedTransaction::try_new(
        solana_message::VersionedMessage::Legacy(reassign_message),
        &[&admin],
    )
    .expect("failed to sign transaction");
    let result = svm.send_transaction(reassign_tx);
    assert!(
        result.is_err(),
        "ImmutableOwner must reject AccountOwner reassignment even by the genuine current owner, \
         but it succeeded"
    );
}

// ---------------------------------------------------------------------------------------------
// A-TOK-10 — full lifecycle on a transfer-fee collateral market, INV-CUS-01/INV-CUS-02 asserted
// after every state-changing instruction, not merely at the end.
// ---------------------------------------------------------------------------------------------

#[test]
fn a_tok_10_full_lifecycle_on_transfer_fee_collateral_market() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fee_recipient = setup_protocol(&mut svm, &admin);

    // --- 1. Market: Token-2022 transfer-fee SOL collateral (2%, no cap) / plain SPL USDC loan ---
    let sol_mint = create_token_2022_mint(
        &mut svm,
        &admin,
        100,
        9,
        admin.pubkey(),
        None,
        &[Token2022Extension::TransferFeeConfig {
            basis_points: 200, // 2%
            maximum_fee: u64::MAX,
        }],
    );
    let usdc_mint = create_spl_mint(&mut svm, &admin, 101, 6, admin.pubkey(), None);
    let sol_extensions = fetch_mint_extension_types(&svm, &sol_mint);
    assert_eq!(sol_extensions, vec![ExtensionType::TransferFeeConfig]);

    let args = reference_market_args(0, COLLATERAL_FEED_ID, LOAN_FEED_ID, false);
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
    assert_eq!(
        fetch_market(&svm, &market).flags & aegis::constants::FLAG_COLLATERAL_HAS_TRANSFER_FEE,
        aegis::constants::FLAG_COLLATERAL_HAS_TRANSFER_FEE
    );

    // --- 2. Lender supplies USDC (plain SPL, unaffected by the collateral-side fee) ---
    let lender = Keypair::new_from_array([110u8; 32]);
    svm.airdrop(&lender.pubkey(), 10_000_000_000).unwrap();
    let lender_ata = create_token_account(
        &mut svm,
        &admin,
        111,
        usdc_mint,
        lender.pubkey(),
        spl_token_interface::ID,
        &[],
    );
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
    assert_custody(&svm, &market, &[lender_position]);

    // --- 3. Borrower deposits fee-bearing collateral: credited must be amount - fee, never the
    // requested amount (INV-CUS-05/06). ---
    let borrower = Keypair::new_from_array([120u8; 32]);
    svm.airdrop(&borrower.pubkey(), 10_000_000_000).unwrap();
    let fee_extensions = &[ExtensionType::TransferFeeConfig];
    let borrower_collateral_ata = create_token_account(
        &mut svm,
        &admin,
        121,
        sol_mint,
        borrower.pubkey(),
        spl_token_2022_interface::ID,
        fee_extensions,
    );
    let deposit_amount = 10_000_000_000u64; // 10 SOL requested
    mint_to(
        &mut svm,
        &admin,
        sol_mint,
        borrower_collateral_ata,
        &admin,
        deposit_amount,
        spl_token_2022_interface::ID,
    );
    let (_, borrower_position) = init_position(&mut svm, &admin, market, borrower.pubkey());
    deposit_collateral(
        &mut svm,
        &borrower,
        market,
        borrower_position,
        collateral_vault,
        borrower_collateral_ata,
        sol_mint,
        spl_token_2022_interface::ID,
        deposit_amount,
    )
    .expect("deposit_collateral must succeed");
    assert_custody(&svm, &market, &[lender_position, borrower_position]);
    let credited = fetch_position(&svm, &borrower_position).collateral_amount;
    let expected_fee = 200_000_000u64; // 2% of 10 SOL, uncapped
    assert_eq!(
        credited,
        deposit_amount - expected_fee,
        "A-TOK-10: credited must be requested minus the real transfer fee, never the requested amount"
    );

    // --- 4. Borrower borrows against a healthy $150.00 SOL / $1.00 USDC price ---
    let borrower_loan_ata = create_token_account(
        &mut svm,
        &admin,
        122,
        usdc_mint,
        borrower.pubkey(),
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
    let borrow_amount = 900_000_000u64; // 900 USDC; collateral value = 9.8 * 150 = 1470, well within 75% LTV
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
        c0,
        l0,
        borrow_amount,
        0,
    )
    .expect("borrow must succeed (HF healthy)");
    assert_custody(&svm, &market, &[lender_position, borrower_position]);

    // --- 5. Interest accrual (permissionless instruction; zero IRM slopes in reference_market_args
    // means zero accrued interest, but the instruction itself is still exercised and custody is
    // still asserted -- accrual must be a genuine no-op on the collateral side regardless). ---
    let mut clock = svm.get_sysvar::<solana_clock::Clock>();
    clock.unix_timestamp += 86_400;
    svm.set_sysvar(&clock);
    aegis_test_kit::accrue_interest(&mut svm, &admin, market, fee_position)
        .expect("accrue_interest must succeed");
    assert_custody(&svm, &market, &[lender_position, borrower_position]);

    // --- 6/7/8. Price crashes; the position becomes liquidatable ---
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
    let liquidator = Keypair::new_from_array([130u8; 32]);
    svm.airdrop(&liquidator.pubkey(), 10_000_000_000).unwrap();
    let liquidator_loan_ata = create_token_account(
        &mut svm,
        &admin,
        131,
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
        132,
        sol_mint,
        liquidator.pubkey(),
        spl_token_2022_interface::ID,
        fee_extensions,
    );
    let debt_before_liq = fetch_position(&svm, &borrower_position).borrow_shares;
    let market_before_liq = fetch_market(&svm, &market);
    let debt_assets_before_liq = aegis_math::to_assets_up(
        debt_before_liq,
        market_before_liq.total_borrow_assets,
        market_before_liq.total_borrow_shares,
    )
    .unwrap();
    liquidate(
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
        debt_assets_before_liq,
        0,
    )
    .expect("liquidation must succeed");
    assert_custody(&svm, &market, &[lender_position, borrower_position]);
    let collateral_fee_accrued_after_liq1 = fetch_market(&svm, &market).collateral_fee_accrued;
    assert!(
        collateral_fee_accrued_after_liq1 > 0,
        "protocol_cut must have accrued into collateral_fee_accrued"
    );

    // --- 9. Deeper insolvency: whether the first liquidation already clamped to zero collateral
    // with debt remaining (bad debt), the adaptive path mirrors phase6_integration.rs exactly. ---
    let position_after_liq1 = fetch_position(&svm, &borrower_position);
    if position_after_liq1.collateral_amount == 0 && position_after_liq1.borrow_shares > 0 {
        println!("  (first liquidation clamped to zero collateral with debt remaining -- bad debt exists)");
    } else {
        // Top up and re-borrow, then crash much further to force bad debt deliberately.
        svm.expire_blockhash();
        mint_to(
            &mut svm,
            &admin,
            sol_mint,
            borrower_collateral_ata,
            &admin,
            deposit_amount,
            spl_token_2022_interface::ID,
        );
        deposit_collateral(
            &mut svm,
            &borrower,
            market,
            borrower_position,
            collateral_vault,
            borrower_collateral_ata,
            sol_mint,
            spl_token_2022_interface::ID,
            deposit_amount,
        )
        .expect("top-up deposit_collateral must succeed");
        assert_custody(&svm, &market, &[lender_position, borrower_position]);

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
            &borrower,
            market,
            borrower_position,
            fee_position,
            loan_vault,
            borrower_loan_ata,
            usdc_mint,
            spl_token_interface::ID,
            c2,
            l2,
            700_000_000,
            0,
        )
        .expect("second borrow must succeed");
        assert_custody(&svm, &market, &[lender_position, borrower_position]);

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
        mint_to(
            &mut svm,
            &admin,
            usdc_mint,
            liquidator_loan_ata,
            &admin,
            2_000_000_000,
            spl_token_interface::ID,
        );
        let debt_before_liq2 = {
            let p = fetch_position(&svm, &borrower_position);
            let m = fetch_market(&svm, &market);
            aegis_math::to_assets_up(
                p.borrow_shares,
                m.total_borrow_assets,
                m.total_borrow_shares,
            )
            .unwrap()
        };
        liquidate(
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
            c3,
            l3,
            debt_before_liq2,
            0,
        )
        .expect("second (clamped) liquidation must succeed");
        assert_custody(&svm, &market, &[lender_position, borrower_position]);
        let position_after_liq2 = fetch_position(&svm, &borrower_position);
        assert_eq!(position_after_liq2.collateral_amount, 0);
        assert!(position_after_liq2.borrow_shares > 0, "bad debt must exist");
    }

    // --- 10. Protocol first-loss: absorb_bad_debt burns fee shares before socializing ---
    let fee_shares_before = fetch_position(&svm, &fee_position).supply_shares;
    absorb_bad_debt(&mut svm, &admin, market, borrower_position, fee_position)
        .expect("absorb_bad_debt must succeed");
    assert_custody(&svm, &market, &[lender_position, borrower_position]);
    assert!(fetch_position(&svm, &fee_position).supply_shares <= fee_shares_before);
    assert_eq!(fetch_position(&svm, &borrower_position).borrow_shares, 0);

    // --- 11. Collateral fee behavior: admin withdraws the accrued protocol cut, denominated in
    // the fee-bearing collateral mint itself -- the admin's own ATA must be sized for it. ---
    let collateral_fee_accrued = fetch_market(&svm, &market).collateral_fee_accrued;
    assert!(collateral_fee_accrued > 0);
    let admin_collateral_ata = create_token_account(
        &mut svm,
        &admin,
        140,
        sol_mint,
        admin.pubkey(),
        spl_token_2022_interface::ID,
        fee_extensions,
    );
    withdraw_collateral_fees(
        &mut svm,
        &admin,
        market,
        collateral_vault,
        admin_collateral_ata,
        sol_mint,
        spl_token_2022_interface::ID,
        collateral_fee_accrued,
    )
    .expect("withdraw_collateral_fees must succeed");
    assert_custody(&svm, &market, &[lender_position, borrower_position]);
    assert_eq!(fetch_market(&svm, &market).collateral_fee_accrued, 0);
    // The admin receives less than `collateral_fee_accrued` (they bear the outbound fee, per
    // account-model.md §6.4) -- but Aegis's own internal accounting decremented by the exact
    // recorded amount regardless, which is exactly what assert_custody just proved.
    let admin_received = fetch_token_account_base(&svm, &admin_collateral_ata).amount;
    assert!(
        admin_received < collateral_fee_accrued,
        "the admin, as an ordinary recipient, must bear the outbound transfer fee"
    );

    // --- 12. Cleanup: lender withdraws ---
    let lender_position_state = fetch_position(&svm, &lender_position);
    aegis_test_kit::withdraw(
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
    assert_custody(&svm, &market, &[lender_position, borrower_position]);

    println!(
        "A-TOK-10: full lifecycle on a transfer-fee collateral market completed; \
         INV-CUS-01/INV-CUS-02 held after every state-changing instruction."
    );
}

// ---------------------------------------------------------------------------------------------
// A-TOK-11 — a transfer-fee rate raised by the fee authority mid-lifecycle must not break
// accounting. Uses the real `SetTransferFee` instruction and the real 2-epoch activation delay.
// ---------------------------------------------------------------------------------------------

#[test]
fn a_tok_11_fee_rate_change_mid_lifecycle_does_not_break_accounting() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fee_recipient = setup_protocol(&mut svm, &admin);

    // --- Market with an initial 1% transfer fee; `admin` is both mint authority and
    // transfer_fee_config_authority (mints.rs's `create_token_2022_mint` wiring). ---
    let initial_bps = 100u16; // 1%
    let sol_mint = create_token_2022_mint(
        &mut svm,
        &admin,
        150,
        9,
        admin.pubkey(),
        None,
        &[Token2022Extension::TransferFeeConfig {
            basis_points: initial_bps,
            maximum_fee: u64::MAX,
        }],
    );
    let usdc_mint = create_spl_mint(&mut svm, &admin, 151, 6, admin.pubkey(), None);
    let args = reference_market_args(0, COLLATERAL_FEED_ID, LOAN_FEED_ID, false);
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

    let lender = Keypair::new_from_array([160u8; 32]);
    svm.airdrop(&lender.pubkey(), 10_000_000_000).unwrap();
    let lender_ata = create_token_account(
        &mut svm,
        &admin,
        161,
        usdc_mint,
        lender.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    let supply_amount = 1_200_000_000u64;
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
    assert_custody(&svm, &market, &[lender_position]);

    let borrower = Keypair::new_from_array([170u8; 32]);
    svm.airdrop(&borrower.pubkey(), 10_000_000_000).unwrap();
    let fee_extensions = &[ExtensionType::TransferFeeConfig];
    let borrower_collateral_ata = create_token_account(
        &mut svm,
        &admin,
        171,
        sol_mint,
        borrower.pubkey(),
        spl_token_2022_interface::ID,
        fee_extensions,
    );
    let (_, borrower_position) = init_position(&mut svm, &admin, market, borrower.pubkey());

    // --- Deposit #1 under the ORIGINAL 1% rate ---
    let deposit1 = 10_000_000_000u64;
    mint_to(
        &mut svm,
        &admin,
        sol_mint,
        borrower_collateral_ata,
        &admin,
        deposit1,
        spl_token_2022_interface::ID,
    );
    deposit_collateral(
        &mut svm,
        &borrower,
        market,
        borrower_position,
        collateral_vault,
        borrower_collateral_ata,
        sol_mint,
        spl_token_2022_interface::ID,
        deposit1,
    )
    .expect("first deposit must succeed");
    assert_custody(&svm, &market, &[lender_position, borrower_position]);
    let credited1 = fetch_position(&svm, &borrower_position).collateral_amount;
    let fee1 = deposit1 - credited1;
    assert_eq!(fee1, 100_000_000, "1% of 10 SOL"); // ceil(10e9 * 100 / 10_000)

    // --- Borrower borrows against the position (healthy) ---
    let borrower_loan_ata = create_token_account(
        &mut svm,
        &admin,
        172,
        usdc_mint,
        borrower.pubkey(),
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
        500_000_000,
        0,
    )
    .expect("borrow must succeed");
    assert_custody(&svm, &market, &[lender_position, borrower_position]);

    // --- The fee authority raises the rate to 5%. Per real Token-2022 semantics
    // (`TransferFeeConfig::get_epoch_fee`), this schedules a `newer_transfer_fee` effective at
    // `current_epoch + 2` and does NOT touch the currently-effective rate. ---
    let raised_bps = 500u16; // 5%
    let epoch_at_change = svm.get_sysvar::<solana_clock::Clock>().epoch;
    set_transfer_fee_rate(&mut svm, &admin, sol_mint, &admin, raised_bps, u64::MAX);
    let config_immediately_after = fetch_transfer_fee_config(&svm, &sol_mint);
    assert_eq!(
        u16::from(
            config_immediately_after
                .get_epoch_fee(epoch_at_change)
                .transfer_fee_basis_points
        ),
        initial_bps,
        "SetTransferFee must not retroactively change the rate for the CURRENT epoch (2-epoch delay)"
    );

    // A deposit submitted in the SAME epoch, before the delay elapses, must still be charged the
    // OLD rate -- proving Aegis reads whatever the token program actually applies (measured
    // delta), not a value it cached at deposit #1.
    svm.expire_blockhash();
    let deposit_same_epoch = 5_000_000_000u64;
    mint_to(
        &mut svm,
        &admin,
        sol_mint,
        borrower_collateral_ata,
        &admin,
        deposit_same_epoch,
        spl_token_2022_interface::ID,
    );
    let collateral_before_same_epoch = fetch_position(&svm, &borrower_position).collateral_amount;
    deposit_collateral(
        &mut svm,
        &borrower,
        market,
        borrower_position,
        collateral_vault,
        borrower_collateral_ata,
        sol_mint,
        spl_token_2022_interface::ID,
        deposit_same_epoch,
    )
    .expect("same-epoch deposit must succeed");
    assert_custody(&svm, &market, &[lender_position, borrower_position]);
    let credited_same_epoch =
        fetch_position(&svm, &borrower_position).collateral_amount - collateral_before_same_epoch;
    assert_eq!(
        deposit_same_epoch - credited_same_epoch,
        50_000_000, // still 1% -- the raise has not activated yet
        "a deposit in the same epoch as SetTransferFee must still be charged the OLD rate"
    );

    // --- Advance past the 2-epoch delay: the new 5% rate is now effective. ---
    advance_epoch(&mut svm, 3);
    let epoch_after_delay = svm.get_sysvar::<solana_clock::Clock>().epoch;
    assert_eq!(
        u16::from(
            fetch_transfer_fee_config(&svm, &sol_mint)
                .get_epoch_fee(epoch_after_delay)
                .transfer_fee_basis_points
        ),
        raised_bps,
        "after the 2-epoch delay, the raised rate must be the effective one"
    );

    // --- Deposit #2 under the NEW 5% rate: measured-delta accounting picks it up automatically,
    // with no code change and no cached rate anywhere in Aegis. ---
    svm.expire_blockhash();
    let deposit2 = 10_000_000_000u64;
    mint_to(
        &mut svm,
        &admin,
        sol_mint,
        borrower_collateral_ata,
        &admin,
        deposit2,
        spl_token_2022_interface::ID,
    );
    let collateral_before_deposit2 = fetch_position(&svm, &borrower_position).collateral_amount;
    deposit_collateral(
        &mut svm,
        &borrower,
        market,
        borrower_position,
        collateral_vault,
        borrower_collateral_ata,
        sol_mint,
        spl_token_2022_interface::ID,
        deposit2,
    )
    .expect("second deposit must succeed");
    assert_custody(&svm, &market, &[lender_position, borrower_position]);
    let credited2 =
        fetch_position(&svm, &borrower_position).collateral_amount - collateral_before_deposit2;
    let fee2 = deposit2 - credited2;
    assert_eq!(fee2, 500_000_000, "5% of 10 SOL"); // ceil(10e9 * 500 / 10_000)
    assert!(
        fee2 > fee1 * 4,
        "the raised rate must actually be reflected in what was credited, not the stale 1% rate"
    );

    // --- The lifecycle continues past the rate change: a debt-free-adjacent withdrawal exercises
    // the outbound leg (vault debits exactly the recorded amount regardless of the current fee
    // rate; the recipient bears whatever fee is in effect on THAT leg) and custody still
    // reconciles exactly. ---
    let owner_collateral_ata = create_token_account(
        &mut svm,
        &admin,
        173,
        sol_mint,
        borrower.pubkey(),
        spl_token_2022_interface::ID,
        fee_extensions,
    );
    let n1 = svm.get_sysvar::<solana_clock::Clock>().unix_timestamp;
    let c1 = set_price(
        &mut svm,
        202,
        PriceFixture::valid(COLLATERAL_FEED_ID, 15_000_000_000, 0, -8, n1),
    );
    let l1 = set_price(
        &mut svm,
        203,
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 0, -8, n1),
    );
    let withdraw_amount = 1_000_000_000u64; // small, well within remaining LTV headroom
    let vault_before_withdraw = fetch_token_account_base(&svm, &collateral_vault).amount;
    withdraw_collateral(
        &mut svm,
        &borrower,
        market,
        borrower_position,
        collateral_vault,
        owner_collateral_ata,
        sol_mint,
        spl_token_2022_interface::ID,
        c1,
        l1,
        withdraw_amount,
    )
    .expect("withdraw_collateral must succeed");
    assert_custody(&svm, &market, &[lender_position, borrower_position]);
    let vault_after_withdraw = fetch_token_account_base(&svm, &collateral_vault).amount;
    assert_eq!(
        vault_before_withdraw - vault_after_withdraw,
        withdraw_amount,
        "the vault must debit exactly the recorded amount on an outbound transfer, \
         regardless of the current transfer-fee rate"
    );
    let recipient_received = fetch_token_account_base(&svm, &owner_collateral_ata).amount;
    assert!(
        recipient_received < withdraw_amount,
        "the recipient bears the (now 5%) outbound fee"
    );

    println!(
        "A-TOK-11: fee raised from {initial_bps}bps to {raised_bps}bps mid-lifecycle \
         (fee1={fee1}, fee2={fee2}); INV-CUS-01/INV-CUS-02 held throughout with no cached rate."
    );
}
