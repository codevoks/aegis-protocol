//! Phase 8 — hostile liquidation callback (`docs/phases/phase-08-composability.md`,
//! `docs/composability.md`, ADR-0013). `A-CPI-01..04`: every attack must fail, and must fail
//! **atomically** -- a before/after snapshot proves zero state diff, not merely "the transaction
//! returned an error" (spec requirement, and `testing-strategy.md` §4.2's own standard).
//!
//! `A-CPI-02` additionally gets a *direct*, non-CPI unit-level test of the Aegis-level reentrancy
//! guard, independent of whatever the runtime's own reentrancy behavior turns out to be
//! (`docs/ecosystem-research.md` §16.1 distinguishes runtime protection from protocol protection;
//! this file proves both, separately, rather than crediting one for the other).

#![allow(clippy::result_large_err)]

mod phase8_common;
use phase8_common::*;

use aegis::error::AegisError;
use aegis_test_kit::{
    assert_aegis_error, fetch_market, fetch_position, liquidate_with_callback,
    liquidate_with_callback_and_compute_limit, set_liquidation_guard, spl_token_interface,
    token_accounts::fetch_token_account_base,
};
use hostile_callback::AttackMode;
use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

#[derive(Debug, PartialEq)]
struct Snapshot {
    total_supply_assets: u64,
    total_supply_shares: u128,
    total_borrow_assets: u64,
    total_borrow_shares: u128,
    collateral_fee_accrued: u64,
    liquidation_guard: u8,
    position_borrow_shares: u128,
    position_collateral_amount: u64,
    collateral_vault_amount: u64,
    loan_vault_amount: u64,
}

fn snapshot(svm: &litesvm::LiteSVM, fx: &Fixture, position: Pubkey) -> Snapshot {
    let market = fetch_market(svm, &fx.market);
    let pos = fetch_position(svm, &position);
    let collateral_vault = fetch_token_account_base(svm, &fx.collateral_vault);
    let loan_vault = fetch_token_account_base(svm, &fx.loan_vault);
    Snapshot {
        total_supply_assets: market.total_supply_assets,
        total_supply_shares: market.total_supply_shares,
        total_borrow_assets: market.total_borrow_assets,
        total_borrow_shares: market.total_borrow_shares,
        collateral_fee_accrued: market.collateral_fee_accrued,
        liquidation_guard: market.liquidation_guard,
        position_borrow_shares: pos.borrow_shares,
        position_collateral_amount: pos.collateral_amount,
        collateral_vault_amount: collateral_vault.amount,
        loan_vault_amount: loan_vault.amount,
    }
}

/// Common setup for every hostile-callback attack: the exact worked-example scenario
/// (`tests/phase6_liquidation.rs`), an under-funded liquidator, and hostile-callback's own
/// (unfunded, uncontrolled) collateral-receiving account.
struct Scenario {
    svm: litesvm::LiteSVM,
    fx: Fixture,
    borrower_position: Pubkey,
    liquidator: solana_keypair::Keypair,
    liquidator_loan_ata: Pubkey,
    liquidator_collateral_ata: Pubkey,
    callback_collateral_account: Pubkey,
    c: Pubkey,
    l: Pubkey,
}

fn setup_scenario() -> Scenario {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy_with_labs();
    let (fx, _fee_recipient) = setup_market(&mut svm, &admin, &mut seeds);
    let (_borrower, borrower_position, _bca, _bla) = setup_borrowed_position(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        10_000_000_000,
        900_000_000,
    );
    let (liquidator, liquidator_loan_ata, liquidator_collateral_ata) =
        liquidator_wallets(&mut svm, &admin, &fx, &mut seeds, 0);

    // hostile-callback receives seized collateral into a plain token account -- it never has a
    // real PDA authority over it (it doesn't need one for any of the four attacks).
    let callback_collateral_account = aegis_test_kit::create_token_account(
        &mut svm,
        &admin,
        seeds.next(),
        fx.collateral_mint,
        admin.pubkey(),
        spl_token_interface::ID,
        &[],
    );

    let n = now(&svm);
    let (c, l) = crash_prices(&mut svm, &mut seeds, n);

    Scenario {
        svm,
        fx,
        borrower_position,
        liquidator,
        liquidator_loan_ata,
        liquidator_collateral_ata,
        callback_collateral_account,
        c,
        l,
    }
}

