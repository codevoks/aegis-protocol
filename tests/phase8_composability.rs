//! Phase 8 — composability and liquidation routing (`docs/phases/phase-08-composability.md`,
//! `docs/composability.md`, ADR-0013). Covers `I-LIQ-CB-01`, `I-LIQ-CB-02`, `INV-AUTH-07`
//! (`A-AUTH-07`), and `INV-RES-07`. The four hostile-callback attacks (`A-CPI-01..04`) live in
//! `tests/phase8_hostile_callback.rs`.

#![allow(clippy::result_large_err)]

mod phase8_common;
use phase8_common::*;

use aegis::error::AegisError;
use aegis::instructions::liquidate::liquidate::build_callback_instruction;
use aegis_test_kit::{
    fetch_market, fetch_position, invariants, liquidate_with_callback, spl_token_interface,
    token_accounts::fetch_token_account_base,
};
use anchor_lang::solana_program::account_info::AccountInfo;
use solana_pubkey::Pubkey;

/// Builds a fake `AccountInfo` for unit-testing `build_callback_instruction` without a real SVM.
/// A free function (not a closure) so its lifetime is expressed properly: the returned
/// `AccountInfo<'a>` borrows exactly `key`, `lamports`, `data` and `owner`, all for the same `'a`.
fn fake_account_info<'a>(
    key: &'a Pubkey,
    storage: &'a mut (u64, [u8; 0]),
    owner: &'a Pubkey,
) -> AccountInfo<'a> {
    AccountInfo::new(
        key,
        false,
        true,
        &mut storage.0,
        &mut storage.1,
        owner,
        false,
    )
}

// ============================================================================================
// I-LIQ-CB-01: honest callback -- seize, swap locally (deterministic rate), repay, one
// transaction, no network.
// ============================================================================================

#[test]
fn i_liq_cb_01_honest_callback_seizes_swaps_locally_and_repays_in_one_transaction() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy_with_labs();
    let (fx, _fee_recipient) = setup_market(&mut svm, &admin, &mut seeds);
    let (_borrower, borrower_position, _borrower_collateral_ata, _borrower_loan_ata) =
        setup_borrowed_position(
            &mut svm,
            &admin,
            &fx,
            &mut seeds,
            1_000_000_000_000,
            10_000_000_000,
            900_000_000,
        );

    // The liquidator is deliberately under-funded: zero loan-asset balance. Without the
    // callback, this liquidation would be impossible (the product problem docs/composability.md
    // §1 names).
    let (liquidator, liquidator_loan_ata, liquidator_collateral_ata) =
        liquidator_wallets(&mut svm, &admin, &fx, &mut seeds, 0);

    // example-liquidator's own reserve, funded ahead of time by its (hypothetical) operator --
    // its collateral-receiving account starts empty; Aegis funds it during the callback CPI.
    let (callback_collateral_account, loan_reserve) =
        setup_example_liquidator_reserve(&mut svm, &admin, &fx, &mut seeds, 2_000_000_000);

    let n = now(&svm);
    let (c, l) = crash_prices(&mut svm, &mut seeds, n);

    let market_before = fetch_market(&svm, &fx.market);
    let total_supply_before = market_before.total_supply_assets;

    // $100/SOL deterministic local rate -- comfortably covers the $900 USDC required repayment
    // from the ~9.92 SOL this worked example seizes (tests/phase6_liquidation.rs's own
    // u_liq_01_worked_example_on_chain uses the identical setup and crash prices).
    let callback_data = example_liquidator_callback_data(100_000_000_000_000_000_000);
    let extra_accounts = example_liquidator_remaining_accounts(loan_reserve);

    liquidate_with_callback(
        &mut svm,
        &liquidator,
        fx.market,
        borrower_position,
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
        900_000_000,
        0,
        example_liquidator::ID,
        callback_collateral_account,
        extra_accounts,
        callback_data,
    )
    .expect("callback liquidation must succeed even though the liquidator holds zero loan asset");

    let position_after = fetch_position(&svm, &borrower_position);
    let market_after = fetch_market(&svm, &fx.market);

    // Same worked-example figures as the no-callback path (tests/phase6_liquidation.rs
    // u_liq_01_worked_example_on_chain) -- the callback changes WHERE seized collateral lands and
    // HOW repayment is funded, never the liquidation math itself.
    assert_eq!(market_after.total_borrow_assets, 0, "debt fully repaid");
    assert_eq!(position_after.borrow_shares, 0);
    assert_eq!(
        position_after.collateral_amount,
        10_000_000_000 - 9_970_348_101
    );
    assert_eq!(market_after.collateral_fee_accrued, 47_477_848);
    assert_eq!(market_after.total_supply_assets, total_supply_before);

    // The liquidator never held any loan asset, and Aegis never touched their loan ATA in the
    // callback branch (ADR-0013): it stays at zero throughout.
    let liquidator_loan = fetch_token_account_base(&svm, &liquidator_loan_ata);
    assert_eq!(liquidator_loan.amount, 0);

    // The callback received exactly `to_liquidator`, then forwarded the required repayment (and
    // kept the rest as its own swap margin -- not Aegis's concern).
    let callback_collateral_after = fetch_token_account_base(&svm, &callback_collateral_account);
    assert_eq!(callback_collateral_after.amount, 9_922_870_253);

    let loan_vault_after = fetch_token_account_base(&svm, &fx.loan_vault);
    assert!(
        loan_vault_after.amount >= 900_000_000,
        "loan_vault must have received at least the required repayment"
    );
    // The deterministic $100/SOL rate intentionally overshoots the $900 USDC required repayment
    // (by design -- a real swap's slippage margin will almost always do the same). The surplus is
    // ordinary, harmless, unaccounted vault surplus -- exactly like a direct donation
    // (INV-CUS-08) -- so the STRICT form of INV-CUS-01 (`loan_vault.amount ==
    // total_supply_assets - total_borrow_assets`, exactly) does not hold here by design; the
    // security-relevant direction (the vault is never SHORT) is asserted explicitly above and
    // below instead. INV-CUS-01's exact-equality form is proven to hold for the no-callback path
    // in `i_liq_cb_02_...` and throughout `tests/phase6_liquidation.rs`, which this callback
    // branch's accounting math is otherwise identical to.
    assert!(
        loan_vault_after.amount
            >= market_after.total_supply_assets - market_after.total_borrow_assets,
        "loan_vault must never be SHORT of what internal accounting requires"
    );

    // Guard cleared on the success path.
    assert_eq!(market_after.liquidation_guard, 0);

    // INV-CUS-02 (collateral side) is untouched by the loan-side swap surplus and holds exactly.
    invariants::assert_inv_cus_02(&svm, &fx.market, &[borrower_position]);
}

