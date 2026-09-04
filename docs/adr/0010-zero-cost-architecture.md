# ADR-0010 — Zero-cost, local-first architecture

**Status:** Accepted · **Date:** 2026-09-04 · **Phase:** 0

## Context

A demonstration protocol can be built against devnet with a hosted RPC and live Pyth feeds. That is
the path of least resistance, and it quietly makes the repository unusable as evidence.

## Decision

**Every required test, the full demo, and the UI must run with no network, no secrets, no API key, no
faucet, and no paid service.** Enforced by CI running with **no secrets configured**: a required test
that needs one fails the build.

Network-dependent work (Jupiter routing, live Pyth, devnet deploys) exists only in an **optional,
tagged tier** excluded from `make test`.

## Why this is architectural, not a convenience

It forces three properties that a devnet-dependent design would let slide:

1. **Determinism.** Prices, time and account state are set explicitly, so every test is reproducible
   and every failure is debuggable. Devnet-dependent oracle tests are flaky by construction.
2. **Adversarial reachability.** Stale prices, wide confidence, oracle outages, extreme volatility and
   bad debt are trivial to produce locally and nearly impossible to produce on demand on a public
   cluster. **The entire Phase 10 security campaign exists because of this constraint.**
3. **Reviewability.** Anyone can clone and reproduce every claim in minutes. A protocol whose evidence
   cannot be independently reproduced is an assertion, not evidence.

## How each dependency is eliminated

| Dependency | Solution |
|---|---|
| Cluster / RPC | LiteSVM in-process; Surfpool in pure local mode |
| SOL | Direct lamport writes / local funding |
| Mints | Created locally, SPL and Token-2022 with chosen extensions |
| **Oracle prices** | **Byte-exact `PriceUpdateV2` account injection (ADR-0008).** Reading a pull price is an account read, not a CPI — the Pyth program need not be deployed |
| Time | Clock warping: a year of accrual in microseconds |
| Swap liquidity | Deterministic local price for the required path; Jupiter only in the optional tier |

The oracle row is the one that usually forces projects onto a network, and ADR-0008 removes it
entirely — a security decision that happens to also be the enabling decision here.

## Consequences

**Positive**
- Fast iteration, deterministic failures, reproducible evidence.
- No cost or credential barrier for any reviewer.
- Adversarial scenarios are cheap, which is why there can be one per threat.

**Negative**
- Local tests cannot catch cluster-specific behavior (real congestion, real oracle latency, real
  liquidity). Acknowledged; the optional tier and a devnet deploy partially cover it, and
  `testing-strategy.md` §10 states what is deliberately untested.
- Fixture builders must track real account layouts. This is a feature: a format change fails loudly.

**Enforcement**
- CI runs with no secrets.
- No hardcoded RPC endpoints or devnet addresses in any default path.
- All fixture keypairs are derived from fixed seeds, never `Keypair::new()` — an unshrinkable fuzz
  failure is worthless.
- `zero-cost-demo.md` §8 lists the specific anti-patterns that erode this without anyone noticing.