// ============================================================================================
// A-CPI-01: hostile callback attempts to move vault funds -- must fail, atomically.
// ============================================================================================

#[test]
fn a_cpi_01_hostile_callback_cannot_drain_the_loan_vault() {
    let mut s = setup_scenario();
    let attacker_destination = aegis_test_kit::create_token_account(
        &mut s.svm,
        &s.liquidator,
        200,
        s.fx.loan_mint,
        s.liquidator.pubkey(),
        spl_token_interface::ID,
        &[],
    );

    let before = snapshot(&s.svm, &s.fx, s.borrower_position);

    let extra_accounts = vec![AccountMeta::new(attacker_destination, false)];
    let result = liquidate_with_callback(
        &mut s.svm,
        &s.liquidator,
        s.fx.market,
        s.borrower_position,
        s.fx.fee_position,
        s.fx.loan_vault,
        s.fx.collateral_vault,
        s.liquidator_loan_ata,
        s.liquidator_collateral_ata,
        s.fx.loan_mint,
        s.fx.collateral_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        s.c,
        s.l,
        900_000_000,
        0,
        hostile_callback::ID,
        s.callback_collateral_account,
        extra_accounts,
        hostile_callback_data(AttackMode::DrainVault {
            amount: 500_000_000,
        }),
    );
    assert!(
        result.is_err(),
        "a hostile callback with no authority over loan_vault must not be able to drain it"
    );

    let after = snapshot(&s.svm, &s.fx, s.borrower_position);
    assert_eq!(
        before, after,
        "A-CPI-01: the failed attack must leave ZERO state diff (atomic rollback), not merely fail"
    );

    let attacker = fetch_token_account_base(&s.svm, &attacker_destination);
    assert_eq!(attacker.amount, 0, "the attacker must receive nothing");
}

// ============================================================================================
// A-CPI-02: hostile callback attempts to reenter `liquidate` on the same market -- must fail.
// ============================================================================================

/// The real, end-to-end CPI attempt: `hostile-callback` tries to `invoke()` Aegis's own
/// `liquidate` from inside the callback CPI. Which specific error surfaces is not asserted
/// precisely here on purpose -- in this harness it comes back as a missing-account/dispatch
/// failure (the callback has no legitimate way to name a complete, well-formed call: it holds no
/// real signer per ADR-0013, and the target program is not otherwise reachable from the limited
/// account set it actually has), which is a different, earlier failure than the runtime's own
/// `InstructionError::ReentrancyNotAllowed` (RV-6, `docs/ecosystem-research.md` §16.1) would be
/// for a call that survived long enough to reach it. Asserting a specific error code here would
/// risk quietly asserting whichever error a future harness/runtime detail produces first, which
/// is exactly the misattribution `docs/ecosystem-research.md` §16.1 warns against -- so this test
/// asserts only the property that actually matters: **no reentry succeeds, and nothing changes**.
/// The Aegis-level guard itself is proven directly, in isolation, by the test below.
#[test]
fn a_cpi_02_hostile_callback_cannot_reenter_liquidate() {
    let mut s = setup_scenario();
    let before = snapshot(&s.svm, &s.fx, s.borrower_position);

    let result = liquidate_with_callback(
        &mut s.svm,
        &s.liquidator,
        s.fx.market,
        s.borrower_position,
        s.fx.fee_position,
        s.fx.loan_vault,
        s.fx.collateral_vault,
        s.liquidator_loan_ata,
        s.liquidator_collateral_ata,
        s.fx.loan_mint,
        s.fx.collateral_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        s.c,
        s.l,
        900_000_000,
        0,
        hostile_callback::ID,
        s.callback_collateral_account,
        vec![],
        hostile_callback_data(AttackMode::Reenter),
    );
    assert!(
        result.is_err(),
        "an attempted CPI back into Aegis's liquidate on the same market must fail"
    );

    let after = snapshot(&s.svm, &s.fx, s.borrower_position);
    assert_eq!(
        before, after,
        "A-CPI-02: the failed reentry attempt must leave ZERO state diff"
    );
}

