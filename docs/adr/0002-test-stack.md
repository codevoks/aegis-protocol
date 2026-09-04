# ADR-0002 — LiteSVM-primary test stack

**Status:** Accepted · **Date:** 2026-09-04 · **Phase:** 0

## Context

Research (`ecosystem-research.md` §5) established the current landscape: **LiteSVM 0.16.0** (in-process
SVM, ~10× faster per test than a validator, now the default `anchor init` template), **Surfpool 1.5.0**
(drop-in `solana-test-validator` replacement, Anchor 1.0's default for `anchor test`/`anchor localnet`,
with just-in-time mainnet account fetching), and **Mollusk** (isolated single-instruction execution and
CU measurement). `solana-test-validator` is superseded for our purposes.

## Decision

A five-tier pyramid where **each tier exists only because it tests something no other tier can**
(`testing-strategy.md` §1):

1. `aegis-math` unit + property tests — no SVM.
2. Mollusk — isolated instructions and **CU measurement**.
3. **LiteSVM — the primary harness** for integration and all adversarial tests.
4. A hand-built stateful invariant fuzzer over LiteSVM.
5. Surfpool in **pure local mode** — JSON-RPC surface, SDK e2e, transaction-size verification.

## Alternatives considered

**TypeScript tests as the primary harness** (the traditional Anchor default). Rejected: an order of
magnitude slower, and it tests program logic through a client layer that adds no signal. TS tests
exist only at tier 5, for the SDK and client — which Rust genuinely cannot test.

**Duplicating the Rust suite in TypeScript.** Rejected explicitly. It doubles maintenance and buys
nothing.

**An off-the-shelf stateful fuzzer (e.g. Trident).** Rejected in favor of a hand-built one:
(a) it avoids a compatibility dependency on a third-party tool tracking a very recent Anchor 1.x;
(b) we need a **biased** operation generator that targets interesting states (near `HF = 1`, dust
amounts, extreme utilization) rather than uniform randomness that mostly bounces off preconditions;
(c) `warp_time` and `move_price` must be first-class operations, since almost every interesting bug in
a lending protocol requires one or both.

**Surfpool mainnet-fork as a required tier.** Rejected — it needs an RPC endpoint, which would break
NFR-4. It is used only in the optional, network-tagged tier.

## Consequences

**Positive**
- The full core suite runs offline, deterministically, in seconds.
- Numeric edge cases are tested at millions-of-cases scale in tier 1, where they are cheap.
- Account injection in LiteSVM is what makes ADR-0008's fixture approach possible.
- Precise CU numbers come from Mollusk, where noise is lowest.

**Negative**
- Most of the test suite is in Rust, so contributors need Rust to work on tests.
- The hand-built fuzzer is more work than adopting one — offset by the control it buys, and by the
  fuzzer itself being meaningful evidence of understanding the protocol's failure modes.

**Enforcement**
- Traceability check: every test ID referenced in `invariants.md` must exist, or CI fails.
- Mutation validation (Phase 10): each [GLOBAL] invariant's check is removed and the fuzzer must catch
  it. An invariant the fuzzer cannot falsify means the fuzzer is inadequate.
