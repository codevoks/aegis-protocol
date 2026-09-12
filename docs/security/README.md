# Security Documentation

| Document | Contents | Created |
|---|---|---|
| [`../threat-model.md`](../threat-model.md) | Trust boundaries, 32 threats, accepted residual risks | Phase 0 |
| [`../invariants.md`](../invariants.md) | 87 invariants across 12 groups | Phase 0 |
| [`threat-traceability.md`](threat-traceability.md) | T-01..T-32 → mitigation → test → status matrix | Phase 10 |
| [`findings.md`](findings.md) | Every issue found during the Phase 10 campaign, with resolution | Phase 10 |
| [`mutation-report.md`](mutation-report.md) | Proof that each [GLOBAL] invariant's test can actually fail | Phase 10 |
| [`review-log.md`](review-log.md) | Systematic manual security review of the highest-risk code paths | Phase 10 |

## Standing rules

1. **An invariant without a falsifying test is a hope.** Every invariant maps to a test, and the
   traceability check is blocking in CI.
2. **Every negative test asserts a specific error and that no state changed.** A test asserting only
   "the transaction failed" can pass for the wrong reason.
3. **Mutation validation is an acceptance criterion, not an aspiration.** If removing a check does not
   make the fuzzer fail, the fuzzer is inadequate and must be improved.
4. **`findings.md` reporting zero findings is not credible.** It usually means the search was too
   shallow. Record bugs found and fixed.

## Current security status

**Phase 10 complete.** Every threat in the frozen catalogue (T-01..T-32) has a named,
specific-error-asserting test, verified non-vacuous. A stateful LiteSVM invariant fuzzer
(`tests/fuzz/`) exists, and all nine `[GLOBAL]` invariants have been mutation-validated against the
real on-chain artifact (`mutation-report.md`). The value-creation search (targeting T-17) found no
unexplained extraction across a 100,000+-operation extended campaign. Traceability between
`invariants.md` and the actual test suite is enforced and blocking in CI
(`scripts/check-traceability.sh`).

**Aegis is still not audited and must not be deployed with real user capital.** Adversarial
self-testing, however thorough, is not a substitute for independent review. See
[`../economic-model.md` §11](../economic-model.md), [`../threat-model.md` §4](../threat-model.md),
and `findings.md`'s own status-honesty note.
