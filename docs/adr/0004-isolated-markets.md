# ADR-0004 — Isolated two-asset markets, not a cross-collateral money market

**Status:** Accepted · **Date:** 2026-09-04 · **Phase:** 0
**This is the most consequential architectural decision in Aegis.**

## Context

The brief proposed an overcollateralized lending protocol. The default mental model is an Aave/Solend
style **cross-collateral money market**: a global pool of reserves, and a user account that may hold
several collateral assets and several debts simultaneously.

Critiquing that shape surfaced three problems.

1. **It is structurally hostile to Sealevel.** A user account referencing N reserves must write-lock
   the reserves it touches, serializing execution across otherwise-unrelated assets. Current Solana
   guidance explicitly recommends moving away from single global-state PDAs toward sharded,
   seed-derived PDAs to minimize write-lock contention.
2. **Its design surface is enormous** (e-mode, isolation mode, siloed borrowing, per-reserve caps,
   multi-asset health) and easy to enumerate but very hard to do well. Attempting it produces
   breadth-shaped shallowness.
3. **Risk is not contained.** Bad debt from one bad collateral asset is socialized across the whole
   protocol.

## Decision

Every Aegis market is an **isolated venue** defined by
`(collateral_mint, loan_mint, oracle_config, risk_params, config_id)`.
A position belongs to exactly one market and has exactly one collateral asset and one debt asset.

Market address = `PDA([b"market", collateral_mint, loan_mint, config_id])` — content-addressed, with
no registry and no counter.

## Alternatives considered

- **Cross-collateral money market.** Rejected — reasons above.
- **One collateral, many loan assets.** Rejected: it reintroduces multi-asset health computation and
  shared writable state for a modest UX gain.
- **Fully permissionless market creation** (Morpho Blue style). Deferred, not rejected. v1 gates
  creation on the admin because risk-first means someone must be accountable for parameters. The
  honest path to permissionless creation is allowlisted parameter sets, and that is a v2 with its own
  ADR.

## Consequences

**Positive**
- **Parallelism by construction.** Distinct markets share no writable account, so they never conflict.
  This is measurable (PERF-C1), not rhetorical.
- **Bounded solvency computation.** Health is a two-asset function with at most two oracle reads —
  bounded compute, a short fixed account list, and a minimal oracle failure surface per transaction.
- **Real risk isolation.** Bad debt provably cannot cross markets (INV-SOLV-05, `I-ISO-01`). "Risk-first"
  becomes a property of the account graph rather than an adjective in a README.
- **Oracle configuration is market-local**, so a bad oracle update cannot contaminate other markets —
  which is also why oracle config is folded into `Market` rather than shared.
- Enables several risk profiles for one asset pair via `config_id`, with no registry.

**Negative**
- **Worse capital efficiency.** A user with two collateral assets needs two positions and cannot
  cross-margin. Stated plainly in `product.md`, not hidden.
- **Liquidity fragmentation** across markets for the same asset.
- Worse UX for multi-collateral borrowers.

The mitigation for all three is the same and is deliberately *not* built in v1: a curated vault layer
that aggregates markets (the MetaMorpho pattern). It is additive on top of this architecture, which is
the test of whether the design can evolve.

**Follow-on decisions this forces**
- Collateral is per-position, not pooled (ADR-0005), which is what lets collateral operations avoid
  writing `Market`.
- `Protocol` must hold no aggregate or counter, or it would become a hot global and destroy the
  parallelism this ADR buys.