// ============================================================================================
// I-LIQ-CB-02: omitting the callback reproduces Phase 6 behavior exactly.
// ============================================================================================

#[test]
fn i_liq_cb_02_omitting_the_callback_matches_the_no_callback_worked_example_exactly() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy_with_labs();
    let (fx, _fee_recipient) = setup_market(&mut svm, &admin, &mut seeds);
    let (_borrower, borrower_position, _borrower_collateral_ata, _borrower_loan_ata) =
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

    // Calls the SAME `liquidate` instruction the Phase 8 program now exposes, but with the
    // callback accounts omitted (`aegis_test_kit::liquidate` sets both to `None` -- see
    // crates/aegis-test-kit/src/market.rs::liquidate_ix). This is not a different code path in
    // the test harness; it is the identical client call a Phase 6-only liquidator would make.
    aegis_test_kit::liquidate(
        &mut svm,
        &liquidator,
        fx.market,
        borrower_position,
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
        900_000_000,
        0,
    )
    .expect("no-callback liquidation must still work exactly as Phase 6");

    let position_after = fetch_position(&svm, &borrower_position);
    let market_after = fetch_market(&svm, &fx.market);

    // Byte-for-byte the same figures as tests/phase6_liquidation.rs::u_liq_01_worked_example_on_chain.
    assert_eq!(market_after.total_borrow_assets, 0);
    assert_eq!(position_after.borrow_shares, 0);
    assert_eq!(
        position_after.collateral_amount,
        10_000_000_000 - 9_970_348_101
    );
    assert_eq!(market_after.collateral_fee_accrued, 47_477_848);
    assert_eq!(
        market_after.liquidation_guard, 0,
        "never touched on this path"
    );

    let liquidator_collateral = fetch_token_account_base(&svm, &liquidator_collateral_ata);
    assert_eq!(liquidator_collateral.amount, 9_922_870_253);

    invariants::assert_inv_cus_01(&svm, &fx.market);
    invariants::assert_inv_cus_02(&svm, &fx.market, &[borrower_position]);
}

// ============================================================================================
// INV-AUTH-07 / A-AUTH-07: the callback CPI's account-meta list never includes Market or
// liquidator, and no entry is ever marked as a signer -- asserted directly against the actual
// constructed `Vec<AccountMeta>`, not by reading the source.
// ============================================================================================

