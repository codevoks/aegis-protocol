# CLAUDE.md — Operating Instructions for Claude in the Aegis Repository

**`AGENTS.md` is the authoritative engineering policy. This file defines how a Claude session should
operate inside this repository. Where they overlap, `AGENTS.md` wins.**

---

## Start of every session — read in this order

1. **`AGENTS.md`** — the engineering constitution. Non-negotiable.
2. **`docs/project-status.md`** — the current phase, what is implemented, what is verified, what is
   left, known issues, and the latest milestone.
3. **The current phase specification** in `docs/phases/phase-NN-*.md`.
4. **`docs/adr/`** — before changing anything architectural, read the ADRs. A decision that looks
   arbitrary usually has one.

Do not start editing before completing these four reads. The specifications are detailed precisely so
that you do not have to re-derive the design, and re-deriving it is how drift starts.

---

## Operating rules

### 1. Frozen documents are authoritative
`product.md`, `economic-model.md`, `account-model.md`, `instruction-catalogue.md`, `oracle-design.md`,
`token-compatibility.md`, `invariants.md`, `threat-model.md` and `governance.md` are **frozen**.

Treat them as the specification. Implement them exactly — including every rounding direction, seed
layout, and precondition. **Where code and a frozen document disagree, the document is right.**

Redesign only when the user explicitly asks you to redesign.

### 2. Never silently begin another phase
Implement exactly one phase. When it is complete, update the status file, tag it, **report, and stop**.
Remaining context is not a reason to continue. Starting Phase N+1 unasked is a process violation, not
initiative.

### 3. Never claim tests were run when they were not
Run them. Read the output. Report what actually happened.
If you did not run something, say "not run." If it failed, show the error. Never infer a pass.

### 4. Report exact validation commands and results
Paste real commands and real output into your report and into `docs/project-status.md`:

```
$ cargo test --workspace
   ... actual output ...
```

Summarized or reconstructed output is not evidence.

### 5. No large speculative refactors
Stay inside the current phase's scope. Do not reorganize modules, rename across the codebase, "clean
up" unrelated code, or upgrade dependencies that are not blocking you. If you spot something worth
changing, note it in your report and leave it.

### 6. Preserve zero-cost, offline execution
Every required test must run with no network, no secrets, no API key, no faucet. If your change
introduces a network dependency in a required path, you have broken NFR-4 — find another way or stop
and ask. See `docs/zero-cost-demo.md` §8 for the specific anti-patterns.

### 7. Never weaken a security check or a test to unblock yourself
If a check blocks you, either the design is wrong or your approach is wrong — surface it.
If a test fails, either the code is wrong or the invariant is wrong — fix the right one.
Deleting, relaxing, or `#[ignore]`-ing a test to make a phase "complete" is the most damaging thing
you can do in this repository.

### 8. Record architectural deviations as ADRs
If implementation reveals that a frozen decision is wrong or unworkable:
1. **Stop.**
2. Explain the problem and your evidence.
3. Propose the change.
4. If accepted, write the ADR, update the affected documents, and update the tests **in the same commit**.

Never absorb a deviation silently. A change that lives only in code is invisible to the next session.

### 9. Keep private learning material out of the repository
No study notes, scratch files, tutorial copies, personal TODOs, or session transcripts. Use the
scratchpad directory for working files. The repository is a public engineering artifact.

### 10. Update project status and evidence after implementation
`docs/project-status.md` must reflect reality when you finish. Track **IMPLEMENTED**, **TESTED**,
**DEMOED**, **DOCUMENTED** and **COMMITTED** separately.
**"Implemented" never means "verified."** Do not mark TESTED without having run the tests.

### 11. Prefer minimal coherent changes
The smallest change that fully satisfies the phase specification. No drive-by improvements, no
opportunistic renames, no unrequested formatting passes.

### 12. Stop and surface true contradictions
If the phase spec, a frozen document, and the code cannot all be satisfied, **stop**. State what you
were doing, which documents conflict, what each reading implies, and what you recommend. Do not pick
the riskier interpretation and proceed.

### 13. Never commit secrets
No keypairs, private keys, mnemonics, API keys, or `.env` files — not in code, tests, fixtures, logs,
comments, or examples. Test keypairs come from fixed seeds in code.

### 14. Respect Git phase discipline
Branch per phase (`phase/NN-name`), conventional commits, tag on completion (`phase-NN-*`), never
force-push `main`.

### 15. Verify versions; do not remember them
Your training data is older than this ecosystem. Before pinning or upgrading anything, run the
verification commands in `docs/ecosystem-research.md` §11.

**High-risk assumptions to actively distrust:**

| You may "remember" | Reality (verify anyway) |
|---|---|
| Anchor 0.29/0.30/0.31 | **Anchor 1.x** — breaking changes throughout |
| `@coral-xyz/anchor` | **`@anchor-lang/core`** |
| `@solana/web3.js` | **`@solana/kit`** |
| `solana-test-validator` | **Surfpool** |
| `init_if_needed` is fine | Banned here (INV-LIFE-01) |
| Duplicate mutable accounts must be checked manually | Anchor 1.0 blocks them by default; never use `dup` |
| `CLOSED_ACCOUNT_DISCRIMINATOR` | Removed in Anchor 1.0 |
| Slot-based staleness | Unsafe — slot times are changing (SIMD-0525). Use unix seconds |
| Release builds check overflow | **They do not.** `overflow-checks = true` is mandatory |

When a tutorial pattern and this repository's documents disagree, the documents win.

---

## Reporting format at the end of a phase

```
PHASE N — <name> — COMPLETE

IMPLEMENTED
  <what was built>

VALIDATED
  $ <command>
  <actual output>

INVARIANTS TESTED
  <IDs, and confirmation each fails when its check is removed>

EVIDENCE
  <files, transcripts, benchmark numbers>

DEVIATIONS
  <ADRs written, or "none">

NOT DONE / KNOWN ISSUES
  <explicit, or "none">

NEXT
  Phase N+1 has NOT been started.
```

If anything is incomplete, say so here plainly. An honest partial report is more useful than a
confident complete-sounding one.

---

## Things that are always wrong in this repository

- Starting the next phase unasked.
- Claiming a passing test you did not run.
- Weakening a check, a bound, or a test to make progress.
- Adding a feature because it "demonstrates" a skill.
- Introducing a network or paid dependency into a required path.
- Editing a frozen document without an ADR.
- Marking something TESTED that was only IMPLEMENTED.
- Committing a secret.
- Using floating point on-chain.
- Reporting "done" when part of the scope was quietly dropped.
