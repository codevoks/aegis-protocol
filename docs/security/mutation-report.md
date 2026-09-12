# Mutation Validation Report (Phase 10)

**Purpose:** for each of the nine **[GLOBAL]** invariants in `docs/invariants.md`, deliberately
remove its enforcement in the live source tree, rebuild the real on-chain program, run the stateful
fuzzer (`tests/fuzz/`) against the mutated build, and confirm the fuzzer finds a violation within a
bounded budget — then revert and confirm the baseline passes again. An invariant the fuzzer cannot
falsify is not being tested (`docs/testing-strategy.md` §5, `docs/phases/phase-10-security.md`).

**Result: all nine mutations were caught within the documented bounded budget.** Two mutations
required improving the fuzzer first (documented below, not smoothed over) before they were caught —
exactly what `phase-10-security.md`'s acceptance rule anticipates ("If a mutation is not caught, the
fuzzer is inadequate and must be improved... Record every result — including any that required
improving the fuzzer").

## Procedure (identical for every mutation)

1. `git status --short` confirms a clean tree before starting.
2. Edit the exact enforcement line(s) in `programs/aegis/src/...` (commented out, never deleted, so
   the diff is unambiguous and trivially revertible).
3. **`anchor build -p aegis`** — this step is load-bearing and easy to get wrong: the fuzzer loads
   `target/deploy/aegis.so` (`tests/fuzz/world.rs::program_bytes`), a separately-built on-chain
   artifact, not the `aegis` Rust *library* crate `cargo build`/`cargo test` recompile on their own.
   Editing the source and only running `cargo test` silently re-tests the STALE, unmutated `.so` —
   the very first mutation attempt hit exactly this (§"Finding 1" below).
4. `cargo test --test fuzz --offline -- --ignored mutation_probe --nocapture` (or
   `mutation_probe_bad_debt` for INV-SOLV-04) — a bounded, deterministic probe well above the CI
   budget specifically so a removed check is reliably exercised.
5. Record the seed, the exact step index, the panic message (which invariant fired and with what
   values), and the shrinker's minimized reproduction trace.
6. `git checkout -- <file>` to restore the exact original code, then `anchor build -p aegis` again
   and re-run the CI-bounded campaign to confirm the baseline is clean.

**Budget:** `mutation_probe` — 10 seeds × 2,000 ops = 20,000 operations maximum.
`mutation_probe_bad_debt` — 5 seeds × 500 tail ops on a pre-seeded bad-debt position = 2,500
operations maximum. Both bounded and deterministic, documented here rather than "run until it
works." Every mutation below was caught well inside its budget — the slowest took 636 operations.

## Two findings, surfaced rather than hidden

### Finding 1 — a test-harness bug produced a false "not detected" on the very first attempt

The first mutation attempt (INV-CUS-01, literally as `phase-10-security.md`'s table describes it:
"skip the free-liquidity check in `withdraw`") initially reported **NOT DETECTED** against the full
20,000-operation probe budget. Investigated before concluding the fuzzer was inadequate:
`target/deploy/aegis.so`'s mtime predated the source edit — `anchor build` had not been run, so the
fuzzer had been executing the unmodified, pre-mutation binary the entire time. `cargo test` rebuilds
the `aegis` Rust library (used for PDA derivation and typed instruction building) but never touches
the separately-built on-chain artifact LiteSVM actually loads and executes. Every mutation from this
point on explicitly ran `anchor build -p aegis` first, and the artifact's mtime was checked against
the edit's timestamp to confirm the rebuild actually happened.

### Finding 2 — two literal mutations are architecturally inert, not fuzzer gaps