#[test]
fn a_auth_07_callback_instruction_never_carries_market_or_liquidator_or_any_signer() {
    let market = Pubkey::new_unique();
    let liquidator = Pubkey::new_unique();
    let position = Pubkey::new_unique();
    let fee_position = Pubkey::new_unique();
    let collateral_vault = Pubkey::new_unique();
    let liquidator_loan_ata = Pubkey::new_unique();
    let liquidator_collateral_ata = Pubkey::new_unique();

    let callback_program = Pubkey::new_unique();
    let callback_collateral_account = Pubkey::new_unique();
    let loan_vault = Pubkey::new_unique();
    let collateral_mint = Pubkey::new_unique();
    let loan_mint = Pubkey::new_unique();
    let collateral_token_program = Pubkey::new_unique();
    let loan_token_program = Pubkey::new_unique();
    // An honest, unrelated remaining account a real DEX route might need.
    let dex_pool = Pubkey::new_unique();

    let owner = Pubkey::new_unique();
    // Each fake `AccountInfo` needs its own independent lamports/data storage -- they all have to
    // coexist as separate bindings below, so a single shared `&mut` pair (as a closure would
    // naturally capture) cannot work.
    let mut storage: [(u64, [u8; 0]); 7] = [(0, []); 7];
    let [s0, s1, s2, s3, s4, s5, s6] = &mut storage;

    let protected_keys = [
        market,
        liquidator,
        position,
        fee_position,
        collateral_vault,
        liquidator_loan_ata,
        liquidator_collateral_ata,
    ];

    let cca_info = fake_account_info(&callback_collateral_account, s0, &owner);
    let lv_info = fake_account_info(&loan_vault, s1, &owner);
    let cm_info = fake_account_info(&collateral_mint, s2, &owner);
    let lm_info = fake_account_info(&loan_mint, s3, &owner);
    let ctp_info = fake_account_info(&collateral_token_program, s4, &owner);
    let ltp_info = fake_account_info(&loan_token_program, s5, &owner);
    let dex_info = fake_account_info(&dex_pool, s6, &owner);

    let ix = build_callback_instruction(
        callback_program,
        &cca_info,
        &lv_info,
        &cm_info,
        &lm_info,
        &ctp_info,
        &ltp_info,
        std::slice::from_ref(&dex_info),
        &protected_keys,
        vec![1, 2, 3],
    )
    .expect("a well-formed, non-colliding remaining account must be accepted");

    assert_eq!(ix.program_id, callback_program);
    assert_eq!(ix.data, vec![1, 2, 3]);

    // The two most important assertions in this test: neither Market nor liquidator's pubkey
    // appears anywhere in the callback's account list, under any privilege level.
    let pubkeys: Vec<Pubkey> = ix.accounts.iter().map(|m| m.pubkey).collect();
    assert!(
        !pubkeys.contains(&market),
        "Market must never reach the callback"
    );
    assert!(
        !pubkeys.contains(&liquidator),
        "the liquidator's account must never reach the callback"
    );
    assert!(!pubkeys.contains(&position));
    assert!(!pubkeys.contains(&fee_position));
    assert!(!pubkeys.contains(&collateral_vault));
    assert!(!pubkeys.contains(&liquidator_loan_ata));
    assert!(!pubkeys.contains(&liquidator_collateral_ata));

    // No entry -- not even the honest DEX passthrough -- is ever marked as a signer.
    for meta in &ix.accounts {
        assert!(
            !meta.is_signer,
            "no callback account meta may ever be a signer (ADR-0013 §1.4): {:?}",
            meta.pubkey
        );
    }

    // Exactly the 6 fixed accounts plus the 1 passthrough.
    assert_eq!(ix.accounts.len(), 7);
    assert_eq!(pubkeys[0], callback_collateral_account);
    assert_eq!(pubkeys[1], loan_vault);
    assert_eq!(pubkeys[6], dex_pool);
}

