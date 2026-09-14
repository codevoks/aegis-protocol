# Phase 13 Release Evidence

Concise, deterministic evidence artifacts for the `v0.1.0` / `phase-13-release` tag. Everything
here is reproducible from a clean clone — see [`docs/project-status.md`](../project-status.md)'s
Phase 13 section for the exact commands each artifact came from, and
[`docs/security/review-log.md`](../security/review-log.md) for the security review this evidence
supports.

| File | What it is | Reproduce with |
|---|---|---|
| `phase-13-demo-transcript.txt` | The full, real output of the mandatory offline demo (`docs/zero-cost-demo.md` §5, all 16 steps) | `make demo` |
| `extended-fuzz-campaign-result.txt` | The final summary line of the 100,000-operation extended fuzz campaign | `make fuzz` |
| `mutation-gate-spot-check.txt` | Three of the nine `[GLOBAL]` invariant mutation gates, freshly re-run end to end against the final codebase | see `docs/project-status.md` Phase 13 §8 for the exact edit/build/probe/revert commands |
| `release-regression.txt` | The full release-candidate regression run (fmt, clippy, guards, `cargo test --workspace`, CU regression, traceability) | `make test && ./scripts/check-*.sh` |

Not included, deliberately: the full 100,000-operation fuzz log (large and not more informative than
its own summary line — every operation's detail is reproducible on demand from the fixed seeds) and
the full `cargo test --workspace` transcript (verbose LiteSVM/Anchor debug logs; the pass/fail
summary in `release-regression.txt` is the load-bearing evidence, and the exact command reproduces
the rest).
