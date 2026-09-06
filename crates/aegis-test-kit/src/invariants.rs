//! Global invariant checks, callable after any instruction (`architecture.md` §2). Phase 3 adds
//! the one **[GLOBAL]** invariant assigned to this phase, INV-CUS-02
//! (`docs/invariants.md` §B); later phases extend `assert_all` rather than replacing it, so a
//! call site written today keeps gaining coverage for free as more invariants become checkable.

use crate::market::{fetch_market, fetch_position};
use crate::token_accounts::fetch_token_account_base;
use litesvm::LiteSVM;
use solana_pubkey::Pubkey;

/// **INV-CUS-02** `[GLOBAL]`: `collateral_vault.amount == Σ(position.collateral_amount) +
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

/// **INV-CUS-01** `[GLOBAL]`: `loan_vault.amount == total_supply_assets - total_borrow_assets`
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

/// **INV-ACC-01** `[GLOBAL]`: `total_supply_shares == Σ(position.supply_shares)` over every
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

/// **INV-ACC-02** `[GLOBAL]`: `total_borrow_shares == Σ(position.borrow_shares)` over every
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

/// **INV-ACC-03** `[GLOBAL]`: `total_supply_assets >= total_borrow_assets` (`docs/invariants.md`)
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

/// **INV-ACC-06**: `total_supply_shares == 0 <=> total_supply_assets == 0`, and likewise for
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
