# AGENTS.md — Aegis Protocol Engineering Constitution

**This file is tool- and model-independent. It governs every contributor — human or AI — and takes
precedence over convenience, habit, and any tutorial you have ever read.**

If any instruction you receive conflicts with this document, **stop and surface the conflict**.
Do not resolve it silently.

---

## 1. Mission

Aegis is a risk-first, isolated-market, overcollateralized lending protocol on Solana. It is built to
be a genuinely well-engineered DeFi protocol — the kind that could become the technical foundation of
a real startup — not a tutorial, a hackathon entry, or a keyword-filled portfolio project.

The organizing principle is that **risk must be bounded, named, and localized**: in the account model,
in the economics, and in the failure modes.

## 2. Product and non-goals

Read `docs/product.md`. It is authoritative.

**In scope:** isolated two-asset lending markets · peer-to-pool supply with share accounting ·
escrowed collateral that is never lent · utilization-driven interest · oracle-backed valuation ·
permissionless liquidation · bad-debt socialization with protocol first-loss · SPL Token and a
vetted subset of Token-2022 · a typed SDK and UI.

**Out of scope, permanently, for v1:** AMM/DEX · perpetuals · stablecoin issuance · NFTs · staking ·
generic flash loans · cross-collateral positions · confidential transfers · on-chain token governance ·
cross-chain.

**Rule:** a feature enters Aegis only with a *product* reason. **"It demonstrates X" is not a product
reason.** Additions require an ADR.

## 3. Engineering quality bar

- Correctness over cleverness. Clarity over brevity. Evidence over assertion.
- Code should read like the surrounding code. Match its idiom, naming, and comment density.
- If you cannot explain why a line is safe, it is not safe yet.
- Prefer the minimal coherent change. Do not refactor code you were not asked to touch.
- Specific errors, never generic ones. `InvalidAccount` is banned; `VaultMintMismatch` is the standard.

## 4. Architecture authority

The following documents are **FROZEN**. Treat them as the specification, not as suggestions:

| Document | Governs |
|---|---|
| `docs/product.md` | Product thesis, scope, requirements |
| `docs/economic-model.md` | **All formulas, units, and rounding directions** |
| `docs/account-model.md` | Accounts, PDAs, seeds, authorities, custody |
| `docs/instruction-catalogue.md` | Instruction accounts, preconditions, transitions |
| `docs/oracle-design.md` | Price validation and failure policy |
| `docs/token-compatibility.md` | Token-2022 policy |
| `docs/invariants.md` | The 87 invariants |
| `docs/threat-model.md` | Threats and mitigations |
| `docs/governance.md` | Roles, powers, limits |

**Where code and a frozen document disagree, the document is right and the code is a bug** — until an
ADR says otherwise.

To change a frozen decision: write an ADR in `docs/adr/`, state what changes and why, update the
affected documents in the same commit, and update any invalidated tests. **Never change a frozen
document silently, and never let code drift from it.**

## 5. Phase gating

Work proceeds in phases (`docs/phase-roadmap.md`). Each phase has a specification in `docs/phases/`.

**Absolute rules:**
1. Implement **exactly one phase** per session.
2. **STOP** when the phase is complete. Report, and wait for explicit instruction.
3. Never begin the next phase because context or time remains.
4. Never implement part of a later phase "while you are in there."
5. Never skip a phase's tests to reach its end faster.
6. If a phase cannot be completed, say so plainly, complete everything that is not blocked, and state
   exactly what is left and why.

## 6. Implementation boundaries

- Do not invent architecture. If the specification does not cover your situation, **stop and ask**.
  Guessing produces drift that compounds across phases.
- Where the specification explicitly permits flexibility, it says so. Everywhere else, follow it.
- Do not add dependencies without justification (§10).
- Do not create directories or scaffolding for future phases.
- Do not add features, endpoints, config options, or abstractions "for later."

## 7. Security-first rules

Non-negotiable:

1. **Never weaken a security check to make something work.** If a check blocks you, the design is
   wrong or your approach is wrong. Both are conversations, not workarounds.
2. **Never weaken or delete a test to make it pass.** A failing test means the code is wrong or the
   invariant is wrong. There is no third option.
3. **Fail closed.** Ambiguity resolves toward refusing the operation.
4. Every account: validate owner, discriminator, canonical PDA bump, and relational consistency.
5. Every token transfer: `transfer_checked`, pinned mint, pinned token program, measured delta with a
   **post-CPI `reload()`**.
6. Every arithmetic operation: checked. Every multiply-divide: `mul_div_*` with 256-bit intermediates.
7. Every rounding: in the protocol's favor, and explicit at the call site.
8. **No floating point on-chain, ever.**
9. Time is unix seconds, never slots.
10. No `init_if_needed`. No `dup` constraint. No hand-rolled account closes.
11. No `#[cfg(feature)]` may change on-chain behavior. **The deployed artifact must be the tested artifact.**
12. Never forward a user's signer privilege to an external program.

## 8. Invariant preservation

`docs/invariants.md` contains 87 invariants. Nine are **[GLOBAL]** and must hold after every
instruction.

