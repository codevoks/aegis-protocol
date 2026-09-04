# Phase 3 — Collateral Flows

**Status: NOT STARTED.** **Prerequisite: Phase 2 complete and tagged.**

## Scope
1. `deposit_collateral` — **no oracle, no pause, `Market` read-only**.
2. `withdraw_collateral` — **zero-debt path only**; returns `OracleNotYetAvailable` if
   `position.borrow_shares > 0` (see `phase-roadmap.md`, "Sequencing the oracle dependency").
3. `close_position`.
4. `token/transfer.rs`: `transfer_checked` helpers with **measured-delta accounting** and mandatory
   post-CPI `reload()`.
5. `invoke_signed` vault outflow using the `Market` PDA seeds.
6. `aegis-test-kit::invariants::assert_all` — the INV-CUS-02 checker.

## Explicit NON-scope
No lending, borrowing, interest, shares, oracle, or liquidation. `withdraw_collateral` must **not**
contain a permissive price path — the debt branch is a hard error until Phase 5.

## Files
`instructions/collateral/{deposit_collateral.rs, withdraw_collateral.rs}` ·
`instructions/position/close_position.rs` · `token/transfer.rs` ·
`crates/aegis-test-kit/src/invariants.rs`

## Concepts demonstrated
CPI to SPL Token and Token-2022 · `invoke_signed` with PDA seeds · `transfer_checked` · measured-delta
accounting and post-CPI account reload · safe account closure (Anchor `close =`, not the removed
`CLOSED_ACCOUNT_DISCRIMINATOR` pattern) · write-set minimization for parallelism.

## Dependencies
Phase 2 (accounts, vaults, policy).

## Security work
- Vault double-validation: canonical PDA **and** `has_one` against `market.collateral_vault`.
- Token program pinned (INV-CUS-07); mint pinned; `transfer_checked` everywhere.
- **Mandatory `reload()`** after every inbound CPI before computing the delta (T-14).
- Owner signature required for withdraw, **not** for deposit (INV-AUTH-03).
- Exact zero checks in `close_position`.
- Assert in a test that `Market` is **not** writable in either collateral instruction (INV-RES-02).

## Tests
`U-TOK-01` (SPL: `credited == amount`), `U-TOK-02` (transfer-fee: `credited == amount − fee`),
`U-WDC-01` (withdraw all with zero debt), `U-LIFE-01` (close requires exact zeros),
`A-LIFE-02` (revival attempt), `A-CUS-01` (substituted vault), `A-CUS-04` (transfer path audit),
`A-CUS-06` (wrong mint), `A-CUS-08` (**direct donation to the vault is never credited**),
`A-AUTH-02` (non-owner withdraw fails), `A-AUTH-03` (deposit by a non-owner succeeds),
`A-TOK-08/09` (token-program substitution), `A-PAR-01` (**`Market` not writable**),
`I-CUS-02` (INV-CUS-02 after every operation).

`A-CUS-08` deserves attention: it establishes early that the protocol never treats a vault balance as
a source of truth, which is what keeps INV-CUS-01/02 stable for the rest of the protocol's life.

## Demo
Deposit collateral (SPL and transfer-fee Token-2022), show `credited` differing from `amount` on the
fee mint, withdraw fully, close the position and reclaim rent, printing INV-CUS-02 at each step.

## Acceptance criteria
- [ ] `deposit_collateral` requires no oracle, no signer from the owner, and does not write `Market`.
- [ ] Measured-delta accounting is used on every inbound transfer, with `reload()`.
- [ ] `withdraw_collateral` with outstanding debt fails with `OracleNotYetAvailable` (no bypass).
- [ ] INV-CUS-02 holds exactly after every operation, including on a transfer-fee mint.
- [ ] Donated tokens are never credited.
- [ ] INV-CUS-02/05/06/07/08, INV-AUTH-02/03, INV-LIFE-02/03, INV-RES-02 tested.
- [ ] Universal checklist satisfied. Tag `phase-03-collateral`.

## Evidence
Test output; a transcript showing `amount` vs `credited` on the fee mint; the account-metadata
assertion proving `Market` is read-only.

**STOP after this phase.**
