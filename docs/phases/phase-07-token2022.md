# Phase 7 — Token-2022 Completion

**Status: COMPLETE.** Tagged `phase-07-token2022`. RV-5 closed (`docs/token-compatibility.md` §0).

## Scope
1. Close RV-5: enumerate the **complete current** Token-2022 extension list (including any added
   after January 2024, e.g. `Pausable`, `ScaledUiAmount`) with discriminants, and classify every one
   into Tier A/B/C in `token-compatibility.md`.
2. Complete the policy engine against that full list, with the **positive allowlist** semantics.
3. Verify the entire protocol lifecycle on a transfer-fee collateral market.
4. Set `ImmutableOwner` on Token-2022 vaults where supported.
5. Add the fee-rate-change test (`A-TOK-11`).

## Explicit NON-scope
No transfer-hook support (rejected in v1, with reasons in `token-compatibility.md`). No confidential
transfers — ever. No transfer-fee **loan** assets. No Aegis-issued share mint.

## Files
`programs/aegis/src/token/policy.rs` (completion) · `crates/aegis-test-kit/src/mints.rs` (extension
factories) · `docs/token-compatibility.md` (updated classification table)

## Concepts demonstrated
Token-2022 TLV extension parsing · positive-allowlist security design · per-role asymmetric policy ·
measured-delta accounting under a changing fee rate · `ExtensionType::try_calculate_account_len` ·
reasoning about extension semantics rather than enumerating extension names.

## Dependencies
Phases 2 (policy engine), 3 (delta accounting), 6 (full lifecycle to test against).

## Security work
- Unknown extension discriminants rejected by default (`A-TOK-05`).
- Transfer-fee mints rejected as loan assets, accepted as collateral (`A-TOK-06`).
- `ack_freeze_authority` recorded in `market.flags` and emitted (`A-TOK-07`).
- A fee rate raised by the fee authority mid-lifecycle must not break accounting (`A-TOK-11`) — the
  test most likely to catch a real bug, because it is the case where any hardcoded fee assumption fails.

## Tests
`A-TOK-01..11` in full · `U-TOK-01..03` · `A-TOK-10` (full lifecycle — supply, borrow, liquidate, bad
debt — on a transfer-fee collateral market with INV-CUS-01/02 asserted after **every** instruction).

## Demo
Side-by-side markets: one classic SPL, one transfer-fee Token-2022 collateral. Run identical flows and
print `amount` vs `credited` at each step, showing the accounting reconciles exactly in both.
Print the rejection table with the specific reason for each unsupported extension.

## Acceptance criteria
- [x] RV-5 closed; the full current extension list classified with sources and date.
- [x] Positive allowlist verified against an unrecognized discriminant (`A-TOK-05`, Phase 2,
      re-run this phase; also re-verified against `Pausable`/`NonTransferable` this phase).
- [x] Full protocol lifecycle passes on a transfer-fee collateral market (`A-TOK-10`).
- [x] Fee-rate change mid-lifecycle handled correctly (`A-TOK-11`).
- [x] INV-CUS-02 exact on every fee-bearing operation (asserted after every instruction in
      `A-TOK-10`/`A-TOK-11`).
- [x] `token-compatibility.md` updated to reflect the verified list.
- [x] INV-CUS-05/06/07 fully tested (`docs/invariants.md` has no `INV-TOK-*` series — see
      `docs/project-status.md`'s Phase 7 invariant-status note).
- [x] Universal checklist satisfied. Tagged `phase-07-token2022`.

**STOP after this phase.**
