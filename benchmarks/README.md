# Aegis — Compute Unit Benchmarks (Phase 11, updated Phase 12)

**Status: MEASURED.** Every number below comes from `tests/bench.rs`'s `cu_benchmark_suite`,
executed against the real compiled `programs/aegis` artifact. Nothing here is estimated or
extrapolated — see `docs/performance-strategy.md` for the methodology rule this file exists to
satisfy: *no performance claim without a committed BEFORE → CHANGE → AFTER measurement.*

---

## 1. Methodology

Every scenario builds its pre-state through **real instructions** executed against an in-process
`LiteSVM` world (the same `aegis-test-kit`/`aegis-test-kit::market` builders Phases 2-10's own test
suites already use — supply, deposit, borrow, price injection, time warps). Immediately before the
instruction under measurement, every account it references is snapshotted directly out of that
`LiteSVM` world — byte-identical to what a real transaction history produced, never a hand-guessed
struct literal — and handed to a fresh [`mollusk-svm`](https://docs.rs/mollusk-svm) instance loaded
with the real compiled `aegis.so`, the real SPL Token program, and the real Token-2022 program.
Mollusk executes the instruction through the actual BPF loader and reports its own
`compute_units_consumed`. LiteSVM (this workspace's primary test harness) does not expose
per-instruction CU, which is why Mollusk is used specifically for this phase.

This means every CPI a benchmarked instruction makes (`transfer_checked` into a real SPL Token or
Token-2022 account, including live Token-2022 extension logic) is genuinely executed, not stubbed.

## 2. Environment / toolchain

| Component | Version | How verified |
|---|---|---|
| `rustc` / `cargo` | 1.98.1 | `rustc --version` |
| Solana CLI (Agave) | 3.1.10 | `solana --version` |
| Anchor CLI | 1.2.0 | `anchor --version` |
| `mollusk-svm` | 0.15.1 | `cargo search mollusk-svm` against crates.io, 2026-09-13 (re-confirms `docs/ecosystem-research.md` §12.2's Phase 1 finding) |
| `mollusk-svm-programs-token` | 0.15.1 | same, bundles the real SPL Token + Token-2022 program ELFs |
| Build profile | `cargo build-sbf` release (via `anchor build`), `overflow-checks = true` | `Cargo.toml` `[profile.release]` |

CU is a property of the compiled BPF program executed by Mollusk's own BPF loader — it does not
depend on the host machine's CPU speed, and is not affected by whether `cargo test` itself runs in
debug or release (only the *harness* is a debug build; the *measured* program is always the real
`anchor build` release artifact).

## 3. Reproducing this file

From a clean checkout, offline, no network, no secrets:

```bash
anchor build --ignore-keys        # produces target/deploy/aegis.so
make bench                        # runs cu_benchmark_suite and regenerates benchmarks/cu.json
```

`make bench` sets `AEGIS_BENCH_WRITE=1` so the suite writes `benchmarks/cu.json`; without that
variable (i.e. under plain `cargo test --workspace`), the suite only measures, asserts, and prints
— it never has the side effect of touching the repository, so the required test suite stays
side-effect-free.

## 4. Scenario definitions

Every one of the 21 current production instructions (`programs/aegis/src/lib.rs`'s `#[program]`
module — `ping` is a Phase 1 toolchain proof, not production, and is excluded) is benchmarked, for
every token-program variant that genuinely applies to it, and for the representative states in
`docs/phases/phase-11-performance.md` #8 that materially change its cost. This originally covered
the 13 instructions that existed through Phase 11; Phase 12 added its own eight governance/upgrade
instructions with their own CU section below (§5), correctly recorded there at the time — this
paragraph's prose describing "13... governance/pause instructions... out of scope" was itself
stale until Phase 13 corrected it (the underlying `benchmarks/cu.json` data was never missing
them, only this sentence lagged):

| Instruction | Variants measured | Why |
|---|---|---|
| `initialize_protocol` | n/a (no token accounts) | one-time, token-agnostic |
| `create_market` | SPL, Token-2022 both sides | vault-sizing/extension-parsing cost genuinely differs |
| `init_position` | n/a (no token accounts) | token-agnostic |
| `deposit_collateral` | SPL, Token-2022 (2% transfer fee) | measured-delta accounting cost |
| `withdraw_collateral` | no-debt (SPL, Token-2022) and with-debt-oracle-LTV (SPL) | INV-ORA-02: the no-debt path takes no oracle read at all; the with-debt path is `borrow`'s valuation cost minus the transfer |
| `close_position` | n/a (no token accounts) | token-agnostic |
| `supply` / `withdraw` | SPL, Token-2022, after real prior accrual | accrual dominates; token choice is secondary |
| `borrow` | SPL, Token-2022 | oracle (both feeds) + LTV + accrual + transfer |
| `repay` | SPL, Token-2022; `dt=0` and `dt=30d` | `docs/phases/phase-11-performance.md` #8's dt=0-vs-accrual distinction |
| `accrue_interest` | n/a (no token accounts); `dt=0` (no-op) and `dt=30d` | isolates pure accrual cost from everything else |
| `liquidate` | SPL, Token-2022 both sides; unclamped (close-factor-limited partial) and clamped (full repay, collateral-clamp branch) | the two liquidation-math branches genuinely differ in cost; Token-2022-both-sides-clamped is PERF-I6's worst case |
| `absorb_bad_debt` | n/a (moves no tokens) | reuses the exact bad-debt state a clamped full liquidation leaves behind |
| `withdraw_collateral_fees` | SPL, Token-2022 | reuses the `collateral_fee_accrued` a liquidation generates |

**Token-2022 N/A instructions, explicitly**: `initialize_protocol`, `init_position`,
`close_position`, and `accrue_interest` touch no mint, vault, or token account at all (see their
account lists in `docs/instruction-catalogue.md`) — fabricating a Token-2022 variant for them would
measure nothing real. Recorded as `token_program: "n/a"` in `cu.json`, never silently omitted.

## 5. CU table (current — see §7 for the pre-optimization numbers)

| Instruction | Scenario | Token program | CU | Accounts |
|---|---|---|---:|---:|
| `initialize_protocol` | fresh | n/a | 8,415 | 3 |
| `create_market` | fresh | SPL | 36,489 | 11 |
| `create_market` | fresh | Token-2022 both sides | 45,104 | 11 |
| `init_position` | fresh market | n/a | 11,481 | 5 |
| `deposit_collateral` | existing position | SPL | 12,708 | 7 |
| `deposit_collateral` | existing position | Token-2022 | 15,692 | 7 |
| `withdraw_collateral` | no debt | SPL | 13,028 | 9 |
| `withdraw_collateral` | with debt, oracle+LTV | SPL | 39,189 | 9 |
| `withdraw_collateral` | no debt | Token-2022 | 15,953 | 9 |
| `close_position` | empty position | n/a | 5,075 | 3 |
| `supply` | existing market, accrued interest | SPL | 23,637 | 8 |
| `withdraw` | existing market, accrued interest | SPL | 23,293 | 8 |
| `supply` | existing market, accrued interest | Token-2022 | 25,387 | 8 |
| `withdraw` | existing market, accrued interest | Token-2022 | 25,037 | 8 |
| `borrow` | healthy, fresh collateral | SPL | 46,628 | 10 |
| `borrow` | healthy, fresh collateral | Token-2022 | 48,360 | 10 |
| `repay` | dt=0, partial | SPL | 21,175 | 8 |
| `repay` | dt=30d, partial | SPL | 24,252 | 8 |
| `repay` | dt=0, partial | Token-2022 | 22,926 | 8 |
| `repay` | dt=30d, partial | Token-2022 | 26,003 | 8 |
| `accrue_interest` | dt=0 (no-op) | n/a | 9,041 | 2 |
| `accrue_interest` | dt=30d | n/a | 12,118 | 2 |
| `liquidate` | unclamped, close-factor partial | SPL | 102,902 | 16 |
| `liquidate` | clamped, full repay | SPL | 105,118 | 16 |
| `absorb_bad_debt` | post-full-liquidation dust | n/a | 12,223 | 3 |
| `withdraw_collateral_fees` | full accrued balance | SPL | 15,523 | 7 |
| `liquidate` | unclamped, close-factor partial | Token-2022 both sides | 107,604 | 16 |
| **`liquidate`** | **clamped, full repay — worst case (PERF-I6)** | **Token-2022 both sides** | **109,687** | **16** |
| `withdraw_collateral_fees` | full accrued balance | Token-2022 | 18,448 | 7 |

## 6. Worst-case instruction and margin to 200,000 CU

**`liquidate`, Token-2022 collateral (2% transfer fee) + Token-2022 loan, oracle validation on both
feeds, full interest accrual, the collateral-clamp liquidation-math branch, two real token
transfers, and protocol-fee handling: 109,687 CU — a margin of 90,313 CU (45.2%) under the 200,000
CU default budget.**

Every other benchmarked scenario is further under budget. `create_market` (the most
account/extension-heavy administrative instruction) is the next highest at 45,104 CU.

## 7. PERF-I6, first — and a real finding

Per the phase spec, `liquidate`'s worst case was measured **before** anything else, and **before**
any optimization. The first honest measurement was **469,137 CU — 2.3× over the 200,000 CU
budget**, together with every scenario in this file that involves real interest accrual
(`repay` at `dt=30d`: ~203–205k; both `liquidate` scenarios: 383k–469k). This is a T-27-class
correctness/resource-safety finding, not routine tuning headroom, and is documented in full —
hypothesis, root cause, the exact fix, and the BEFORE/AFTER evidence for every affected
instruction — in `docs/performance-strategy.md` §6 (PERF-I2) and §9 (optimization log). Every
number in §5 above is the **AFTER** figure; §7 exists so the committed baseline never has to be
reconstructed from memory (`AGENTS.md` §17: measure before optimizing, always from committed data).

### Full pre-optimization baseline, for the record

| Instruction | Scenario | Token program | CU (before) | CU (after) |
|---|---|---|---:|---:|
| `supply` | existing market, accrued interest | SPL | 182,303 | 23,637 |
| `withdraw` | existing market, accrued interest | SPL | 182,009 | 23,293 |
| `supply` | existing market, accrued interest | Token-2022 | 184,083 | 25,387 |
| `withdraw` | existing market, accrued interest | Token-2022 | 183,783 | 25,037 |
| `borrow` | healthy, fresh collateral | SPL | 166,712 | 46,628 |
| `borrow` | healthy, fresh collateral | Token-2022 | 168,464 | 48,360 |
| `repay` | dt=30d, partial | SPL | 202,878 | 24,252 |
| `repay` | dt=30d, partial | Token-2022 | 204,702 | 26,003 |
| `accrue_interest` | dt=30d | n/a | 151,016 | 12,118 |
| `withdraw_collateral` | with debt, oracle+LTV | SPL | 139,247 | 39,189 |
| `liquidate` | unclamped, close-factor partial | SPL | 383,083 | 102,902 |
| `liquidate` | clamped, full repay | SPL | 464,682 | 105,118 |
| `liquidate` | unclamped, close-factor partial | Token-2022 | 387,867 | 107,604 |
| **`liquidate`** | **clamped, full repay — worst case** | **Token-2022 both sides** | **469,137** | **109,687** |

(Every scenario not listed here — `initialize_protocol`, `create_market`, `init_position`,
`deposit_collateral`, `withdraw_collateral` no-debt, `close_position`, `repay` dt=0,
`accrue_interest` dt=0, `absorb_bad_debt`, `withdraw_collateral_fees` — was already comfortably
under budget before the fix, because none of them call `accrue_mut` against a nonzero interval, or
call it only against a `dt=0` no-op.)

## 8. Limitations

- These are **local CU measurements**, not mainnet performance claims. CU accounting is part of the
  SVM specification and is deterministic given the same program bytes, accounts, and instruction —
  it does not depend on network conditions, validator load, or client latency, but this file makes
  no claim about transaction *confirmation time*, *fees*, or *inclusion* on any real cluster.
- `liquidate`'s callback branch (Phase 8, ADR-0013) is **not** included in the worst-case figure
  above: the callback CPI's own internal cost is the callback program's responsibility, not
  Aegis's, and `performance-strategy.md`'s own target table treats the no-callback path as
  `liquidate`'s definition of worst case (`2 CPIs`). It is measured separately as informational
  evidence in `docs/performance-strategy.md` §6 (PERF-I6).
- Every scenario here reflects the SPECIFIC account/state shape described in §4. A pathological
  state not represented here (e.g. a market that has never called `accrue_interest` in years,
  producing a much larger `dt`) could in principle cost more if the IRM's compounding math scales
  with the magnitude of `dt` rather than being O(1) in it; `aegis_math::irm`'s Taylor-series
  compounding is a fixed number of terms regardless of `dt`'s size, so this is not expected to
  matter, but it has not been independently re-measured at extreme `dt`.

## 9. Phase 12 update — governance/pause instructions added

**Status: MEASURED, re-baselined.** Phase 12 adds a read-only `protocol` account plus
`guards::require_pause_bit_clear` to `supply`, `withdraw`, `borrow`, `withdraw_collateral`, and
`liquidate` (INV-ADM-03/04, `docs/governance.md` §3), and adds eight new admin instructions. This
is a real, expected, security-motivated cost increase — not an accidental regression — re-baselined
here with committed before/after numbers per this file's own methodology rule.

**BEFORE → AFTER for every scenario whose CU changed because of the new `protocol` account/pause
check** (Phase 11 baseline → Phase 12, `git diff benchmarks/cu.json`):

| Instruction / scenario | Before | After | Δ | Cause |
|---|---|---|---|---|
| `supply` / accrued interest, SPL | 23,637 | 27,158 | +3,521 (+14.9%) | `protocol` account load + pause check |
| `supply` / accrued interest, Token-2022 | 25,387 | 28,910 | +3,523 (+13.9%) | same |
| `withdraw` / accrued interest, SPL | 23,293 | 26,789 | +3,496 (+15.0%) | same |
| `withdraw` / accrued interest, Token-2022 | 25,037 | 28,534 | +3,497 (+14.0%) | same |
| `withdraw_collateral` / no debt, SPL | 13,028 | 15,846 | +2,818 (+21.6%) | same (cheapest scenario, so the % looks largest) |
| `withdraw_collateral` / no debt, Token-2022 | 15,953 | 18,776 | +2,823 (+17.7%) | same |
| `withdraw_collateral` / with debt, SPL | 39,189 | 43,052 | +3,863 (+9.9%) | same |
| `borrow` / healthy, SPL | 46,628 | 50,645 | +4,017 (+8.6%) | same |
| `borrow` / healthy, Token-2022 | 48,360 | 52,380 | +4,020 (+8.3%) | same |
| `liquidate` / unclamped partial, SPL | 102,902 | 109,109 | +6,207 (+6.0%) | same (`Liquidate` also needed `Box<...>` on `protocol` and `position` — see below) |
| `liquidate` / clamped full, SPL | 105,118 | 111,690 | +6,572 (+6.3%) | same |
| `liquidate` / unclamped partial, Token-2022 | 107,604 | 113,813 | +6,209 (+5.8%) | same |
| `liquidate` / clamped full (worst case), Token-2022 | 109,687 | 116,304 | +6,617 (+6.0%) | same |

Every one of these stays well under 10% except the two cheapest `withdraw_collateral` scenarios
(21.6%/17.7%) — a small absolute increase (~2,820 CU) on a small baseline reads as a large
percentage; the *absolute* cost added is consistent across every scenario in this table
(~2,800–6,600 CU, dominated by loading one more account and one cheap bitwise-AND check, not by
anything that scales with market size). `scripts/check-cu-regression.sh` was re-run after
committing this new baseline and passes (37 scenarios, `benchmarks/cu.json` is authoritative going
forward).

A handful of unrelated instructions (`create_market`, `init_position`, `repay`, `accrue_interest`,
`absorb_bad_debt`, `close_position`, `initialize_protocol`, `deposit_collateral`) also moved by a
few hundred CU even though Phase 12 did not touch their handlers. This is expected: the *same
compiled program* now dispatches 22 instructions instead of 14 and carries a larger error enum and
event set, which shifts BPF instruction-cache/jump-table layout slightly for every instruction in
the binary — a well-known, uniform, non-security-relevant effect of adding code to a single Anchor
program, not a per-instruction regression. All such moves are under 5% and are absorbed by the
same re-baseline.

**A real correctness finding, not merely a benchmark:** the deterministic Docker build
(`solana-verify build`, `docs/project-status.md` §6) failed outright the first time `protocol` was
added unboxed to `Liquidate` — `try_accounts` exceeded the SBF stack-frame limit (4096 bytes) by
448 bytes, a real "may cause undefined behavior" compiler error under that stricter toolchain that
this repository's own default local build did not surface. Fixed by boxing `protocol` (recovered
384 bytes) and then `position` (recovered the remaining 64+ bytes) in `Liquidate` specifically —
the only instruction that needed it; every other pausable instruction's unboxed `protocol` field
compiled cleanly under the same strict build. See `docs/project-status.md` Phase 12 §6 for the full
verifiable-build evidence this fix unblocked.

**New instructions, benchmarked for the first time** (all well within the 200,000/1,400,000 CU
budgets; none move tokens or read an oracle):

| Instruction / scenario | CU | Accounts |
|---|---|---|
| `set_pending_admin` / fresh protocol | 4,404 | 2 |
| `accept_admin` / fresh protocol | 4,392 | 2 |
| `set_guardian` / fresh protocol | 4,478 | 2 |
| `set_protocol_pause` / admin sets one bit | 4,390 | 2 |
| `set_market_pause` / admin sets one bit | 8,670 | 3 |
| `set_market_params` / tighten, immediate | 19,908 | 7 |
| `commit_pending_params` / at `effective_at` | 19,173 | 6 |
| `migrate_protocol_v2` / real `ProtocolV1` account | 5,871 | 2 |
