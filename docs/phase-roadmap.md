# Aegis — Phase Roadmap

**Status: FROZEN (Phase 0). Phase order and boundaries may change only via ADR.**

> **The implementation model MUST STOP after completing exactly one phase.**
> Starting the next phase without an explicit human instruction is a process violation, regardless of
> how much time or context remains.

---

## Roadmap overview

| Phase | Name | Ships | Gate |
|---|---|---|---|
| 0 | Planning & design | This `docs/` tree, `AGENTS.md`, `CLAUDE.md`, ADRs | **COMPLETE** |
| 1 | Toolchain & repository foundation | Workspace, pinned toolchain, CI, `aegis-math` skeleton, verified versions | Re-verify RV-1, RV-2 |
| 2 | State, PDAs & custody primitives | `Protocol`, `Market`, `Position`, vaults, `create_market` with token policy | |
| 3 | Collateral flows | `deposit_collateral`, `withdraw_collateral` (no-debt path), `init/close_position` | |
| 4 | Lending, borrowing & interest | `supply`, `withdraw`, `borrow`, `repay`, `accrue_interest` | |
| 5 | Oracle | `PriceSource`, Pyth adapter, fixtures, fail-closed policy | Close RV-3, RV-4 |
| 6 | Health, liquidation & bad debt | `liquidate`, `absorb_bad_debt`, `withdraw_collateral_fees` | |
| 7 | Token-2022 | Full extension policy, transfer-fee collateral markets | Close RV-5 |
| 8 | Composability | Liquidation callback + liquidator bot + optional Jupiter routing | Close RV-6, RV-8 |
| 9 | SDK, client & UI | `@aegis/sdk` on `@solana/kit`, Next.js app, full user flows | Close RV-7 |
| 10 | Security campaign | Stateful fuzzer, mutation validation, full adversarial suite | |
| 11 | Performance | CU benchmarks, contention verification, `labs/` three-way comparison | |
| 12 | Governance & upgrades | Two-step admin, guardian, pause, timelock, migrations, verifiable builds | |
| 13 | Integration, review & polish | Demo, runbooks, self-review, README | |

**Dependency note:** phases 3, 4 and 6 depend on the oracle in phase 5. Section "Sequencing the oracle
dependency" below states exactly how that is handled without shipping insecure code.

---

## Sequencing the oracle dependency (read before Phase 3)

Phases 3, 4 and 6 need prices before Phase 5 builds the oracle. Three options were considered:

1. Move the oracle to Phase 3. Rejected — it front-loads the hardest external dependency before the
   account model is proven, and RV-3/RV-4 are unresolved.
2. Ship a temporary permissive price path. **Rejected outright** — it means shipping, and testing,
   code that bypasses price validation. That is precisely the failure mode ADR-0008 exists to avoid.
3. **Adopted:** ship the *oracle-independent* subset first, and make the oracle-dependent branches
   **unreachable by construction** rather than permissive.

Concretely:

- **Phase 3** ships `withdraw_collateral` supporting **only** the zero-debt path. If
  `position.borrow_shares > 0`, it returns `OracleNotYetAvailable`. This is a hard failure, not a
  bypass: the protocol is *more* restrictive than final, never less.
- **Phase 4** ships `borrow` **gated the same way** — no borrowing at all until Phase 5. `supply`,
  `withdraw`, `repay` and `accrue_interest` need no oracle and are fully functional. This means the
  interest and share machinery is completely tested before any price enters the system.
- **Phase 5** replaces both gates with real oracle validation and enables borrowing.
- **Phase 6** builds liquidation on the now-real oracle.

Every intermediate state is a **strictly safe** protocol that simply does less. No phase ever ships a
weakened check that a later phase is supposed to strengthen — a pattern that is easy to forget and
catastrophic to forget.

---

## Per-phase specifications

Full specifications live in `docs/phases/phase-NN-*.md`. Each contains: scope, **explicit non-scope**,
files, concepts demonstrated, dependencies, security work, tests, demo, documentation updates,
acceptance criteria, required evidence, and the Git tag.

---

## Universal phase completion checklist

Applies to **every** phase. A phase is not complete until all are true:

- [ ] All acceptance criteria in the phase spec are met.
- [ ] `make test` passes **offline** on a clean clone with no secrets.
- [ ] Every invariant assigned to this phase has a test that **fails when its check is removed**.
- [ ] `cargo clippy -- -D warnings` and `cargo fmt --check` pass.
- [ ] All CI grep guards pass (`CI-NOFLOAT`, `CI-NOINITIF`, `CI-NODUP`, `CI-NOCLOSE`, `CI-NOSLOT`, `CI-NOLOOP`).
- [ ] Traceability check passes (every test ID referenced in `invariants.md` exists).
- [ ] `docs/project-status.md` updated with IMPLEMENTED / TESTED / DEMOED / DOCUMENTED / COMMITTED per item.
- [ ] Any architectural deviation is recorded as an ADR — **not** silently absorbed.
- [ ] Exact validation commands and their **real** output are pasted into the status file.
- [ ] Git tag `phase-NN-complete` created.
- [ ] **STOP.** Report completion and await explicit instruction.

---

## Git milestone discipline

| Phase | Tag | Branch convention |
|---|---|---|
| 1 | `phase-01-foundation` | `phase/01-foundation` |
| 2 | `phase-02-state` | `phase/02-state` |
| 3 | `phase-03-collateral` | `phase/03-collateral` |
| 4 | `phase-04-lending` | `phase/04-lending` |
| 5 | `phase-05-oracle` | `phase/05-oracle` |
| 6 | `phase-06-liquidation` | `phase/06-liquidation` |
| 7 | `phase-07-token2022` | `phase/07-token2022` |
| 8 | `phase-08-composability` | `phase/08-composability` |
| 9 | `phase-09-sdk-ui` | `phase/09-sdk-ui` |
| 10 | `phase-10-security` | `phase/10-security` |
| 11 | `phase-11-performance` | `phase/11-performance` |
| 12 | `phase-12-governance` | `phase/12-governance` |
| 13 | `phase-13-release` | `phase/13-release` |

Rules: conventional commits; no secrets ever; no force-push to `main`; every phase merges as a
reviewable unit; the tag is created only after the completion checklist passes.

---

## Estimated relative effort

Not calendar time — relative weight, so effort is not accidentally spent in the wrong place.

| Phase | Weight | Note |
|---|---|---|
| 1 | ▓▓ | Mostly verification and configuration |
| 2 | ▓▓▓▓ | Account model + token policy is substantial |
| 3 | ▓▓ | Small surface, high invariant density |
| 4 | ▓▓▓▓▓ | The economic core — the most correctness-critical phase |
| 5 | ▓▓▓ | External dependency risk (RV-3, RV-4) |
| 6 | ▓▓▓▓▓ | The most dangerous instruction in the protocol |
| 7 | ▓▓▓ | Policy engine + extension test matrix |
| 8 | ▓▓▓ | Optional-tier work; bounded |
| 9 | ▓▓▓▓ | SDK + UI + cross-language vectors |
| 10 | ▓▓▓▓▓ | **The phase that makes the repository credible** |
| 11 | ▓▓▓ | Measurement, not speculation |
| 12 | ▓▓▓ | Migrations need care |
| 13 | ▓▓ | Integration and honest self-review |

Phases 4, 6 and 10 carry the most weight and the most risk. If time is constrained, **cut phases 8 and
9 scope before cutting phase 10.** A protocol with a UI and no security campaign is worth less than a
protocol with a security campaign and a plain CLI demo — and this instruction exists precisely because
the opposite temptation is strong.
