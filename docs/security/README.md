# Security Documentation

| Document | Contents | Created |
|---|---|---|
| [`../threat-model.md`](../threat-model.md) | Trust boundaries, 32 threats, accepted residual risks | Phase 0 |
| [`../invariants.md`](../invariants.md) | 87 invariants across 12 groups | Phase 0 |
| `findings.md` | Every issue found during development, with resolution | Phase 10 |
| `mutation-report.md` | Proof that each [GLOBAL] invariant's test can actually fail | Phase 10 |
| `review-log.md` | The Phase 13 self-conducted security review | Phase 13 |

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

**Phase 0.** No code exists, therefore nothing has been tested. The threat model and invariant
catalogue are complete and frozen; the campaign that validates them is Phase 10.

**Aegis is not audited and must not be deployed with real user capital.** See
[`../economic-model.md` §11](../economic-model.md) and [`../threat-model.md` §4](../threat-model.md).
