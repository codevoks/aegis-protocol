//! Value-creation tracking (`docs/phases/phase-10-security.md` #4, targets **T-17**).
//!
//! **What counts as inflow/outflow/legitimate gain (the conservation model, documented plainly so
//! this checker's claims are honest rather than inflated):**
//!
//! | Flow | Counted as |
//! |---|---|
//! | `supply` (loan assets in) | inflow, that actor, loan side |
//! | `repay` (loan assets in) | inflow, that actor, loan side |
//! | `withdraw` (loan assets out) | outflow, that actor, loan side |
//! | `borrow` (loan assets out) | outflow, that actor, loan side |
//! | `deposit_collateral` (collateral in) | inflow, that actor, collateral side |
//! | `withdraw_collateral` (collateral out) | outflow, that actor, collateral side |
//! | `liquidate` repayment (loan assets in) | inflow, the LIQUIDATOR, loan side (generous: credits
//! |   them for a payment that discharges someone else's debt, never used to justify their own
//! |   extraction beyond this model's bound) |
//! | `liquidate` seizure (collateral out) | tracked separately as `liquidation_collateral_received`,
//! |   never compared 1:1 against the liquidator's own `deposit_collateral` — it is a reward paid
//! |   by the liquidated position's collateral, not the liquidator's own round-trip |
//! | interest accrued in a market | tracked as `market_total_interest_accrued`, the sole source of
//! |   legitimate loan-side yield this model accounts for |
//! | protocol fee, direct donations | excluded entirely — never credited to any actor by design
//! |   (INV-CUS-08), so they cannot appear as an actor's inflow or outflow here |
//!
//! **Two checks, of different strength:**
//! 1. **Collateral side — exact, always-on.** Collateral never earns yield in this protocol, so
//!    `cumulative_collateral_out(actor) <= cumulative_collateral_in(actor)` must hold for EVERY
//!    actor, at EVERY step, with no exception. This is a tight bound: any violation is a genuine
//!    bug (a rounding exploit, a double-credit, or a custody accounting break).
//! 2. **Loan side — conservative upper bound.** Lenders legitimately earn interest, so
//!    `cumulative_loan_out(actor) <= cumulative_loan_in(actor) + market_total_interest_accrued` is
//!    the check. This bound is deliberately generous — it does not attempt to apportion interest
//!    fairly between lenders (that would require reasoning about time-weighted share of the pool,
//!    which `P-SHARE-1..4` already covers exactly, for an isolated actor, in `aegis-math`'s own
//!    property-test tier). The fuzzer's job is different and complementary: catching a GROSS,
//!    whole-protocol conservation break across arbitrary multi-actor, multi-step, multi-market
//!    interleavings that no isolated single-actor property test would think to construct. A
//!    violation of this looser bound is still definitely a bug; the converse is not claimed.

#[derive(Default, Clone, Copy, Debug)]
pub struct ActorMarketLedger {
    pub loan_in: u128,
    pub loan_out: u128,
    pub collateral_in: u128,
    pub collateral_out: u128,
    pub liquidation_collateral_received: u128,
}

#[derive(Default)]
pub struct Ledger {
    pub per_actor: Vec<ActorMarketLedger>,
    pub market_total_interest_accrued: u128,
}

impl Ledger {
    pub fn ensure(&mut self, num_actors: usize) {
        if self.per_actor.len() < num_actors {
            self.per_actor
                .resize(num_actors, ActorMarketLedger::default());
        }
    }

    pub fn record_loan_in(&mut self, actor: usize, amount: u64) {
        self.per_actor[actor].loan_in += amount as u128;
    }
    pub fn record_loan_out(&mut self, actor: usize, amount: u64) {
        self.per_actor[actor].loan_out += amount as u128;
    }
    pub fn record_collateral_in(&mut self, actor: usize, amount: u64) {
        self.per_actor[actor].collateral_in += amount as u128;
    }
    pub fn record_collateral_out(&mut self, actor: usize, amount: u64) {
        self.per_actor[actor].collateral_out += amount as u128;
    }
    pub fn record_liquidation_collateral_received(&mut self, actor: usize, amount: u64) {
        self.per_actor[actor].liquidation_collateral_received += amount as u128;
    }
    pub fn record_interest(&mut self, amount: u64) {
        self.market_total_interest_accrued += amount as u128;
    }

    /// Panics with a diagnostic naming the actor and the exact figures on any violation.
    pub fn assert_no_value_creation(&self, actor_names: &[&str], market_label: &str) {
        for (idx, l) in self.per_actor.iter().enumerate() {
            assert!(
                l.collateral_out <= l.collateral_in,
                "T-17 VALUE CREATION (collateral, exact bound) in market {market_label}: actor {} \
                 withdrew {} collateral base units but only ever deposited {}",
                actor_names.get(idx).unwrap_or(&"?"),
                l.collateral_out,
                l.collateral_in,
            );
            let bound = l.loan_in + self.market_total_interest_accrued;
            assert!(
                l.loan_out <= bound,
                "T-17 VALUE CREATION (loan, conservative bound) in market {market_label}: actor {} \
                 extracted {} loan base units but only ever contributed {} plus the market's ENTIRE \
                 lifetime accrued interest of {} (bound {bound})",
                actor_names.get(idx).unwrap_or(&"?"),
                l.loan_out,
                l.loan_in,
                self.market_total_interest_accrued,
            );
        }
    }
}
