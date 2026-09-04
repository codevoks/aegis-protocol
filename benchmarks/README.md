# Benchmarks

**Status: empty. Populated in Phase 11.**

## The rule

**No performance claim without a committed BEFORE → CHANGE → AFTER measurement.**
Never state that something is "faster", "optimized", or "efficient" without a number from this harness.

## Contents (from Phase 11)

- `cu.json` — machine-readable compute-unit measurements per instruction, per token-program variant,
  with the commit SHA and toolchain versions.
- This file — the human-readable table and the written conclusions.
- The `labs/` three-way comparison: the same custody primitive in Anchor, native `solana-program`, and
  Pinocchio, quantifying what Anchor's safety actually costs.

## Optimization format (mandatory)

```
### OPT-nn: <what changed>
BEFORE:  borrow = 78,412 CU   (commit abc1234)
CHANGE:  <precise description>
AFTER:   borrow = 61,905 CU   (commit def5678)
DELTA:   -16,507 CU (-21.0%)
RISK:    <what this could break, and which test covers it>
```

CI compares against the committed baseline; a >10% regression on any instruction fails the build.
Improvements must update the baseline in the same commit, so this file never drifts from reality.

See [`../docs/performance-strategy.md`](../docs/performance-strategy.md) for targets, planned
investigations, and the contention claims that Phase 11 must verify rather than assert.
