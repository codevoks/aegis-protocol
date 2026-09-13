# Aegis — Performance, Compute and Contention Strategy

**Status: Phase 0 hypotheses now MEASURED (Phase 11, 2026-09-13).** Every number below is either a
real figure from `benchmarks/cu.json` / `tests/bench/`, or explicitly marked `HYPOTHESIS DISPROVED`
with the measurement that disproved it. See `benchmarks/README.md` for the full table and
methodology.

> **Rule: no performance claim without a committed BEFORE → CHANGE → AFTER measurement.**
> Every number in this document that is not marked `MEASURED` is a hypothesis.

---

## 1. Constraints that actually bind

Identified before optimizing, so effort goes where it matters:

| Constraint | Binds Aegis? | Assessment |
|---|---|---|
| **Writable account contention** | **Yes — primary** | `Market` is written by supply/withdraw/borrow/repay/liquidate. This is the throughput ceiling per market and the one thing architecture can address. |
| **Transaction account count / size** | **Yes — secondary** | `liquidate` needs 14 accounts + program. Legacy transactions are 1232 bytes; SIMD-0296 raises this to 4096 but **must not be assumed**. |
| Compute units | Moderate | Every instruction should sit far under 200k. The risk is not the average case but an instruction that *cannot* fit — for `liquidate` that would mean guaranteed bad debt (T-27). |
| CPI count | Low | At most 2 token CPIs per instruction. |
| Account size / rent | **Low — deliberately** | Agave 4.2's SIMD-0437 cut `lamports_per_byte` by ~90%. A 641-byte `Market` is now cheap. |
| Serialization | Low | Fixed-size accounts, no `Vec`, no realloc. |
| PDA derivation | Low | `find_program_address` is ~1.5k CU per attempt; avoided in the hot path by passing stored bumps. |

**Explicit non-optimization:** account fields are kept explicit and readable (`u128` for WAD
parameters, `_reserved` padding) rather than bit-packed. Under the reduced rent this costs a trivial
amount of SOL and buys clarity and migration headroom. Recorded here so it reads as a decision rather
than an oversight.

---

## 2. Contention design (the primary work, done in Phase 0)

The most important performance work is architectural and is already done — it lives in the account
model, not in a later optimization pass.

| Decision | Contention effect |
|---|---|
| **Isolated markets** (ADR-0004) | No writable account is shared across markets → unlimited cross-market parallelism. |
| **`Protocol` is read-only in every user instruction** | A single global writable account would serialize the *entire protocol*. Aegis has no global counter, no registry, no aggregate. |
| **Collateral is per-position, not pooled** (ADR-0005) | `deposit_collateral` / `withdraw_collateral` do not write `Market`. |
| **`accrue_view` for solvency checks** | Lets `withdraw_collateral` check health against fully-accrued debt without taking the `Market` write lock. |
| **Fees as supply shares / a `Market` scalar** | No shared fee account, so no extra contention point in `liquidate`. |
| **Stateless IRM** | No IRM state account to write. |

### Measurable claims (Phase 11) — MEASURED

