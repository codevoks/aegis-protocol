# Aegis — Topic Coverage Matrix and Gap Analysis

**Status: FROZEN (Phase 0). Classifications may change only via ADR.**

Classification: **PRODUCTION** (in the shipped program/SDK/app) · **LAB** (scoped `labs/` artifact) ·
**TEST** (demonstrated through the test suite) · **DOC** (architecture/ADR/security documentation) ·
**NOT COVERED** (with a stated reason).

Multiple classifications are normal and are the honest answer — a topic can be both PRODUCTION and TEST.

---

## 1. Matrix

| # | Topic | PROD | LAB | TEST | DOC | Evidence |
|---|---|:--:|:--:|:--:|:--:|---|
| 1 | Rust ownership & borrowing | ✅ | ✅ | | | Whole program; `aegis-math` `no_std`; `labs/vault-native` manual lifetimes |
| 2 | Rust traits / enums / generics | ✅ | | | ✅ | `PriceSource` trait; `AegisError`; generic `mul_div` helpers; ADR-0008 |
| 3 | Rust error handling | ✅ | | ✅ | ✅ | Single `#[error_code]` enum, banded codes; adversarial tests assert *specific* errors |
| 4 | Solana runtime & account model | ✅ | ✅ | ✅ | ✅ | `account-model.md`; all three lab implementations |
| 5 | Sealevel parallel execution | ✅ | | ✅ | ✅ | Isolated markets; `Protocol` read-only; `A-PAR-01/02`; PERF-C1..C3 |
| 6 | Signer / writable semantics | ✅ | | ✅ | ✅ | INV-AUTH-03 asymmetry; `A-AUTH-*` |
| 7 | Program ownership validation | ✅ | | ✅ | ✅ | T-02; `A-AUTH-06` |
| 8 | Allocation / rent / account lifecycle | ✅ | | ✅ | ✅ | `init_position`/`close_position`; INV-LIFE-*; SIMD-0437 rent analysis |
| 9 | PDAs | ✅ | ✅ | ✅ | ✅ | 4 PDA types; `account-model.md` §7 |
| 10 | Canonical bumps | ✅ | | ✅ | ✅ | INV-LIFE-05; `A-LIFE-03` |
| 11 | CPI | ✅ | ✅ | ✅ | ✅ | Token CPIs; Phase 8 callback; `labs/` three ways |
| 12 | `invoke_signed` | ✅ | ✅ | ✅ | ✅ | Market PDA vault signing; `labs/vault-native`, `labs/vault-pinocchio` |
| 13 | Anchor | ✅ | ✅ | ✅ | ✅ | Production framework; ADR-0001; `labs/vault-anchor` |
| 14 | Native Solana Rust | | ✅ | ✅ | ✅ | `labs/vault-native` + CU bench; ADR-0003 |
| 15 | Pinocchio | | ✅ | ✅ | ✅ | `labs/vault-pinocchio` + CU bench; ADR-0003 |
| 16 | SPL Token | ✅ | ✅ | ✅ | ✅ | Vaults; `transfer_checked`; `U-TOK-01` |
| 17 | ATAs | ✅ | | ✅ | ✅ | User-side ATAs; ADR-0005 documents *why vaults are not ATAs* |
| 18 | Token-2022 / extensions | ✅ | | ✅ | ✅ | Positive allowlist at `create_market`; `token-compatibility.md`; `A-TOK-01..11` |
| 19 | Vaults & authorities | ✅ | ✅ | ✅ | ✅ | Two vaults, one signer PDA; INV-CUS-01..09 |
| 20 | DeFi fixed-point arithmetic | ✅ | | ✅ | ✅ | WAD + 256-bit `mul_div`; `P-ARITH-*`; ADR-0009 |
| 21 | Collateralized lending | ✅ | | ✅ | ✅ | The product |
| 22 | Interest / index accounting | ✅ | | ✅ | ✅ | Share-based accrual + Taylor compounding; `P-IRM-*` |
| 23 | Health factor | ✅ | | ✅ | ✅ | `economic-model.md` §6.3; `U-HEALTH-*` |
| 24 | Liquidation | ✅ | | ✅ | ✅ | Derived bonus bound; close factor; clamp path; `P-LIQ-*` |
| 25 | Oracle architecture | ✅ | | ✅ | ✅ | `PriceSource`; O-1..O-11; fail-closed policy |
| 26 | Pyth integration | ✅ | | ✅ | ✅ | `pyth-solana-receiver-sdk` 2.x adapter; byte-exact fixtures |
| 27 | Events | ✅ | | ✅ | ✅ | One event per state transition; `events.rs` |
| 28 | Cross-program composability | ✅ | | ✅ | ✅ | Phase 8 liquidation callback; Jupiter routing (optional tier) |
| 29 | Transaction construction | ✅ | | ✅ | ✅ | SDK builders on `@solana/kit`; `I-TX-01` size verification |
| 30 | Compute budget awareness | ✅ | ✅ | ✅ | ✅ | Per-instruction targets; `benchmarks/cu.json` |
| 31 | CU optimization | | ✅ | ✅ | ✅ | Phase 11 BEFORE/AFTER protocol; `labs/cu-bench` |
| 32 | Account contention | ✅ | | ✅ | ✅ | The core architectural claim; PERF-C1..C3 |
| 33 | Security engineering | ✅ | | ✅ | ✅ | 32 threats, 87 invariants, adversarial suite |
| 34 | Property testing | | | ✅ | ✅ | `proptest` over `aegis-math` |
| 35 | Fuzzing | | | ✅ | ✅ | Stateful invariant fuzzer + mutation validation |
| 36 | Exploit regression | | | ✅ | ✅ | Every found bug frozen as a permanent named test |
| 37 | SDK / client | ✅ | | ✅ | ✅ | `@aegis/sdk` on `@solana/kit` v8 |
| 38 | Frontend integration | ✅ | | ✅ | ✅ | Next.js app; full user flows against local Surfpool |
| 39 | Upgrade / governance | ✅ | | ✅ | ✅ | Two-step admin, guardian asymmetry, timelock, `Migration<From,To>` |
| 40 | Architecture documentation | | | | ✅ | This `docs/` tree + Mermaid diagrams |
| 41 | Security documentation | | | | ✅ | `threat-model.md`, `invariants.md`, `security/` |
| 42 | Benchmark evidence | | ✅ | ✅ | ✅ | Committed `benchmarks/cu.json` + CI regression gate |
| 43 | Migrations | ✅ | | ✅ | ✅ | Phase 12; `I-UPG-01/02` |
| 44 | Observability | ✅ | | ✅ | ✅ | Events; `accrue_interest`; invariant-report tooling |

