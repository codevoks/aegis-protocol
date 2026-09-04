# ADR-0008 — Oracle abstraction, and deterministic local prices without a mock program

**Status:** Accepted · **Date:** 2026-09-04 · **Phase:** 0

## Context

Two requirements had to be met simultaneously:

1. A **deterministic local oracle** so tests are reproducible, adversarial price conditions are
   reachable, and the zero-cost requirement (NFR-4) holds.
2. A **real Pyth integration** exercising genuine production code.

The obvious reading of "support both" is an `OracleKind` enum with `Mock` and `Pyth` variants, or a
small mock-oracle program that the main program trusts.

## Decision

**Two separate decisions:**

1. **The abstraction exists** — a `PriceSource` trait returning a validated `PriceBand { lo, hi }`,
   with `require_valid_price` implementing checks O-1..O-11. In v1 it has **exactly one implementer:
   Pyth pull**. The trait exists because a second real implementer (Switchboard, or a redundant
   median composite) is a plausible v2 that must not require restructuring the health-check call sites.

2. **Determinism is achieved by test-fixture account injection, not by a code path.** The test kit
   constructs **byte-exact `PriceUpdateV2` accounts** — correct owner, discriminator, feed ID, price,
   confidence, exponent, publish time — and injects them via LiteSVM's/Surfpool's account-setting
   APIs. **There is no `Mock` variant and no mock-oracle program anywhere in the production program.**

The enabling technical fact: consuming a Pyth pull price is an **account read, not a CPI**. The Pyth
receiver program does not need to be deployed locally at all.

## Alternatives considered

**A `Mock` variant in the production enum.** Rejected. The deployed artifact would permanently contain
a code path whose only purpose is to bypass price validation, guarded by a config flag. Config flags
get set wrong, and this one would be catastrophic.

**A separate mock-oracle program, test-only.** Rejected. The production program would still have to
accept a second, weaker oracle kind and trust a program ID from configuration.

**Either of the above, on testing grounds alone.** This is the decisive argument: with a mock, **the
tests exercise the mock's deserialization path, not Pyth's**. The code that actually runs in
production would be the least-tested code in the protocol. That is exactly backwards for the most
security-critical external input in the system.

**Feature-gating the mock out of release builds.** Rejected under the repository-wide rule that no
`#[cfg(feature)]` may change on-chain behavior — the deployed artifact must be the tested artifact.

## Consequences

**Positive**
- Zero test-only code in the deployed program.
- Tests exercise the **real** Pyth deserialization and validation path, byte for byte.
- Fully deterministic and fully offline: no Hermes, no RPC, no API key, and no Pyth program deployment.
- Adversarial conditions become trivial to construct exactly: stale, wide-confidence, wrong-feed,
  partially-verified, future-dated, and absurd-value prices are all just different bytes.
- If the fixture is wrong, the tests fail — which is the correct and desirable coupling.

**Negative**
- The fixture builder must track the real account layout, so a Pyth format change breaks it. This is
  a *feature*: it fails loudly at exactly the moment we would want to know.
- Requires closing RV-3 (upgraded receiver program ID after the 2026-08-26 Pyth Core upgrade) and RV-4
  (`VerificationLevel` shape) before Phase 5 begins.

**Related policy decisions recorded here**
- **Fail closed** on any oracle validation failure, for borrow, withdraw-collateral-with-debt, and
  liquidate. The trade-off against bad-debt accumulation is argued in `oracle-design.md` §4.1.
- **Risk-reducing operations never require a price**: repay, deposit collateral, absorb bad debt,
  supply, withdraw of loan assets, and debt-free collateral withdrawal.
- **Identity is the feed ID, not the account address.** Pull-oracle accounts are ephemeral and
  permissionlessly posted; pinning an address would be wrong.
- **Oracle config is stored per-market**, not in a shared account, so a bad config change cannot
  contaminate other markets (consistent with ADR-0004).
