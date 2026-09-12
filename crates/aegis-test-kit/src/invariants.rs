//! Global invariant checks, callable after any instruction (`architecture.md` §2). Phase 3 adds
//! the one **[GLOBAL]** invariant assigned to this phase, INV-CUS-02
//! (`docs/invariants.md` §B); later phases extend `assert_all` rather than replacing it, so a
//! call site written today keeps gaining coverage for free as more invariants become checkable.
//!
//! Phase 10 (`docs/phases/phase-10-security.md`) completes the set: this file now implements all
//! nine **[GLOBAL]** invariants and adds [`assert_all_global`], the single entry point the
//! stateful fuzzer (`tests/fuzz/`) calls after every generated action, success or failure.

use crate::market::{fetch_market, fetch_position};
use crate::token_accounts::fetch_token_account_base;
use aegis::state::Market;
use litesvm::LiteSVM;
use solana_pubkey::Pubkey;

/// **F-INV-02** / **INV-CUS-02** `[GLOBAL]`: `collateral_vault.amount == Σ(position.collateral_amount) +
/// market.collateral_fee_accrued`, exactly — an equality, not a bound (`docs/invariants.md`).
///
/// `positions` must list every `Position` PDA that can hold collateral in this market; Phase 3 has
/// no on-chain registry of positions (`account-model.md` §2 rejects one by design), so the caller
/// supplies the set it created. This is what makes `A-CUS-08` provable: a direct donation to the
/// vault, credited to no position, makes this assertion fail loudly rather than silently pass.
pub fn assert_inv_cus_02(svm: &LiteSVM, market: &Pubkey, positions: &[Pubkey]) {
    let market_state = fetch_market(svm, market);
    let vault = fetch_token_account_base(svm, &market_state.collateral_vault);

    let sum_position_collateral: u128 = positions
        .iter()
        .map(|p| fetch_position(svm, p).collateral_amount as u128)
        .sum();
    let expected = sum_position_collateral + market_state.collateral_fee_accrued as u128;

    assert_eq!(
        vault.amount as u128, expected,
        "INV-CUS-02 violated: collateral_vault.amount ({}) != \
         Σ(position.collateral_amount) + market.collateral_fee_accrued ({expected})",
        vault.amount,
    );
}

/// **F-INV-01** / **INV-CUS-01** `[GLOBAL]`: `loan_vault.amount == total_supply_assets - total_borrow_assets`
/// exactly (`docs/invariants.md`) — the vault-reconciliation identity that is also the definition
/// of "free liquidity" `withdraw`/`borrow` are bounded by (`economic-model.md` §2).
pub fn assert_inv_cus_01(svm: &LiteSVM, market: &Pubkey) {
    let market_state = fetch_market(svm, market);
    let vault = fetch_token_account_base(svm, &market_state.loan_vault);
    let expected = market_state.total_supply_assets - market_state.total_borrow_assets;
    assert_eq!(
        vault.amount, expected,
        "INV-CUS-01 violated: loan_vault.amount ({}) != total_supply_assets - total_borrow_assets ({expected})",
        vault.amount,
    );
}

/// **F-INV-03** / **INV-ACC-01** `[GLOBAL]`: `total_supply_shares == Σ(position.supply_shares)` over every
/// position in the market, **including `fee_position`** (`docs/invariants.md`).
///
/// `positions` must list every non-fee `Position` PDA that can hold supply shares in this market
/// (Phase 4 has no on-chain position registry, so the caller supplies the set it created, same
/// convention as `assert_inv_cus_02`); `fee_position` is passed separately since it always exists
/// and is easy to forget.
pub fn assert_inv_acc_01(
    svm: &LiteSVM,
    market: &Pubkey,
    positions: &[Pubkey],
    fee_position: &Pubkey,
) {
    let market_state = fetch_market(svm, market);
    let mut sum: u128 = fetch_position(svm, fee_position).supply_shares;
    sum += positions
        .iter()
        .map(|p| fetch_position(svm, p).supply_shares)
        .sum::<u128>();
    assert_eq!(
        market_state.total_supply_shares, sum,
        "INV-ACC-01 violated: total_supply_shares ({}) != Σ(position.supply_shares) incl. fee_position ({sum})",
        market_state.total_supply_shares,
    );
}