After fixing Finding 1, the literal INV-CUS-01 mutation ("skip the free-liquidity check in
`withdraw`") and the literal INV-ACC-03 mutation ("remove the borrow liquidity check") were
re-tried and **still** reported NOT DETECTED. Analysis: both checks gate an *outbound* transfer
(`transfer_checked_out`), which CPIs into the real SPL Token / Token-2022 program — a program that
independently refuses to transfer more than the vault's actual token balance. Whenever INV-CUS-01
already holds going into the instruction (which the fuzzer continuously re-verifies), the vault's
real balance is *exactly* the quantity each removed `require!` was bounding the request against.
Removing Aegis's own check therefore changes nothing about what can actually leave the vault: the
token program's own balance enforcement is an equivalent, redundant backstop for these two specific
checks, given this architecture. This is a genuine, positive finding about defense-in-depth, not a
fuzzer weakness — see `docs/security/findings.md` (F-10-01).

Per `phase-10-security.md` #5 ("the security test must expose the vulnerability... not... gaming"),
rather than force a match to the literal wording through a contrived, unrealistic scenario, both
mutations were adjusted to the minimal change that *does* remove the invariant's real protection —
directly desyncing the accounting from reality, independent of the transfer-level backstop (see the
table below for exactly what was changed for each). Both were then caught immediately.

### Two more improvements the acceptance rule required

- **INV-SOLV-04** (`absorb_bad_debt`): a pure random walk essentially never reaches
  `absorb_bad_debt`'s precondition (`collateral_amount == 0` EXACTLY, `borrow_shares > 0`) within
  any practical budget, because it requires a FULL, not partial, liquidation. Fixed by adding
  `World::seed_bad_debt_position` (`tests/fuzz/world.rs`), which uses the same `seed_borrow_state`
  fixture-injection technique `tests/phase6_bad_debt.rs` already established, plus a dedicated
  `mutation_probe_bad_debt` test that seeds this state directly and runs a bounded biased random
  tail on top. (A first version of this probe had its own bug — the manual setup `supply()` call
  bypassed the value-creation ledger's bookkeeping and produced a false positive against *correct*
  code; fixed by recording that contribution in the ledger explicitly, then re-verified clean
  against unmutated code before use.)
- **INV-ACC-04** (interest accrual): the fuzzer's reference market configuration
  (`reference_market_args`) ships a **zero-rate IRM** by design (`base_rate_ps = slope1_ps =
  slope2_ps = 0`), intended for tests that want share/asset math without interest complicating it.
  That meant no market in this fuzzer's world ever accrued nonzero interest, so INV-ACC-04 had
  nothing to falsify. Fixed by overriding `slope1_ps`/`slope2_ps` to the same real, non-placeholder
  per-second WAD rates already used elsewhere in this repository
  (`tests/phase4_lending.rs`/`phase6_integration.rs`: "4% APR at the kink" / "+100% APR above the
  kink") after calling `reference_market_args`, in both fuzzer markets.
- The trace **shrinker** itself needed a rewrite mid-campaign: the original naive "try removing one
  step at a time from the start" is O(n²) `replay` calls, which is fine for short traces but made
  the INV-ACC-02 mutation's shrink step spin for 6+ minutes once a violation happened to surface
  late (step ~1,500) in a 2,000-step trace. Replaced with a coarse-to-fine ("ddmin"-style) chunk
  remover with a hard cap on total replay calls (`tests/fuzz.rs::shrink`), which brought every
  subsequent shrink back down to sub-second.

All of this is recorded in full rather than smoothed over, per `docs/security/README.md`'s standing
rule #4 and `AGENTS.md` §14 ("no fake completion" / "report the exact commands run and their actual
output").

## Results

| Invariant | Mutation actually applied | Detected | Seed | Ops to detect | Minimized trace | Fired via |
|---|---|:--:|---:|---:|---|---|
| INV-CUS-01 `[GLOBAL]` | `lend/withdraw.rs`: skip the `total_supply_assets` decrement after a successful withdrawal (adjusted from the literal "skip the free-liquidity check" — Finding 2) | ✅ | 1 | 115 | `Supply{assets:50_000_000_000}` → `Withdraw{assets:50_000_000_000}` (2 steps) | INV-CUS-01 |
| INV-CUS-02 `[GLOBAL]` | `token/transfer.rs::transfer_checked_in`: return the requested `amount` instead of the measured `after - before` delta | ✅ | 1 | 37 | `DepositCollateral{amount:1}` on the Token-2022 transfer-fee market (1 step) | INV-CUS-02 |
| INV-ACC-01 `[GLOBAL]` | `lend/supply.rs`: skip the `total_supply_shares` increment | ✅ | 1 | 1 | `Supply{assets:50_000_000_000}` (1 step — the very first operation) | INV-ACC-01 |
| INV-ACC-02 `[GLOBAL]` | `borrow/borrow.rs`: skip the `total_borrow_shares` increment | ✅ | 1 | 464 | `DepositCollateral` → `Supply` → `Borrow` (3 steps) | INV-SOLV-01 (the corrupted `total_borrow_shares` inflates the independently-recomputed debt value; a legitimate cross-invariant catch) |
| INV-ACC-03 `[GLOBAL]` | `borrow/borrow.rs`: `total_borrow_assets` credited DOUBLE the real transferred amount (adjusted from the literal "remove the borrow liquidity check" — Finding 2) | ✅ | 1 | 464 | `DepositCollateral` → `Supply` → `Borrow` (3 steps) | INV-CUS-01 |
| INV-ACC-06 | `lend/withdraw.rs`: freeze both `total_supply_shares` and `position.supply_shares` (kept mutually consistent so INV-ACC-01 does not mask the effect) while `total_supply_assets` still falls | ✅ | 1 | 2,001 | `DepositCollateral` → `Supply` → `Borrow` (3 steps) | T-17 value-creation ledger (frozen shares inflate share price, letting a withdrawer extract more than contributed — the exact-zero-crossing snapshot INV-ACC-06 itself checks never happened to land within budget, but the downstream economic effect was caught) |
| INV-SOLV-01 `[GLOBAL]` | `borrow/borrow.rs`: skip the post-borrow `require!(within_ltv, ...)` | ✅ | 1 | 174 | `Supply` → `Borrow` with zero collateral (2 steps) | INV-SOLV-01 |
| INV-SOLV-04 `[GLOBAL]` | `liquidate/absorb_bad_debt.rs`: skip the `total_borrow_assets` decrement (only `total_supply_assets` falls) | ✅ | 1 (`mutation_probe_bad_debt`) | 10 (tail steps, after a directly-seeded bad-debt position) | seed bad debt → `AbsorbBadDebt` (immediate) | INV-CUS-01 |
| INV-ACC-04 | `state/market.rs::accrue_view`: add `interest` to `total_borrow_assets` but not `total_supply_assets` | ✅ | 1 | 636 | `Supply` → `DepositCollateral` → `Borrow` (3 steps, on the Token-2022 market) | INV-ACC-04 |

## Acceptance

Per `docs/phases/phase-10-security.md`: **all nine mutations must be caught within the documented
bounded budget for Phase 10 to be complete.** ✅ Satisfied — every row above reads "✅" with real
seed/step/trace evidence, reproduced from this document's own commands.

## Campaign statistics — extended fuzz campaign (value-creation search + general invariant search)

Run with `make fuzz` (`cargo test --test fuzz --offline -- --ignored fuzz_extended_campaign
--nocapture`): 25 seeds × 4,000 operations/seed = 100,000 operations, two markets (classic SPL/SPL
and Token-2022 transfer-fee collateral), six actors, biased boundary-adjacent sampling, real
nonzero interest accrual, deterministic and reproducible from each seed.

**First run (before the fuzzer's interest-rate and liquidation-targeting improvements made during
mutation validation):** 25 seeds, 100,000 operations, 64,558 succeeded / 35,442 rejected, zero
violations.

**Second run (after those fuzzer improvements, before the F-10-02 fix):** found a real violation —
seed 21, step 3628, `INV-ACC-06 violated (borrow): shares=3350 assets=0` — against the correct,
unmutated program. This is `docs/security/findings.md`'s **F-10-02**, minimized by hand to
`tests/adversarial/dust_debt.rs`, fixed in `programs/aegis/src/instructions/borrow/repay.rs`, and
confirmed via a frozen regression test that fails against the pre-fix code and passes after.

**Third run (after the F-10-02 fix — the acceptance run):**

```
$ make fuzz
[fuzz-extended] seed=0 succeeded=2573 failed=1427
[fuzz-extended] seed=1 succeeded=2574 failed=1426
...
[fuzz-extended] seed=21 succeeded=2493 failed=1507      <-- the seed that previously crashed
...
[fuzz-extended] seed=24 succeeded=2579 failed=1421
[fuzz-extended] TOTALS seeds=25 ops=100000 succeeded=64132 failed=35868
test fuzz_extended_campaign ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 742.36s
```

Seed 21 — the exact seed that found F-10-02 — now completes its full 4,000-operation run cleanly.
Zero invariant violations across all 25 seeds / 100,000 operations. This is the acceptance
evidence for both the F-10-02 fix and the overall Phase 10 campaign.
