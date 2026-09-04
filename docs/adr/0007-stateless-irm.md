# ADR-0007 — Stateless piecewise-linear interest rate model

**Status:** Accepted · **Date:** 2026-09-04 · **Phase:** 0

## Context

The interest rate model determines borrow cost and lender yield. Options range from a fixed rate, to a
utilization curve, to adaptive controllers that respond to sustained utilization over time.

## Decision

A **stateless piecewise-linear curve with a kink**, parameterized per market and stored in `Market`:
`base_rate_ps`, `slope1_ps`, `slope2_ps`, `u_kink`, `max_rate_ps` — all **per-second WAD rates**.

The rate is a pure function of `(utilization, params)`. There is no IRM state account and no IRM
accrual step.

Compounding uses a **third-order Taylor expansion** of `e^x − 1`, where `x = rate × elapsed`:
`growth = x + x²/2 + x³/6`.

## Alternatives considered

**Fixed rate.** Rejected: no supply/demand mechanism, so utilization could reach 100% with no
corrective pressure and lenders could not withdraw.

**Adaptive / PID curve** (Morpho's AdaptiveCurveIRM). Rejected for v1: it requires IRM state, hence an
extra writable account or extra `Market` fields updated on every accrual, and it introduces
time-dependent behavior that is much harder to test deterministically. It is a clean v2 **behind the
same signature**, which is the point of keeping the interface pure.

**Per-second compounding by loop.** Rejected: unbounded CU, and trivially DoS-able for a dormant market.

**Linear (simple) interest.** Rejected: it under-charges materially over long intervals, and the
Taylor approach costs almost nothing more.

**Exact fixed-point `exp`.** Rejected for v1: substantially more code and CU for an error that is
already negligible and always in the borrower's favor.

## Consequences

**Positive**
- Zero IRM state: no extra account, no extra write, no cross-market coupling.
- Fully deterministic and exhaustively testable — `P-IRM-1` (monotonicity in utilization),
  `P-IRM-2` (Taylor under-approximates), `P-IRM-3` (sub-additivity).
- **`taylor3(x) ≤ e^x − 1` for `x ≥ 0`**, so the approximation always *discounts* the borrower and can
  never over-charge. The error direction is a designed property, not an accident, and is asserted
  against a high-precision reference.
- Per-second rates avoid a division in the hot path.
- Accrual is a pure function of `(state, now)`, which is what makes `accrue_view` sound — and
  `accrue_view` is what lets `withdraw_collateral` check solvency without write-locking `Market`
  (claim PERF-C2).

**Negative**
- The curve does not adapt to sustained utilization; parameters are set by the admin per market.
- Taylor error grows with `x`. Bounded: for `x ≤ 0.1` (a full day at ~3650% APR) the relative error is
  < 0.05%, and `max_rate_ps` caps `x` further. Because accrual runs on essentially every interaction,
  `x` is normally ≤ 1e-3.
- Parameters are chosen by illustration, not by quantitative risk research — recorded in
  `economic-model.md` §11 as a v1 simplification.

**Reference parameters** (SOL/USDC): base 0%, slope1 4% at an 80% kink, slope2 +100% above the kink,
max 1000% APR. At 90% utilization this yields ≈ 54% APR.
