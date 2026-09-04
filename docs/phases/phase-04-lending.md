# Phase 4 — Lending, Borrowing and Interest

**Status: NOT STARTED.** **Prerequisite: Phase 3 complete and tagged.**

> This is the economic core and the most correctness-critical phase. Everything here is specified in
> `economic-model.md` §1–4. **Implement it exactly, including every rounding direction.** If the code
> and the document disagree, the document wins.

## Scope
1. `aegis-math`: `shares.rs` (virtual-offset conversions), `irm.rs` (utilization, rate, `taylor3`),
   accrual (`accrue_view` / `accrue_mut` including fee-share minting).
2. `supply`, `withdraw` (loan asset).
3. `borrow` — **gated**: returns `OracleNotYetAvailable` until Phase 5. Everything except the price
   read and the LTV check is implemented and tested.
4. `repay` — **no oracle, unpausable**.
5. `accrue_interest` — standalone, permissionless.
6. Events: `Supplied`, `Withdrawn`, `Borrowed`, `Repaid`, `InterestAccrued`.

## Explicit NON-scope
No oracle, no health factor, no liquidation, no bad debt. **No permissive borrow path** — `borrow`
fails hard rather than borrowing without a price check.

## Files
`crates/aegis-math/src/{shares.rs, irm.rs}` · `state/market.rs` (accrual) ·
`instructions/lend/{supply.rs, withdraw.rs}` · `instructions/borrow/{borrow.rs, repay.rs, accrue.rs}`

## Concepts demonstrated
Share-based accounting with virtual offsets · inflation-attack mitigation · per-second compounding via
a Taylor series without floats · utilization-driven interest · protocol fees as supply shares ·
rounding discipline · lazy accrual · view-vs-mutating state computation · property-based testing of
financial invariants.

## Dependencies
Phase 3 (custody), Phase 1 (`mul_div_*`).

## Implementation notes (do not deviate)
- **Every** rounding direction from `economic-model.md` §1.3, one unit test each.
- `VIRTUAL_SHARES = 1e6`, `VIRTUAL_ASSETS = 1`. **Do not remove them** — §3.2 explains why.
- Fee shares priced against `total_supply_assets − fee_amount` (§4.3). Getting this denominator wrong
  silently gives lenders part of the protocol fee, and no obvious test catches it — `P-FEE-1` does.
- `accrue_view` must be a pure function; `accrue_mut` calls it. INV-ACC-08 (`P-ACCRUE-1`) asserts
  their equality, and it is the property that keeps Phase 3's parallelism claim honest.
- `dt == 0` is a successful no-op.
- Exactly one of `assets`/`shares` non-zero on all four instructions.

## Security work
- `withdraw` bounded by free liquidity (`total_supply_assets − total_borrow_assets`) — this **is** the
  vault-reconciliation identity, not a separate rule.
- `repay` clamped to actual debt; never pulls excess tokens.
- `repay` requires no owner signature, no oracle, and cannot be paused.
- `fee_position` PDA-constrained to `PDA(market, market.fee_recipient)`.
- `position` and `fee_position` must be distinct accounts (T-11 — Anchor 1.0 blocks duplicate mutable
  accounts by default; `A-ACC-01` verifies it).

## Tests
Unit: `U-SHARE-01/02`, `U-IRM-01..05` (including the worked example `U-IRM-03`), `U-ROUND-01..14`,
`U-WD-01`, `U-REPAY-01/02`, `U-BORROW-01/02`, `U-GUARD-01..03`.
Property: `P-SHARE-1..4`, `P-IRM-1..3`, `P-FEE-1`, `P-ACCRUE-1/2`, `P-ARITH-3`.
Adversarial: `A-SHARE-01` (**inflation attack with offsets disabled must succeed, enabled must be
unprofitable**), `A-ACC-01` (duplicate mutable accounts), `A-CUS-08`.
Integration: `I-CUS-01` (INV-CUS-01 after every operation), multi-user supply/withdraw with interest,
one year of accrual on a dormant market, 100% utilization.

## Demo
Lender supplies; borrow is attempted and correctly refused (`OracleNotYetAvailable`); time is warped
30 days with a seeded borrow position; interest accrual, utilization, borrow APY and supply APY are
printed; protocol fee shares accrue; the lender withdraws principal plus interest.

*(Note: a seeded borrow position for the accrual demo must be constructed through test-kit state
injection, not by relaxing `borrow`. Do not weaken the gate to make the demo easier.)*

## Acceptance criteria
- [ ] Every worked example in `economic-model.md` §3.3, §4.4 passes as a test with the exact numbers.
- [ ] All 14 rounding directions individually tested.
- [ ] `P-SHARE-1..4` pass — round-tripping never creates value.
- [ ] `P-FEE-1` passes — fee dilution is exactly `fee_amount`.
- [ ] `P-ACCRUE-1` passes — `accrue_view` == `accrue_mut`.
- [ ] `A-SHARE-01` demonstrates the inflation attack succeeding without offsets and failing with them.
- [ ] `borrow` is hard-gated; no code path permits borrowing without a price.
- [ ] `repay` works with no oracle and cannot be paused.
- [ ] INV-CUS-01, INV-ACC-01..09, INV-BOR-02/03/05, INV-REP-01..05 tested.
- [ ] Universal checklist satisfied. Tag `phase-04-lending`.

## Evidence
Full test output; the accrual demo transcript with APY figures; a table of the 14 rounding directions
and their tests; `A-SHARE-01` output showing both branches.

**STOP after this phase.**
