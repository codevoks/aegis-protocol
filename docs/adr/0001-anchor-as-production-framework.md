# ADR-0001 — Anchor as the production framework

**Status:** Accepted · **Date:** 2026-09-04 · **Phase:** 0

## Context

Aegis's on-chain program can be written with Anchor, with native `solana-program`, or with Pinocchio.
Research (`ecosystem-research.md` §1, §6) established that Anchor reached its first stable major
release, **v1.0.0 (2026-04-02)**, with v1.1.2 current, and that **Pinocchio is production-proven** —
Anza rewrote SPL Token in it (`p-token`, ~4,645 → ~76 CU per transfer), live on mainnet spring 2026.

So this is a real choice between two credible options, not a default.

## Decision

**Anchor 1.x for the production program.**

## Alternatives considered

**Native `solana-program`.** Rejected. It requires hand-rolling account deserialization, owner checks,
discriminator checks, and signer checks. For a lending protocol holding user funds, that is a large,
recurring security budget spent on work a framework does correctly. The topic is covered by a lab
instead (ADR-0003).

**Pinocchio.** Rejected for production. Its advantage is compute, and **compute is not Aegis's binding
constraint** — contention and correctness are (`performance-strategy.md` §1). Writing a
security-critical, account-validation-heavy protocol in `no_std` with manual everything trades a large
security budget for savings we do not need. It is genuinely valuable for hot, simple, heavily-audited
programs like a token program; Aegis is neither simple nor hot in that sense.

## Consequences

**Positive**
- Declarative constraints (`has_one`, `seeds`, `bump`, `Signer`, `Account<T>`) eliminate whole
  vulnerability classes at the type level.
- **Anchor 1.0 disallows duplicate mutable accounts by default** — removing T-11 by construction, a
  bug class that has cost real protocols real money.
- IDL generation gives the SDK typed clients for free.
- `Migration<'info, From, To>` provides a real, framework-supported schema-migration primitive
  (Phase 12), so we do not hand-roll one.
- Framework-level CPI program-address checking.

**Negative**
- CU overhead versus native/Pinocchio. **Quantified, not hand-waved:** Phase 11's `labs/` benchmark
  measures exactly what Anchor's safety costs. Claiming "Anchor is worth it" without measuring the
  price would be the kind of unsupported assertion this repository forbids.
- Anchor 1.x is recent, so most tutorials and model training data describe 0.3x semantics. Mitigated
  by the breaking-change table in `ecosystem-research.md` §1 and the trap table in `CLAUDE.md`.

**Constraints this imposes**
- `idl-build` feature required in `Cargo.toml` or IDL generation silently breaks.
- Exactly one `#[error_code]` enum.
- No direct `solana-program` dependency; use `anchor-lang`'s re-export.
- `@anchor-lang/core`, not `@coral-xyz/anchor`, on the TypeScript side.
- `dup` constraint must never be used.