/// The direct, protocol-level half of `A-CPI-02`: with `market.liquidation_guard` already set
/// (simulating "a callback is in flight"), an ORDINARY `liquidate` call -- no CPI, no hostile
/// program involved at all -- must be rejected by Aegis's own guard with the specific error,
/// independent of the runtime's reentrancy behavior. This is what proves the protocol-level
/// defense exists and works, rather than merely coinciding with a runtime rejection.
#[test]
fn a_cpi_02_the_aegis_level_guard_independently_rejects_liquidate_when_set() {
    let mut s = setup_scenario();
    set_liquidation_guard(&mut s.svm, s.fx.market, 1);

    let result = aegis_test_kit::liquidate(
        &mut s.svm,
        &s.liquidator,
        s.fx.market,
        s.borrower_position,
        s.fx.fee_position,
        s.fx.loan_vault,
        s.fx.collateral_vault,
        s.liquidator_loan_ata,
        s.liquidator_collateral_ata,
        s.fx.loan_mint,
        s.fx.collateral_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        s.c,
        s.l,
        900_000_000,
        0,
    );
    assert_aegis_error(&result, AegisError::LiquidationCallbackReentrancy);
}

// ============================================================================================
// A-CPI-03: hostile callback deliberately exhausts the compute budget -- transaction fails
// cleanly, atomically, no partial state.
// ============================================================================================

#[test]
fn a_cpi_03_compute_exhausting_callback_fails_cleanly_with_no_partial_state() {
    let mut s = setup_scenario();
    let before = snapshot(&s.svm, &s.fx, s.borrower_position);

    // A bounded outer compute limit, well above what the non-callback parts of `liquidate` need
    // but far below what 5,000,000 loop iterations cost in the SBF VM.
    let result = liquidate_with_callback_and_compute_limit(
        &mut s.svm,
        &s.liquidator,
        s.fx.market,
        s.borrower_position,
        s.fx.fee_position,
        s.fx.loan_vault,
        s.fx.collateral_vault,
        s.liquidator_loan_ata,
        s.liquidator_collateral_ata,
        s.fx.loan_mint,
        s.fx.collateral_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        s.c,
        s.l,
        900_000_000,
        0,
        hostile_callback::ID,
        s.callback_collateral_account,
        vec![],
        hostile_callback_data(AttackMode::BurnCompute {
            iterations: 5_000_000,
        }),
        400_000,
    );
    assert!(
        result.is_err(),
        "a callback that exhausts the compute budget must cause the transaction to fail"
    );

    let after = snapshot(&s.svm, &s.fx, s.borrower_position);
    assert_eq!(
        before, after,
        "A-CPI-03: compute exhaustion must leave ZERO partial state -- Solana's own atomicity, \
         not special-cased rollback logic"
    );
}

// ============================================================================================
// A-CPI-04: hostile callback returns Ok(()) without repaying anything -- Aegis must reject based
// on the measured post-callback delta, never the callback's return status.
// ============================================================================================

#[test]
fn a_cpi_04_callback_returning_ok_without_repayment_is_rejected_on_the_measured_delta() {
    let mut s = setup_scenario();
    let before = snapshot(&s.svm, &s.fx, s.borrower_position);

    let result = liquidate_with_callback(
        &mut s.svm,
        &s.liquidator,
        s.fx.market,
        s.borrower_position,
        s.fx.fee_position,
        s.fx.loan_vault,
        s.fx.collateral_vault,
        s.liquidator_loan_ata,
        s.liquidator_collateral_ata,
        s.fx.loan_mint,
        s.fx.collateral_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        s.c,
        s.l,
        900_000_000,
        0,
        hostile_callback::ID,
        s.callback_collateral_account,
        vec![],
        hostile_callback_data(AttackMode::NoRepayment),
    );
    assert_aegis_error(&result, AegisError::LiquidationCallbackRepaymentShortfall);

    let after = snapshot(&s.svm, &s.fx, s.borrower_position);
    assert_eq!(
        before, after,
        "A-CPI-04: a callback that returns Ok(()) without repaying must still leave ZERO state \
         diff -- the callback's successful return is never proof of repayment"
    );
}