/// **F-INV-04** / **INV-ACC-02** `[GLOBAL]`: `total_borrow_shares == Σ(position.borrow_shares)` over every
/// position in the market (`docs/invariants.md`).
pub fn assert_inv_acc_02(svm: &LiteSVM, market: &Pubkey, positions: &[Pubkey]) {
    let market_state = fetch_market(svm, market);
    let sum: u128 = positions
        .iter()
        .map(|p| fetch_position(svm, p).borrow_shares)
        .sum();
    assert_eq!(
        market_state.total_borrow_shares, sum,
        "INV-ACC-02 violated: total_borrow_shares ({}) != Σ(position.borrow_shares) ({sum})",
        market_state.total_borrow_shares,
    );
}

/// **F-INV-05** / **INV-ACC-03** `[GLOBAL]`: `total_supply_assets >= total_borrow_assets` (`docs/invariants.md`)
/// — the precondition that makes "free liquidity" a meaningful non-negative quantity.
pub fn assert_inv_acc_03(svm: &LiteSVM, market: &Pubkey) {
    let market_state = fetch_market(svm, market);
    assert!(
        market_state.total_supply_assets >= market_state.total_borrow_assets,
        "INV-ACC-03 violated: total_supply_assets ({}) < total_borrow_assets ({})",
        market_state.total_supply_assets,
        market_state.total_borrow_assets,
    );
}

/// **F-INV-06** / **INV-ACC-06**: `total_supply_shares == 0 <=> total_supply_assets == 0`, and likewise for
/// borrow (`docs/invariants.md`) — no orphaned assets without shares, or shares without assets.
pub fn assert_inv_acc_06(svm: &LiteSVM, market: &Pubkey) {
    let market_state = fetch_market(svm, market);
    assert_eq!(
        market_state.total_supply_shares == 0,
        market_state.total_supply_assets == 0,
        "INV-ACC-06 violated (supply): shares={} assets={}",
        market_state.total_supply_shares,
        market_state.total_supply_assets,
    );
    assert_eq!(
        market_state.total_borrow_shares == 0,
        market_state.total_borrow_assets == 0,
        "INV-ACC-06 violated (borrow): shares={} assets={}",
        market_state.total_borrow_shares,
        market_state.total_borrow_assets,
    );
}

/// **F-INV-07** / **INV-SOLV-01** `[GLOBAL]`: after `borrow` or the debt-bearing path of `withdraw_collateral`,
/// `debt_value <= collateral_value * max_ltv / WAD`, valued conservatively (collateral at the
/// confidence lower bound floored, debt at the upper bound ceiled — INV-ORA-03,
/// `docs/invariants.md` §D).
///
/// Recomputes the bound independently via `aegis_math` from the position/market's live on-chain
/// state — it never trusts the program's own `within_ltv` result, which is the whole point of a
/// mutation-resistant checker. `collateral_lo`/`loan_hi` are the SAME confidence-adjusted band the
/// just-completed operation validated (before any subsequent `move_price`): INV-SOLV-01 is a
/// property of that operation's own outcome, not of the position at an arbitrary later time — a
/// later price crash is *supposed* to make a position liquidatable, and asserting this bound
/// against a stale action's inputs after other actions have run would produce false positives.
/// The fuzzer calls this immediately after a successful `borrow`/debt-bearing
/// `withdraw_collateral`, never blindly after every action.
pub fn assert_inv_solv_01(
    svm: &LiteSVM,
    market: &Pubkey,
    position: &Pubkey,
    collateral_lo: u128,
    loan_hi: u128,
) {
    let market_state = fetch_market(svm, market);
    let position_state = fetch_position(svm, position);
    if position_state.borrow_shares == 0 {
        return;
    }
    let debt_assets = aegis_math::to_assets_up(
        position_state.borrow_shares,
        market_state.total_borrow_assets,
        market_state.total_borrow_shares,
    )
    .expect("INV-SOLV-01 checker: to_assets_up must not fail on live on-chain totals");
    let cv = aegis_math::collateral_value(
        position_state.collateral_amount,
        collateral_lo,
        market_state.collateral_decimals,
    )
    .expect("INV-SOLV-01 checker: collateral_value must not fail on live on-chain state");
    let dv = aegis_math::debt_value(debt_assets, loan_hi, market_state.loan_decimals)
        .expect("INV-SOLV-01 checker: debt_value must not fail on live on-chain state");
    let within = aegis_math::is_within_max_ltv(cv, dv, market_state.max_ltv)
        .expect("INV-SOLV-01 checker: is_within_max_ltv must not fail on live on-chain state");
    assert!(
        within,
        "INV-SOLV-01 violated: debt_value ({dv}) exceeds collateral_value*max_ltv/WAD (cv={cv}, max_ltv={})",
        market_state.max_ltv,
    );
}

