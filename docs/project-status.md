# Aegis — Project Status

**Last updated: 2026-09-04**
**Current phase: Phase 0 — Planning & Design — COMPLETE**
**Next phase: Phase 1 — Toolchain & Repository Foundation — NOT STARTED**

> This file is the first thing any contributor or model reads after `AGENTS.md`. It must always
> reflect reality. **"Implemented" never means "verified."** The five states below are tracked
> separately and independently, on purpose.

---

## State definitions

| State | Means |
|---|---|
| **IMPLEMENTED** | The code exists and compiles. |
| **TESTED** | Tests exist, were **actually run**, and passed — and the invariant tests fail when their check is removed. |
| **DEMOED** | Exercised end-to-end in the runnable demo. |
| **DOCUMENTED** | Reflected accurately in `docs/`. |
| **COMMITTED** | Merged and tagged. |

A row may be IMPLEMENTED without being TESTED. That is normal and must be recorded honestly, never
rounded up.

---

## Phase status

| Phase | Name | Status | Tag |
|---|---|---|---|
| 0 | Planning & design | ✅ **COMPLETE** | `phase-00-planning` |
| 1 | Toolchain & repository foundation | ⬜ NOT STARTED | — |
| 2 | State, PDAs & custody primitives | ⬜ NOT STARTED | — |
| 3 | Collateral flows | ⬜ NOT STARTED | — |
| 4 | Lending, borrowing & interest | ⬜ NOT STARTED | — |
| 5 | Oracle | ⬜ NOT STARTED | — |
| 6 | Health, liquidation & bad debt | ⬜ NOT STARTED | — |
| 7 | Token-2022 | ⬜ NOT STARTED | — |
| 8 | Composability | ⬜ NOT STARTED | — |
| 9 | SDK, client & UI | ⬜ NOT STARTED | — |
| 10 | Security campaign | ⬜ NOT STARTED | — |
| 11 | Performance | ⬜ NOT STARTED | — |
| 12 | Governance & upgrades | ⬜ NOT STARTED | — |
| 13 | Integration & release | ⬜ NOT STARTED | — |

## Component status

| Component | IMPL | TEST | DEMO | DOC | COMMIT |
|---|:--:|:--:|:--:|:--:|:--:|
| `aegis-math` — arithmetic | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| `aegis-math` — shares | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| `aegis-math` — IRM/accrual | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| `aegis-math` — health | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| `aegis-math` — liquidation | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| `Protocol` / `Market` / `Position` | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| Vaults & custody | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| Token-2022 policy engine | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| Collateral instructions | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| Lend/borrow instructions | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| Oracle (Pyth adapter) | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| Liquidation & bad debt | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| Governance & migrations | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| `aegis-test-kit` | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| Invariant fuzzer | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| CU benchmarks | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| `labs/` (Anchor/native/Pinocchio) | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| TypeScript SDK | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| Web app | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| Liquidator bot | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |

**Everything is DOCUMENTED and nothing is IMPLEMENTED. That is the correct and expected state at the
end of Phase 0**, and it is the single most important thing for the next session to understand.

## Invariant status

87 invariants defined across 12 groups (9 marked **[GLOBAL]**). **0 implemented, 0 tested.**
See `docs/invariants.md` for the per-phase assignment.

---

## Environment — measured 2026-09-04

| Tool | Version | Status |
|---|---|---|
| `solana` (Agave CLI) | 2.2.21 | ❌ **STALE — Phase 1 must upgrade** |
| `rustc` / `cargo` | 1.88.0 | ⚠️ Verify against Anchor 1.x MSRV |
| `node` | v22.12.0 | ✅ |
| `anchor` / `avm` | not installed | ❌ Phase 1 |
| `surfpool` | not installed | ❌ Phase 1 |
| Git repository | not initialized | ❌ Phase 1 |

Raw output:

