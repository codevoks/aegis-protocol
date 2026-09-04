# Phase 10 — Adversarial, Property and Fuzz Security Campaign

**Status: NOT STARTED.** **Prerequisite: Phase 9 complete and tagged.**

> **This is the phase that makes the repository credible.** If schedule pressure forces a cut, cut
> Phase 8 and Phase 9 scope before cutting this. A lending protocol with a UI and no security campaign
> is worth less than one with a security campaign and a CLI demo.

## Scope
1. **Complete the adversarial suite**: one named test per threat `T-01`…`T-32`, each asserting a
   **specific** error, and each verified to fail if its mitigation is removed.
2. **Build the stateful invariant fuzzer** (`testing-strategy.md` §5) over LiteSVM.
3. **Mutation validation**: for each of the nine **[GLOBAL]** invariants, remove its enforcement in a
   scratch branch and confirm the fuzzer finds a violation within a bounded budget.
4. **Value-creation search**: a fuzz objective hunting for any sequence where an actor's assets-out
   exceed assets-in (targets T-17).
5. **Traceability enforcement**: a CI check parsing `docs/invariants.md` and asserting every referenced
   test ID exists.
6. **Exploit-regression suite**: every bug found is frozen as a permanent named test citing its threat ID.
7. `docs/security/` — findings, the review log, and the mutation-validation report.

## Explicit NON-scope
No new protocol features. No performance work (Phase 11). No refactoring beyond what a discovered bug
requires. **No weakening a test to make it pass** — if a test fails, either the code is wrong or the
invariant is wrong; a third option does not exist.

## Files
`tests/adversarial/*.rs` · `tests/fuzz/` · `crates/aegis-test-kit/src/invariants.rs` (completed) ·
`scripts/check-traceability.sh` · `docs/security/{findings.md, mutation-report.md, review-log.md}`

## Concepts demonstrated
Adversarial testing · stateful property-based fuzzing · invariant-driven verification · mutation
testing to validate that tests can actually fail · exploit-regression discipline · security
documentation.

## Fuzzer requirements (do not simplify these)

- Operations include `warp_time` and `move_price` as first-class actions. A fuzzer that cannot advance
  time or move prices will only ever find shallow bugs, because almost every interesting state in a
  lending protocol requires one or both.
- Amount sampling is **biased**, not uniform: `0`, `1`, dust, `min_debt ± 1`, near-max, exact-balance,
  and values chosen to land near `HF = 1`. Uniform random amounts mostly bounce off preconditions and
  waste the budget.
- Invariants are asserted after **every** operation, whether it succeeded or failed.
- All keypairs and the RNG are seeded; every failure must be reproducible and shrinkable from its seed.
- Multiple users and multiple markets in one run, so cross-market isolation is exercised.

## Mutation validation (the acceptance gate that matters)

For each **[GLOBAL]** invariant, delete its enforcement and confirm the fuzzer catches it:

| Invariant | Mutation | Expected |
|---|---|---|
| INV-CUS-01 | Skip the free-liquidity check in `withdraw` | Violation found |
| INV-CUS-02 | Credit `amount` instead of the measured delta | Violation found (fee mint) |
| INV-ACC-01 | Skip `total_supply_shares` update in `supply` | Violation found |
| INV-ACC-02 | Skip `total_borrow_shares` update in `borrow` | Violation found |
| INV-ACC-03 | Remove the borrow liquidity check | Violation found |
| INV-ACC-06 | Allow shares without assets | Violation found |
| INV-SOLV-01 | Skip the post-borrow LTV check | Violation found |
| INV-SOLV-04 | Update only one total in `absorb_bad_debt` | Violation found |
| INV-ACC-04 | Add interest to borrows but not supplies | Violation found |

**If a mutation is not caught, the fuzzer is inadequate and must be improved before the phase is
complete.** Record every result — including any that required improving the fuzzer — in
`docs/security/mutation-report.md`. An invariant the fuzzer cannot falsify is not being tested.

## Acceptance criteria
- [ ] Every threat T-01..T-32 has a named test asserting a **specific** error (not merely "failed").
- [ ] Each adversarial test verified to fail when its mitigation is removed.
- [ ] The stateful fuzzer runs with time warping, price movement, biased sampling, multi-user,
      multi-market, seeded and reproducible.
- [ ] All nine mutations are caught; the report is committed.
- [ ] The value-creation search finds nothing over an extended campaign.
- [ ] The traceability check passes and is blocking in CI.
- [ ] Every bug found is a permanent regression test naming its threat ID.
- [ ] `docs/security/findings.md` documents everything found, **including bugs found and fixed** —
      a security document reporting zero findings is not credible and usually means the search was
      too shallow.
- [ ] Universal checklist satisfied. Tag `phase-10-security`.

## Evidence
Full adversarial output; fuzz campaign statistics (operations executed, states explored, seeds); the
mutation report; the findings document; the traceability report.

**STOP after this phase.**
