# Aegis — Performance, Compute and Contention Strategy

**Status: FROZEN (Phase 0). All numbers below are TARGETS/HYPOTHESES until Phase 11 measures them.**

> **Rule: no performance claim without a committed BEFORE → CHANGE → AFTER measurement.**
> Every number in this document that is not marked `MEASURED` is a hypothesis. Phase 11 replaces them
> with real figures or records that the hypothesis was wrong.

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

### Measurable claims (Phase 11)

| ID | Claim | Method |
|---|---|---|
| PERF-C1 | Transactions in different markets never conflict | Assert disjoint writable sets across two markets; execute concurrently in Surfpool |
| PERF-C2 | `deposit_collateral` from N users in the same market conflict only on `collateral_vault`, not on `Market` | Inspect the compiled account metas; assert `Market` is not writable |
| PERF-C3 | `Market` is the only intra-market contention point for lending operations | Write-set enumeration test |

PERF-C2 is a **regression risk, not just a claim**: any future change that adds a `Market` write to a
collateral instruction silently destroys it. `A-PAR-01` asserts the account metadata directly, so the
property is guarded by a test rather than by memory.

---

## 3. Compute budget targets (HYPOTHESES)

| Instruction | Target CU | Notes |
|---|---|---|
| `init_position` | < 15k | One account init |
| `deposit_collateral` | < 30k | 1 token CPI + reload |
| `withdraw_collateral` (no debt) | < 30k | No oracle |
| `withdraw_collateral` (with debt) | < 60k | + 2 oracle reads + valuation |
| `supply` / `withdraw` | < 50k | accrual + share math + 1 CPI |
| `borrow` | < 75k | accrual + 2 oracle reads + health + 1 CPI |
| `repay` | < 50k | accrual + share math + 1 CPI |
| `accrue_interest` | < 25k | Pure state update |
| `liquidate` | **< 110k** | accrual + 2 oracle reads + liquidation math + **2 CPIs** |
| `absorb_bad_debt` | < 35k | No tokens, no oracle |
| `create_market` | < 120k | 3 account inits + extension parsing |

`liquidate` is the one to watch. If it approaches the 200k default budget, the mitigation order is:
(1) reduce redundant account deserialization, (2) cache derived values, (3) require the client to set
an explicit `ComputeBudget` instruction. Option 3 is a **last resort** — an instruction that needs a
raised budget to function is fragile exactly when it matters most.

**Anticipated cost drivers** (to be confirmed, not assumed):
- Pyth `PriceUpdateV2` deserialization: expect 5–10k CU each. If measured materially higher, that
  changes the design conversation about caching a validated price within a transaction.
- Token-2022 `transfer_checked` with extensions: more than legacy SPL. Since transfer hooks are
  rejected, this is bounded.
- `mul_div` with 256-bit intermediates: ~100–300 CU each; a handful per instruction. Cheap, and
  correctness is non-negotiable regardless.

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

## 6. Planned investigations (Phase 11)

| ID | Question | Method | Action if confirmed |
|---|---|---|---|
| PERF-I1 | What fraction of `borrow`'s CU is Pyth deserialization? | Benchmark with and without oracle validation | If dominant, evaluate validating both feeds in one pass and reusing the result |
| PERF-I2 | Cost of `mul_div` with 256-bit intermediates vs naive `u128`? | Microbenchmark in `aegis-math` | Report only. **Correctness is not negotiable for CU**; a documented cost is the deliverable |
| PERF-I3 | Token-2022 vs SPL Token transfer cost in our exact call shape? | Two market fixtures | Document as a Token-2022 adoption cost |
| PERF-I4 | Does passing stored bumps instead of re-deriving save meaningfully? | With/without benchmark | Adopt if material (expected ~1.5k CU/PDA) |
| PERF-I5 | Cost of the account `reload()` after each transfer CPI | Benchmark | Report. **Required for correctness (INV-CUS-05) regardless of cost** |
| PERF-I6 | Does `liquidate` fit 200k with Token-2022 on both sides? | Worst-case fixture | If not, this is a **correctness** problem (T-27) and drives a design change, not a tuning pass |

PERF-I6 is the one that could force an architectural change, so it runs first.

---

## 7. The `labs/` CU comparison

`labs/` implements the same custody primitive — initialize a vault PDA, deposit, withdraw with
`invoke_signed` — three ways: **Anchor**, **native `solana-program`**, and **Pinocchio**. All three
are benchmarked in `labs/cu-bench`.

This is not resume padding, and the justification is specific:

1. It benchmarks the *actual* Aegis custody primitive, so the numbers inform a real decision.
2. It quantifies **what Anchor's safety costs in CU** — the honest counterpart to ADR-0001's choice of
   Anchor for security reasons. Claiming "Anchor is worth it" without measuring the price is exactly
   the unsupported claim this document forbids.
3. Pinocchio is production-proven (Anza's `p-token`: token transfer ~4,645 → ~76 CU), so this is a
   current, non-toy comparison.

Expected shape of the result (**hypothesis, to be measured**): native meaningfully cheaper than
Anchor; Pinocchio meaningfully cheaper than native. The deliverable is the measured table plus a short
written conclusion about when the trade would be worth making for Aegis — and the honest expected
conclusion is *not yet*, because Aegis's binding constraint is contention and correctness, not CU.

---

## 8. Non-goals

| Non-goal | Reason |
|---|---|
| Zero-copy (`AccountLoader`) for all accounts | Accounts are small and fixed; the ergonomic and safety cost is not repaid. Revisit only if measurement shows deserialization dominating. |
| Bit-packing account fields | Rent is ~10× cheaper after SIMD-0437; clarity wins. |
| Custom serialization | Anchor's Borsh layout is fine for these sizes. |
| Rewriting the production program in Pinocchio | ADR-0003. Security budget beats CU budget for a lending protocol. |
| Micro-optimizing before measuring | Forbidden by the BEFORE/AFTER rule. |