```
$ solana --version
solana-cli 2.2.21 (src:23e01995; feat:3073396398, client:Agave)
$ rustc --version
rustc 1.88.0 (6b00bc388 2025-06-23)
$ cargo --version
cargo 1.88.0 (873a06493 2025-05-10)
$ node --version
v22.12.0
$ anchor --version
zsh: command not found: anchor
$ avm --version
zsh: command not found: avm
```

---

## Open research gates

| ID | Question | Gate phase | Status |
|---|---|---|---|
| RV-1 | Resolved `solana-*` crate versions under `anchor-lang 1.1.2` | 1 | OPEN |
| RV-2 | Current Mollusk crate name/version and CU API | 1 | OPEN |
| RV-3 | Upgraded Pyth receiver program ID (post 2026-08-26) | 5 | OPEN |
| RV-4 | `VerificationLevel` shape in `pyth-solana-receiver-sdk` 2.x | 5 | OPEN |
| RV-5 | Complete current Token-2022 extension list and discriminants | 7 | OPEN |
| RV-6 | Does the runtime permit `A → B → A` CPI reentrancy? | 8 | OPEN |
| RV-7 | SIMD-0296 (4096-byte tx) availability and `@solana/kit` support | 9 | OPEN |
| RV-8 | Current Jupiter integration surface | 8 | OPEN |

## Known issues

None — no code exists yet.

## Deferred work

Tracked in `docs/product.md` §3 (non-goals) and `docs/economic-model.md` §11 (v1 simplifications).
Named v2 candidates: tokenized supply shares · permissionless market creation with allowlisted
parameter sets · adaptive IRM · multi-oracle median with fallback · Dutch-auction liquidation ·
cross-market vault curation layer · transfer-hook support behind a hook allowlist.

## Current architectural decisions

| ADR | Decision | Status |
|---|---|---|
| 0001 | Anchor as the production framework | Accepted |
| 0002 | LiteSVM-primary test stack | Accepted |
| 0003 | Native/Pinocchio as scoped labs, not production | Accepted |
| 0004 | Isolated two-asset markets | Accepted |
| 0005 | Collateral escrowed and never lent; explicit PDA vaults | Accepted |
| 0006 | Peer-to-pool with internal shares, not a share token | Accepted |
| 0007 | Stateless piecewise-linear IRM | Accepted |
| 0008 | Oracle abstraction; deterministic prices via fixture injection, no mock program | Accepted |
| 0009 | WAD fixed point with 256-bit `mul_div` intermediates | Accepted |
| 0010 | Zero-cost local-first architecture | Accepted |
| 0011 | `@solana/kit` as the client stack | Accepted |
| 0012 | Progressive upgrade-authority hardening | Accepted |

---

## Phase 0 self-audit

Performed before declaring Phase 0 complete. Each answer is recorded, including where it forced a
change to the design.

