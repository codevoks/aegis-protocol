# Phase 13 — Integration, Security Review and Release

**Status: NOT STARTED.** **Prerequisite: Phase 12 complete and tagged.**

> The final phase is about honesty as much as polish: making sure every claim in the repository is
> backed by something a reader can run, and that every limitation is stated rather than implied.

## Scope
1. The complete end-to-end demo (`zero-cost-demo.md` §5), scripted and reproducible offline.
2. A **self-conducted security review** of the whole protocol against the threat model and the
   invariant catalogue, written up in `docs/security/review-log.md`.
3. Operational runbooks R-1..R-5 (`governance.md` §7).
4. The final `README.md`.
5. A final pass over every document: remove stale content, replace hypotheses with measurements,
   reconcile every claim with its evidence.
6. `docs/project-status.md` finalized.
7. Optional: a devnet deployment and a hosted demo, clearly marked as optional.

## Explicit NON-scope
No new features. No new instructions. No refactoring unless the review finds a defect. No mainnet
deployment — see the acceptance criteria.

## The self-review (do this properly, not as a formality)

Work through, and record the answer to, every question in the Phase 0 final self-audit
(`docs/project-status.md` §"Phase 0 self-audit"), plus:

1. Re-read the six token-movement paths (`account-model.md` §6.3). Does the code contain a seventh?
2. Re-read every `require!` in `liquidate`. Is any reachable state unhandled?
3. For each of the 87 invariants: does its test **actually fail** when the check is removed?
4. For each of the 32 threats: is the mitigation still present after all the phases of change?
5. Is there any code path where a vault balance is read as a source of truth rather than as a delta?
6. Is there any remaining `#[cfg(feature)]` that changes on-chain behavior?
7. Does any README or doc claim exceed what the tests demonstrate?
8. Can a reader reproduce every claim in under 15 minutes on a clean clone?

Question 7 is the one to be strictest about. Overclaiming is the failure mode that most damages a
repository's credibility, and it is invisible from the inside.

## `README.md` requirements

Must contain, honestly:
- What Aegis is, in three sentences.
- The architectural thesis: isolated markets, collateral never lent, risk-first.
- Quickstart: clone → `make setup` → `make test` → `make demo`, offline.
- A results section with **real** numbers: test count, CU table, fuzz statistics, invariant count.
- Links into `docs/`.
- **A prominent, unambiguous statement that Aegis is not audited and must not be deployed with real
  capital**, with a pointer to `economic-model.md` §11 and the accepted residual risks in
  `threat-model.md` §4.

Must **not** contain: claims without evidence, "production-ready", "battle-tested", "secure",
"audited", or any feature the code does not implement.

## Acceptance criteria
- [ ] `make demo` runs the full scenario offline and prints invariant checks at every step.
- [ ] All runbooks written.
- [ ] The self-review is complete and written up, **including anything it found**.
- [ ] Every document reconciled with the implementation; no stale hypotheses remain.
- [ ] `project-status.md` shows accurate IMPLEMENTED / TESTED / DEMOED / DOCUMENTED / COMMITTED state
      for every item.
- [ ] The README's numbers match the actual test and benchmark output.
- [ ] No claim in the repository is unsupported by a runnable artifact.
- [ ] The non-deployment statement is prominent.
- [ ] A clean clone reproduces everything with no network and no secrets.
- [ ] Universal checklist satisfied. Tag `phase-13-release` and `v0.1.0`.

## Evidence
The full demo transcript; the security review document; the final test, benchmark and fuzz output; a
clean-clone reproduction log.

**This is the final planned phase. STOP and report.**
