# Phase 5 — Oracle

**Status: NOT STARTED.** **Prerequisite: Phase 4 complete and tagged.**
**Research gates RV-3 and RV-4 must be closed before writing any code in this phase.**

## Scope
1. Close RV-3 (upgraded Pyth receiver program ID after the 2026-08-26 Core upgrade) and RV-4
   (`VerificationLevel` shape) against `docs.pyth.network`. Record findings in
   `ecosystem-research.md`.
2. `oracle/mod.rs`: `PriceBand`, `PriceSource`, `require_valid_price` implementing checks O-1..O-11.
3. `oracle/pyth.rs`: the sole implementer, using `pyth-solana-receiver-sdk` 2.x.
4. `aegis-math/health.rs`: conservative valuation and health factor.
5. Remove the Phase 3/4 gates: enable `borrow` and the debt path of `withdraw_collateral`.
6. `aegis-test-kit/pyth_fixture.rs`: **byte-exact** price-update account construction.

## Explicit NON-scope
No liquidation (Phase 6). No mock oracle program and no `Mock` provider variant — ever (ADR-0008).
No Hermes client in the on-chain path.

## Files
`programs/aegis/src/oracle/{mod.rs, pyth.rs}` · `crates/aegis-math/src/health.rs` ·
`crates/aegis-test-kit/src/pyth_fixture.rs` · updates to `borrow.rs`, `withdraw_collateral.rs`

## Concepts demonstrated
Oracle security architecture · pull-oracle account model (**feed ID is the identity, not the address**) ·
confidence intervals and conservative valuation · staleness in unix seconds (never slots) ·
fail-closed design with an argued trade-off · deterministic testing by account injection rather than by
mocking.

## Dependencies
Phase 4 (accrual, share math). External: `pyth-solana-receiver-sdk` 2.x (depends on `anchor-lang ^1.0.2`,
so Anchor 1.x compatibility is confirmed — see `ecosystem-research.md` §4).

## Implementation notes
- Reading a price update is an **account read, not a CPI**. The Pyth program need not be deployed
  locally — this is what makes the whole phase testable offline.
- Anchor's `Account<'info, PriceUpdateV2>` performs the owner check (O-1) automatically; **still assert
  it explicitly** so the check survives a future refactor to a different account wrapper.
- Use `get_price_no_older_than(&Clock, max_age, &feed_id)` — it performs O-3 and O-5 together.
- Verify `verification_level == Full` (O-4) explicitly; this is easy to omit and rarely tested.
- Validate the oracle **before any state write** so INV-ORA-07 holds by construction.

## Security work
All eleven checks, each individually tested. The `MIN_PRICE_WAD`/`MAX_PRICE_WAD` sanity bounds turn an
absurd oracle value into a clean error rather than an arithmetic abort.

## Tests
`A-ORACLE-01` (**deposit_collateral succeeds with a maximally broken oracle**),
`A-ORACLE-02` (**repay succeeds likewise**), `A-ORACLE-03` (stale → borrow/withdraw fail),
`A-ORACLE-04` (zero/negative/absurd price), `A-ORACLE-05` (confidence over threshold),
`A-ORACLE-06` (fake account / wrong owner), `A-ORACLE-07` (wrong feed ID),
`A-ORACLE-08` (partial verification level), `A-ORACLE-09` (future publish time),
`A-ORACLE-10` (outage across a price move; accounting consistent on recovery),
`A-ORACLE-11` (same-transaction price timing), `A-ORACLE-12` (same account passed for both feeds),
`A-ORACLE-13` (**a failed oracle check leaves no state modified**).
Unit: `U-HEALTH-01/02` (the worked examples from `economic-model.md` §6.5).
Property: `P-VAL-1` (decimals `0..=12` × `expo −12..0`), `P-VAL-2` (monotonicity).
Boundary: age exactly at the threshold vs +1; confidence exactly at the threshold vs +1.

`A-ORACLE-01/02` are *positive* safety tests and are the ones most likely to be skipped. They assert
the property that distinguishes Aegis's oracle policy from a naive one.

## Demo
Set prices; borrow succeeds; make the price stale and show borrow failing while repay and
`deposit_collateral` still succeed; restore the price; move it and print health factors.

## Acceptance criteria
- [ ] RV-3 and RV-4 closed and recorded in `ecosystem-research.md` with sources and today's date.
- [ ] All eleven checks implemented and each individually tested.
- [ ] The fixture builder produces bytes the **real** `pyth-solana-receiver-sdk` deserializes.
- [ ] No mock oracle program and no `Mock` provider variant exists anywhere.
- [ ] Risk-reducing operations verified to work with a fully broken oracle.
- [ ] `borrow` and the debt path of `withdraw_collateral` enabled with real validation.
- [ ] Valuation correct across the full decimals × exponent matrix.
- [ ] All tests run offline with no Hermes and no Pyth program deployed.
- [ ] INV-ORA-01..07, INV-BOR-01, INV-SOLV-01 tested.
- [ ] Universal checklist satisfied. Tag `phase-05-oracle`.

## Evidence
Test output; the fixture-vs-real-SDK deserialization proof; the failure matrix for O-1..O-11;
confirmation that no network was used.

**STOP after this phase.**
