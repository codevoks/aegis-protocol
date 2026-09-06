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

/// Runs every custody invariant Phase 3 can check. At minimum, `docs/phases/phase-03-collateral.md`
/// #11 requires INV-CUS-02; this is the single entry point later phases add to rather than
/// replace, so call sites written against `assert_all` keep gaining coverage automatically.
pub fn assert_all(svm: &LiteSVM, market: &Pubkey, positions: &[Pubkey]) {
    assert_inv_cus_02(svm, market, positions);
}