/// `CallbackAccountNotPermitted`: a hostile keeper trying to smuggle `market` (or any other
/// protected key) into the callback's account list via `remaining_accounts` is rejected outright,
/// before any CPI is attempted.
#[test]
fn callback_account_not_permitted_rejects_a_smuggled_protected_key() {
    let market = Pubkey::new_unique();
    let protected_keys = [market];

    let owner = Pubkey::new_unique();
    let mut storage: [(u64, [u8; 0]); 7] = [(0, []); 7];
    let [s0, s1, s2, s3, s4, s5, s6] = &mut storage;

    let cca = Pubkey::new_unique();
    let lv = Pubkey::new_unique();
    let cm = Pubkey::new_unique();
    let lm = Pubkey::new_unique();
    let ctp = Pubkey::new_unique();
    let ltp = Pubkey::new_unique();

    let cca_info = fake_account_info(&cca, s0, &owner);
    let lv_info = fake_account_info(&lv, s1, &owner);
    let cm_info = fake_account_info(&cm, s2, &owner);
    let lm_info = fake_account_info(&lm, s3, &owner);
    let ctp_info = fake_account_info(&ctp, s4, &owner);
    let ltp_info = fake_account_info(&ltp, s5, &owner);
    let smuggled = fake_account_info(&market, s6, &owner);

    let err = build_callback_instruction(
        Pubkey::new_unique(),
        &cca_info,
        &lv_info,
        &cm_info,
        &lm_info,
        &ctp_info,
        &ltp_info,
        std::slice::from_ref(&smuggled),
        &protected_keys,
        vec![],
    )
    .unwrap_err();
    assert_eq!(
        err,
        anchor_lang::error::Error::from(AegisError::CallbackAccountNotPermitted)
    );
}

// ============================================================================================
// Cross-market isolation during a callback (A-PAR-02 spirit): a callback in flight on Market A
// never touches Market B's state, and the reentrancy guard is per-Market, not global.
// ============================================================================================

#[test]
fn callback_on_one_market_never_touches_an_unrelated_market() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy_with_labs();

    let (fx_a, fee_recipient_a) = setup_market(&mut svm, &admin, &mut seeds);
    let fx_b = setup_second_market(&mut svm, &admin, fee_recipient_a, &mut seeds);

    let (_bo_a, pos_a, _bca_a, _bla_a) = setup_borrowed_position(
        &mut svm,
        &admin,
        &fx_a,
        &mut seeds,
        1_000_000_000_000,
        10_000_000_000,
        900_000_000,
    );

    let market_b_before = fetch_market(&svm, &fx_b.market);

    let (liquidator, liquidator_loan_ata, liquidator_collateral_ata) =
        liquidator_wallets(&mut svm, &admin, &fx_a, &mut seeds, 0);
    let (callback_collateral_account, loan_reserve) =
        setup_example_liquidator_reserve(&mut svm, &admin, &fx_a, &mut seeds, 2_000_000_000);

    let n = now(&svm);
    let (c, l) = crash_prices(&mut svm, &mut seeds, n);

    liquidate_with_callback(
        &mut svm,
        &liquidator,
        fx_a.market,
        pos_a,
        fx_a.fee_position,
        fx_a.loan_vault,
        fx_a.collateral_vault,
        liquidator_loan_ata,
        liquidator_collateral_ata,
        fx_a.loan_mint,
        fx_a.collateral_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        c,
        l,
        900_000_000,
        0,
        example_liquidator::ID,
        callback_collateral_account,
        example_liquidator_remaining_accounts(loan_reserve),
        example_liquidator_callback_data(100_000_000_000_000_000_000),
    )
    .expect("callback liquidation on market A must succeed");

    // `Market` derives neither `PartialEq` nor `Debug` (Anchor's `#[account]` does not add them),
    // so the "completely untouched" claim is checked field-by-field on the fields a callback
    // liquidation could conceivably disturb if cross-market isolation were broken.
    let market_b_after = fetch_market(&svm, &fx_b.market);
    assert_eq!(
        market_b_before.total_supply_assets,
        market_b_after.total_supply_assets
    );
    assert_eq!(
        market_b_before.total_supply_shares,
        market_b_after.total_supply_shares
    );
    assert_eq!(
        market_b_before.total_borrow_assets,
        market_b_after.total_borrow_assets
    );
    assert_eq!(
        market_b_before.total_borrow_shares,
        market_b_after.total_borrow_shares
    );
    assert_eq!(
        market_b_before.collateral_fee_accrued,
        market_b_after.collateral_fee_accrued
    );
    assert_eq!(
        market_b_before.last_accrual_ts,
        market_b_after.last_accrual_ts
    );
    assert_eq!(
        market_b_before.liquidation_guard,
        market_b_after.liquidation_guard
    );
    assert_eq!(
        market_b_after.liquidation_guard, 0,
        "Market B's guard must never be touched by a callback liquidation on Market A"
    );
}
