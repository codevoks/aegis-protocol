# Phase 11 — Performance, Compute and Contention

**Status: NOT STARTED.** **Prerequisite: Phase 10 complete and tagged.**

> **No performance claim without a committed BEFORE → CHANGE → AFTER measurement.** Measure first,
> optimize second, and only if the measurement justifies it.

## Scope
1. Build the Mollusk CU benchmark harness; commit `benchmarks/cu.json` and `benchmarks/README.md`.
2. Establish the baseline for **every** instruction, for SPL and Token-2022 variants.
3. Run investigations PERF-I1..I6 (`performance-strategy.md` §6). **PERF-I6 first** — if `liquidate`
   does not fit 200k CU with Token-2022 on both sides, that is a correctness problem (T-27), not a
   tuning problem, and it changes the design.
4. Verify contention claims PERF-C1..C3 by asserting compiled account metadata and by concurrent
   execution in Surfpool.
5. Build `labs/`: the same custody primitive in **Anchor**, **native `solana-program`**, and
   **Pinocchio**, plus `labs/cu-bench` comparing all three.
6. Add the CI regression gate (>10% on any instruction fails the build).
7. Optimize **only** where measurement justifies it, using the mandatory BEFORE/AFTER format.

## Explicit NON-scope
No protocol logic changes except those justified by a committed measurement. No speculative
micro-optimization. No rewriting the production program in Pinocchio (ADR-0003). No bit-packing
account fields (rent is ~10× cheaper after SIMD-0437 — `performance-strategy.md` §1).

## Files
`benchmarks/{cu.json, README.md}` · `tests/bench/` ·
`labs/{vault-anchor, vault-native, vault-pinocchio, cu-bench}/` · `scripts/check-cu-regression.sh`

## Concepts demonstrated
Compute-unit measurement and budgeting · account-contention analysis · Sealevel parallelism verified
rather than asserted · native Solana Rust · Pinocchio · quantifying a framework's safety cost ·
evidence-based optimization.

## The `labs/` comparison

Three implementations of the **actual Aegis custody primitive** — initialize a vault PDA, deposit,
withdraw via `invoke_signed`:

| Lab | Framework | Demonstrates |
|---|---|---|
| `vault-anchor` | Anchor 1.x | The production idiom, as the baseline |
| `vault-native` | `solana-program` | Manual account parsing, manual validation, manual serialization |
| `vault-pinocchio` | Pinocchio | `no_std`, zero-dependency, zero-copy accounts |

Deliverable: a measured CU table plus a **written conclusion** stating when the trade would be worth
making for Aegis. The honest expected answer is *not yet* — Aegis's binding constraint is contention
and correctness, not CU — and the value of the lab is that this is now a measured claim rather than an
opinion. Pinocchio is production-proven (Anza's `p-token`: ~4,645 → ~76 CU for a token transfer), so
the comparison is current and non-trivial.

## Contention verification

| ID | Method |
|---|---|
| PERF-C1 | Assert the writable sets of two different markets are disjoint; execute concurrently in Surfpool |
| PERF-C2 | Assert from compiled account metadata that `Market` is **not** writable in `deposit_collateral` / `withdraw_collateral` (`A-PAR-01`) |
| PERF-C3 | Enumerate write sets per instruction and confirm `Market` is the sole intra-market contention point for lending operations |

PERF-C2 is a **regression guard**: any future change adding a `Market` write to a collateral
instruction silently destroys the parallelism property, and only this assertion catches it.

## Acceptance criteria
- [ ] Every instruction benchmarked, SPL and Token-2022; `benchmarks/cu.json` committed.
- [ ] Every instruction is under 200k CU; `liquidate` measured in its **worst case** (Token-2022 on
      both sides) and shown to fit with margin.
- [ ] PERF-I1..I6 answered with data; PERF-I6 answered **first**.
- [ ] PERF-C1..C3 verified with tests, not assertions.
- [ ] All three `labs/` implementations work and are benchmarked; the comparison table and written
      conclusion are committed.
- [ ] The CI regression gate is active and demonstrated to fire on a deliberate regression.
- [ ] Every optimization documented in BEFORE/CHANGE/AFTER/DELTA/RISK form.
- [ ] `performance-strategy.md` updated: every `HYPOTHESIS` replaced with a `MEASURED` figure, or the
      hypothesis explicitly recorded as wrong.
- [ ] INV-RES-01..07 tested.
- [ ] Universal checklist satisfied. Tag `phase-11-performance`.

**STOP after this phase.**
