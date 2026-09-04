# ADR-0003 — Native Solana Rust and Pinocchio as scoped labs, not production

**Status:** Accepted · **Date:** 2026-09-04 · **Phase:** 0

## Context

Low-level Solana development is a genuine and current skill: Pinocchio is `no_std`, zero-dependency,
and **production-proven** — Anza rewrote SPL Token in it (`p-token`, ~4,645 → ~76 CU per transfer),
live on mainnet since spring 2026. It would be wrong to dismiss it as a toy, and equally wrong to
adopt it for a lending protocol just to claim the topic.

## Decision

Production is Anchor (ADR-0001). Native and Pinocchio appear in **`labs/`**: the *same custody
primitive* — initialize a vault PDA, deposit, withdraw via `invoke_signed` — implemented three ways
(`vault-anchor`, `vault-native`, `vault-pinocchio`) and benchmarked against each other in
`labs/cu-bench` (Phase 11).

## Why this is coherent depth rather than resume padding

Tested against the brief's own standard:

1. **It benchmarks the actual Aegis custody primitive**, not an unrelated counter program. The numbers
   inform a real decision about a real component.
2. **It quantifies what Anchor's safety costs in CU.** ADR-0001 chose Anchor for security reasons;
   asserting "Anchor is worth it" without measuring the price would be exactly the unsupported claim
   this repository forbids. The lab turns an opinion into a measurement.
3. **It is bounded.** One primitive, three implementations, one benchmark table, one written
   conclusion. It cannot grow into a second protocol, and the phase spec fixes its scope.

The honest expected conclusion is that the trade is **not** worth making for Aegis today, because the
binding constraint is contention and correctness rather than CU. A lab that confirms a negative
result is still evidence — of judgement.

## Alternatives considered

**Write production in Pinocchio.** Rejected. Aegis is validation-heavy and security-critical; hand
-rolling every owner, discriminator, signer and PDA check in `no_std` spends a large security budget to
save compute we are not short of.

**Skip low-level coverage entirely.** Rejected. It is a real skill, it is directly relevant to
understanding what Anchor does for you, and the marginal cost here is small and fixed.

**A Pinocchio version of the whole protocol.** Rejected outright — two implementations of a lending
protocol means two places for solvency bugs to live, and it doubles the security surface for no product
gain.

## Consequences

**Positive**
- Demonstrates native account parsing, manual validation, `invoke_signed`, and `no_std` design.
- Produces a real, measured CU comparison instead of a folk claim.
- Keeps the production program's security model uniform.

**Negative**
- Extra code to maintain. Mitigated by fixing the scope narrowly and building it in Phase 11, after
  the protocol is complete and its custody primitive is settled.

**Constraint**
- `labs/` is **never** a dependency of `programs/aegis`, and is excluded from the production build.