| ID | Claim | Method | Result |
|---|---|---|---|
| PERF-C1 | Transactions in different markets never conflict | Assert disjoint writable sets across two markets (`tests/bench/contention.rs::perf_c1_disjoint_markets_static`); execute concurrently in Surfpool (`bots/liquidator`'s `perf-c1` demo) | **CONFIRMED.** Two markets' `supply` instructions (two different users) share zero writable accounts. Live Surfpool run: two `deposit_collateral` transactions on two different markets, submitted concurrently via `Promise.all` with neither awaited before the other, both confirmed successfully. |
| PERF-C2 | `deposit_collateral`/`withdraw_collateral` never write `Market` | Inspect the compiled account metas | **CONFIRMED** — already covered by `A-PAR-01` (`tests/phase3_adversarial.rs::market_is_not_writable_in_collateral_instructions`, Phase 3), re-asserted as part of the full write-set table in `tests/bench/contention.rs::perf_c3_write_set_enumeration` so both checks are derived from one consistent enumeration rather than two lists that could drift. |
| PERF-C3 | `Market` is the only intra-market contention point for lending operations | Write-set enumeration test (`tests/bench/contention.rs::perf_c3_write_set_enumeration`) | **CONFIRMED.** Across every instruction that writes `Market` (`supply`, `withdraw`, `borrow`, `repay`, `accrue_interest`, `liquidate`, `absorb_bad_debt`, `withdraw_collateral_fees`), the only other writable non-vault, non-user-ATA account is the per-user `Position` (or `fee_position`, itself just another `Position`). `Protocol` is read-only or entirely absent in every one of the 13 current production instructions (there is no admin `set_*` instruction yet — Phase 12). |

PERF-C2 is a **regression risk, not just a claim**: any future change that adds a `Market` write to a
collateral instruction silently destroys it. `A-PAR-01` asserts the account metadata directly, so the
property is guarded by a test rather than by memory.

---

## 3. Compute budget — MEASURED (worst-case scenario per instruction, `benchmarks/cu.json`)

| Instruction | Target (Phase 0) | **Measured (AFTER OPT-01)** | Verdict |
|---|---|---:|---|
| `init_position` | < 15k | **11,481** | met |
| `deposit_collateral` | < 30k | **15,692** (Token-2022) | met |
| `withdraw_collateral` (no debt) | < 30k | **15,953** (Token-2022) | met |
| `withdraw_collateral` (with debt) | < 60k | **39,189** (SPL) | met |
| `supply` / `withdraw` | < 50k | **25,387** / **25,037** (Token-2022, with real prior accrual) | met |
| `borrow` | < 75k | **48,360** (Token-2022) | met |
| `repay` | < 50k | **26,003** (Token-2022, dt=30d) | met |
| `accrue_interest` | < 25k | **12,118** (dt=30d) | met |
| `liquidate` | **< 110k** | **109,687** (Token-2022 both sides, clamped, worst case) | **met, by 313 CU** |
| `absorb_bad_debt` | < 35k | **12,223** | met |
| `create_market` | < 120k | **45,104** (Token-2022 both sides) | met |

Every Phase 0 target is met **after** OPT-01 (§9). Every one of these targets was **exceeded before
OPT-01** for any instruction that calls `accrue_mut`/`accrue_view` against a nonzero interval —
`liquidate` reached 469,137 CU, 2.3× the 200,000 CU default budget, which is why PERF-I6 (§6) is the
finding that shaped this whole section. See `benchmarks/README.md` §7 for the complete
before/after table across every scenario.

`liquidate`'s post-fix margin to the Phase 0 target (<110k) is deliberately thin (313 CU) because
that target was itself derived from the same architecture this measurement now confirms — it is not
a coincidence that a correct, unblocked implementation lands almost exactly on the number the design
predicted. Margin to the 200,000 CU **default budget** (the actual correctness gate, `INV-RES-01`)
is a comfortable 90,313 CU (45.2%).

**Cost drivers — measured, not assumed** (§6 has the full investigation):
- The 256-bit-intermediate division algorithm in `aegis-math::u256::div_u128` was, before OPT-01,
  the dominant cost in every instruction that calls `accrue_mut`/`accrue_view` — not the oracle, not
  Token-2022. See PERF-I2.
- Pyth `PriceUpdateV2` deserialization + valuation + LTV/HF math together cost ~25k CU for `borrow`'s
  two feeds (PERF-I1) — real, but no longer dominant now that the division cost is fixed, so no
  further caching optimization is justified by measurement.
- Token-2022 `transfer_checked` with a live transfer-fee extension costs ~1.7k–3k CU more than
  classic SPL Token per transfer (PERF-I3) — small and bounded, as anticipated.

---

## 4. Transaction composition

Worst case is `liquidate`: 14 accounts + 1 program + signatures. A legacy transaction allows 1232
bytes; ~32 bytes per account plus signature and instruction-data overhead leaves this comfortable but
not luxurious.

Requirements:
- `I-TX-01` asserts every instruction's realistic transaction fits **1232 bytes without an address
  lookup table**. Designing to the older limit means Aegis works everywhere, and SIMD-0296's 4096
  bytes becomes headroom rather than a dependency (RV-7).
- The SDK must support bundling `init_position` + the first user action in one transaction.
- Address lookup tables are an *optimization* for advanced flows (Phase 8/9), never a requirement.

---

## 5. Benchmark methodology

**Harness:** Mollusk, one instruction per benchmark, fixed account fixtures, deterministic inputs.
**Output:** `benchmarks/cu.json` (machine-readable) + `benchmarks/README.md` (human table), both
committed.

```json
{
  "commit": "<sha>", "date": "<iso8601>", "toolchain": {"anchor":"...","solana":"...","rustc":"..."},
  "measurements": [{"instruction":"borrow","scenario":"spl_token_with_debt","cu":0,"accounts":10}]
}
```

**CI:** compares against the committed baseline; a regression >10% on any instruction **fails the
build**. Improvements require the baseline to be updated in the same commit, so the file never drifts.

**Optimization protocol** — mandatory format for any performance change:

```
### OPT-nn: <what changed>
BEFORE:  borrow = 78,412 CU   (commit abc1234, benchmarks/cu.json)
CHANGE:  <precise description of the code change>
AFTER:   borrow = 61,905 CU   (commit def5678)
DELTA:   −16,507 CU (−21.0%)
RISK:    <what this change could break, and which test covers it>
```

An optimization without BEFORE and AFTER numbers from the committed harness is not merged. This is
stated in `AGENTS.md` as a hard rule.

---

## 6. Investigations — ANSWERED (Phase 11, 2026-09-13)

### PERF-I6 (run first, per the phase spec)

**Question:** Does `liquidate` fit 200k CU with Token-2022 on both sides?

**Hypothesis:** Yes, comfortably (Phase 0 target: <110k).

**Experiment:** `tests/bench.rs::perf_i6_liquidate_worst_case` — a market with Token-2022 on both
sides (collateral carries a live 2% transfer fee; the loan side is the only Token-2022 configuration
`create_market` accepts for a loan asset), a maximal real borrow, then a severe collateral price
crash driving the position deep enough underwater that a full-repay liquidation request also hits
the collateral-clamp recomputation branch (the more expensive of the liquidation-math paths) — built
entirely through real `supply`/`deposit_collateral`/`borrow`/`liquidate` instructions, never
fixture-injected debt, then measured through Mollusk against the real compiled `aegis.so`.

**Measurement (first honest run, before any optimization): 469,137 CU — 2.3× the 200,000 CU budget.**
Every other scenario touching real interest accrual was also over or dangerously close to budget
(`repay` at dt=30d: ~203–205k CU; `borrow`/`supply`/`withdraw`: 160k–184k CU) — see
`benchmarks/README.md` §7 for the complete pre-fix table.

**Conclusion:** initially **DISCONFIRMED** — this is exactly the T-27-class correctness/resource-
safety finding the phase spec anticipated, not routine tuning headroom. Root-caused (below, PERF-I2)
and fixed by OPT-01 (§9). **After the fix: 109,687 CU — comfortably under budget (45.2% margin), and
the hypothesis is now CONFIRMED.**

**Action taken:** OPT-01 (§9) — a measurement-justified, safety-preserving algorithmic fix to the
shared `mul_div` primitive, not a redesign of `liquidate`, the account model, or any invariant.

---

### PERF-I2 — the root cause

**Question:** Cost of `mul_div` with 256-bit intermediates vs a naive `u128` implementation?

**Hypothesis (Phase 0):** ~100–300 CU per call; a handful of calls per instruction; cheap, and
correctness is non-negotiable regardless of the answer.

**Investigation:** PERF-I6's finding demanded a real answer, not the Phase 0 estimate. Direct code
inspection of `crates/aegis-math/src/u256.rs::div_u128` found a **256-iteration bit-serial binary
long-division loop**, executed on **every single call** to `mul_div_floor`/`mul_div_ceil` —
regardless of whether the true 256-bit intermediate value actually needs more than 128 bits (the
overwhelmingly common case at this crate's actual WAD-scale magnitudes). A controlled experiment
(temporarily adding a fast path, measuring, `git checkout`-ing it back out, establishing the full
BEFORE baseline, then re-applying it) isolated the effect precisely:

| Scenario (isolates the division cost — no oracle, no tokens) | CU before | CU after fast path |
|---|---:|---:|
| `accrue_interest`, dt=30 days (2 accounts, pure state update) | 151,016 | 12,118 |

A single instruction with **zero token accounts and zero oracle reads** dropped by **92.0%** from
one change to one private helper function. This is unambiguous: the division algorithm, not the
oracle and not Token-2022, was the dominant cost everywhere `accrue_mut`/`accrue_view` runs (i.e.
`supply`, `withdraw`, `borrow`, `repay`, `accrue_interest`, and `liquidate`).

**Hypothesis verdict: the Phase 0 estimate (~100–300 CU/call) was WRONG by roughly two orders of
magnitude** for the actual `mul_div` implementation as originally written — a bit-serial 256-bit
division loop is dramatically more expensive on a BPF target than the estimate assumed, precisely
because a `u128` operation itself already lowers to multiple native 64-bit operations, and 256
iterations of several such operations each adds up fast under BPF's per-instruction accounting.

**Action:** OPT-01 (§9) — documented in full BEFORE/CHANGE/AFTER/DELTA/RISK form, applied.

---

### PERF-I1

**Question:** What fraction of `borrow`'s CU is Pyth deserialization (+ the surrounding oracle
validation and valuation math it enables)?

**Method:** `borrow` (48,360 CU, Token-2022, post-OPT-01) vs `repay` at dt=0 (22,926 CU,
Token-2022) — both scenarios share the same market shape, the same single token transfer, and the
same (`dt=0`) accrual cost; `borrow` adds exactly two price-account reads, `O-1..O-11` validation,
collateral/debt valuation, and the post-state LTV check on top.

**Measurement:** delta ≈ **25,434 CU** attributable to oracle validation + valuation +
LTV check for two feeds — about 53% of `borrow`'s total cost.

**Conclusion:** real and non-trivial, but **no longer dominant** now that OPT-01 has removed the
division bottleneck, and `borrow` sits at 48,360 CU with a **151,640 CU margin (76%)** under the
200,000 CU budget. **Action: no action** — the mitigation strategy-doc reserved for this case
("evaluate validating both feeds in one pass and reusing the result") is not justified by the
measurement: there is no correctness or headroom problem left to solve, and adding that complexity
now would be exactly the speculative optimization `AGENTS.md` §17 forbids ("optimize only if the
measurement justifies it").

---

### PERF-I3

**Question:** Token-2022 vs SPL Token transfer cost in Aegis's exact call shape?

**Measurement**, all post-OPT-01, same scenario pairs differing only in token program:

| Instruction (1 transfer) | SPL | Token-2022 (2% transfer fee) | Delta |
|---|---:|---:|---:|
| `deposit_collateral` | 12,708 | 15,692 | +2,984 |
| `borrow` | 46,628 | 48,360 | +1,732 |
| `liquidate` (2 transfers) | 105,118 | 109,687 | +4,569 (≈+2,285/transfer) |

**Conclusion:** CONFIRMED as a small, bounded adoption cost — roughly 1.7k–3k CU per transfer,
consistent with the Phase 0 expectation ("more than legacy SPL... bounded" because transfer hooks
are rejected by policy). **Action: document only**, as planned; no optimization is justified at this
magnitude.

---

### PERF-I4

**Question:** Does passing stored bumps instead of re-deriving PDAs with `find_program_address` save
meaningfully?

**Method:** `grep -rn find_program_address programs/aegis/src/` — **zero matches**. The program has
used `seeds = [...], bump = account.bump` (the stored-bump form, per `account-model.md` §7 rule 1)
in every instruction since Phase 2; there has never been a runtime `find_program_address` call to
compare against.

**Conclusion:** the optimization this investigation was designed to evaluate was **already adopted
architecturally**, before Phase 11 existed. There is no BEFORE state in this codebase to measure
against, so there is nothing to report as a Phase 11 delta — this is confirmed-by-inspection, not
confirmed-by-measurement, and is recorded as such rather than fabricating a synthetic comparison.
**Action: none; already done.**

---

### PERF-I5

**Question:** Cost of the mandatory account `reload()` after each transfer CPI?

**Method:** `reload()` is one additional account deserialization per inbound transfer
(`token/transfer.rs`'s `transfer_checked_in`), required unconditionally for correctness
(`INV-CUS-05`) regardless of cost. It is not optional and was never a candidate for removal, so no
with/without A/B exists in the real program to benchmark cleanly without weakening a required
check — which `AGENTS.md` §7 forbids outright. Its cost is folded into every transfer-bearing
scenario in `benchmarks/cu.json` (e.g. `deposit_collateral`'s 12,708–15,692 CU includes one).

**Conclusion:** **reported, not isolated further** — exactly as the Phase 0 plan specified
("Required for correctness regardless of cost"). Given every transfer-bearing instruction sits far
under budget post-OPT-01, there is no measurement basis to justify spending more effort isolating
this single-digit-thousand-CU cost precisely. **Action: none.**

---

## 7. Optimization log

### OPT-01: fast-path native `u128` division in `U256::div_u128`

```
BEFORE:  accrue_interest (dt=30d, 2 accounts, no oracle, no tokens) = 151,016 CU
         liquidate (Token-2022 both sides, clamped, worst case)     = 469,137 CU
         (commit ec97dd9 + this session's uncommitted tests/bench/ harness, benchmarks/README.md §7
         has the complete 14-scenario before table)

CHANGE:  crates/aegis-math/src/u256.rs::div_u128 — added a 4-line fast path: when the true 256-bit
         value fits entirely in the low limb (`self.hi == 0`, the overwhelmingly common case at
         this crate's actual WAD-scale magnitudes), return `(self.lo / d, self.lo % d)` — native
         u128 division, PROVABLY identical to what the existing 256-iteration bit-serial
         long-division loop below it would compute for the same input (not an approximation, not a
         different rounding rule: a value that fits in `lo` alone has quotient.hi == 0 by
         construction, which is exactly the loop's own success condition). The slow path is
         unchanged, byte-for-byte, as the exact fallback for genuinely-256-bit values.

AFTER:   accrue_interest (dt=30d)                                    = 12,118 CU   (-92.0%)
         liquidate (Token-2022 both sides, clamped, worst case)      = 109,687 CU  (-76.6%)
         Every one of the 29 benchmarked scenarios in benchmarks/cu.json now fits under 200,000 CU.

DELTA:   -138,898 CU (-92.0%) on accrue_interest; -359,450 CU (-76.6%) on worst-case liquidate.

RISK:    None identified. This is a pure algorithmic substitution behind a private, well-isolated
         function with an unchanged public signature and unchanged error semantics (same
         `Option<(u128, u128)>` overflow contract: a value that fits in `lo` can never overflow the
         quotient, so the `hi != 0 => None` branch is unreachable from the fast path, consistent
         with the slow path's own behavior on the same input). Verified: (1) the full `aegis-math`
         test suite unchanged and passing, including `matches_bignum_reference` (a property test
         comparing against an independent bignum implementation across many random inputs) and
         every `U-ARITH-*`/rounding-law/share-property test; (2) the full workspace suite
         (`cargo test --workspace --offline`, 35 test-result blocks) unchanged and passing after
         rebuilding the on-chain program; (3) `cargo clippy --workspace --all-targets -- -D
         warnings` and `cargo fmt --all --check` both clean. No invariant, economic formula,
         rounding direction, or account layout changed — `docs/economic-model.md` and
         `docs/invariants.md` required no edit and none was made.
```

No other optimization was found to be justified by measurement. Every scenario in
`benchmarks/cu.json` has a comfortable margin (the tightest, `liquidate`'s worst case, still has
45.2% headroom) after OPT-01 alone; per `AGENTS.md` §17 and `performance-strategy.md` §5's own rule,
no further change was made without a measurement showing it was needed.

---

## 8. The `labs/` CU comparison — MEASURED

`labs/` implements the same custody primitive — initialize a vault PDA, deposit, withdraw via a
PDA-signed CPI — three ways: **`vault-anchor`** (Anchor 1.x), **`vault-native`** (plain
`solana-program`, manual account parsing/validation/serialization), and **`vault-pinocchio`**
(`no_std`, hand-rolled). All three enforce the identical security checks (signer, mint, token
program, vault/PDA, authority, source/destination ownership, amount semantics — see each lab's own
module doc comment) and are benchmarked against each other in `labs/cu-bench/tests/compare.rs`,
using the exact same Mollusk methodology as the main protocol's own benchmarks.

This is not resume padding, and the justification is specific:

1. It benchmarks the *actual* Aegis custody primitive, so the numbers inform a real decision.
2. It quantifies **what Anchor's safety costs in CU** — the honest counterpart to ADR-0001's choice of
   Anchor for security reasons. Claiming "Anchor is worth it" without measuring the price is exactly
   the unsupported claim this document forbids.
3. Pinocchio is production-proven (Anza's `p-token`: token transfer ~4,645 → ~76 CU), so this is a
   current, non-toy comparison.

### Measured table

| Operation | Anchor | Native | Pinocchio | Native Δ | Pinocchio Δ |
|---|---:|---:|---:|---:|---:|
| `initialize` | 15,206 | 12,411 | 5,263 | −2,795 (−18.4%) | −9,943 (−65.4%) |
| `deposit` | 8,441 | 6,134 | 1,837 | −2,307 (−27.3%) | −6,604 (−78.2%) |
| `withdraw` | 8,541 | 6,181 | 1,874 | −2,360 (−27.6%) | −6,667 (−78.1%) |

**Hypothesis verdict: CONFIRMED.** Native is meaningfully cheaper than Anchor (18–28%); Pinocchio is
dramatically cheaper than both (65–78% under Anchor), consistent with the magnitude of Anza's own
`p-token` result (a token transfer's CU dropping by roughly an order of magnitude when rewritten in
Pinocchio) — this lab's `deposit`/`withdraw` numbers (each one `TransferChecked` CPI plus the
surrounding validation) land in the same ballpark reduction.

### Written conclusion: is this trade worth making for Aegis?

**Not yet — the same conclusion this document predicted before measuring, now backed by a number
instead of an opinion.** The reasoning:

1. **CU is not Aegis's binding constraint.** Every production instruction, after OPT-01 (§7), sits
   comfortably under the 200,000 CU budget — the tightest, `liquidate`'s worst case, still has 45%
   headroom. There is no CU emergency Pinocchio would be solving. The measured savings here (a few
   thousand CU on a simple two-account transfer) would shrink `liquidate`'s ~110k CU by, at most, a
   few percent even if applied everywhere plausible — nowhere near enough to justify the tradeoff
   below.
2. **The engineering-complexity and audit-surface cost is real and large.** Aegis's threat model
   (32 threats, 87 invariants) leans heavily on Anchor's automatic account validation
   (discriminator, owner, PDA-seed, duplicate-mutable-account rejection) to make that surface
   reviewable. Rewriting `programs/aegis` in native or Pinocchio style would mean manually
   re-deriving and re-checking every one of those properties at every one of 13 instructions'
   call sites — exactly the class of manual bookkeeping this lab's own `vault-native`/
   `vault-pinocchio` code had to hand-write for a THREE-instruction primitive, replicated across
   an order of magnitude more instructions and account relationships, for a lending protocol,
   not a lab.
3. **Aegis's actual binding constraint is contention** (`Market` write-locking within one market,
   PERF-C1..C3), and neither native nor Pinocchio changes the account-locking model at all — a
   faster program doesn't parallelize supply/borrow/repay/liquidate within one market, because
   the lock is on `Market`'s address, not on CPU time spent processing it.
4. **When would this change?** If a future measurement showed a *specific* instruction was
   compute-bound in a way that mattered — e.g. an instruction that genuinely could not fit 200k CU
   even after legitimate optimization, and only a framework rewrite closed the gap — that would be
   a real, measurement-driven case for a *scoped* native/Pinocchio rewrite of that one instruction,
   not the whole program. No such case exists today: OPT-01 alone (a targeted math-primitive fix,
   not a framework change) already brought every instruction comfortably under budget.

The lab's value is exactly what ADR-0003 predicted: turning "Anchor is worth it for a lending
protocol" from an assumption into a measured, revisitable claim.

---

## 9. Non-goals

| Non-goal | Reason |
|---|---|
| Zero-copy (`AccountLoader`) for all accounts | Accounts are small and fixed; the ergonomic and safety cost is not repaid. Revisit only if measurement shows deserialization dominating. |
| Bit-packing account fields | Rent is ~10× cheaper after SIMD-0437; clarity wins. |
| Custom serialization | Anchor's Borsh layout is fine for these sizes. |
| Rewriting the production program in Pinocchio | ADR-0003. Security budget beats CU budget for a lending protocol. |
| Micro-optimizing before measuring | Forbidden by the BEFORE/AFTER rule. |