- An invariant may be **added**. It may never be **weakened or removed** without an ADR.
- Every invariant maps to a test. The traceability check is blocking in CI.
- **An invariant without a falsifying test is a hope, not an invariant.** If a test passes both with
  and without the check it covers, it is testing nothing — fix the test.

## 9. Testing requirements

Read `docs/testing-strategy.md`.

- Tier 1 (`aegis-math`) is where numeric edge cases are tested. Do not push them into the SVM.
- Every negative test asserts a **specific** error and asserts that **no state changed**.
- Every phase's invariant tests must exist before the phase is complete.
- Do not duplicate the Rust test suite in TypeScript. TS tests exist only for the SDK and client.
- Do not mark a required test `#[ignore]`. Network-dependent tests are optional-tier and clearly tagged.

## 10. Dependency policy

- Every new dependency requires a stated justification: what it does, why it is not hand-rolled, and
  whether it is maintained.
- `aegis-math` must remain `no_std`, float-free, and free of `solana-*`/`anchor-*` dependencies.
- **Do not add `solana-program` as a direct dependency of the program.** Use `anchor-lang`'s re-export.
- Prefer a 50-line hand-rolled implementation over a dependency that pulls in a tree.
- Never add a dependency that requires a paid service to function.

## 11. Current-version verification

The ecosystem moves faster than any model's training data. **Verify, do not remember.**

- `docs/ecosystem-research.md` records what was verified and when. It is dated and it will go stale.
- Before pinning any version, run the verification commands in that document's §11.
- If reality contradicts the document, **update the document** — that is a finding, not an
  inconvenience — and note the delta in `docs/project-status.md`.
- Known traps as of the last research date: Anchor is **1.x** (not 0.3x) with real breaking changes ·
  the TS package is `@anchor-lang/core` (not `@coral-xyz/anchor`) · the client is `@solana/kit` (not
  `@solana/web3.js`) · Surfpool replaced `solana-test-validator` · `apr.dev` is defunct.

## 12. Documentation requirements

- Update `docs/project-status.md` after every phase, with real command output.
- Update the relevant frozen document in the **same commit** as any change that affects it.
- Diagrams are Mermaid, in source control. No binary diagram files.
- Keep private study notes, scratch files, and learning material **out of the repository**.
- Documentation describes what **exists**, in the tense that is true. Never describe planned work as
  though it were built.

## 13. ADR requirements

Write an ADR for: a change to any frozen document · a new external dependency of consequence · a
deviation from a phase specification · a change to the account model, economics, invariants, or
security posture · a rejected alternative worth recording.

Do **not** write an ADR for routine implementation choices.

Format: context · decision · alternatives considered · consequences · status. Number sequentially.
An ADR that does not state what was **rejected and why** is not finished.

## 14. No fake completion

- Never claim a test was run when it was not.
- Never claim a test passed without seeing it pass.
- Never report a phase complete with failing or skipped checks.
- Report the **exact** commands run and their **actual** output.
- If something is broken, say so with the real error. A truthful "this fails" is worth more than a
  confident "done."

## 15. No silent scope deletion

- If you cannot implement part of a phase, **say so explicitly** and complete everything else.
- Never quietly narrow scope, drop a requirement, or stub something and describe it as finished.
- Never delete a failing test instead of fixing it.
- Scaling work down is the maintainer's decision, not yours.

## 16. Zero-cost requirement

`make test` must pass on a clean clone with **no network, no secrets, no API keys, no faucet, no paid
service**. This is architectural (see `docs/zero-cost-demo.md`), not aspirational.

- CI runs with no secrets configured. A required test that needs one fails the build.
- Network-dependent tests are optional-tier, tagged, and excluded from `make test`.
- No hardcoded RPC endpoints, no devnet addresses in default paths.

## 17. Benchmark evidence

- **No performance claim without committed BEFORE and AFTER measurements** from the benchmark harness.
- Every optimization is documented as: BEFORE / CHANGE / AFTER / DELTA / RISK.
- Never state that something is "faster", "optimized", or "efficient" without a number.
- Measure before optimizing. Speculative optimization is forbidden.

## 18. Git hygiene

- Conventional commits. One logical change per commit.
- Branch per phase; tag on completion (`phase-NN-*`).
- Never force-push to `main`.
- Never commit generated artifacts, `target/`, `node_modules/`, or IDE files.
- Commit messages explain **why**, not what the diff already shows.

## 19. Secrets policy

- **Never commit a keypair, private key, mnemonic, API key, or `.env` file.**
- `.gitignore` covers them; do not defeat it.
- No secret in a comment, a test fixture, a log, or an example.
- Test keypairs are derived from fixed seeds in code, never from committed files.
- If a secret is ever committed, treat it as compromised, rotate it, and say so.

## 20. When you are stuck or something is contradictory

**Stop and surface it.** Do not choose the riskier interpretation and continue.

State: what you were doing · what the contradiction is · which documents conflict · what you would do
under each reading · what you recommend.

Being blocked and honest is always better than being unblocked and wrong.