**Every listed topic is covered.** That is a claim about *planned* evidence; §4 states what would make
each claim false.

---

## 2. Explicit NOT COVERED

| Topic | Status | Reason |
|---|---|---|
| AMM / swap curve implementation | **NOT COVERED** | Non-goal. Building one to claim the topic is exactly the padding the brief forbids. Swap needs are met by *integrating* (Phase 8). |
| Perpetuals / funding rates | **NOT COVERED** | Different protocol, different risk engine. |
| Stablecoin / CDP issuance | **NOT COVERED** | Would replace the peer-to-pool thesis. |
| NFT / Metaplex | **NOT COVERED** | No product reason whatsoever. |
| Staking / LST issuance | **NOT COVERED** | No product reason. LSTs may later be collateral; issuing them is out of scope. |
| Confidential transfers | **NOT COVERED** | Irreconcilable with INV-CUS-01/02 — see `token-compatibility.md`. |
| Transfer hooks (executing them) | **NOT COVERED (v1)** | Rejected by policy: unbounded CU and arbitrary failure would make liquidation DoS-able (T-27). Analyzed in depth, which is the coverage that matters. |
| Cross-chain / bridging | **NOT COVERED** | Enormous trust surface, no product reason. |
| ZK / confidential compute | **NOT COVERED** | No product reason. |
| Firedancer-specific tuning | **NOT COVERED** | Client-specific tuning is not meaningful for a program at this scale. |
| Solana Mobile / SMS | **NOT COVERED** | Not a protocol concern. |
| On-chain governance voting | **NOT COVERED** | Governance theatre without a stakeholder set (`governance.md` §8). |
| Solana native (non-Anchor) *production* program | **NOT COVERED as PRODUCTION** | Deliberate: ADR-0001/0003. Covered as LAB with a measured CU comparison, which is the honest form of this evidence. |
| Program-derived token mints (share tokens) | **NOT COVERED (v1)** | ADR-0006; would add mint-authority custody surface for composability v1 does not need. |
| Address lookup tables | **NOT COVERED as required** | Every instruction fits a legacy transaction (INV-RES-06). Using ALTs to solve a problem we do not have would be padding. |
| Rust async / Tokio | **PARTIAL** | Present in the liquidator bot only; not a Solana-program skill. |

