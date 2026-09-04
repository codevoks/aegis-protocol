# Phase 6 — Health, Liquidation and Bad Debt

**Status: NOT STARTED.** **Prerequisite: Phase 5 complete and tagged.**

> `liquidate` is the most dangerous instruction in Aegis. Implement `economic-model.md` §7–8 exactly,
> including the collateral-clamp path and the strict `HF < WAD` comparison.

## Scope
1. `aegis-math/liquidation.rs`: `max_repay` (close factor + dust rule), seizure, bonus, protocol cut,
   collateral clamp with upward-rounded repay recomputation.
2. `liquidate`.
3. `absorb_bad_debt` — protocol fee shares burned first, then socialization.
4. `withdraw_collateral_fees`.
5. Events: `Liquidated`, `BadDebtAbsorbed`, `CollateralFeesWithdrawn`.

## Explicit NON-scope
No liquidation callback / flash liquidation (Phase 8). No Jupiter routing. No Dutch auction. No
liquidator bot (Phase 8). No admin-forced liquidation — ever.

## Files
`crates/aegis-math/src/liquidation.rs` ·
`instructions/liquidate/{liquidate.rs, absorb_bad_debt.rs}` ·
`instructions/admin/withdraw_collateral_fees.rs`

## Concepts demonstrated
Liquidation mechanics and incentive design · derived parameter constraints (`LT·(1+b) < WAD`) ·
close factors and dust floors · bad-debt socialization with protocol first-loss · risk isolation
between markets · permissionless keeper incentives · rounding in an adversarial setting.

## Dependencies
Phase 5 (oracle, health), Phase 4 (shares, accrual).

## Implementation notes (do not deviate)
- **`HF < WAD` is strict.** `HF == WAD` is not liquidatable (E-12).
- Close factor → 100% when `HF < full_liq_hf` **or** when a partial repayment would leave dust debt.
- The collateral clamp recomputes `repay_assets` **rounded up** — the liquidator pays more, never less.
- `protocol_cut` is taken from the **bonus only**, never from the principal-equivalent seizure.
- `protocol_cut` stays physically in the collateral vault and is recorded in `collateral_fee_accrued`.
- `absorb_bad_debt` requires `collateral_amount == 0` **exactly**, requires no oracle, and cannot be
  paused. `fee_position` is **required**, not optional — making it optional would let a caller push
  extra loss onto lenders.
- Both totals fall by `bad_assets` so no tokens move and INV-CUS-01 is preserved.

## Security work
- Strict health comparison; conservative prices on both sides.
- Seizure never exceeds position collateral.
- Repayment never exceeds outstanding debt.
- `A-ADM-02`: attempt to withdraw user collateral via `withdraw_collateral_fees` — **must fail**. This
  is the concrete proof of INV-ADM-01 and the strongest single argument that Aegis is non-custodial.
- Cross-market isolation asserted (`I-ISO-01`).
- `position` and `fee_position` distinct (T-11).

## Tests
Unit: `U-LIQ-01` (the worked example from `economic-model.md` §7.5 with exact figures),
`U-LIQ-02` (`HF == WAD` not liquidatable), `U-LIQ-03` (clamp path), `U-LIQ-04` (dust rule),
`U-LIQ-05` (full seizure with debt remaining), `U-LIQ-06` (`total_supply_assets` never rises),
`U-LIQ-07` (self-liquidation permitted and unprofitable), `U-BD-01/02`.
Property: `P-LIQ-1` (**HF improves when `HF > LT(1+b)`**), `P-LIQ-2`, `P-LIQ-3` (profitability),
`P-LIQ-4`, `P-BADDEBT-1`.
Adversarial: `A-LIQ-01` (healthy position), `A-ADM-02`, `A-PAR-02` (no shared writable state across
markets), `A-ORACLE-*` re-run against `liquidate`.
Integration: full lifecycle to bad debt; `I-ISO-01` (bad debt in market A leaves market B untouched);
INV-CUS-01/02 asserted after every step.

## Demo
The full sequence from `zero-cost-demo.md` §5, steps 11–15: price drop → liquidation with the bonus and
protocol cut printed → crash to zero collateral → `absorb_bad_debt` showing protocol fee shares
absorbing first → a lender withdrawing and realizing the socialized loss.

## Acceptance criteria
- [ ] `U-LIQ-01` matches the documented worked example exactly.
- [ ] `P-LIQ-1` passes — the derived health-improvement property holds.
- [ ] `P-LIQ-3` passes — liquidation is profitable whenever `HF < WAD` and the clamp is not hit.
- [ ] Clamp path never seizes more collateral than exists.
- [ ] `absorb_bad_debt` requires zero collateral, no oracle, and cannot be paused.
- [ ] Protocol fee shares are burned before any lender loss.
- [ ] INV-CUS-01 holds exactly through bad-debt absorption.
- [ ] `A-ADM-02` proves the admin cannot withdraw user collateral.
- [ ] `I-ISO-01` proves cross-market isolation.
- [ ] INV-LIQ-01..09, INV-SOLV-02..07, INV-ADM-01/08, INV-CUS-09, INV-RES-03 tested.
- [ ] Universal checklist satisfied. Tag `phase-06-liquidation`.

## Evidence
Test output; the liquidation demo transcript with the exact figures from §7.5; the bad-debt transcript
showing first-loss ordering; the isolation test output.

**STOP after this phase.**
