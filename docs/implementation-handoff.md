# Aegis — Implementation Handoff Instructions

**For the implementation model (e.g. Claude Sonnet) taking over from Phase 0.**

---

## How to start a phase session

Give the implementation model a prompt of this shape:

```
Implement Aegis Phase N exactly according to the frozen Phase 0 specification
and the current repository state.

Read, in order:
  1. AGENTS.md
  2. docs/project-status.md
  3. docs/phases/phase-NN-<name>.md
  4. Any ADR relevant to what you are building

Implement only Phase N. Stop when it is complete and report.
```

Nothing else should be required. If the model asks a design question the specification already
answers, the specification needs a fix — record that as a finding.

---

## What the implementation model must NOT need to decide

These are fully specified. **If a session starts redesigning any of them, stop it** — it means either
the prompt did not point at the specification, or the specification has a gap worth fixing.

| Already decided | Where |
|---|---|
| Every economic formula, unit, and rounding direction | `economic-model.md` §1–8 |
| Every account: fields, sizes, seeds, bumps, authorities, lifecycle | `account-model.md` §3–10 |
| Every instruction: accounts, writability, preconditions, transitions, events, attacks | `instruction-catalogue.md` |
| Oracle validation checks O-1..O-11 and the failure policy | `oracle-design.md` §2, §4 |
| Token-2022 policy per extension **and per role** | `token-compatibility.md` §2–4 |
| All 87 invariants and their test IDs | `invariants.md` |
| All 32 threats and their mitigations | `threat-model.md` §2 |
| Which test tier tests what | `testing-strategy.md` §1 |
| Module and crate structure | `architecture.md` §2–3 |
| Error code bands and naming | `architecture.md` §8 |
| Governance roles and their limits | `governance.md` §1–4 |
| Phase scope and non-scope | `docs/phases/` |

## Where flexibility is explicitly allowed

The implementation model **may** choose freely, without an ADR:

1. **Internal function decomposition** within a module — helper functions, parameter ordering, naming
   of private items.
2. **The 256-bit intermediate implementation** — hand-rolled two-limb `u256` or a vetted `no_std`,
   float-free crate. (Hand-rolled is recommended: ~50 lines, no dependency question.)
3. **Test file organization** — as long as every required test ID exists and is discoverable.
4. **Fuzzer generator weights and biasing heuristics**, provided the requirements in
   `phase-10-security.md` are met.
5. **UI layout, styling, and component structure** (Phase 9), provided the required data is surfaced.
6. **Liquidator bot architecture** (Phase 8), provided it works offline.
7. **CI job granularity**, provided every listed check runs and blocks.
8. **Comment wording and doc-comment style**, matching surrounding code.

Anything not on this list follows the specification. When unsure, **ask rather than choose**.

---

## Non-negotiables (repeat of `AGENTS.md`, restated because they are the ones most likely to slip)

1. **One phase per session. Stop at the end. Report. Wait.**
2. **Never weaken a check or a test to make progress.**
3. **Never claim a test passed without running it and reading the output.**
4. **Never silently drop scope.** Say what you could not do and why.
5. **Verify versions; do not remember them.** Anchor is 1.x; the TS package is `@anchor-lang/core`;
   the client is `@solana/kit`; Surfpool replaced `solana-test-validator`.
6. **`overflow-checks = true`** in the release profile. Release builds do not check overflow by default.
7. **No floating point on-chain. Ever.**
8. **Every required test runs offline with no secrets.**
9. **Record deviations as ADRs**, in the same commit as the document and test updates.
10. **Update `docs/project-status.md`** with real command output before declaring a phase complete.

---

## The five mistakes most likely to happen

Named specifically so they can be watched for:

1. **Using Anchor 0.3x patterns.** `init_if_needed`, `@coral-xyz/anchor`, manual duplicate-mutable
   -account checks, `CLOSED_ACCOUNT_DISCRIMINATOR`, `#[interface]`. All wrong under Anchor 1.x.
2. **Skipping the post-CPI `reload()`.** Everything appears to work on SPL Token and silently breaks
   accounting on a transfer-fee mint. This is T-14 and it is invisible without a Token-2022 test.
3. **Getting a rounding direction backwards.** The suite catches it only if all 14 unit tests are
   written. Do not write "most of them."
4. **Marking a phase complete with a skipped or `#[ignore]`d test.** The traceability check exists
   because this is easy to do accidentally.
5. **Adding a `Market` write to a collateral instruction.** It silently destroys the parallelism claim
   (PERF-C2). `A-PAR-01` is the only thing that catches it.

---

## Reporting format

Use the format in `CLAUDE.md` §"Reporting format at the end of a phase". Every phase report must
include real command output, the invariant IDs tested, evidence, deviations (or "none"), what was not
done (or "none"), and an explicit statement that the next phase has not been started.

---

## Escalate to the human when

- A frozen document appears wrong or unimplementable → **stop, explain, propose**.
- Two documents contradict each other → **stop, quote both, recommend**.
- A verified version contradicts `ecosystem-research.md` in a way that invalidates a *decision*
  (not merely a version number) → **stop and report**.
- A phase cannot be completed → complete everything unblocked, then report precisely what is left.
- You are about to weaken any check, bound, or test → **stop. This is never the right answer.**