---

## 3. Honest self-assessment of coverage *quality*

Breadth is not evidence. Where the depth is genuinely load-bearing versus merely present:

| Topic | Depth | Note |
|---|---|---|
| Fixed-point arithmetic | **Deep** | 256-bit intermediates justified by a concrete overflow case; 14 tested rounding directions |
| Liquidation economics | **Deep** | The bonus/threshold bound is *derived*, and the death-spiral band is designed for |
| Oracle safety | **Deep** | 11 checks, fail-closed policy with a stated trade-off, risk-reducing operations kept open |
| Token-2022 | **Deep** | Per-role policy (fee mints as collateral but not as loan asset) with a real reason |
| Sealevel/contention | **Deep** | Drove the whole architecture; measurable and regression-guarded |
| Account model | **Deep** | Four PDAs, every rejected account justified |
| Pinocchio / native | **Moderate — deliberately** | A scoped, benchmarked comparison of the real custody primitive. Not production, and the matrix says so |
| Frontend | **Moderate** | Functional and complete; not a design showcase |
| Governance | **Moderate** | Real mechanisms (two-step, guardian asymmetry, tighten/loosen timelock); no on-chain voting |
| Composability | **Moderate** | One well-motivated integration, not a breadth tour |
| Fuzzing | **Deep** | Stateful, invariant-driven, with mutation validation proving the fuzzer works |

Anything marked "Moderate" is scoped that way on purpose and is labelled rather than inflated.

---

## 4. Gap analysis — what would make these claims false

Each row is a real failure mode for the repository, with the mitigation that prevents it.

| Gap | Risk | Mitigation |
|---|---|---|
| **Documentation outruns implementation** | The most likely failure: 30+ excellent planning docs and a half-built program. Worse than no plan. | Phase gating; `project-status.md` tracks IMPLEMENTED/TESTED/DEMOED separately; no phase is complete without evidence |
| **Invariants documented but not tested** | 87 invariants nobody checks | Build-enforced traceability: a missing test ID fails CI |
| **Tests that cannot fail** | Suite passes with the check removed | Mutation validation is a Phase 10 acceptance criterion |
| **Performance claims without data** | "Optimized for parallelism" with no measurement | BEFORE/AFTER protocol; committed `benchmarks/cu.json`; CI regression gate |
| **Zero-cost claim quietly broken** | A test needing an RPC creeps in | CI runs with no secrets; a network-dependent required test fails the build |
| **Token-2022 as a checkbox** | Accepting a mint without understanding it | Positive allowlist; per-role policy; 11 `A-TOK-*` tests |
| **Ecosystem research going stale** | Building on Anchor 0.31 patterns in an Anchor 1.x world | Dated research doc; Phase 1 re-verification; RV-1..RV-8 gates |
| **Scope creep** | An AMM appears "for coverage" | Non-goals list; ADR requirement; "demonstrates X" is not a product reason |
| **Sonnet redesigning during implementation** | Architectural drift across phases | Frozen specs; `CLAUDE.md` requires an ADR for deviations |
| **Economic model looking production-ready** | Overclaiming | `economic-model.md` §11 and the README state plainly that v1 must not hold real capital |
| **The lab becoming a second protocol** | Pinocchio lab expanding into a rewrite | `labs/` scope is fixed: one custody primitive, three implementations, one benchmark |
| **Phase 9 UI absorbing the schedule** | Frontend polish crowding out security work | UI is functional-only; Phase 10 is the priority phase |

---

## 5. The single most important claim

> Every capability asserted in the README is backed by a file, a test, or a benchmark that a reader can
> run offline in minutes.

If that stops being true, the repository has failed at its purpose regardless of how much of the
matrix is ticked. `project-status.md` exists to keep it true, and it separates IMPLEMENTED from
TESTED from DEMOED specifically so that "done" cannot be claimed loosely.