/// **INV-ACC-04**: interest accrual leaves `total_supply_assets - total_borrow_assets` unchanged
/// (`docs/invariants.md` §C). Accrual is a pure transfer from borrowers to lenders — plus the
/// protocol fee share, itself drawn from the supply side, never created — and moves no tokens, so
/// free liquidity (== `loan_vault.amount` whenever INV-CUS-01 holds) cannot move.
///
/// Call this around the fuzzer's dedicated `accrue_interest` action specifically: every other
/// instruction also calls `accrue_mut` internally, but then legitimately changes the totals again
/// via its own deposit/withdraw/borrow/repay, which would swamp this specific signal. Only
/// `accrue_interest`'s entire effect *is* accrual, which is what isolates the property.
pub fn assert_inv_acc_04(before: &Market, after: &Market) {
    let free_before = before.total_supply_assets as i128 - before.total_borrow_assets as i128;
    let free_after = after.total_supply_assets as i128 - after.total_borrow_assets as i128;
    assert_eq!(
        free_before, free_after,
        "INV-ACC-04 violated: accrual changed total_supply_assets - total_borrow_assets from {free_before} to {free_after}",
    );
}

/// Every **[GLOBAL]** invariant (`docs/invariants.md`) checkable from a market's current
/// on-chain snapshot alone: INV-CUS-01, INV-CUS-02, INV-ACC-01, INV-ACC-02, INV-ACC-03,
/// INV-ACC-06. Called by the stateful fuzzer (`tests/fuzz/`) after **every** generated action,
/// whether it succeeded or failed (`testing-strategy.md` §5).
///
/// The remaining three GLOBAL invariants are deliberately not folded in here, because their
/// definitions are about a *transition*, not a snapshot, and a blanket post-action check would
/// either be redundant or produce false positives:
/// - **INV-SOLV-04** ("INV-CUS-01 holds through and after `absorb_bad_debt`") is exactly
///   INV-CUS-01 evaluated after that specific instruction — already covered here, since this
///   function runs after every action including `absorb_bad_debt`. No separate checker exists.
/// - **INV-SOLV-01** is a property of a successful `borrow`/debt-bearing `withdraw_collateral`'s
///   own outcome, not of a position at an arbitrary later time — see [`assert_inv_solv_01`],
///   called right after those two action kinds succeed.
/// - **INV-ACC-04** isolates interest accrual specifically, since every other instruction also
///   legitimately changes free liquidity via its own token movement — see [`assert_inv_acc_04`],
///   called around the fuzzer's dedicated `accrue_interest` action.
pub fn assert_all_global(
    svm: &LiteSVM,
    market: &Pubkey,
    positions: &[Pubkey],
    fee_position: &Pubkey,
) {
    assert_inv_cus_01(svm, market);
    assert_inv_cus_02(svm, market, positions);
    assert_inv_acc_01(svm, market, positions, fee_position);
    assert_inv_acc_02(svm, market, positions);
    assert_inv_acc_03(svm, market);
    assert_inv_acc_06(svm, market);
}

/// Runs every custody/accounting invariant Phase 3+4 can check for a market's lending side. At
/// minimum, `docs/phases/phase-04-lending.md` requires INV-CUS-01 after every meaningful operation
/// (`I-CUS-01`); this is the single entry point later phases add to rather than replace.
pub fn assert_all_lending(
    svm: &LiteSVM,
    market: &Pubkey,
    positions: &[Pubkey],
    fee_position: &Pubkey,
) {
    assert_inv_cus_01(svm, market);
    assert_inv_acc_01(svm, market, positions, fee_position);
    assert_inv_acc_02(svm, market, positions);
    assert_inv_acc_03(svm, market);
    assert_inv_acc_06(svm, market);
}

/// Runs every custody invariant Phase 3 can check. At minimum, `docs/phases/phase-03-collateral.md`
/// #11 requires INV-CUS-02; this is the single entry point later phases add to rather than
/// replace, so call sites written against `assert_all` keep gaining coverage automatically.
pub fn assert_all(svm: &LiteSVM, market: &Pubkey, positions: &[Pubkey]) {
    assert_inv_cus_02(svm, market, positions);
}