| Question | Answer |
|---|---|
| Is this a coherent lending protocol? | Yes. Supply, borrow, interest, liquidation, and loss absorption form a closed economic loop with a named source of liquidity and a named loss-bearer. |
| Is any feature present solely for resume coverage? | Examined each. The `labs/` Pinocchio work is coverage-motivated but justified because it benchmarks the *actual* custody primitive and quantifies Anchor's safety cost. Everything else has a product reason. AMM, perps, stablecoin, NFT, staking and flash loans were rejected outright. |
| Can the account model parallelize? | Yes, and it is the architecture's organizing constraint. Markets share no writable state; `Protocol` is read-only in every user instruction; collateral operations do not write `Market`. PERF-C1..C3 make this measurable rather than rhetorical. |
| Is shared writable state minimized? | Yes. One global account, never written by users. No counters, no registries, no aggregates. |
| Are authorities unambiguous? | Yes. Exactly one signer PDA (the `Market`), signing only for its own two vaults. |
| Could user-provided accounts redirect assets? | No. Vaults are double-validated by canonical PDA **and** stored-pubkey `has_one`. |
| Could the wrong token program be accepted? | No. The token program is pinned per asset at market creation and compared on every use. `token_interface` types alone are explicitly noted as insufficient. |
| Could Token-2022 semantics invalidate accounting? | Addressed by a positive allowlist, per-role policy (fee mints as collateral but not as loan asset), and measured-delta accounting with a mandatory post-CPI reload. |
| Could vault balances diverge from internal accounting? | INV-CUS-01/02 are exact equalities asserted after every instruction by the fuzzer. INV-CUS-08 (donations never credited) is what keeps them stable. |
| Could rounding be exploited? | 14 rounding directions specified and individually tested; `P-SHARE-1..4` assert round-trips never create value; a dedicated fuzz objective hunts for value creation. |
| What happens when oracle data is unavailable? | Fail closed for borrow, withdraw-with-debt, and liquidate. Risk-reducing operations — repay, deposit collateral, absorb bad debt, debt-free withdrawal — stay open. The trade-off is argued in `oracle-design.md` §4.1 and the residual risk is accepted explicitly. |
| What happens during extreme volatility? | `max_conf_bps` halts activity on wide confidence; conservative bounds skew every valuation against the user; the LTV/LT gap absorbs ordinary moves. |
| How does bad debt arise? | Five named mechanisms in `economic-model.md` §8.1, none hand-waved. |
| How does liquidation fail? | Unprofitability, oracle outage, frozen collateral, dust, and the death-spiral band. Each is mitigated or explicitly accepted. |
| Which admin action could cause catastrophic damage? | None involving funds — INV-ADM-01 makes it structurally impossible, and `A-ADM-02` proves it. The real catastrophic power is the **upgrade authority** (T-30), stated plainly as the largest residual risk. |
| Which assumptions would be unacceptable for real money? | Single-source oracle · illustrative rather than researched risk parameters · no supply caps · no external audit · single upgrade authority. All listed in `economic-model.md` §11 and `threat-model.md` §4. |
| Are tests capable of falsifying important invariants? | Mutation validation is a Phase 10 **acceptance criterion**: each [GLOBAL] invariant's check is removed and the fuzzer must catch it. An invariant the fuzzer cannot falsify means the fuzzer is inadequate. |
| Is every portfolio claim backed by future observable evidence? | `coverage-matrix.md` maps every topic to a specific artifact, and §4 lists what would make each claim false. |
| Could a Sonnet session execute the phases without inventing architecture? | Yes — economics, accounts, instructions, invariants, and tests are specified to formula and field level. The main residual risk is documentation outrunning implementation, which is the first row of the gap analysis. |
| Have unnecessary technologies been rejected explicitly? | Yes: Pinocchio for production (ADR-0003), a mock oracle program (ADR-0008), ATA vaults (ADR-0005), a share token (ADR-0006), a stateful IRM (ADR-0007), address lookup tables as a requirement, on-chain governance, and every non-goal in `product.md` §3. |

### Changes forced by this audit

1. **Oracle sequencing.** The original phase order would have had phases 3–4 shipping a permissive
   price path before Phase 5. Replaced with hard gating (`OracleNotYetAvailable`), so every
   intermediate state is strictly *more* restrictive than final — never less.
2. **`fee_position` made mandatory in `absorb_bad_debt`.** As an optional account, a caller could omit
   it to skip protocol first-loss and push extra loss onto lenders. Now PDA-constrained and required,
   and `create_market` initializes it so the branch cannot exist.
3. **`min_debt` dust floor added** after analyzing T-25; without it, dust positions accumulate as
   permanently unliquidatable bad debt.
4. **The liquidation bonus bound was derived rather than assumed.** Working through
   `HF' > HF ⟺ (1+b) < HF/LT` produced both the on-chain config constraint and the recognition of the
   death-spiral band, which in turn justified `full_liq_hf`.
5. **256-bit `mul_div` established as mandatory** after finding a concrete legal state
   (`shares × total_assets ≈ 3.2e44`) that overflows a naive `u128` implementation. `U-ARITH-04`
   exists specifically to pin this.

---

## Next action

**Hand Phase 1 to the implementation model. Phase 1 has NOT been started.**
