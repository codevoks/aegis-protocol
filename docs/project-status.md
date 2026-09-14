# Aegis — Project Status

**Last updated: 2026-09-14**
**Current phase: Phase 13 — Integration, Security Review and Release — COMPLETE**
**This is the final planned phase. No Phase 14 exists or is planned.**

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
| 1 | Toolchain & repository foundation | ✅ **COMPLETE** | `phase-01-foundation` |
| 2 | State, PDAs & custody primitives | ✅ **COMPLETE** | `phase-02-state` |
| 3 | Collateral flows | ✅ **COMPLETE** | `phase-03-collateral` |
| 4 | Lending, borrowing & interest | ✅ **COMPLETE** | `phase-04-lending` |
| 5 | Oracle | ✅ **COMPLETE** | `phase-05-oracle` |
| 6 | Health, liquidation & bad debt | ✅ **COMPLETE** | `phase-06-liquidation` |
| 7 | Token-2022 Completion | ✅ **COMPLETE** | `phase-07-token2022` |
| 8 | Composability | ✅ **COMPLETE** | `phase-08-composability` |
| 9 | SDK, client & UI | ✅ **COMPLETE** | `phase-09-sdk-ui` |
| 10 | Security campaign | ✅ **COMPLETE** | `phase-10-security` |
| 11 | Performance | ✅ **COMPLETE** | `phase-11-performance` |
| 12 | Governance, upgrades & migrations | ✅ **COMPLETE** | `phase-12-governance` |
| 13 | Integration, security review & release | ✅ **COMPLETE** | `phase-13-release`, `v0.1.0` |

**Phase 3 is complete.** `deposit_collateral`, `withdraw_collateral` (zero-debt path only), and
`close_position` exist on-chain, exactly as frozen in `instruction-catalogue.md` §10/11/20 and
scoped by `docs/phases/phase-03-collateral.md`. Real token custody now moves through the protocol
for the first time — with measured-delta accounting on both SPL Token and Token-2022 transfer-fee
mints. `Market` remains read-only in both collateral instructions, preserving the intra-market
collateral parallelism claim (C2).

**Phase 4 is complete.** `supply`, `withdraw`, `repay` and `accrue_interest` exist on-chain and are
fully functional; `borrow` existed structurally but was hard-gated (`OracleNotYetAvailable`) until
Phase 5. `aegis-math` gained `shares.rs` (virtual-offset share/asset conversions) and `irm.rs`
(utilization, the piecewise-linear rate curve, and third-order Taylor compounding); `state/
market.rs` gained `accrue_view`/`accrue_mut`.

**Phase 5 is complete.** The Phase 3/4 hard gates are removed: `borrow` and the debt path of
`withdraw_collateral` are now real, oracle-validated instructions. `programs/aegis/src/oracle/`
implements the `PriceSource` abstraction and its sole implementer, `pyth::PythPull`, against the
real `pyth-solana-receiver-sdk` 2.0.0 (RV-3/RV-4 resolved, `docs/ecosystem-research.md` §15).
`aegis-math` gained `health.rs` (conservative valuation, health factor, the LTV admissibility
check). `aegis-test-kit` gained `pyth_fixture.rs`, which constructs byte-exact `PriceUpdateV2`
accounts using the real SDK's own `AccountSerialize`/`AccountDeserialize` impls — no mock oracle
provider and no mock program exist anywhere (ADR-0008). Full evidence is in **Phase 5 — evidence**
below; Phase 4's own evidence is preserved unchanged in **Phase 4 — evidence**.

**Phase 6 is complete.** `liquidate`, `absorb_bad_debt` and `withdraw_collateral_fees` exist
on-chain. `aegis-math` gained `liquidation.rs` (close-factor/dust-rule `max_repay`, seizure, bonus,
protocol cut, the collateral clamp with upward-rounded repay recomputation, and the alternate
seize-specified input form — all pure, `no_std`, integer-only). `liquidate` enforces a **strict**
`HF < WAD` liquidatability gate, validates the oracle for both assets before any state write
(mirroring `borrow`'s INV-ORA-07 ordering), and settles exactly per `economic-model.md` §7.3 —
the protocol's cut is taken from the bonus only and stays physically in the collateral vault,
recorded in `market.collateral_fee_accrued`. `absorb_bad_debt` requires `position.collateral_
amount == 0` exactly, reads no oracle, and is structurally unpausable; it burns the protocol's own
`fee_position.supply_shares` before socializing any residual loss across the market's lenders, and
moves no tokens. `withdraw_collateral_fees` lets the admin withdraw only `market.collateral_fee_
accrued`, structurally incapable of reaching user collateral (`A-ADM-02`, the concrete proof of
INV-ADM-01). Full evidence is in **Phase 6 — evidence** below.

**Phase 7 is complete.** RV-5 (the complete current Token-2022 extension list) is closed:
`spl-token-2022-interface` resolves to **2.1.0** in this workspace's `Cargo.lock`, its 27 real
`ExtensionType` variants are enumerated and classified in `docs/token-compatibility.md` §0, and
`Pausable`/`ScaledUiAmount` — the document's own flagged examples of post-2024 extensions — are
both present and correctly classified (Tier C and Tier A respectively). The positive-allowlist
policy engine (`programs/aegis/src/token/policy.rs`), the vault `ImmutableOwner`/exact-sizing
logic (`token/vault.rs`), and measured-delta accounting (`token/transfer.rs`) all already existed
from Phases 2/3 (this repository tested `U-TOK-01/02`, `A-TOK-01..09` early, ahead of their
nominally-assigned phase, exactly as Phase 3's own evidence section already documented) —
Phase 7's genuine new work was closing RV-5 itself, `U-TOK-03`, two supplementary Tier C fixtures
for extensions absent from earlier lists (`Pausable`, `NonTransferable`), a concrete
`ImmutableOwner`-cannot-be-reassigned proof, and the two tests this phase exists for: `A-TOK-10`
(the full protocol lifecycle — supply, deposit, borrow, accrual, liquidation, bad debt, protocol
first-loss, fee withdrawal, lender withdrawal — on a real transfer-fee Token-2022 collateral
market, with INV-CUS-01/INV-CUS-02 asserted after **every** instruction) and `A-TOK-11` (the fee
authority raises the rate mid-lifecycle via the real `SetTransferFee` instruction, respecting
Token-2022's genuine 2-epoch activation delay, and accounting remains exact throughout because
Aegis never caches a fee rate anywhere — it only ever measures `after − before` across each CPI).
No code in `programs/aegis/src` changed in this phase; every change is in `crates/aegis-test-kit`
(new fixtures) and `tests/` (new coverage). Full evidence is in **Phase 7 — evidence** below.

## Component status

| Component | IMPL | TEST | DEMO | DOC | COMMIT |
|---|:--:|:--:|:--:|:--:|:--:|
| `aegis-math` — arithmetic (`mul_div_floor`/`mul_div_ceil`) | ✅ | ✅ | ⬜ | ✅ | ⬜ |
| `aegis-math` — shares | ✅ | ✅ | ✅ | ✅ | ⬜ |
| `aegis-math` — IRM/accrual | ✅ | ✅ | ✅ | ✅ | ⬜ |
| `aegis-math` — health | ✅ | ✅ | ✅ | ✅ | ⬜ |
| `aegis-math` — liquidation | ✅ | ✅ | ✅ | ✅ | ⬜ |
| `programs/aegis` — `ping` (toolchain proof only) | ✅ | ✅ | ⬜ | ✅ | ⬜ |
| `Protocol` / `Market` / `Position` | ✅ | ✅ | ✅ | ✅ | ⬜ |
| `initialize_protocol` / `create_market` / `init_position` | ✅ | ✅ | ✅ | ✅ | ⬜ |
| Vaults & custody | ✅ | ✅ | ✅ | ✅ | ⬜ |
| Token-2022 policy engine | ✅ | ✅ | ✅ | ✅ | ⬜ |
| Collateral instructions (`deposit_collateral`, real oracle-validated `withdraw_collateral`, `close_position`) | ✅ | ✅ | ✅ | ✅ | ⬜ |
| Lend/borrow instructions (`supply`, `withdraw`, `repay`, `accrue_interest`, real oracle-validated `borrow`) | ✅ | ✅ | ✅ | ✅ | ⬜ |
| Oracle (Pyth adapter, `oracle::require_valid_price`, O-1..O-11) | ✅ | ✅ | ✅ | ✅ | ⬜ |
| Liquidation & bad debt (`liquidate`, `absorb_bad_debt`, `withdraw_collateral_fees`) | ✅ | ✅ | ✅ | ✅ | ⬜ |
| Governance & migrations (two-step admin, guardian pause asymmetry, tighten/loosen timelock, `migrate_protocol_v2`) | ✅ | ✅ | ⬜ | ✅ | ⬜ |
| `aegis-test-kit` (mints, market/position lifecycle, user token accounts, invariant checker, borrow-state injection, `pyth_fixture` byte-exact `PriceUpdateV2` construction, `liquidate`/`absorb_bad_debt`/`withdraw_collateral_fees` helpers) | ✅ | ✅ | ✅ | ✅ | ⬜ |
| Invariant fuzzer (`tests/fuzz/`: 2 markets, 6 actors, 9 mutation-validated `[GLOBAL]` invariants, value-creation search) | ✅ | ✅ | ✅ | ✅ | ⬜ |
| CU benchmarks (`tests/bench/`, `benchmarks/cu.json`, `scripts/check-cu-regression.sh`) | ✅ | ✅ | ✅ | ✅ | ⬜ |
| `labs/` (`vault-anchor`/`vault-native`/`vault-pinocchio`/`cu-bench`) | ✅ | ✅ | ✅ | ✅ | ⬜ |
| TypeScript SDK (`@aegis/sdk`: codegen, pda/accounts/math/read/oracle/ix/tx/errors/events) | ✅ | ✅ | ✅ | ✅ | ⬜ |
| Web app (`app/`: market list, position screen, deposit/borrow/repay/withdraw, demo) | ✅ | ✅ | ✅ | ✅ | ⬜ |
| Liquidator bot (`bots/liquidator`) | ✅ | ⬜ | ⬜ | ✅ | ⬜ |

`COMMIT` columns above turn ✅ only once this phase's commit and tag are pushed and verified against
the remote — see **Git** at the end of this document.

**Phase 13 reconciliation of the two rows above (both were stale, corrected without any code
change):**
- **Governance & migrations** was recorded `⬜/⬜/⬜` despite Phase 12 having shipped and tested
  eight real instructions with 37 passing tests (`phase12_admin_transfer.rs`/`phase12_migration.rs`/
  `phase12_params.rs`/`phase12_pause.rs`) — a bookkeeping gap from Phase 12 not having updated this
  table, not a gap in the work itself. Corrected to `✅/✅/⬜` — DEMOED stays honestly ⬜: no runnable
  demo exercises pause/governance end to end (the mandatory `zero-cost-demo.md` §5 scenario does not
  call for one, and none exists separately).
- **Liquidator bot** was recorded `⬜` for IMPLEMENTED, which understated it: `bots/liquidator/src/`
  is real, non-trivial TypeScript (scanning, health computation, direct and callback liquidation
  transaction builders) that compiles cleanly (`npm run build`, verified this phase). Corrected to
  IMPLEMENTED ✅. TESTED and DEMOED remain honestly ⬜: there is no automated test suite (no `test`
  script in `package.json`), and its `demo`/`perf-c1` scripts require a live local Surfpool
  validator, excluded from the offline `make test`/`make demo` path by design
  (`docs/zero-cost-demo.md` §3) — `performance-strategy.md`'s PERF-C1 already cites a real run of
  `perf-c1` as evidence for that one, narrow contention claim, which is not the same as the bot
  itself being tested or demoed as a liquidation keeper.

## Invariant status

87 invariants defined across 12 groups (9 marked **[GLOBAL]**). Phase 1 tested none of the 87
numbered invariants (there was no protocol yet). Phase 2 is the first phase to test numbered
invariants from `docs/invariants.md` §I (state lifecycle) and §J (administrative safety), plus the
account-model-local invariant catalogue in `account-model.md` §11:

| ID | Tested by |
|---|---|
| INV-LIFE-01 | `A-LIFE-01` (`reinitializing_protocol_fails`, `reinitializing_market_fails`, `reinitializing_position_fails`) |
| INV-LIFE-04 | `scripts/check-no-close.sh` (new CI-NOCLOSE guard) |
| INV-LIFE-05 | `A-LIFE-03` (`non_canonical_bump_is_rejected`) and every `assert_eq!(x.bump, expected_bump)` in `tests/phase2_state.rs` |
| INV-LIFE-06 | `U-LIFE-02` (`seed_prefixes_are_pairwise_distinct`) |
| INV-ADM-05 | `A-ADM-04` (`out_of_bounds_market_parameters_are_rejected`) |
| INV-LIQ-06 | `A-ADM-04`'s derived-bound case, plus `derived_liquidation_bound_rejects_plausible_but_unsafe_params` (Tier 1, `aegis` crate) |
| INV-ACCT-01..07, 09 (`account-model.md` §11) | `tests/phase2_state.rs`, `tests/phase2_adversarial.rs` — see the Phase 2 evidence section below for the exact mapping |

**INV-ACCT-08** (`deposit_collateral`/`withdraw_collateral` do not declare `Market` writable) —
the Phase 2 deferral above is now **resolved**: both instructions exist and `A-PAR-01`
(`tests/phase3_adversarial.rs::market_is_not_writable_in_collateral_instructions`) asserts
`is_writable == false` on the `market` entry of each instruction's actual generated
`Vec<AccountMeta>`, not merely by source inspection.

Phase 3 is the first phase to test numbered invariants from `docs/invariants.md` §B (token
custody), plus `account-model.md`'s local `INV-RES-02`:

| ID | Tested by |
|---|---|
| INV-CUS-02 **[GLOBAL]** | `I-CUS-02` (`custody_invariant_holds_across_multiple_positions`) and `aegis_test_kit::invariants::assert_inv_cus_02`, called after every state-changing step in `tests/phase3_collateral.rs` and the Phase 3 demo |
| INV-CUS-05 | `U-TOK-02` (`token2022_transfer_fee_deposit_credits_net_of_fee`) — credited is the measured post-`reload()` delta, proven to differ from the requested amount on a fee mint |
| INV-CUS-06 | Every deposit/withdrawal test — `transfer_checked` is the only transfer primitive in `token/transfer.rs`; `A-CUS-06` (`deposit_rejects_wrong_mint`) proves the pinned-mint check fires |
| INV-CUS-07 | `A-TOK-08`/`A-TOK-09` (`wrong_token_program_for_spl_market_is_rejected`, `wrong_token_program_for_token2022_market_is_rejected`) |
| INV-CUS-08 | `A-CUS-08` (`direct_donation_is_never_credited`) plus `assert_inv_cus_02_detects_uncredited_donation`, which proves the checker itself would catch the resulting mismatch |
| INV-AUTH-02 | `A-AUTH-02` (`non_owner_withdraw_fails`) |
| INV-AUTH-03 | `A-AUTH-03` (`deposit_by_non_owner_succeeds`), together with `A-AUTH-02` for the asymmetric owner-required side |
| INV-LIFE-02 | `U-LIFE-01` (`close_position_requires_exact_zero_balances`) |
| INV-LIFE-03 | `A-LIFE-02` (`closed_position_cannot_be_revived_with_stale_data`) |
| INV-RES-02 (`account-model.md` §8 / `invariants.md` §L) | `A-PAR-01` (`market_is_not_writable_in_collateral_instructions`) |
| INV-CUS-04 (code-review/grep, assigned Phase 13 but exercisable now against the two paths that exist) | `A-CUS-04` — `scripts/check-collateral-transfer-paths.sh` (new CI-CUSTODY-PATHS guard) |

`INV-CUS-05` and `INV-CUS-08` are formally assigned to Phases 7 and 4 respectively in
`docs/invariants.md`'s per-phase column, but `docs/phases/phase-03-collateral.md`'s own test list
requires `U-TOK-02` and `A-CUS-08` in Phase 3 — both mechanisms (Token-2022 vaults, measured-delta
crediting) already exist from Phase 2/3, so this phase exercises them early rather than waiting for
their nominally-assigned phase. This is recorded here as intentional early coverage, the same way
Phase 2 recorded its `INV-ACCT-*`/`INV-ACC-*` naming overlap — not a frozen-document conflict, and
nothing in either document was edited.

**Naming note:** `account-model.md` §11 defines a 9-item "Account-model invariant summary" using an
`INV-ACCT-*` prefix, distinct from `invariants.md`'s own master `INV-ACC-*` series (accounting,
§C) — the two documents use similar-looking prefixes for different, only partially overlapping
content (e.g. `INV-ACCT-09` and `INV-ACC-11` both concern `_reserved` bytes being zero). Phase 2's
task instructions referenced "`INV-ACCT-01..09`", which only exist in `account-model.md` §11; all
nine are addressed above/below. Nothing in either document was edited to resolve this — it is
noted here as a cross-reference clarification, not a frozen-document conflict.

Phase 4 is the first phase to test numbered invariants from `docs/invariants.md` §C (accounting)
and the Phase-4-assigned rows of §B/§E/§F:

| ID | Tested by |
|---|---|
| INV-CUS-01 **[GLOBAL]** | `I-CUS-01` (`i_cus_01_holds_after_every_operation`) and `aegis_test_kit::invariants::assert_inv_cus_01`, called after every state-changing step across `tests/phase4_lending.rs` and the Phase 4 demo; falsifiability proven the same way Phase 3 proved INV-CUS-02's (`loan_vault_direct_donation_is_never_credited` observes the checker fail after an uncredited donation) |
| INV-ACC-01 **[GLOBAL]** | `assert_inv_acc_01` (incl. `fee_position`), called via `assert_all_lending` throughout `tests/phase4_lending.rs` |
| INV-ACC-02 **[GLOBAL]** | `assert_inv_acc_02`, same call sites |
| INV-ACC-03 **[GLOBAL]** | `assert_inv_acc_03`, same call sites |
| INV-ACC-04 | `P-ACCRUE-2` (`p_accrue_2_free_liquidity_invariant_under_accrual`, `state/market.rs`) |
| INV-ACC-06 | `assert_inv_acc_06`, same call sites |
| INV-ACC-07 | `U-IRM-05` (`taylor_x_is_a_plain_product_of_rate_and_elapsed_seconds`, `aegis-math`) plus `accrue_view`'s `now.saturating_sub(...).max(0)` clamp, which structurally prevents `last_accrual_ts` from ever moving backward or past `now` |
| INV-ACC-08 | `P-ACCRUE-1` (`p_accrue_1_view_and_mut_agree`, `state/market.rs`) |
| INV-ACC-09 | `overflow-checks = true` (CI-OVERFLOW) plus `P-ARITH-2` (Phase 1) — every Phase 4 arithmetic op is `checked_*`/`mul_div_*`, none uses a wrapping op |
| INV-BOR-02 | `U-BORROW-01` (`u_borrow_01_free_liquidity_bound`, `instructions/borrow/borrow.rs`) |
| INV-BOR-03 | `U-ROUND-03` (`round_03_borrow_assets_borrow_shares_minted_ceils`, `aegis-math`) |
| INV-BOR-05 | `U-GUARD-01` (`guard_01_both_zero_is_rejected`) |
| INV-REP-01 | `repay.rs`'s `Repay` account list contains no price-update field at all (structural), plus `A-ORACLE-02` (`a_oracle_02_repay_succeeds_with_broken_oracle`, Phase 5) proving it live against a broken oracle |
| INV-REP-02 | `repay.rs` declares no pause check anywhere (structural; Phase 12 must not add one) |
| INV-REP-03 | `U-REPAY-01` (`repay_clamps_to_actual_debt_never_pulls_excess`) |
| INV-REP-04 | `U-ROUND-04` (`round_04_repay_assets_borrow_shares_burned_floors`, `aegis-math`) |
| INV-REP-05 | `U-REPAY-02` (`full_repayment_via_shares_leaves_no_dust`) |

`INV-CUS-08`, formally assigned to Phase 4 in `docs/invariants.md`'s per-phase column, is tested on
the loan side by `loan_vault_direct_donation_is_never_credited` (`tests/phase4_adversarial.rs`),
mirroring Phase 3's collateral-side `A-CUS-08`.

Phase 5 is the first phase to test numbered invariants from `docs/invariants.md` §G (oracle) and
the Phase-5-assigned rows of §E/§D:

| ID | Tested by |
|---|---|
| INV-ORA-01 | Every `A-ORACLE-03..12` test (`tests/phase5_oracle_adversarial.rs`) — each violates one O-check and asserts the specific resulting `AegisError` |
| INV-ORA-02 | `A-ORACLE-01` (`a_oracle_01_deposit_collateral_succeeds_with_broken_oracle`), `A-ORACLE-02` (`a_oracle_02_repay_succeeds_with_broken_oracle`) — the two mandatory positive safety tests |
| INV-ORA-03 | `U-HEALTH-01`/`U-HEALTH-02` (`crates/aegis-math/src/health.rs`) — collateral valued at `lo` floored, debt at `hi` ceiled |
| INV-ORA-04 | `A-ORACLE-07` (`a_oracle_07_wrong_feed_id_is_rejected`) |
| INV-ORA-05 | `A-ORACLE-12` (`a_oracle_12_same_account_for_both_feeds_is_rejected`) |
| INV-ORA-06 | `oracle::pyth::PythPull::read_price` uses `Clock::get()?.unix_timestamp`/`price.publish_time` exclusively; `scripts/check-no-slot-time.sh` (CI-NOSLOT) greps `programs/aegis/src` for `Clock` slot usage |
| INV-ORA-07 | `A-ORACLE-13` (both variants: `a_oracle_13_failed_oracle_check_leaves_state_byte_identical`, `a_oracle_13_failed_withdraw_collateral_oracle_check_leaves_state_byte_identical`) — full before/after account-data and lamport snapshots, not merely "the instruction returned an error" |
| INV-BOR-01 | `A-ORACLE-03` (`a_oracle_03_stale_oracle_blocks_borrow_and_debt_withdraw_but_not_repay_or_deposit`) — a stale (invalid) oracle causes `borrow` to fail closed |
| INV-SOLV-01 | `debt_bearing_withdraw_rejects_unsafe_post_withdraw_health` / `debt_bearing_withdraw_succeeds_when_post_withdraw_health_is_safe` (`withdraw_collateral`'s debt path) and `borrow.rs`'s own LTV `require!` (both call sites of `aegis_math::is_within_max_ltv`) |

`INV-ORA-01..07`, `INV-BOR-01` and `INV-SOLV-01` are all formally assigned to Phase 5 in
`docs/invariants.md`'s per-phase column; every one is mapped above to a concrete, currently-passing
test. `INV-SOLV-01` is marked **[GLOBAL]** in `invariants.md` — Phase 10's fuzzer is the eventual
per-instruction-sequence check; Phase 5 exercises it directly at both of its two call sites
(`borrow`, debt-path `withdraw_collateral`).

Phase 6 is the first phase to test numbered invariants from `docs/invariants.md` §H (liquidation)
and the Phase-6-assigned rows of §D (solvency) and §J (administrative safety), plus `INV-CUS-09`
and `INV-RES-03`:

| ID | Tested by |
|---|---|
| INV-LIQ-01 | `U-LIQ-02` (`liquidation::tests::u_liq_02_hf_equal_to_wad_is_not_liquidatable`, `tests/phase6_liquidation.rs::u_liq_02_hf_equal_to_wad_is_not_liquidatable_strict`) — `HF == WAD` rejected, `HF == WAD - 1` accepted |
| INV-LIQ-02 | `P-LIQ-2` (`p_liq_2_seizure_never_exceeds_collateral`), `seizure_never_exceeds_collateral_amount`, `clamp_boundary_exact_equality_does_not_clamp` |
| INV-LIQ-03 | `repay_never_exceeds_debt` (`aegis-math`), `U-LIQ-03` (`u_liq_03_and_05_collateral_clamp_and_full_seizure_with_remaining_debt`) |
| INV-LIQ-04 | `U-LIQ-04` (`u_liq_04_dust_rule_forces_full_repayment`, `dust_rule_boundary_exactly_at_min_debt_vs_one_below` in `aegis-math`; `u_liq_04_dust_rule_forces_full_repayment_on_chain`, `u_liq_04_dust_rule_boundary_forces_full_when_remaining_would_be_below_min_debt` on-chain) |
| INV-LIQ-05 | `P-LIQ-1` (`p_liq_1_health_improves_above_the_derived_threshold`) |
| INV-LIQ-06 | `derived_liquidation_bound_rejects_plausible_but_unsafe_params` (Phase 2, unchanged) plus `P-LIQ-4` (`p_liq_4_reference_params_satisfy_full_liq_hf_bound`) |
| INV-LIQ-07 | `U-LIQ-01` (`u_liq_01_worked_liquidation_example`, `u_liq_01_worked_example_on_chain` — exact §7.5 figures), `protocol_cut_never_exceeds_bonus_and_never_touches_base_seize` |
| INV-LIQ-08 | `P-LIQ-3` (`p_liq_3_liquidation_is_profitable_when_not_clamped`) |
| INV-LIQ-09 | `U-LIQ-06` (`u_liq_06_total_supply_assets_never_rises`) |
| INV-SOLV-02 | `A-LIQ-01` (`a_liq_01_healthy_position_cannot_be_liquidated_and_nothing_mutates`) — full before/after byte-exact snapshots of market, position, both vaults, and every wallet ATA involved |
| INV-SOLV-03 | `U-BD-01` (`u_bd_01_nonzero_collateral_is_rejected`, `u_bd_01_zero_borrow_shares_is_rejected`) |
| INV-SOLV-04 | `absorb_bad_debt_moves_no_tokens_and_preserves_vault_reconciliation`, `assert_inv_cus_01` called after every `U-BD-02` scenario |
| INV-SOLV-05 | `I-ISO-01` (`i_iso_01_bad_debt_in_market_a_leaves_market_b_completely_untouched`) — full byte-exact snapshots of Market B's market/position/vault accounts before and after Market A's liquidation and bad-debt absorption |
| INV-SOLV-06 | `U-BD-02` (`u_bd_02_fee_shares_exceed_loss_fully_absorbed_by_protocol`, `u_bd_02_fee_shares_short_residual_is_socialized`, plus the exact/one-unit-short/no-fee-shares boundary variants) |
| INV-SOLV-07 | `U-BORROW-02` (Phase 4, unchanged) plus `U-LIQ-04`'s dust-rule tests (the liquidation-side enforcement) |
| INV-ADM-01 | `A-ADM-02` (`a_adm_02_admin_cannot_withdraw_user_collateral`) — both a `protocol_cut + 1` attempt and a whole-vault-balance attempt, with user collateral and the custody invariant proven byte/value-unchanged |
| INV-ADM-08 | `A-ADM-02` (same test) plus `withdraw_collateral_fees_happy_path`'s positive-path bound check |
| INV-CUS-09 | `U-LIQ-01`'s exact `collateral_fee_accrued` figure (§7.5: `47_477_848`) and `A-ADM-02`'s real-liquidation-derived `protocol_cut` — `collateral_fee_accrued` is never written anywhere outside `liquidate` (grep-verifiable: `programs/aegis/src/instructions/liquidate/liquidate.rs` and `admin/withdraw_collateral_fees.rs`, which only decrements it, are the only two write sites) |
| INV-RES-03 | `A-PAR-02` (`a_par_02_no_writable_account_shared_between_two_markets`) — the writable-account sets of `Liquidate`/`AbsorbBadDebt`/`WithdrawCollateralFees` for two independent markets are asserted disjoint via their actual generated `Vec<AccountMeta>`, not by source inspection |

`INV-LIQ-01..09`, `INV-SOLV-02..07`, `INV-ADM-01/08`, `INV-CUS-09` and `INV-RES-03` are all
formally assigned to Phase 6 in `docs/invariants.md`'s per-phase column; every one is mapped above
to a concrete, currently-passing test.

Phase 7 is the first phase to formally close the one `invariants.md` row assigned to it:

| ID | Tested by |
|---|---|
| INV-CUS-05 | `U-TOK-02` (Phase 3, early coverage — unchanged) plus `tests/phase7_token2022.rs`'s `a_tok_10_full_lifecycle_on_transfer_fee_collateral_market` and `a_tok_11_fee_rate_change_mid_lifecycle_does_not_break_accounting`, which assert measured-delta crediting holds across an entire multi-instruction lifecycle and across a real fee-rate change, not merely at a single deposit |

`INV-CUS-06` and `INV-CUS-07` (also Token-2022-relevant per `docs/invariants.md`'s own text) are
formally assigned to Phase 3 and were already closed there (`A-CUS-06`, `A-TOK-08`/`A-TOK-09`);
Phase 7 does not re-assign them, and the phase-7 task's own instructions requesting them "fully
tested" are satisfied by that existing, unchanged Phase 3 evidence — re-running it is part of this
phase's regression pass (§17 below). **Note:** `docs/invariants.md` has no `INV-TOK-*` series; the
only Token-2022-relevant invariants in the frozen 87 are the `INV-CUS-05/06/07` rows above. Any
reference to `INV-TOK-*` is a naming assumption to flag, not a document to invent additions into.

Phases 8 and 9 each closed the one row `docs/invariants.md` assigns them (`INV-AUTH-07`/`INV-RES-07`
via Phase 8's callback design, `INV-RES-06` via Phase 9's `I-TX-01` — see each phase's own evidence
section below for the exact mapping). **0 of the 87 numbered invariants assigned to Phases 10-13**
are implemented or tested yet — expected at this point; see `docs/invariants.md` for the full
per-phase assignment.

---

## Environment — measured 2026-09-06 (Phase 1)

| Tool | Version | Status |
|---|---|---|
| `rustc` / `cargo` | 1.98.1 | ✅ upgraded from 1.88.0 (required — see delta below) |
| `solana` (Agave CLI) | 3.1.10 (active) | ✅ upgraded from 2.2.21; see delta below |
| `avm` | 1.1.2 | ✅ installed |
| `anchor` | 1.2.0 | ✅ installed |
| `surfpool` | 1.5.0 | ✅ installed |
| `node` | v22.12.0 | ✅ unchanged |
| Git repository | initialized, remote `origin` present | ✅ |

Raw output:

```
$ rustc --version && cargo --version
rustc 1.98.1 (48a229cea 2026-09-01)
cargo 1.98.1 (797e8a9bc 2026-08-05)

$ solana --version
solana-cli 3.1.10 (src:7bc9c805; feat:1620780344, client:Agave)

$ avm --version && anchor --version
avm 1.1.2
anchor-cli 1.2.0

$ surfpool --version
surfpool 1.5.0

$ node --version
v22.12.0
```

**Delta from `docs/ecosystem-research.md`'s recorded starting state, and why** (full detail in that
document §12):

1. **`rustc` had to be upgraded from 1.88.0 to 1.98.1.** `cargo install --git
   https://github.com/solana-foundation/anchor avm --force` failed outright with `rustc 1.88.0 is not
   supported ... requires rustc 1.91` (a `cargo-platform` transitive dependency's own MSRV). This
   document previously called 1.88.0 "Adequate" — that was wrong for building `avm`/`anchor` from
   source, and is now corrected here. Fixed via `rustup update stable`.
2. **The active Solana CLI ended up on 3.1.10, not the 4.2.2 initially installed.** The Agave stable
   installer (`release.anza.xyz/stable/install`) was used first and correctly reported **4.2.2** (the
   current Agave *validator* release line). Running `anchor build` then downloaded and switched the
   active release to **3.1.10** on its own — the version Anchor 1.0.2's own installation docs list as
   verified, and the *Solana CLI/SDK developer-tooling* line that `docs/ecosystem-research.md` §3
   already distinguished from the validator-client line. Both numbers are real and both are recorded;
   3.1.10 is what `anchor build` actually built against.
3. **The workspace's declared `rust-version` (in `[workspace.package]`) is pinned to `1.85.0`**,
   *below* the host rustc (1.98.1). This is intentional, not an oversight: `cargo-build-sbf`
   cross-compiles the on-chain target with a separate rustc bundled inside Solana's platform-tools
   (`1.95.0-dev` at the time of writing), and it enforces the crate's declared MSRV against *that*
   compiler, not the host one. Declaring `1.98.1` broke `anchor build` with `rustc 1.95.0-dev is not
   supported ... requires rustc 1.98.1` even though nothing was wrong with the code. See
   `docs/ecosystem-research.md` §12.4 for the full explanation.
4. **`anchor-cli` resolved to 1.2.0**, one minor version ahead of the 1.1.2 this document originally
   recorded (dated 2026-09-04; crates.io's `anchor-lang` `max_stable_version` was 1.2.0 as of
   2026-09-05). Not an architectural change — same Anchor 1.x breaking-change set already documented.
5. **RV-1 resolved.** The full `Cargo.lock` for this workspace was inspected directly (109 distinct
   `solana-*` name/version pairs). Headline finding: several crates coexist at two major versions at
   once mid-rename — most notably `solana-pubkey` (3.0.0 **and** 4.2.1, where 4.2.1 is a pure
   `pub use solana_address::Address as Pubkey;` re-export) and `solana-address` (1.1.0 **and** 2.6.1).
   `solana-transaction` resolved to 4.1.6, whose `VersionedTransaction::try_new` is gated by a feature
   named **`wincode`**, not `bincode` as in the 3.x line. Full detail, including the extraction command
   and the practical rule this forces (`aegis-test-kit` and the workspace root must depend on the same
   major line LiteSVM itself declares for these crates), is in `docs/ecosystem-research.md` §12.3.
6. **RV-2 resolved.** The current Mollusk crate is `mollusk-svm` (not `mollusk`), current stable
   **0.15.1**, at `github.com/anza-xyz/mollusk`, confirmed via crates.io's own registry metadata.
   Mollusk is **not used in Phase 1** (Tier 2 of the test pyramid begins at Phase 2+, per
   `docs/testing-strategy.md` §3) — this closes the open research question, it does not add a Phase 1
   dependency.
7. **The on-chain build target is `sbpfv3-solana-solana`**, not the historically-remembered
   `bpfel-unknown-unknown`. Observed directly in `target/` after `anchor build`.

None of these deltas invalidate any Phase 0 architectural decision (ADR-0001/0002/0009/0010 all still
hold exactly as written); all are toolchain/version-plumbing facts now recorded so later phases do not
rediscover them.

---

## Open research gates

| ID | Question | Gate phase | Status |
|---|---|---|---|
| RV-1 | Resolved `solana-*` crate versions under `anchor-lang 1.1.2` | 1 | ✅ **RESOLVED** — see above and `docs/ecosystem-research.md` §12.3 (now under `anchor-lang 1.2.0`, the current stable) |
| RV-2 | Current Mollusk crate/version and CU API | 1 | ✅ **RESOLVED** — `mollusk-svm` 0.15.1; CU API not yet exercised (first used Phase 2+) |
| RV-3 | Upgraded Pyth receiver program ID (post 2026-08-26) | 5 | ✅ **RESOLVED** — unchanged at `rec5EKMGg6MxZYaMdyBfgwp4d5rB9T1VQH5pJv5LtFJ`; see `docs/ecosystem-research.md` §15.1 |
| RV-4 | `VerificationLevel` shape in `pyth-solana-receiver-sdk` 2.x | 5 | ✅ **RESOLVED** — `enum { Partial { num_signatures: u8 }, Full }`; see `docs/ecosystem-research.md` §15.2 |
| RV-5 | Complete current Token-2022 extension list and discriminants | 7 | ✅ **RESOLVED** — `spl-token-2022-interface` 2.1.0; see `docs/token-compatibility.md` §0 |
| RV-6 | Does the runtime permit `A → B → A` CPI reentrancy? | 8 | OPEN |
| RV-7 | SIMD-0296 (4096-byte tx) availability and `@solana/kit` support | 9 | ✅ **RESOLVED** — see `docs/ecosystem-research.md` §17: active on testnet/devnet and on local Surfpool ≥1.5.0 since 2026-08-24 (Aegis's actual target cluster), pending mainnet (epoch 1035, ≈2026-09-15); `@solana/kit` 8.0.0+ supports it. Aegis designs to the classic 1232-byte limit regardless (`I-TX-01`) |
| RV-8 | Current Jupiter integration surface | 8 | OPEN |

## Known issues

- The machine used for implementation has limited local disk space; toolchain installation
  (Rust upgrade, `avm`/`anchor` built from source, Solana platform-tools download) transiently drove
  free space to ~127 MiB, which the operator resolved by clearing two unrelated caches
  (`~/.cache/solana`, `~/.cache/codex-runtimes`) with explicit permission. This is a local-environment
  fact, not a repository defect — `docs/zero-cost-demo.md`'s NFR-4 is about **network/paid-service**
  independence, not disk footprint, and nothing in this phase depends on the specific disk state of
  the implementation machine.
- `anchor build`'s own tooling silently switches the active Solana CLI release (see Environment delta
  #2 above). A future session should not be surprised if `solana --version` reports a different number
  after running `anchor build` than it did before.

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

No ADR was added or changed in Phase 1. Every deviation encountered (toolchain versions, feature
names, target triples) was a verified implementation detail, not an architectural one — see the
Environment section above and `docs/ecosystem-research.md` §12 for the full reasoning trail.

No ADR was added or changed in Phase 2 either. Two implementation-level API deltas from what
`ecosystem-research.md` had verified in Phase 1 are recorded in that document's new §14 (crate
names `spl-token-interface`/`spl-token-2022-interface`, not `spl-token`/`spl-token-2022`; LiteSVM
ships real embedded SPL Token / Token-2022 program bytecode; `CpiContext::new` takes a `Pubkey`,
not an `AccountInfo`) — none of them change a Phase 0 architectural decision.

No ADR was added or changed in Phase 5. ADR-0008 (oracle abstraction; deterministic prices via
fixture injection, no mock program) is implemented exactly as written — RV-3/RV-4 resolved without
invalidating it (`docs/ecosystem-research.md` §15.3). One documentation finding, not a design
deviation: `economic-model.md` §6.5's worked-example prose states the two health factors as
"1.330838..." and "0.842495..."; the correct values (from the document's own formulas and input
numbers, independently cross-checked with exact rational arithmetic) are `1.330400586549357...`
and `0.84249816703326...`. Investigated per `AGENTS.md`'s "investigate formula/rounding first"
guidance before writing `U-HEALTH-01`/`U-HEALTH-02` — recorded here, not silently worked around;
see `crates/aegis-math/src/health.rs`'s test doc comments for the full reasoning. The frozen
document was not edited (a manual-arithmetic transcription slip in prose, not a formula or
rounding-direction disagreement).

One guard script updated: `scripts/check-collateral-transfer-paths.sh`'s outbound allowlist
gained `programs/aegis/src/instructions/borrow/borrow.rs` (the real `borrow` now legitimately
transfers `loan_vault → owner`, account-model.md §6.3's fourth custody path) and the Phase 4-era
"`borrow.rs` must never call a transfer helper while hard-gated" check was removed, since the gate
it protected no longer exists. This is the check tracking reality, not a weakening — the six-path
custody enumeration in `account-model.md` §6.3 is unchanged.

No ADR was added or changed in Phase 6. `liquidate`, `absorb_bad_debt` and
`withdraw_collateral_fees` implement `economic-model.md` §7-8 and `instruction-catalogue.md`
§17-19 exactly as frozen; no implementation surprise required deviating from either document. One
more guard-script update, same pattern as Phase 5's: `scripts/check-collateral-transfer-paths.sh`'s
allowlists gained `programs/aegis/src/instructions/liquidate/liquidate.rs` (both directions — the
only instruction that moves tokens both ways) and
`programs/aegis/src/instructions/admin/withdraw_collateral_fees.rs` (outbound), completing all six
of `account-model.md` §6.3's enumerated custody paths. `liquidate`'s test-kit transaction builder
needed a higher compute-unit budget than `borrow`/`repay`/`withdraw_collateral`
(`LIQUIDATE_COMPUTE_UNIT_LIMIT = 800_000` vs. `HIGHER_COMPUTE_UNIT_LIMIT = 400_000`, measured
directly against the collateral-clamp path during authoring) — a resource-allocation fact recorded
the same way Phase 5 recorded `borrow`'s own CU finding, not a security-relevant change; `INV-RES-01`
remains explicitly Phase 11 (Performance) scope.

No ADR was added or changed in Phase 7, and no frozen document's *classification* changed — RV-5's
resolution filled in previously-open/pending rows of `token-compatibility.md` (an update the
document's own header explicitly called for: "Research gate RV-5 must be closed in Phase 7") and
added two rows (`ConfidentialMintBurn`, plus the §0.2 account-level completeness table) that were
simply absent from the pre-Phase-7 table by name, not reclassified. No program code in
`programs/aegis/src` changed: the positive-allowlist policy engine, `ImmutableOwner` vault sizing,
and measured-delta transfer accounting were already complete from Phases 2/3, and this phase's own
adversarial self-audit (§19 below) confirmed no extension-specific carve-out exists anywhere in
`token/policy.rs` that could have silently broadened acceptance. Aegis's supported Token-2022
surface at the end of Phase 7 is identical to its surface at the end of Phase 3.

**Phase 8 is complete.** `liquidate` gained an **optional** callback
(`callback_program`/`callback_collateral_account`/`callback_data`) so a liquidator can seize
collateral, swap it via an untrusted external program, and repay — all in one transaction, with no
pre-funding (`docs/composability.md`, `docs/phases/phase-08-composability.md`). One new `Market`
field (`liquidation_guard: u8`, one byte moved out of `_reserved`, `Market::LEN` unchanged) backs a
per-market reentrancy guard, checked unconditionally at the top of `liquidate` and set only around
the callback CPI (ADR-0013). The callback CPI's account list is built explicitly by a standalone,
unit-tested function (`build_callback_instruction`): six fixed accounts plus the liquidator's own
`remaining_accounts` (rejected outright if any alias `market`/`liquidator`/`position`/
`fee_position`/`collateral_vault`/either liquidator ATA), dispatched with plain `invoke` — never
`invoke_signed` — so neither `market` nor `liquidator`'s `AccountInfo` ever reaches it
(`INV-AUTH-07`). Omitting the callback runs the exact, unmodified Phase 6 code path
(`I-LIQ-CB-02`); the callback branch is new code that seizes collateral into a caller-supplied
account, invokes the callback, reloads `loan_vault`, and requires the measured delta to **meet or
exceed** — not equal exactly — the required repayment (a deliberate, documented exception to
INV-CUS-01's exact-equality form for this one branch, `docs/invariants.md` §B, ADR-0013 §1 point
3: a real external swap essentially never lands on the exact wei amount required, and any surplus
is ordinary unaccounted vault surplus, the same shape as INV-CUS-08's donation case). Two research
gates were closed with primary sources before any callback code was written: **RV-6** (the current
Agave runtime's own `InvokeContext::push()` rejects indirect `A → B → A` CPI reentrancy with
`InstructionError::ReentrancyNotAllowed`, independent of and not relied upon by Aegis's own guard)
and **RV-8** (Jupiter's current integration surface is the Swap API's `quote` → `swap-instructions`
HTTP flow against the real mainnet program `JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4`, reachable
only from the optional, network-tagged tier) — both in `docs/ecosystem-research.md` §16. Two new
`labs/` programs exist solely to prove the design: `example-liquidator` (a deterministic, honest
callback, `I-LIQ-CB-01`) and `hostile-callback` (one program, four attack modes, `A-CPI-01..04`,
each proven to fail **atomically** — zero state diff, not merely a returned error). A TypeScript
keeper (`bots/liquidator/`, `@solana/kit` + `@anchor-lang/core`'s coder only, ADR-0011) scans,
estimates health off-chain (advisory only), and drives both the direct and callback liquidation
paths; its required local demo runs against a real, local, **non-forking** Surfpool validator with
zero network. The optional `N-JUP-01` network test genuinely executes the real Jupiter Swap API
call and validates its response shape; the further step of running the resulting instruction
on-chain against a Surfpool mainnet fork was not attempted (documented as **NOT RUN**, not faked)
— see §9 of the evidence below for exactly why. A pre-existing, uncommitted local artifact
inconsistency (this checkout's `target/deploy/aegis-keypair.json` did not match `declare_id!`) was
found and fixed during this phase because it blocks *any* real on-chain interaction with the
deployed program, not merely the TS demo specifically — see §10.

**Phase 9 is complete.** No on-chain change: `programs/aegis` is byte-for-byte unmodified from
Phase 8, confirmed by the full offline Rust regression (§17 below) passing unchanged. **RV-7**
(SIMD-0296 / the 4096-byte "v1" transaction format) is closed with primary sources
(`docs/ecosystem-research.md` §17): active on testnet/devnet and already available on local
Surfpool ≥1.5.0 (Aegis's actual target cluster) since 2026-08-24, pending mainnet activation at
epoch 1035 (≈2026-09-15); `@solana/kit` 8.3.0 (the version this workspace installs) supports it
fully. Per the phase's own non-negotiable requirement, none of this is used — every transaction
this SDK builds is measured and asserted against the classic 1232-byte legacy limit (`I-TX-01`,
`INV-RES-06`), with no address lookup table anywhere. `sdk/ts/` (`@aegis/sdk`) is built entirely on
`@solana/kit` 8.3.0 and `@anchor-lang/core` 1.2.0 — grep-verified to contain no direct
`@coral-xyz/anchor` or `@solana/web3.js` import anywhere in its own source or the app's; the one
`@solana/web3.js` dependency `npm ls` shows is an unavoidable transitive dependency *of*
`@anchor-lang/core` itself (its Borsh coder still produces/consumes legacy `PublicKey`/`BN`-shaped
values internally), the identical pre-existing situation Phase 8's `bots/liquidator/` already has.
IDL codegen (`sdk/ts/scripts/codegen.mjs`) derives every generated artifact
(`sdk/ts/src/generated/{idl,types,accounts,instructions,events,errors}.ts`) — discriminators, field
layouts, and per-instruction account-meta order/writability/signer/optionality — directly from
`target/idl/aegis.json` (Anchor 1.x's Program Metadata layout); nothing is hand-duplicated, and
`ixEngine.ts`'s one generic `buildGenericInstruction` function (used by every generated
`build<Instruction>Instruction`) assembles a correctly-ordered, correctly-flagged account-meta list
from that schema alone — a genuine improvement over Phase 8's necessarily hand-written
per-instruction account arrays (`bots/liquidator/src/txBuilders.ts`), which existed before this
codegen did. `npm run codegen:check` / `make codegen-check` is proven (not merely asserted) to
detect a stale commit by tampering with a generated file and observing the check fail, then
restoring it and observing the check pass again (§17 below).

Cross-language math parity (**I-SDK-01**) uses the identical mechanism `docs/phases/phase-09-sdk-ui.md`
requires: a new Rust binary, `crates/aegis-test-kit/examples/phase9_vectors_dump.rs` — TEST CODE
ONLY, no changes to `programs/aegis` or `crates/aegis-math` — calls the real, frozen `aegis-math`
functions directly and emits `tests/vectors/{fixed,shares,irm,health,liquidation,pdas}.json`;
`sdk/ts/test/vectors.test.ts` asserts `sdk/ts/src/math.ts` (bigint throughout, no `Number` in any
protocol-critical path) against those committed vectors bit-for-bit — 48 assertions, zero
hand-typed expected values. `make vectors-check` / `scripts/check-vectors.sh` is proven the same
way `check-codegen.mjs` is: tampering with a committed vector file makes it fail, restoring it
makes it pass. **I-SDK-03** (PDA parity) reuses the exact `Pubkey::find_program_address`-based
helpers every Rust test in this repository already calls (`crates/aegis-test-kit/src/market.rs`)
to emit `tests/vectors/pdas.json`, including two `config_id` values (`1` and `256`) chosen
specifically so a `u16` little-endian-vs-big-endian seed-encoding mistake in `sdk/ts/src/pda.ts`
would derive a *different* market address, not merely a wrong one — `sdk/ts/test/pda.test.ts`
passes all 12 cases, including an explicit assertion that those two addresses differ.

**I-TX-01 / INV-RES-06** (`sdk/ts/test/tx-size.test.ts`) builds a REAL signed transaction — a real
generated Ed25519 fee-payer signer, a real `ComputeBudget` instruction, a real blockhash-shaped
lifetime, `@solana/kit`'s own v0 message compiler and transaction codec — for every user-facing
instruction, including `liquidate` both without and with a realistic callback account surface
(ADR-0013's `remaining_accounts` pattern), and asserts the exact serialized byte count via
`@solana/transactions`' own `getTransactionSize`/`isTransactionWithinSizeLimit`. **No architectural
blocker was found**: the largest transaction (`liquidate` with a callback) is 801 bytes, well under
the 1232-byte limit, with 431 bytes of headroom. Full table in §17 below.

**I-SDK-02** (`sdk/ts/test/e2e.test.ts`) runs the complete lifecycle — `initialize_protocol`,
`create_market`, `init_position` bundled with a lender's first `supply` and a borrower's first
`deposit_collateral` (item 16's init-bundling, `ix.ts::withInitPositionIfNeeded`), `borrow`,
`accrue_interest`, `repay`, `withdraw_collateral`, `close_position`, a lender `withdraw`, and a
scripted-price-drop `liquidate` — against a real, local, **offline** Surfpool validator this test
starts, deploys the real `aegis.so` to, and tears down itself; every step asserts the resulting
account state and decoded event, never merely "the transaction did not throw." Deterministic Pyth
`PriceUpdateV2` fixtures are injected via the exact same `aegis_test_kit::PriceFixture`-generated
bytes Phase 8's own demo already uses (`crates/aegis-test-kit/examples/phase8_price_fixture_dump.rs`),
reused rather than reimplemented, over `surfnet_setAccount` — zero network, zero Hermes.

The `app/` Next.js application (Pages: `/`, `/market/[address]`, `/demo`) consumes `@aegis/sdk`
exclusively for every read and every transaction — no hand-coded account parsing or instruction
building in a React component. It surfaces every required market risk parameter (`ack_freeze_authority`,
oracle staleness bound, oracle max confidence, pause bits) and position health (collateral, debt,
health factor, LTV, liquidation price) via `read.ts`'s `MarketView`/`PositionView`. Wallet connection
is a local, `@solana/kit`-native ephemeral signer (`app/src/lib/localSigner.ts`) — a deliberate,
disclosed scope decision (see DEVIATIONS) rather than a browser wallet-extension adapter, both
because `make app`'s clean-clone acceptance criterion must not depend on a specific browser
extension being installed and because current wallet-adapter packages generally carry the same
unavoidable transitive `@solana/web3.js` dependency already discussed above. **I-UI-01** (the full
local user flow) was exercised for real, in a real browser, against a real local Surfpool validator
running the real deployed program, using the Claude Browser tool — not merely asserted; the full
transcript is in §17 below, including the two real bugs this exercise found and fixed (a
duplicate-mutable-account error from reusing the lender's own address as the market's fee
recipient, and an oracle-staleness failure from using the browser's wall-clock instead of the
warped validator's own `Clock` sysvar for a freshly-injected price fixture) — recorded as findings,
not smoothed over.

---

## Phase 8 — evidence

### 0. Phase gate (verified before any code was written)

```
$ git log -1 --format="%H %s" phase-07-token2022
c9b97eb857dfcad4ed44f257287f64fc413aa32b docs(phase-7): record Phase 7 completion evidence, RV-5 resolution, status, and README
$ git log -1 --format="%H %s" HEAD
c9b97eb857dfcad4ed44f257287f64fc413aa32b docs(phase-7): record Phase 7 completion evidence, RV-5 resolution, status, and README
```
Tag == HEAD. Working tree clean (`git status --short` empty). `cargo test --workspace --offline`:
25 test-result blocks, 0 failed. `cargo fmt --all --check`: clean. `cargo clippy --workspace
--all-targets -- -D warnings`: clean. `docs/project-status.md` matched reality exactly (phase
table, tags). No `labs/`, no `bots/` existed yet.

### 1. RV-6 and RV-8 — research gates closed before implementation

Full write-up: `docs/ecosystem-research.md` §16. Summary:

- **RV-6:** `anza-xyz/agave`, `program-runtime/src/invoke_context.rs`, `InvokeContext::push()` —
  `if contains && !is_last { return Err(InstructionError::ReentrancyNotAllowed); }`. Indirect
  `A → B → A` CPI reentrancy is rejected by the runtime itself. Direct self-recursion (`A → A`) is
  allowed, bounded by `MAX_INSTRUCTION_STACK_DEPTH = 5` (4 CPI levels) on the currently shipping
  runtime. Cross-checked against <https://solana.com/docs/core/cpi/cpi-execution> (fetched
  2026-09-11). **Aegis's guard does not depend on this finding** (`docs/composability.md` §2) — it
  informs `A-CPI-02`'s test design (two separate assertions: the CPI-level attempt, and a direct,
  non-CPI unit test of the Aegis-level guard) and the documentation, not whether the defense
  exists.
- **RV-8:** Jupiter's Swap API (`api.jup.ag/swap/v1/quote` then `/swap-instructions`) is the
  current integration surface exposing raw instructions (Ultra API does not); the v6 router is
  deployed on mainnet at `JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4` (confirmed on-chain via
  Solscan/Solana.fm, 2026-09-11). Both steps are genuine HTTP calls to Jupiter's hosted API — no
  offline mode exists, which is exactly why the required path uses a deterministic local rate
  instead and the Jupiter path is optional/network-tagged.

### 2. On-chain implementation

`programs/aegis/src/instructions/liquidate/liquidate.rs`'s `handler` branches on
`callback_program.is_some()` after a shared prefix (token-program pin checks, oracle validation,
`accrue_mut`, HF check, `compute_liquidation_by_*`) identical to Phase 6. The no-callback branch is
the exact Phase 6 code, unmoved in logic. `Market` gained `liquidation_guard: u8` (ADR-0013,
`docs/account-model.md` §4); `Market::LEN` unchanged at 640. New error band 6160-6179
(`architecture.md` §8): `LiquidationCallbackReentrancy`, `LiquidationCallbackNotExecutable`,
`LiquidationCallbackAccountMismatch`, `CallbackAccountNotPermitted`,
`LiquidationCallbackRepaymentShortfall`. `Liquidated` event gained `callback_program: Option<Pubkey>`.

```
$ cargo build -p aegis
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.82s
```

### 3. `labs/example-liquidator` and `labs/hostile-callback`

Both new Anchor-program workspace members (`Cargo.toml` `members`), both registered in
`Anchor.toml`'s `[programs.localnet]`. **Anchor CLI's own notion of "the workspace" only discovers
programs under `programs/`** — verified directly (`anchor build -p example_liquidator` → "is not
part of the workspace" despite being a normal Cargo workspace member) — so both are built with
`cargo build-sbf --manifest-path labs/<name>/Cargo.toml` directly (documented in the `Makefile`'s
`build` target), the same underlying command `anchor build` itself shells out to.

```
$ cargo build-sbf --manifest-path labs/example-liquidator/Cargo.toml
    Finished `release` profile [optimized] target(s) in 1m 24s
$ cargo build-sbf --manifest-path labs/hostile-callback/Cargo.toml
    Finished `release` profile [optimized] target(s) in 0.65s
```

### 4. Regression — full offline suite

```
$ cargo fmt --all --check
(exit 0, no output)

$ cargo clippy --workspace --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 8.00s
(zero warnings)

$ cargo test --workspace --offline
test result: ok. 32 passed; 0 failed; 0 ignored ... (aegis unittests, 32)
test result: ok. 55 passed; 0 failed ... (aegis-math unittests, 55)
test result: ok. 1 passed ... (inflation_attack)
test result: ok. 4 passed ... (liquidation_property)
test result: ok. 3 passed ... (property)
test result: ok. 6 passed ... (rounding_law)
test result: ok. 4 passed ... (shares_property)
test result: ok. 3 passed ... (aegis-test-kit unittests)
test result: ok. 8 passed ... (phase2_adversarial)
test result: ok. 5 passed ... (phase2_state)
test result: ok. 9 passed ... (phase2_token_policy)
test result: ok. 10 passed ... (phase3_adversarial)
test result: ok. 5 passed ... (phase3_collateral)
test result: ok. 8 passed ... (phase4_adversarial)
test result: ok. 9 passed ... (phase4_lending)
test result: ok. 21 passed ... (phase5_oracle_adversarial)
test result: ok. 5 passed ... (phase6_admin)
test result: ok. 13 passed ... (phase6_bad_debt)
test result: ok. 1 passed ... (phase6_integration)
test result: ok. 17 passed ... (phase6_liquidation)
test result: ok. 6 passed ... (phase7_token2022)
test result: ok. 5 passed ... (phase8_composability)      <-- NEW
test result: ok. 5 passed ... (phase8_hostile_callback)    <-- NEW
test result: ok. 1 passed ... (smoke)
test result: ok. 0 passed; 0 failed; 1 ignored ... (network -- N-JUP-01, correctly excluded)
+ 6 doc-test blocks (0 tests each, one per crate)
= 32 test-result blocks total, 0 failed.
```

Every existing Phase 1-7 test file passes **unchanged** — the strongest available evidence that
`I-LIQ-CB-02` holds and that no-callback behavior was not disturbed, beyond the dedicated test.

```
$ for s in scripts/check-*.sh; do ./"$s"; done
check-collateral-transfer-paths: OK — vault token movement goes through exactly the shared helpers, from exactly their enumerated call sites
check-cpi-allowlist: OK — the only raw CPI target outside the token/system programs is the one opt-in, unsigned liquidation callback dispatch     <-- NEW
check-no-close: OK
check-no-dup: OK
check-no-float: OK
check-no-init-if-needed: OK
check-no-slot-time: OK
check-overflow-checks: OK
```
The new `check-cpi-allowlist.sh` was proven to fire on a real violation (a temporary
`invoke_signed(` fixture line appended to `lib.rs`, reverted immediately after, `git diff` showing
only the intended Phase 8 changes afterward) — the same proof-of-life pattern the five pre-existing
guards already use.

### 5. `A-CPI-01..04` — hostile callback, atomic-rollback proof

`tests/phase8_hostile_callback.rs`, 5 tests (4 attacks + 1 direct guard unit test), all passing.
Each attack test snapshots `Market` (accounting totals, `liquidation_guard`), `Position`, and both
vault balances **before and after** the failed transaction and asserts byte-for-byte equality —
not merely that the transaction returned an error:

- **`A-CPI-01`** (`DrainVault`): the hostile callback's only account is `collateral_account` (the
  one thing it actually received) — neither a signer nor `loan_vault`'s real owner (the Market
  PDA). The real SPL Token program rejects the transfer. Zero state diff.
- **`A-CPI-02`** (`Reenter` + direct guard test): the CPI-level attempt fails (in this harness, on
  a missing-account/dispatch error, since the callback has no real signer or complete account set
  to offer — documented honestly rather than misattributed to the runtime's separate
  `ReentrancyNotAllowed` check, which a call that never gets that far cannot exercise). The
  **direct** test — `set_liquidation_guard(svm, market, 1)` then an ordinary `liquidate` call, no
  CPI at all — asserts `AegisError::LiquidationCallbackReentrancy` precisely, proving the
  protocol-level guard independent of runtime behavior.
- **`A-CPI-03`** (`BurnCompute`): a real, loop-carried-dependency compute-burning loop
  (5,000,000 iterations) against a bounded 400,000 CU outer limit. Fails with a genuine compute
  exhaustion; zero partial state.
- **`A-CPI-04`** (`NoRepayment`): the callback instruction itself returns `Ok(())`; the outer
  `liquidate` transaction still fails, on the measured `loan_vault` delta (`0 < 900_000_000`), with
  `AegisError::LiquidationCallbackRepaymentShortfall`. The callback's successful return is never
  consulted.

### 6. `I-LIQ-CB-01` and `I-LIQ-CB-02`

`tests/phase8_composability.rs`, 5 tests, all passing:

- `i_liq_cb_01_honest_callback_seizes_swaps_locally_and_repays_in_one_transaction`: the exact
  `tests/phase6_liquidation.rs` worked-example scenario (10 SOL collateral, 900 USDC debt, crash to
  $95.00/$1.0000), a liquidator with **zero** loan-asset balance, `example-liquidator` as the
  callback at a deterministic $100/SOL local rate. Succeeds; position/market figures match the
  no-callback worked example exactly (`10_000_000_000 - 9_970_348_101` remaining collateral,
  `47_477_848` protocol cut); `INV-CUS-02` holds exactly; the security-relevant direction of
  `INV-CUS-01` (vault never short) holds; the liquidator's loan ATA never moves.
- `i_liq_cb_02_omitting_the_callback_matches_the_no_callback_worked_example_exactly`: the identical
  scenario, `aegis_test_kit::liquidate` (callback accounts `None`), asserted against the same exact
  figures as `tests/phase6_liquidation.rs::u_liq_01_worked_example_on_chain`.
- `a_auth_07_callback_instruction_never_carries_market_or_liquidator_or_any_signer`: calls
  `build_callback_instruction` directly (no SVM) with fabricated `AccountInfo`s, asserts the
  returned `Vec<AccountMeta>` contains neither `market` nor `liquidator` (nor `position`,
  `fee_position`, `collateral_vault`, either liquidator ATA), and that **every** meta has
  `is_signer == false` — including the one honest passthrough account.
- `callback_account_not_permitted_rejects_a_smuggled_protected_key`: a `remaining_account` equal to
  `market`'s pubkey is rejected with `CallbackAccountNotPermitted` before any CPI is attempted.
- `callback_on_one_market_never_touches_an_unrelated_market`: a callback liquidation on Market A
  leaves every checked field of Market B (including its own `liquidation_guard`) unchanged.

### 7. Demo transcripts (both genuinely executed, not reconstructed)

**Rust, `cargo run -p aegis-test-kit --example phase8_demo`** (offline, in-process LiteSVM):

```
Aegis Phase 8 demo -- composability and liquidation routing
=== 4. A liquidator with ZERO loan-asset balance uses the callback ===
  liquidator's loan-asset balance: 0.000000 USDC (cannot cover the $900 repayment without the callback)
  -> liquidate(repay_assets=900 USDC, callback=example_liquidator)
  <- Ok: seized collateral -> callback -> deterministic local swap -> loan_vault
  [after callback liquidation]
    position: collateral=0.029651899 SOL borrow_shares=0
    market:   total_borrow_assets=0.000000 USDC ... collateral_fee_accrued=0.047477848 SOL liquidation_guard=0
  INV-CUS-02 (collateral custody) holds exactly.
=== 5. A second unhealthy borrower (Borrower B) -- ordinary, no-callback liquidation ===
  -> liquidate(repay_assets=900 USDC, callback=None)  [I-LIQ-CB-02]
  <- Ok: identical Phase 6 code path, byte-for-byte (I-LIQ-CB-02)
  [after no-callback liquidation]
    position: collateral=0.029651899 SOL borrow_shares=0
Phase 8 demo complete.
```
(Full transcript: every section 1-5 ran and printed exactly the figures the two Rust test files
also assert.)

**TypeScript keeper, `npm run demo` (`bots/liquidator/`)**, against a real, local,
**non-forking** Surfpool validator (`surfpool start --offline`) — zero network, zero devnet, zero
Jupiter, zero paid API:

```
=== Aegis Phase 8 liquidator keeper -- local demo ===
(no network, no fork -- surfpool --offline)
[1] Starting a plain local Surfpool validator (offline)...
    Surfpool is healthy.
[2] Deploying aegis.so and example_liquidator.so...
    Program Id: DbRhjkZV1QSxMj5AvrYdgVsyEz8nKhoCLnSLGSKsqaF9
    Program Id: CyexeWx6KSzkD4HtCges24DYWMt8ny4wnsjvE39iao1v
[3] Creating collateral (9dp) and loan (6dp) mints...
[4] Protocol + market setup...
[5] Lender supplies liquidity...
    lender supplied 1,000,000.000000 USDC
[6] Borrower deposits collateral and borrows at $150.00/SOL...
    borrower deposited 10.000000000 SOL, borrowed 900.000000 USDC
[7] SOL crashes to $95.00 -- the position is now liquidatable.
[8] A liquidator with ZERO loan-asset balance runs the keeper...
    liquidator loan-asset balance: 0 (relies entirely on the callback)
    example-liquidator reserve pre-funded: 2,000.000000 USDC

=== Keeper result ===
  LIQUIDATED position 3LLsqSjRsjNmzKcZD92TRWTe9HrNJDKhjiV2X3CKipp2 via callback=true, repay=900000000
  signature: ChyZ7t4fD2fp5JSRttMRXTGLZxxHP9aWgJ2h7jcfckSAgPEQxUYRtvPuJ3zgFndhu8wr3aZtbM9UkeYxxZUaBC7

Demo complete: the keeper found and executed a callback liquidation for a
liquidator holding zero loan-asset balance, entirely against a local, offline
Surfpool validator.
```
Exit code 0. `npx tsc --noEmit -p tsconfig.json` (strict mode): exit 0, zero errors.

### 8. `N-JUP-01` — optional, network-tagged (real API call, on-chain leg NOT RUN)

```
$ cargo test --test network -- --ignored --nocapture
running 1 test
N-JUP-01: live Jupiter Swap API integration surface confirmed -- quote outAmount=102881210,
swap instruction targets program JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4 with 45 accounts.
On-chain execution against a Surfpool mainnet fork was NOT attempted in this pass -- documented
as NOT RUN, not faked as passing.
test jupiter_route::n_jup_01_real_jupiter_quote_and_swap_instructions_have_the_documented_shape ... ok
```
This genuinely called the real `api.jup.ag` (network access was available in this environment),
and genuinely confirmed the RV-8 finding — same program ID, real route data. **The further step of
constructing a callback that relays that instruction and executing it against a Surfpool mainnet
fork with a real Aegis market was NOT attempted**: it needs a dedicated Jupiter-relay callback
program (a live quote's route can target any of several different underlying AMM programs, each
with its own account layout — routing it correctly through the fixed callback account contract is
its own scoped piece of work) and mainnet-mint-denominated fixture state, which is genuinely
optional, out-of-scope engineering for this pass, not a blocker for anything required. `cargo test
--workspace` confirms this test is `ignored` by default (excluded from `make test`).

### 9. Pre-existing local artifact fix (found and fixed during this phase)

This local checkout's `target/deploy/aegis-keypair.json` did not match `declare_id!`'s
`2GtoBADM175vkjf5UYpbD198Ry1cJadXMGo8sCQvXndh` (an uncommitted, gitignored local build artifact
whose prior history is unknown). This was discovered attempting the TS keeper's real local
deployment: `anchor build`/`cargo build-sbf` warn about the mismatch but still build; **the first
real instruction sent to the deployed program failed with `AnchorError: DeclaredProgramIdMismatch`
(Error Number: 4100)** — Anchor's `#[program]` dispatcher checks the actual invocation address
against the compiled-in `crate::ID` on every call. This is not cosmetic; it blocks any real
on-chain interaction with the program, on any cluster, until fixed. `anchor keys sync` corrected
`declare_id!` (`programs/aegis/src/lib.rs`) and `Anchor.toml` to match the existing local keypair,
**`DbRhjkZV1QSxMj5AvrYdgVsyEz8nKhoCLnSLGSKsqaF9`**, and the program was rebuilt
(`--arch v0` — see below). The full regression suite (§4) was re-run after this change and confirms
no other behavior depends on the literal old ID; the three other Phase 8 files that referenced it
(`labs/hostile-callback/src/lib.rs`'s `aegis_program_id()`, `bots/liquidator/src/config.ts`'s
default, `bots/liquidator/src/demo.ts`'s explanatory comment) were updated accordingly. Historical
evidence elsewhere in this document (Phase 4/5/6's own "Deployed program 2GtoBADM..." log excerpts)
is **left unchanged** — those are accurate records of what those phases actually ran at the time,
not a claim about the program's current address.

Separately, `anchor build`'s default SBPF target (`--arch v3`) produced a `.so` this local
Surfpool/`solana-cli` combination's deploy path rejects outright (`ELF error: Detected sbpf_version
required by the executable which are not enabled`) — unrelated to the ID mismatch, discovered in
the same debugging session. All three programs are rebuilt with `--arch v0` (`cargo build-sbf`'s
own natural default) for this local environment; `cargo test --workspace` (LiteSVM) passed
identically both before and after this arch change, confirming it has no bearing on the offline
test suite.

### 10. Self-audit (spec item #34 / AGENTS.md-style adversarial review)

| Question | Answer |
|---|---|
| Can the callback receive the Market PDA's signer? | No — never included in the callback CPI's account list at all; `invoke`, never `invoke_signed`, is used for the dispatch. Proven directly (`A-AUTH-07`), not merely by source inspection. |
| Can the callback receive the liquidator's signer? | No — same account list, same proof. |
| Can a `remaining_account` accidentally carry signer privilege? | Every meta this file constructs is unconditionally `is_signer: false`; additionally, six protected keys are rejected outright if present at all (`CallbackAccountNotPermitted`). |
| Can the callback move vault funds? | No (`A-CPI-01`) — it has no authority (owner/delegate) over either vault. |
| Can the callback reenter the same market? | No, on two independent grounds: the Aegis-level guard (proven directly) and the runtime's own indirect-reentrancy rejection (RV-6). |
| Can pre-CPI cached state be trusted accidentally? | `loan_vault` is explicitly reloaded after the callback and only the post-reload balance is used; `market`/`position` in-memory state was already fully computed from oracle-validated data before the CPI and is never re-read from anywhere the callback could have touched (the callback never receives `market` or `position` at all). |
| Is every relevant mutable account reloaded? | `loan_vault` — yes, explicitly. `collateral_vault`, `market`, `position` are never given to the callback, so nothing else could have been mutated by it. |
| Can the callback lie via return data? | Its return value and `callback_data` are never consulted for the repayment decision — only the measured `loan_vault` delta (`A-CPI-04`). |
| Can the callback repay less than required? | Rejected: `actual_delta >= outcome.repay_assets` is enforced; anything less fails atomically. |
| Can the callback manipulate Position/Market then satisfy only the vault delta? | It never receives `Position` or `Market` at all — there is no account through which it could touch them. |
| Are full post-conditions rechecked? | Yes: the guard, the reload, the delta check, and — via the shared code path — the same `hf_after`/accounting math both branches use. |
| Can compute exhaustion partially commit state? | No (`A-CPI-03`) — Solana's own transaction atomicity, not special-cased rollback logic. |
| Can callback failure leave the guard stuck? | No — any failure reverts the whole transaction, including the guard write; the guard is only ever observed as `1` from *within* the same, still-executing transaction. |
| Can global locking break parallelism? | No global lock exists; the guard is a `Market`-scoped field, and `callback_on_one_market_never_touches_an_unrelated_market` proves cross-market isolation directly. |
| Can the callback bypass oracle/liquidation economics? | No — `outcome` is fully computed from oracle-validated, already-accrued state *before* the callback branch even starts; the callback cannot influence it. |
| Can Token-2022 semantics break repayment measurement? | The delta is measured via `InterfaceAccount::reload()` on the real vault, the same measured-delta discipline Phases 2/3/7 already established for exactly this reason; no callback-specific fee assumption exists anywhere in `liquidate.rs`. |
| Does the no-callback path still exactly preserve Phase 6? | Yes — unmoved code, plus a passing, unmodified `tests/phase6_liquidation.rs` and the dedicated `I-LIQ-CB-02` test. |
| Did a general flash-loan facility accidentally appear? | No — the callback is reachable only from inside `liquidate`, only after collateral seizure, only with the fixed six-account contract; there is no borrow-and-return primitive anywhere else. |
| Did Jupiter become required? | No — `N-JUP-01` is `#[ignore]`d and excluded from `make test`; nothing in `programs/aegis/src` references Jupiter at all (`check-cpi-allowlist.sh` greps for this explicitly). |
| Does the keeper trust stale off-chain calculations as authoritative? | No — `src/health.ts` is explicitly documented as advisory-only, and every liquidation attempt is a real transaction subject to on-chain re-validation; a rejection is logged and treated as a normal outcome. |

## Phase 7 — evidence

### 1. RV-5 — research gate closure

See `docs/token-compatibility.md` §0 for the full write-up. Summary: `spl-token-2022-interface`
resolves to **2.1.0** (`Cargo.lock`, cross-checked against the crate's own vendored source and
crates.io); its `ExtensionType` enum has 27 real variants, all classified in §0.1/§0.2 by Mint vs.
Account applicability, transfer/CPI/accounting/authority/sizing impact, collateral/loan-asset
safety, and Tier. `Pausable` and `ScaledUiAmount` — this document's own flagged examples of
extensions added after older lists — are both present and were already correctly classified
before this phase began; Phase 7 confirms that against the real crate rather than a remembered
list, and closes the gate formally. No classification changed and no new extension was admitted.

### 2. Policy engine, vault, and transfer-accounting code — unchanged, and correct

`programs/aegis/src/token/policy.rs`, `token/vault.rs`, and `token/transfer.rs` were already
complete from Phases 2/3 (project-status.md's own Phase 3 evidence recorded early coverage of
`U-TOK-01/02` and `A-TOK-05..09` ahead of their nominally-assigned phase). Phase 7 changed **zero**
lines in `programs/aegis/src` — verified by `git diff --stat` before commit (§20 below) — because
the positive-allowlist `match` in `evaluate_mint` already names every Tier A/B extension
explicitly and rejects everything else, including every extension RV-5 surfaced, through the one
shared catch-all arm. This is the concrete meaning of "positive allowlist, not blocklist": no code
change was needed for Aegis to correctly reject `Pausable` and `NonTransferable`, because it never
had to recognize them by name to reject them.

### 3. Role asymmetry — re-verified unchanged

`transfer_fee_mint_accepted_as_collateral_rejected_as_loan_asset` (Phase 2) remains the primary
proof; `a_tok_10_full_lifecycle_on_transfer_fee_collateral_market` and
`a_tok_11_fee_rate_change_mid_lifecycle_does_not_break_accounting` (this phase) add the full-
lifecycle and fee-rate-change dimensions on the collateral side specifically, since that is the
only role a transfer-fee mint may occupy.

### 4. `ImmutableOwner` — sizing, initialization order, and effectiveness

Sizing and initialization order were already implemented in `token/vault.rs` (Phase 2/3,
`ExtensionType::try_calculate_account_len`, `InitializeImmutableOwner` before `InitializeAccount3`)
and already tested (`transfer_fee_mint_accepted_as_collateral_rejected_as_loan_asset` asserts a
Token-2022 vault's size exceeds the legacy 165-byte constant). Phase 7 adds the missing piece: a
concrete proof the extension is *effective*, not merely present —
`immutable_owner_blocks_reassignment_even_by_the_genuine_current_owner` builds a standalone
Token-2022 account via the identical instruction sequence `token/vault.rs` uses, then shows that
even the account's real, correctly-signing current owner cannot reassign it via a real
`SetAuthority(AccountOwner)` instruction. Combined with `scripts/check-collateral-transfer-paths.sh`
(unaffected by this phase) proving no code in `programs/aegis/src` ever calls `set_authority` at
all, "the vault owner cannot later be reassigned" is proven both structurally (Aegis never asks)
and behaviorally (the extension refuses even if asked).

### 5. `A-TOK-10` — full lifecycle on a transfer-fee collateral market

`tests/phase7_token2022.rs::a_tok_10_full_lifecycle_on_transfer_fee_collateral_market`: a real
Token-2022 mint with a 2% transfer fee (no cap) as collateral against a plain SPL loan asset,
carried through `create_market` → `init_position` ×2 → `supply` → `deposit_collateral`
(credited = 9.8 SOL from a 10 SOL request, asserted exactly) → `borrow` → `accrue_interest` →
a price crash → `liquidate` (protocol_cut confirmed > 0) → the adaptive bad-debt path (mirroring
`tests/phase6_integration.rs`'s own adaptive pattern) → `absorb_bad_debt` →
`withdraw_collateral_fees` (paid in the fee-bearing mint itself; the admin's own ATA sized for it,
and the admin — an ordinary recipient here — confirmed to bear the outbound fee, i.e. receives
less than `collateral_fee_accrued`, while Aegis's internal accounting is proven to have decremented
by the exact recorded amount regardless) → `withdraw` (lender cleanup). `assert_inv_cus_01`/
`assert_inv_cus_02` are called after **every** state-changing instruction, not only at the end —
12 call sites in one test.

### 6. `A-TOK-11` — fee-rate change mid-lifecycle

`tests/phase7_token2022.rs::a_tok_11_fee_rate_change_mid_lifecycle_does_not_break_accounting`. Uses
the real `SetTransferFee` instruction (`spl_token_2022_interface::extension::transfer_fee::instruction::set_transfer_fee`)
and respects Token-2022's genuine 2-epoch activation delay
(`TransferFeeConfig::get_epoch_fee`) rather than assuming the change is immediate:

| Step | Rate in effect | Requested | Credited | Fee |
|---|---|---|---|---|
| Deposit #1 (before any change) | 1% | 10.000000000 SOL | 9.900000000 SOL | 100000000 |
| Deposit — same epoch as `SetTransferFee(5%)` | **still 1%** (delay not yet elapsed) | 5.000000000 SOL | 4.950000000 SOL | 50000000 |
| Deposit #2 (after `advance_epoch(3)`) | **5%** (now effective) | 10.000000000 SOL | 9.500000000 SOL | 500000000 |

The same-epoch deposit is the load-bearing assertion: it proves Aegis is reading whatever the
token program actually applies at the moment of the CPI (measured delta), not a value read once
and cached — the exact bug this test exists to catch, per `token-compatibility.md` §7's own
framing. `market.rs`'s `Market` struct has no fee-rate field at all (grep-verifiable), so there is
nothing to have cached in the first place; this test proves that structural fact is also
behaviorally correct. A subsequent `withdraw_collateral` (outbound leg, under the now-5% rate)
confirms the vault still debits exactly the recorded amount regardless of the current fee, with
the recipient bearing the fee — INV-CUS-01/INV-CUS-02 asserted after every step, including both
deposits, the borrow, and the withdrawal.

### 7. RV-5 supplementary evidence

`pausable_mint_rejected_as_collateral` and `non_transferable_mint_rejected` build real Token-2022
mints carrying `Pausable`/`NonTransferable` (via real `InitializePausableConfig`/
`InitializeNonTransferableMint` instructions, not synthetic bytes) and confirm `create_market`
rejects both. `u_tok_03_cached_decimals_match_mint_for_every_supported_configuration` confirms
`market.collateral_decimals`/`loan_decimals` equal the real mint decimals for both a classic
SPL/SPL configuration (9/6 decimals) and a Token-2022 transfer-fee collateral configuration (8
decimals, deliberately different from the first, to rule out a hardcoded/copy-pasted value).

### 8. Test-kit additions

`crates/aegis-test-kit/src/mints.rs`: `Token2022Extension::Pausable`/`NonTransferable` variants;
`set_transfer_fee_rate` (real `SetTransferFee` CPI); `fetch_transfer_fee_config` (reads back
`older_transfer_fee`/`newer_transfer_fee`); `advance_epoch` (the same direct-`Clock`-sysvar-
mutation technique `tests/phase6_integration.rs` already used for `unix_timestamp` warps, applied
to `epoch`). `crates/aegis-test-kit/src/user_tokens.rs`:
`create_immutable_owner_account`. `crates/aegis-test-kit/src/token_accounts.rs`:
`fetch_mint_decimals`. No production code (`programs/aegis/src`) changed.

### 9. Tests — commands actually run and results

```
$ anchor build --ignore-keys
    Finished `release` profile [optimized] target(s)
    Finished `test` profile [unoptimized + debuginfo] target(s)

$ cargo test --workspace
```

New Phase 7 test file:

| File | Tests |
|---|---|
| `tests/phase7_token2022.rs` | 6 |

Full workspace count, in the exact order `cargo test --workspace` reports each suite (every
`test result: ok.` line summed, unedited transcript):

```
32 (aegis unit) + 55 (aegis-math unit) + 1 (inflation_attack) + 4 (liquidation_property)
+ 3 (property) + 6 (rounding_law) + 4 (shares_property) + 3 (aegis-test-kit unit)
+ 8 (phase2_adversarial) + 5 (phase2_state) + 9 (phase2_token_policy) + 10 (phase3_adversarial)
+ 5 (phase3_collateral) + 8 (phase4_adversarial) + 9 (phase4_lending)
+ 21 (phase5_oracle_adversarial) + 5 (phase6_admin) + 13 (phase6_bad_debt)
+ 1 (phase6_integration) + 17 (phase6_liquidation) + 6 (phase7_token2022) + 1 (smoke)
= 226
```

(the three `Doc-tests` crates contribute 0 each, as in every prior phase). **226 tests total, 0
failures** — this repository's own Phase 6 evidence (§ "Phase 6 — evidence" below) recorded "219
tests total" at the time; re-running the identical `cargo test --workspace` command against the
current, unmodified pre-Phase-7 state (the baseline check performed before any Phase 7 change,
`git rev-parse HEAD` at `08a8ea3`) actually measures **220**, one higher than that historical
figure — a pre-existing one-test discrepancy in Phase 6's own arithmetic, not introduced by this
phase and not corrected here (Phase 6's evidence section is left as originally written, per
`AGENTS.md` §12: describe what exists, in the tense that is true, without silently rewriting a
prior phase's record). What this phase can and does state truthfully: the measured baseline
immediately before Phase 7 began was 220 passing tests, 0 failures; the measured total immediately
after is 226 passing tests, 0 failures; the difference (6) is exactly `tests/phase7_token2022.rs`,
and no other file's test count changed. **0 failures** in every run performed during this phase.

### 10. Demo

```
$ make demo
anchor build
cargo run -p aegis-test-kit --example phase7_demo
```

Full transcript (every figure below was printed by the actual run, not reconstructed):

```
Aegis Protocol — Phase 7 demo (Token-2022 completion)
Zero-cost, local, offline: in-process LiteSVM, no devnet, no RPC, no API key.

=== 1. Two markets: A = classic SPL collateral, B = Token-2022 transfer-fee (2%) collateral ===

=== 2. Borrower deposits collateral on both markets (B requests slightly more to offset the fee) ===
  Market A (classic SPL)                  requested 10.000000000 SOL  credited 10.000000000 SOL  fee 0.000000000 SOL
  Market B (Token-2022, 2% transfer fee)  requested 10.300000000 SOL  credited 10.094000000 SOL  fee 0.206000000 SOL

=== 3. Borrower borrows 900 USDC on both markets at SOL=$150.00 / USDC=$1.00 ===
  Market A (classic SPL)                  borrowed 900.000000 USDC
  Market B (Token-2022, 2% transfer fee)  borrowed 900.000000 USDC

=== 4. Interest accrual (permissionless instruction) on both markets ===
  accrue_interest succeeded on both markets; custody invariants held.

=== 5. SOL crashes to $95.00 -- both positions become liquidatable ===

=== 6. Liquidation on both markets -- full debt repaid, collateral seized ===
  Market A (classic SPL)                  repaid 900.000000 USDC  vault decreased by 9.922870253 SOL  liquidator received 9.922870253 SOL  protocol_cut 0.047477848 SOL
  Market B (Token-2022, 2% transfer fee)  repaid 900.000000 USDC  vault decreased by 9.922870253 SOL  liquidator received 9.724412847 SOL  protocol_cut 0.047477848 SOL
    (Market B bears the outbound transfer fee on the liquidator's leg -- the vault-side accounting above still reconciles exactly, per INV-CUS-02)

=== 7. Cleanup: lenders withdraw on both markets ===
  Market A (classic SPL)                  lender redeemed 1200.000000 USDC
  Market B (Token-2022, 2% transfer fee)  lender redeemed 1200.000000 USDC

=== 8. Verified extension-policy rejection table (real create_market attempts) ===
  Extension                                            Result                           Reason
  TransferHook                                         REJECTED                         UnsupportedTokenExtension
  PermanentDelegate                                    REJECTED                         UnsupportedTokenExtension
  MintCloseAuthority                                   REJECTED                         UnsupportedTokenExtension
  DefaultAccountState = Frozen                         REJECTED                         UnsupportedTokenExtension
  Pausable (RV-5)                                      REJECTED                         UnsupportedTokenExtension
  NonTransferable (RV-5)                               REJECTED                         UnsupportedTokenExtension
  Unrecognized discriminant (positive allowlist)       REJECTED                         InvalidMintAccountData
  TransferFeeConfig as LOAN asset (accepted as collateral above) REJECTED                         TransferFeeNotAllowedForLoanAsset

Phase 7 demo complete. INV-CUS-01/INV-CUS-02 held after every instruction on both markets.
```

Note the vault-decrease figure is **identical** between Market A and Market B
(9.922870253 SOL) — the liquidation math (seizure, bonus, protocol cut) is computed from debt and
price alone, is independent of which token program the collateral mint uses, and the *only*
place Token-2022 changes what a party observes is the recipient's received amount (the liquidator
gets 9.724412847 SOL, not 9.922870253, on Market B) — exactly the asymmetry
`account-model.md` §6.4 specifies: outbound transfers debit exactly the recorded amount, and the
recipient bears any fee.

### 11. Regression — prior-phase guarantees re-run

```
$ cargo fmt --all -- --check         # clean
$ cargo clippy --workspace --all-targets -- -D warnings   # clean, zero warnings
$ for f in scripts/check-*.sh; do bash "$f"; done
check-collateral-transfer-paths: OK
check-no-close: OK
check-no-dup: OK
check-no-float: OK
check-no-init-if-needed: OK
check-no-slot-time: OK
check-overflow-checks: OK
$ anchor build --ignore-keys          # succeeds
$ cargo test --workspace              # 226 passed, 0 failed (see §9)
```

Every Phase 1-6 test file passes completely unmodified. `git diff --stat` against
`phase-06-liquidation` (checked before commit, §20 below) confirms zero lines changed in
`programs/aegis/src` — this phase is test-kit and test additions plus documentation, exactly as
§2 above states.

### 12. Security self-audit (phase spec §28)

| Question | Answer |
|---|---|
| Can an unknown extension pass? | No — `A-TOK-05` (Phase 2, re-run) |
| Can a newly-added current extension escape classification? | No — `Pausable`/`NonTransferable` both tested this phase; RV-5 (§0) enumerates all 27 real variants and every one not explicitly allowlisted hits the same catch-all rejection |
| Can a transfer-fee loan asset pass? | No — `A-TOK-06` (Phase 2, re-run) and the demo's rejection table (§10) |
| Can a transfer-hook mint pass? | No — `A-TOK-01` (Phase 2, re-run) |
| Can a confidential-transfer mint pass? | No, structurally — no match arm admits `ConfidentialTransfer*`/`ConfidentialMintBurn`; see `token-compatibility.md` §7.1's explicit reasoning for why no dedicated fixture was built |
| Can a freeze-authority mint pass without acknowledgement? | No — `A-TOK-07` (Phase 2, re-run) |
| Can the mint owner/token program be substituted? | No — `A-TOK-08`/`A-TOK-09` (Phase 3, re-run) |
| Can vault account size be under-allocated? | No — `ExtensionType::try_calculate_account_len` is the only sizing path (`token/vault.rs`, unchanged); `A-TOK-10`'s fee-mint vault exercised end-to-end |
| Can `ImmutableOwner` initialization be omitted? | No — unconditional in `token/vault.rs` for every Token-2022 vault (unchanged); `immutable_owner_blocks_reassignment_even_by_the_genuine_current_owner` proves it is also *effective* |
| Can the token authority be reassigned? | No — proven this phase, both structurally (grep: no `set_authority` call anywhere in `programs/aegis/src`) and behaviorally |
| Can mid-lifecycle fee changes break internal accounting? | No — `A-TOK-11` |
| Is any fee calculation hardcoded/cached? | No — `Market` has no fee-rate field; grep-verifiable |
| Is nominal amount credited anywhere after a fee-bearing CPI? | No — `token/transfer.rs` unchanged; every credit is `after − before` post-`reload()` |
| Is any required post-CPI `reload()` missing? | No — unchanged from Phase 3, re-exercised by `A-TOK-10`/`A-TOK-11` |
| Can raw token donation become user credit? | No — `A-CUS-08` (Phase 4, re-run; unrelated to this phase but re-verified in the full regression) |
| Can scaled/display semantics alter raw protocol balances? | No — `token-compatibility.md` §0.3: no call site touches `ScaledUiAmount`'s UI-conversion instructions |
| Does the full liquidation/bad-debt lifecycle reconcile under transfer fees? | Yes — `A-TOK-10` |
| Did support accidentally expand beyond frozen v1 policy? | No — §2 above; zero lines changed in `programs/aegis/src` |

No Phase-7-scoped finding required a fix; every question above was answered by an existing
mechanism plus new test evidence, not a code change.

## Phase 6 — evidence

### 1. Liquidation math (`crates/aegis-math/src/liquidation.rs`)

Implements `economic-model.md` §7.1-§7.3 exactly: `max_repay` (close-factor/full-liquidation split
plus the dust rule), `compute_liquidation_by_repay` (the primary form, including the §7.2
collateral clamp with upward-rounded repay recomputation), and `compute_liquidation_by_seize` (the
alternate seize-specified input form `instruction-catalogue.md` §17 requires, sharing the identical
ceil-rounded "seize → repay" inversion the clamp itself uses — there is exactly one such formula in
the crate). `is_liquidatable(hf) = hf < WAD` is the **only** eligibility comparison in the module,
and it is strict by construction (`U-LIQ-02`, `is_liquidatable_never_uses_inclusive_comparison`).

### 2. `liquidate` (`instructions/liquidate/liquidate.rs`)

Sequencing mirrors `borrow`'s INV-ORA-07 precedent exactly: `require_valid_price` for both feeds is
the first fallible operation, strictly before `accrue_mut` or any other state write, so a failed
oracle check leaves nothing modified (`A-LIQ-01`, oracle-adversarial re-runs below). After accrual,
`HF < WAD` is checked **strictly** (`INV-LIQ-01`/`INV-SOLV-02`) before any liquidation math runs.
Settlement follows `economic-model.md` §7.3 verbatim: `repay_shares` from `repay_assets` floored
then clamped to `position.borrow_shares`; `total_supply_assets` is deliberately never touched
(`INV-LIQ-09`); `collateral_fee_accrued` increases by exactly `protocol_cut`, the only write site
for that field anywhere in the program (`INV-CUS-09`); net seized collateral is transferred to the
liquidator, signed by the `Market` PDA, while the protocol's cut stays physically in the vault. No
owner signature is required (self-liquidation is permitted, `U-LIQ-07`).

### 3. `absorb_bad_debt` (`instructions/liquidate/absorb_bad_debt.rs`)

No price-update account anywhere in its `Accounts` struct — structurally impossible to make
oracle-dependent. No pause check exists in the handler at all (same precedent as `repay`/
`deposit_collateral`). Preconditions are exact: `collateral_amount == 0` (no dust tolerance) and
`borrow_shares > 0`. `fee_position` is mandatory and PDA-constrained to `PDA(market,
market.fee_recipient)`, identical to every other instruction's own fee-position handling — a
substituted account fails Anchor's seeds constraint before the handler body ever runs
(`fee_position_cannot_be_substituted_with_another_account`). Settlement is `economic-model.md`
§8.2's exact algorithm: `absorbed_by_protocol = min(bad_assets, fee_assets)`, `burn_shares =
to_shares_up(absorbed_by_protocol, ...)` clamped to `fee_position.supply_shares`, then both
`total_supply_assets` and `total_borrow_assets` fall by exactly `bad_assets` — no CPI, no token
movement anywhere in the instruction.

### 4. `withdraw_collateral_fees` (`instructions/admin/withdraw_collateral_fees.rs`)

Admin-only (`has_one = admin` against `Protocol`), bounded by `amount <= market.
collateral_fee_accrued` — the sole precondition standing between the admin and the vault, and the
concrete mechanism behind INV-ADM-01/INV-ADM-08. No `Position` account appears anywhere in this
instruction's account list, so there is nothing to read a user's balance from even if the bound
were somehow bypassed.

### 5. Worked example (`U-LIQ-01`) — exact `economic-model.md` §7.5 figures

Both the pure-math test (`liquidation::tests::u_liq_01_worked_liquidation_example`) and the full
on-chain instruction test (`tests/phase6_liquidation.rs::u_liq_01_worked_example_on_chain`, real
`supply`/`deposit_collateral`/`borrow`/`liquidate` transactions through LiteSVM) assert every
documented intermediate and final value exactly:

| Quantity | §7.5 value | Asserted |
|---|---|---|
| `repay_assets` | 900.000000 USDC | ✅ both tests |
| `base_seize` | 9.495569620 SOL | ✅ pure-math test |
| `total_seize` | 9.970348101 SOL | ✅ both tests |
| `bonus_amount` | 0.474778481 SOL | ✅ both tests |
| `protocol_cut` | 0.047477848 SOL | ✅ both tests |
| `to_liquidator` | 9.922870253 SOL | ✅ both tests |
| remaining collateral | 0.029651899 SOL | ✅ both tests |
| remaining debt | 0 | ✅ both tests |

### 6. Collateral clamp (`U-LIQ-03`) — concrete evidence and rounding

`liquidation::tests::u_liq_03_and_05_collateral_clamp_recomputes_repay_upward_and_leaves_remaining_debt`
and `tests/phase6_liquidation.rs::u_liq_03_and_05_collateral_clamp_and_full_seizure_with_remaining_debt`
reduce available collateral to 9 SOL (insufficient for the naive 9.970348101 SOL seizure) and
assert: `total_seize` clamps to exactly 9.000000000 SOL (INV-LIQ-02, exact equality at the bound,
not merely `<=`); `repay_assets` recomputes DOWN from the requested 900 USDC to **812.408947
USDC**, via the identical ceil-rounded formula the clamp shares with `compute_liquidation_by_seize`
(`u_liq_03_clamp_repay_matches_independent_reinversion` proves the two agree exactly); the
liquidator still pays the full recomputed amount, never less, because every step of that
recomputation ceils. `clamp_boundary_exact_equality_does_not_clamp` pins the boundary itself: a
naive seizure exactly equal to available collateral does NOT clamp; one unit less does. The
resulting position (`collateral_amount == 0`, `borrow_shares > 0`) is then fed directly into a real
`absorb_bad_debt` call in the same test, proving the bridge end-to-end.

### 7. Close-factor / dust behavior (`U-LIQ-04`)

`max_repay` applies the close-factor split (`HF < full_liq_hf` → 100%; else `close_factor × debt`)
and then the dust rule (a resulting remainder strictly inside `(0, min_debt)` forces full
repayment instead). `dust_rule_boundary_exactly_at_min_debt_vs_one_below` pins the exact boundary:
remaining `== min_debt` does not force dust; remaining `== min_debt - 1` does. On-chain,
`u_liq_04_dust_rule_boundary_forces_full_when_remaining_would_be_below_min_debt` proves the concrete
effect by requesting an amount (10 USDC) that STRICTLY EXCEEDS the plain close-factor cap
(9.999999 USDC) and showing it succeeds only because the dust rule raised `max_repay` to the full
debt.

### 8. Property tests (`P-LIQ-1..4`) — results and assumptions

All four live in `crates/aegis-math/tests/liquidation_property.rs` (512 cases each,
`proptest`), documented per-test with their construction and assumptions:

| ID | Result | Assumption stated in the test |
|---|---|---|
| `P-LIQ-1` | ok — HF strictly improves for ANY non-clamped repay amount, not merely a full liquidation | Excludes the clamp path by construction (a distinct, documented claim from `P-LIQ-2`); tests both the full and close-factor branches |
| `P-LIQ-2` | ok — seizure never exceeds collateral, across random (including clamp-triggering) states | None beyond `INV-LIQ-06`'s own config bound |
| `P-LIQ-3` | ok — profitable whenever `HF < WAD` and the clamp is not hit | `liq_bonus > 0` stated explicitly ("a zero-bonus market has no liquidation incentive by design") |
| `P-LIQ-4` | ok — reference params satisfy `full_liq_hf (0.95) >= LT·(1+b) (0.84)` | Deterministic check against the documented reference parameter set, not a generated property |

### 9. `liquidate` — instruction/account/oracle/custody behavior

14 accounts, matching `instruction-catalogue.md` §17 exactly (no `protocol` account — same
precedent `borrow`/`withdraw_collateral`/`supply` already established: pause instructions do not
exist until Phase 12, so nothing can ever set a pause bit, and adding the account now would be dead
code). Oracle: `require_valid_price` for both feeds, first fallible operation. Custody: seizure and
repayment both go through the shared `token::transfer::{transfer_checked_in,
transfer_checked_out}` helpers exclusively (`scripts/check-collateral-transfer-paths.sh`), signed
by the `Market` PDA for the outbound leg, pinned mint/token-program throughout.

### 10. Bad debt — absorption logic and zero-collateral requirement

See §3 above. `U-BD-01` (`tests/phase6_bad_debt.rs::u_bd_01_nonzero_collateral_is_rejected`,
`u_bd_01_zero_borrow_shares_is_rejected`) proves both halves of the precondition independently,
including that even 1 unit of collateral blocks absorption — no dust exception.

### 11. First loss — concrete protocol-fee-shares-first evidence

`tests/phase6_bad_debt.rs`'s `U-BD-02` suite constructs real `fee_position.supply_shares` (not a
fixture — minted via the identical accounting a real `supply()` CPI would produce, backed by real
tokens in the vault) and a `bad_assets` figure relative to the fee position's own recoverable
value, then asserts:

| Scenario | Result |
|---|---|
| Fee shares exceed the loss (E-17) | Fully absorbed by the protocol; `total_supply_assets` falls by exactly `bad_assets`, lenders' pro-rata share price is otherwise undiluted |
| Fee shares fall short | `fee_position.supply_shares` driven to exactly 0 (fully exhausted) before any residual is socialized |
| Fee shares exactly cover the loss (boundary) | `fee_position` left with ~0 recoverable value (≤ 1 base unit, from `to_shares_up`'s ceil rounding) |
| Fee shares one unit short (boundary) | `fee_position` fully exhausted (0 shares) |
| No fee shares at all | `absorbed_by_protocol == 0`; the entire loss is socialized |

`tests/phase6_admin.rs::i_iso_01_bad_debt_in_market_a_leaves_market_b_completely_untouched` and the
full lifecycle test (`tests/phase6_integration.rs`) both show the same first-loss ordering against
a REAL, interest-accrual-derived fee position, not a constructed one.

### 12. Cross-market isolation (`I-ISO-01`) and no shared writable state (`A-PAR-02`)

`I-ISO-01`: two independent markets (same mint pair, distinct `config_id` — the hardest case, since
a bug that accidentally shared a vault or a totals field would only be caught here) are created;
Market B receives real lender/borrower activity; Market A is driven through a full real borrow →
crash → clamped liquidation → `absorb_bad_debt` cycle; every one of Market B's accounts (market,
fee position, both real positions, both vaults) is proven **byte-identical** before and after, not
merely "logically unaffected." `A-PAR-02`: the actual generated `Vec<AccountMeta>` for `Liquidate`/
`AbsorbBadDebt`/`WithdrawCollateralFees` against both markets are collected and their writable-only
subsets asserted disjoint; `protocol` (the one account genuinely shared) is confirmed read-only in
every one of them.

### 13. Non-custodial admin proof (`A-ADM-02`)

`tests/phase6_admin.rs::a_adm_02_admin_cannot_withdraw_user_collateral`: a real user deposits real
collateral; a real liquidation accrues a real `collateral_fee_accrued`. Two attack attempts —
`protocol_cut + 1`, and the entire vault balance (user collateral plus the cut) — both fail with
`InsufficientCollateralFees`; the user's `Position.collateral_amount`, the vault's raw balance, and
`INV-CUS-02` are all proven completely unchanged after both attempts. A final legitimate withdrawal
of exactly `protocol_cut` still succeeds, proving the rejections were about the amount, not a
broken instruction. This is the concrete, portfolio-grade evidence for `INV-ADM-01`.

### 14. Oracle adversarial re-run against `liquidate`

`tests/phase6_liquidation.rs` re-runs the O-1..O-11 failure matrix against `liquidate` specifically
(not merely relying on the shared `require_valid_price` guard's `borrow` coverage): stale price,
wrong owner, wrong feed ID, partial verification, excessive confidence, future publish time, zero
price, and duplicate price accounts — each asserting the identical, specific `AegisError` `borrow`'s
own Phase 5 suite established for the same check. `A-LIQ-01` additionally proves a healthy-position
rejection leaves market, position, both vaults, and every wallet ATA involved byte-identical.

### 15. Tests — commands actually run and results

```
$ cargo test --workspace
```

New Phase 6 test files and counts (all currently passing, part of the 219 total below):

| File | Tests |
|---|---|
| `crates/aegis-math/src/liquidation.rs` (unit, inline) | 16 |
| `crates/aegis-math/tests/liquidation_property.rs` | 4 |
| `tests/phase6_liquidation.rs` | 17 |
| `tests/phase6_bad_debt.rs` | 13 |
| `tests/phase6_admin.rs` | 5 |
| `tests/phase6_integration.rs` | 1 |

Full workspace count (every `test result: ok.` line summed, unedited transcript):

```
32 + 55 + 1 + 4 + 3 + 6 + 4 + 3 + 8 + 5 + 9 + 10 + 5 + 8 + 9 + 21 + 5 + 13 + 17 + 1 = 219
```

(the three `Doc-tests` crates contribute 0 each). **219 tests total, 0 failures.**

### 16. Demo

```
$ make demo
anchor build
cargo run -p aegis-test-kit --example phase6_demo
```

Full transcript (abbreviated to the load-bearing lines; every figure below was printed by the
actual run, not reconstructed):

```
=== 3. Borrower A deposits 10 SOL, borrows 900 USDC at SOL=$150.00 ===
  borrowed 900.000000 USDC; health factor 1.3333 (healthy)

=== 11. SOL crashes to $95.00 +/- $0.20 -- Borrower A becomes liquidatable ===
  health factor now 0.8424 (< 1.0, and < full_liq_hf 0.95: full liquidation permitted)

=== 12. Liquidator A repays the full 900 USDC debt ===
  repay_assets:      900.000000 USDC
  total_seize:       9.970348101 SOL (base + bonus)
  bonus_amount:      0.474778481 SOL
  protocol_cut:      0.047477848 SOL (from the bonus only)
  to_liquidator:     9.922870253 SOL
  remaining collateral on A: 0.029651899 SOL
  remaining debt on A:       0 (fully repaid)
  INV-LIQ-*, INV-CUS-01, INV-CUS-02: hold

=== Time passes: 180 days of interest accrue on Borrower B's debt ===
  fee_position.supply_shares after accrual: 1658966659717 (real protocol fee shares, not a fixture)

=== 13. SOL crashes to $40.00 -- Borrower B liquidated to zero collateral, bad debt created ===
  Borrower B's accrued debt just before the crash: 916.798682 USDC
  Borrower B collateral after liquidation: 0.000000000 SOL (fully seized -- the clamp fired)
  Borrower B remaining debt: 535.846301 USDC (bad debt -- collateral exhausted)

=== 14. absorb_bad_debt -- protocol fee shares absorb the loss first ===
  bad_assets (total loss recognized):       535.846301 USDC
  protocol fee shares burned:                1658966659717 -> 987557 (first-loss)
  total_supply_assets:                       1216.798682 USDC -> 680.952381 USDC
  INV-CUS-01, INV-SOLV-04: hold exactly -- no tokens moved

=== 15. Lender withdraws all shares -- realizing the socialized residual loss ===
  lender originally supplied: 1200.000000 USDC
  lender's redeemable value now: 680.952380 USDC (LESS than principal)
  lender's loan-asset wallet balance after full withdrawal: 680.952380 USDC
  realized shortfall vs. original principal: 519.047620 USDC

=== 16. Admin withdraws the protocol's own accrued collateral fee ===
  admin withdrew 0.095096895 SOL of protocol-owned collateral fees

=== Final invariant report ===
  INV-CUS-01: holds
  INV-CUS-02: holds
```

Uses `economic-model.md` §4.1's actual documented reference IRM parameters (not the shared
`reference_market_args()` test fixture, which deliberately zeroes the IRM slopes for tests that
don't care about accrual) so the 180-day warp produces genuine, nonzero interest and a genuine,
nonzero protocol fee — not a fixture standing in for one.

### 17. Regression — prior-phase guarantees re-run

```
$ cargo fmt --all -- --check         # clean
$ cargo clippy --workspace --all-targets -- -D warnings   # clean, zero warnings
$ for f in scripts/check-*.sh; do bash "$f"; done
check-collateral-transfer-paths: OK — vault token movement goes through exactly the shared
  helpers, from exactly their enumerated call sites  (allowlist updated for liquidate/
  withdraw_collateral_fees, see "Current architectural decisions" above)
check-no-close: OK
check-no-dup: OK
check-no-float: OK
check-no-init-if-needed: OK
check-no-slot-time: OK
check-overflow-checks: OK
$ anchor build --ignore-keys          # succeeds, IDL regenerated (15 instructions, 14 events)
$ cargo test --workspace              # 219 passed, 0 failed (see §15)
```

Every Phase 1-5 test file passes completely unmodified — this phase added new files
(`crates/aegis-math/src/liquidation.rs`, `crates/aegis-math/tests/liquidation_property.rs`,
`programs/aegis/src/instructions/liquidate/{mod,liquidate,absorb_bad_debt}.rs`,
`programs/aegis/src/instructions/admin/withdraw_collateral_fees.rs`, four `tests/phase6_*.rs`
files, `crates/aegis-test-kit/examples/phase6_demo.rs`) and additive changes to shared files
(`error.rs`, `events.rs`, `guards.rs`, `instructions/{admin,liquidate}/mod.rs`, `lib.rs` in both
`programs/aegis` and `crates/aegis-test-kit`, `crates/aegis-test-kit/src/market.rs`,
`scripts/check-collateral-transfer-paths.sh`, `Makefile`) — no existing test, assertion, or
error mapping was weakened, removed, or had its expected result changed.

### 18. Deviations

None. No frozen document was edited. No ADR was written — none was needed (see "Current
architectural decisions" above for the two implementation-level facts recorded there: the guard-
script allowlist update and `liquidate`'s own compute-unit budget, neither an architectural or
economic deviation).

### 19. Security self-audit

| Question | Answer |
|---|---|
| Can `HF == WAD` be liquidated? | No — `is_liquidatable` is `hf < WAD`, strict; `U-LIQ-02` proves the exact boundary in both directions on-chain. |
| Can liquidation repay more than debt? | No — `max_repay <= debt_assets` always (close-factor branch is `floor(debt × cf)` with `cf <= WAD`; full branch IS `debt_assets`); the clamp path additionally `min()`s with `debt_assets` explicitly. |
| Can it seize more collateral than exists? | No — the clamp sets `total_seize = collateral_amount` exactly when the naive figure would exceed it; `P-LIQ-2` and `seizure_never_exceeds_collateral_amount` prove this across many states. |
| Can clamp rounding favor the liquidator? | No — every step of the clamp's repay recomputation (`seize_to_repay`) ceils; `u_liq_03_clamp_repay_matches_independent_reinversion` proves the applied `repay_assets` matches an independent re-derivation from the clamped `total_seize` exactly. |
| Can the protocol cut touch principal-equivalent collateral? | No — `protocol_cut = floor(bonus_amount × liq_protocol_fee / WAD)`, computed from `bonus_amount` alone; `protocol_cut_never_exceeds_bonus_and_never_touches_base_seize` asserts `to_liquidator >= base_seize` always. |
| Can the protocol cut leave custody accounting inconsistent? | No — `collateral_fee_accrued` increases by exactly the amount that stays in the vault (`to_liquidator = total_seize - protocol_cut` is the only amount transferred out); `INV-CUS-02` holds exactly in every test. |
| Can a healthy position mutate before rejection? | No — `A-LIQ-01` full before/after byte-exact snapshots of market, position, both vaults, and every wallet ATA prove zero mutation. |
| Can invalid oracle data mutate state? | No — oracle validation is the first fallible statement in `handler`, before `accrue_mut` or any account write, identical ordering to `borrow`'s own `A-ORACLE-13`-proven guarantee. |
| Can a liquidator spoof the vault/mint/token program? | No — PDA-seeded + `address =`/`has_one` constraints on every account, identical pattern to every prior phase's instructions. |
| Can a liquidator bypass the close factor? | No — `repay_exceeding_max_repay_is_rejected` proves `max_repay + 1` fails; the dust-rule tests prove the bound itself is computed correctly. |
| Can dust debt remain when full liquidation should trigger? | No — the dust rule forces `max_repay` to the full debt whenever a partial repay would leave `(0, min_debt)`; boundary-pinned in both directions. |
| Can bad debt be absorbed while collateral remains? | No — `U-BD-01` proves even 1 unit of collateral blocks it. |
| Can `fee_position` be omitted or substituted? | No — mandatory, PDA-constrained account; `fee_position_cannot_be_substituted_with_another_account` proves a non-canonical account is rejected before the handler runs, and the position is left with its debt unabsorbed. |
| Can lender loss happen before protocol fee first-loss? | No — `absorbed_by_protocol = min(bad_assets, fee_assets)` is computed and applied before the socialization step in every `U-BD-02` scenario, including the exact-boundary and one-unit-short cases. |
| Can bad-debt absorption depend on the oracle? | No — no price-update account exists anywhere in `AbsorbBadDebt`'s account list; `absorb_bad_debt_succeeds_with_no_oracle_posted_anywhere` proves it with literally no price posted in the whole `LiteSVM` instance. |
| Can pause block bad-debt cleanup? | No — `absorb_bad_debt_ignores_forced_pause_bits` forces both `Market.paused` and `Protocol.paused` to `0b1111` via direct state injection (the only way to test this before Phase 12's pause instructions exist) and proves absorption still succeeds. |
| Can the admin withdraw user collateral? | No — `A-ADM-02`, the flagship proof (§13 above). |
| Can Market A's loss affect Market B? | No — `I-ISO-01`, byte-exact evidence (§12 above). |
| Can shared global writable state break isolation? | No — `A-PAR-02` proves the writable-account sets of every Phase 6 instruction are disjoint across two markets, and that the one genuinely shared account (`protocol`) is read-only. |
| Are all rounding directions exact? | Yes — `U-LIQ-01` matches `economic-model.md` §7.5 bit-for-bit; `U-ROUND-13/14/15` (`crates/aegis-math/tests/rounding_law.rs`, pre-existing since Phase 4/5) pin the three liquidation-specific rounding rows individually. |
| Did Phase 8 callback/Jupiter/bot logic accidentally enter this phase? | No — `grep -ri "jupiter\|callback\|flash" programs/aegis/src` returns nothing; `liquidate`'s dual input form (`repay_assets`/`seize_collateral`) is a pure internal math inversion with no external CPI, exactly as `instruction-catalogue.md` §17 specifies for Phase 6 (the callback itself is explicit Phase 8 non-scope). |

Every Phase-6-scoped question above is answered "no attack succeeds" / "yes, correctly" with a
named, currently-passing test.

---

## Phase 5 — evidence

### 1. Research gates RV-3 and RV-4 (closed before any oracle code was written)

Full detail in `docs/ecosystem-research.md` §15, fetched 2026-09-06 from primary sources —
crates.io's registry API, the actual `pyth-solana-receiver-sdk` 2.0.0 crate source (downloaded and
read directly), and `docs.pyth.network` fetched directly:

| Gate | Finding | Source |
|---|---|---|
| RV-3 | Receiver program ID unchanged across the 2026-08-26 Core upgrade: `rec5EKMGg6MxZYaMdyBfgwp4d5rB9T1VQH5pJv5LtFJ`. `PriceUpdateV2` unchanged. | `docs.pyth.network/price-feeds/core/contract-addresses/solana`; `pyth-solana-receiver-sdk-2.0.0/src/lib.rs` |
| RV-4 | `enum VerificationLevel { Partial { num_signatures: u8 }, Full }`, unchanged in shape; `get_price_no_older_than` hardcodes a `Full` requirement. | `pyth-solana-receiver-sdk-2.0.0/src/price_update.rs` |
| Version | `pyth-solana-receiver-sdk` **2.0.0** (published 2026-06-15), `anchor-lang = "^1.0.2"` — satisfied by this workspace's `anchor-lang 1.2.0` | crates.io registry API |

Neither finding invalidated ADR-0008 or any other Phase 0 decision (§15.3).

### 2. Oracle abstraction (`programs/aegis/src/oracle/`)

`oracle/mod.rs` defines `PriceBand { lo, hi, published_at }` and the `PriceSource` trait exactly
per `oracle-design.md` §1 (a raw `&AccountInfo`, not a concrete Pyth account wrapper, so a v2
non-Pyth implementer would not need to depend on Pyth's types). `require_valid_price` is the
single, shared enforcement point every priced instruction calls — no instruction improvises its
own oracle checks. Dispatch is by `market.oracle_kind`; only `ORACLE_KIND_PYTH_PULL = 0` (Pyth
pull) is accepted in v1, matching ADR-0008 §1's "one implementer" framing.

`oracle/pyth.rs`'s `PythPull` is the sole implementer, built against the real SDK types verified
above. No `Mock` variant, no mock-oracle program, anywhere — confirmed by grep (`grep -ri mock
programs/aegis/src` returns nothing) and by construction (the trait's only implementer is
`PythPull`).

### 3. O-1..O-11 — full failure matrix

| Check | Meaning | Implementation | Test |
|---|---|---|---|
| O-1 | Account owner == Pyth receiver program | `oracle::pyth::PythPull::read_price`, explicit `require_keys_eq!` (in addition to what an `Account<>` wrapper would do automatically — this repo deliberately doesn't use one, per the frozen trait signature) | `A-ORACLE-06` (`a_oracle_06_wrong_owner_account_is_rejected`) |
| O-2 | Correct discriminator / real `PriceUpdateV2` deserialization | `PriceUpdateV2::try_deserialize`, mapped to `OracleAccountInvalidData` on failure | Exercised structurally by every passing test (a malformed buffer would fail here); `pyth_fixture::tests::deserializes_via_real_sdk` proves the fixture path round-trips through the real deserializer |
| O-3 | `feed_id == market.{collateral,loan}_feed_id` | Explicit pre-check + `get_price_no_older_than`'s own feed check | `A-ORACLE-07` (`a_oracle_07_wrong_feed_id_is_rejected`) |
| O-4 | `verification_level == Full` | Explicit pre-check + `get_price_no_older_than`'s hardcoded `Full` requirement | `A-ORACLE-08` (`a_oracle_08_partial_verification_level_is_rejected`) |
| O-5 | `now - publish_time <= max_price_age_secs` | `get_price_no_older_than`, unix seconds only | `A-ORACLE-03` + boundary (`a_oracle_03_boundary_age_exactly_at_threshold_vs_plus_one`) |
| O-6 | `publish_time <= now + 60s` | `oracle::pyth`'s explicit `MAX_FUTURE_PRICE_SKEW_SECS` check | `A-ORACLE-09` + boundary (`a_oracle_09_boundary_exactly_at_max_future_skew`) |
| O-7 | `price > 0` | `aegis_math::conservative_price_band` | `A-ORACLE-04` (`a_oracle_04_zero_price_is_rejected`, `a_oracle_04_negative_price_is_rejected`) |
| O-8 | `conf <= price * max_conf_bps / 10_000` | `aegis_math::conservative_price_band` | `A-ORACLE-05` + boundary (`a_oracle_05_boundary_confidence_exactly_at_threshold_vs_plus_one`) |
| O-9 | `lo >= MIN_PRICE_WAD`, `hi <= MAX_PRICE_WAD` | `aegis_math::conservative_price_band` | `A-ORACLE-04` (`a_oracle_04_absurd_price_is_rejected`) |
| O-10 | Exponent scaling checked, never overflows/panics | `aegis_math::health::scale_to_wad_{floor,ceil}`, `checked_pow`/`checked_mul`/`try_from` throughout | `health::tests::exponent_driven_overflow_is_a_clean_error_not_a_panic` (Tier 1); same bound exercised on-chain by `A-ORACLE-04` |
| O-11 | The two price accounts are distinct | `oracle::require_valid_price`, explicit `require_keys_neq!` on the two `AccountInfo` keys | `A-ORACLE-12` (`a_oracle_12_same_account_for_both_feeds_is_rejected`) |

Every row has concrete, currently-passing test evidence — `cargo test --workspace` output below.

### 4. Byte-exact Pyth fixture (`crates/aegis-test-kit/src/pyth_fixture.rs`)

`PriceFixture::to_price_update()` constructs a real `pyth_solana_receiver_sdk::price_update::
PriceUpdateV2` value; `.serialize()` calls that value's own `AccountSerialize::try_serialize`
(the exact impl the `#[account]` macro generated inside the real SDK crate) — never a hand-rolled
byte layout. `pyth_fixture::tests::deserializes_via_real_sdk` proves the round trip: construct →
serialize → deserialize with `PriceUpdateV2::try_deserialize` (the identical call
`oracle::pyth::PythPull` makes) → assert every field survives exactly. A second test proves the
`Partial` verification-level variant round-trips too; a third proves full determinism (the same
fixture always serializes to the same bytes). `inject_price_update`/`set_price` inject these bytes
directly via `LiteSVM::set_account` — no Hermes, no RPC, no Pyth program deployed (confirmed: the
Pyth program ID never appears in any `add_program`/`deploy` call anywhere in this repository —
`grep -r rec5EKMGg6MxZYaMdyBfgwp4d5rB9T1VQH5pJv5LtFJ` outside `oracle/pyth.rs` and
`pyth_fixture.rs` returns nothing).

### 5. Health / valuation (`crates/aegis-math/src/health.rs`)

`conservative_price_band` implements O-7/O-8/O-9 and the WAD normalization (`economic-model.md`
§6.1) in pure, `no_std`, float-free arithmetic. `collateral_value`/`debt_value`/`health_factor`/
`is_within_max_ltv` implement §6.2/§6.3 exactly (floor for collateral, ceil for debt).

**`U-HEALTH-01`/`U-HEALTH-02`** (§6.5 worked examples) pass with the corrected values (see the
documentation finding above): `collateral_value = $1,497.00`, `debt_value = $900.18`,
`HF = 1.330400586549357...` (healthy) for the first vector; `collateral_value = $948.00`,
`HF = 0.84249816703326...` (liquidatable, below `full_liq_hf = 0.95`) for the second.

**`P-VAL-1`** (`p_val_1_decimals_x_expo_matrix_preserves_scale`): for every `decimals ∈ 0..=12`
crossed with every `expo ∈ -12..=0` (169 cases), a token worth exactly $1.00 values to exactly
1 WAD — proving normalization never silently shifts economic scale, checked by exact equality, not
approximation. Boundary cases (`decimals ∈ {0,12}`, `expo ∈ {-12,0}`) asserted individually too.

**`P-VAL-2`** (`economic-model.md` §10's own definition — `collateral_value` monotone
non-decreasing in `collateral_amount` and in `price_c_lo`): both directions asserted directly.
Additionally (task's broader monotonicity ask): `debt_value` monotone in `debt_assets`/price, and
`health_factor` monotone non-decreasing in `collateral_value` / non-increasing in `debt_value`.

### 6. Borrow — real, oracle-validated (`instructions/borrow/borrow.rs`)

The Phase 3/4 `OracleNotYetAvailable` gate is gone. `handler` now, in order: (1) validates the
oracle (`oracle::require_valid_price`) — the first fallible operation, before any state write
(INV-ORA-07); (2) `accrue_mut`; (3) `compute_borrow` (unchanged Phase 4 pure function — share
math, free-liquidity bound, `min_debt` floor); (4) values collateral at the confidence lower bound
(floored) and the post-borrow debt at the upper bound (ceiled), then requires
`debt_value <= collateral_value * max_ltv / WAD` (INV-SOLV-01); (5) mutates position/market
totals; (6) transfers `loan_vault → owner`; (7) emits `Borrowed` (now actually reachable, unlike
Phase 4).

### 7. `withdraw_collateral` — debt-aware (`instructions/collateral/withdraw_collateral.rs`)

A debt-free position (`borrow_shares == 0`) still reads no oracle at all (E-08, unchanged from
Phase 3). A debt-bearing position now: validates the oracle, computes accrued debt via
`Market::accrue_view` (pure — `Market` is still never written by this instruction, preserving
claim C2), and evaluates health against the **post-withdrawal** collateral amount
(`collateral_amount - amount`), never the pre-withdrawal one — `debt_bearing_withdraw_
rejects_unsafe_post_withdraw_health` and `debt_bearing_withdraw_succeeds_when_post_withdraw_
health_is_safe` prove both directions.

### 8. Fail-closed policy (risk-increasing operations under a bad oracle)

Every one of O-1..O-9/O-11 individually violated causes `borrow` to fail with the specific mapped
`AegisError` and zero state change (`A-ORACLE-03` through `A-ORACLE-09`, `A-ORACLE-12`). No
oracle-failure path can be bypassed by omission, substitution, or a crafted account — each is a
dedicated adversarial test against a genuinely constructed bad fixture, not a source-code
assertion.

### 9. Risk-reducing policy (evidence deposit and repay work under a broken oracle)

`A-ORACLE-01` (`a_oracle_01_deposit_collateral_succeeds_with_broken_oracle`): `deposit_collateral`
succeeds with **no price account posted anywhere in the LiteSVM instance at all** for either of
the market's configured feeds — not merely omitted from one call, genuinely absent.
`A-ORACLE-02` (`a_oracle_02_repay_succeeds_with_broken_oracle`): real debt is established via a
real `borrow` under a valid price, the oracle is then made stale (representative of an outage per
`oracle-design.md` §4.1), and `repay` is proven to still fully clear the debt. Neither instruction
declares a price-update account in its `#[derive(Accounts)]` struct at all — there is nothing a
caller could break.

### 10. State atomicity (`A-ORACLE-13`, INV-ORA-07)

Both variants (`borrow`, debt-path `withdraw_collateral`) take full before/after snapshots — raw
account `data` bytes and `lamports` — for every account the failing instruction could conceivably
touch (`Market`, `Position`, both vaults, the borrower's own loan ATA) and assert byte-exact
equality after a genuine O-1 failure (wrong-owner price account). This is stronger than "the
instruction returned an error": it proves nothing was written, not merely that execution
eventually errored.

### 11. Tests — commands actually run and results

```
$ cargo test --workspace
   ... (full, unedited transcript; only test names already shown verbatim elsewhere in this
        file are elided by "...", never pass/fail outcomes or counts) ...

     Running unittests src/lib.rs (target/debug/deps/aegis-...)
running 31 tests
... (unchanged from Phase 4) ...
test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (target/debug/deps/aegis_math-...)
running 39 tests
... (26 unchanged from Phase 4, plus 13 new in health::tests) ...
test health::tests::confidence_boundary ... ok
test health::tests::zero_and_negative_price_are_rejected ... ok
test health::tests::absurdly_small_and_large_prices_are_rejected ... ok
test health::tests::exponent_driven_overflow_is_a_clean_error_not_a_panic ... ok
test health::tests::typical_band_matches_hand_computation ... ok
test health::tests::u_health_01_healthy_position ... ok
test health::tests::u_health_02_price_drop_to_liquidatable ... ok
test health::tests::p_val_2_collateral_value_monotone_in_amount ... ok
test health::tests::p_val_2_collateral_value_monotone_in_price ... ok
test health::tests::debt_value_monotone_in_assets_and_price ... ok
test health::tests::health_factor_monotone_in_collateral_and_debt ... ok
test health::tests::p_val_1_decimals_x_expo_matrix_preserves_scale ... ok
test health::tests::p_val_1_boundary_values ... ok
test result: ok. 39 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running unittests src/lib.rs (target/debug/deps/aegis_test_kit-...)
running 3 tests
test pyth_fixture::tests::deserializes_via_real_sdk ... ok
test pyth_fixture::tests::deserializes_partial_verification_level ... ok
test pyth_fixture::tests::is_fully_deterministic ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/phase2_adversarial.rs / phase2_state.rs / phase2_token_policy.rs (unchanged)
test result: ok. 8 passed ... / ok. 5 passed ... / ok. 9 passed ...

     Running tests/phase3_adversarial.rs (unchanged in substance; the one Phase-4-gate test that
     tested removed behavior was replaced with a NOTE pointing at its Phase 5 successor)
running 10 tests
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s

     Running tests/phase3_collateral.rs (unchanged)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s

     Running tests/phase4_adversarial.rs (the two Phase-4-gate tests for removed behavior were
     replaced with a NOTE pointing at their Phase 5 successors; the other 8 unchanged)
running 8 tests
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s

     Running tests/phase4_lending.rs (unchanged)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s

     Running tests/phase5_oracle_adversarial.rs
running 21 tests
test a_oracle_01_deposit_collateral_succeeds_with_broken_oracle ... ok
test a_oracle_02_repay_succeeds_with_broken_oracle ... ok
test a_oracle_03_boundary_age_exactly_at_threshold_vs_plus_one ... ok
test a_oracle_03_stale_oracle_blocks_borrow_and_debt_withdraw_but_not_repay_or_deposit ... ok
test a_oracle_04_absurd_price_is_rejected ... ok
test a_oracle_04_negative_price_is_rejected ... ok
test a_oracle_04_zero_price_is_rejected ... ok
test a_oracle_05_boundary_confidence_exactly_at_threshold_vs_plus_one ... ok
test a_oracle_05_confidence_over_threshold_is_rejected ... ok
test a_oracle_06_wrong_owner_account_is_rejected ... ok
test a_oracle_07_wrong_feed_id_is_rejected ... ok
test a_oracle_08_partial_verification_level_is_rejected ... ok
test a_oracle_09_boundary_exactly_at_max_future_skew ... ok
test a_oracle_09_future_publish_time_is_rejected ... ok
test a_oracle_10_outage_across_a_price_move_then_recovery ... ok
test a_oracle_11_same_transaction_price_read_is_self_consistent ... ok
test a_oracle_12_same_account_for_both_feeds_is_rejected ... ok
test a_oracle_13_failed_oracle_check_leaves_state_byte_identical ... ok
test a_oracle_13_failed_withdraw_collateral_oracle_check_leaves_state_byte_identical ... ok
test debt_bearing_withdraw_rejects_unsafe_post_withdraw_health ... ok
test debt_bearing_withdraw_succeeds_when_post_withdraw_health_is_safe ... ok
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.47s

     Running tests/smoke.rs (unchanged)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

   Doc-tests aegis / aegis_math / aegis_test_kit — 0 tests each, ok
```

**163 tests total, 0 failures** (31 + 39 + 3 + 8 + 5 + 9 + 10 + 5 + 8 + 9 + 21 + 1 + 0 + 0 + 0 = 163
— the aegis-math workspace-level `inflation_attack`/`property`/`rounding_law`/`shares_property`
suites, unchanged from Phase 4 at 1 + 3 + 6 + 4 = 14 tests, are folded into that count). The full,
unedited transcript was captured directly from the command above.

A compute-budget finding, not a correctness bug: `borrow`, debt-path `withdraw_collateral`, and
`repay` when a nontrivial accrual gap has elapsed all legitimately need more than Solana's default
200,000 CU (two `PriceUpdateV2` deserializations plus several 256-bit `mul_div_*` divisions —
accrual, share conversion, and for the oracle-priced pair, valuation and the LTV check — in one
instruction). `aegis-test-kit`'s transaction builders for these three instructions now prepend a
`ComputeBudgetInstruction::set_compute_unit_limit(400_000)` instruction, exactly what a real client
would need to do. This is a resource-allocation concern, not a security check — `INV-RES-01`'s
200k-budget target is explicitly Phase 11 (Performance) scope; no prior phase's test had exercised
`repay` together with a large accrual gap (only the cheaper, 2-account standalone
`accrue_interest`), so this is a real, previously-undiscovered cost characteristic being recorded
now rather than a Phase 5 regression.

Required test IDs, all passing:

| ID | Test | File |
|---|---|---|
| `A-ORACLE-01..13` | See §3/§9/§10 above | `tests/phase5_oracle_adversarial.rs` |
| `U-HEALTH-01`/`U-HEALTH-02` | Worked examples | `crates/aegis-math/src/health.rs` |
| `P-VAL-1`/`P-VAL-2` | Decimals×expo matrix; monotonicity | `crates/aegis-math/src/health.rs` |
| — | Real oracle-validated borrow (happy path) | `tests/phase5_oracle_adversarial.rs::debt_bearing_withdraw_succeeds_when_post_withdraw_health_is_safe` and every `A-ORACLE-*` test's own successful setup borrow |
| — | Debt-aware `withdraw_collateral` (both directions) | `debt_bearing_withdraw_rejects_unsafe_post_withdraw_health`, `debt_bearing_withdraw_succeeds_when_post_withdraw_health_is_safe` |
| — | Real SDK deserializes the fixture | `pyth_fixture::tests::deserializes_via_real_sdk` |

### 12. Demo

```
$ make demo
anchor build
cargo run -p aegis-test-kit --example phase5_demo
Aegis Protocol — Phase 5 demo (oracle-backed borrowing)
Zero-cost, local, offline: in-process LiteSVM, no devnet, no RPC, no API key, no Hermes.

=== 1. Protocol, market and positions ===
Lender supplied 1000000000000 (1,000,000.000000 USDC)
Borrower deposited 10000000000 (10.000000000 SOL) collateral

=== 2. Deterministic valid Pyth price updates (SOL $150.00, USDC $1.00, zero conf) ===
collateral_price_update: EWn7dE93GeQJu72WEkEmC5MZpm5FhiJzkcJEf1xpRdWP (real PriceUpdateV2, owner = pyth receiver program)
loan_price_update:       EahQmXc3rwhY3CH1g3ZgUx8L4vHTNmzpK1xtiQ1RAxq6

=== 3. borrow succeeds (real oracle validation, real LTV check) ===
  borrowed:                  900000000 (900.000000 USDC)
  position.borrow_shares:    900000000000000
  collateral value: $1500.00  debt value: $900.00
  health factor: 1.3333
  INV-CUS-01: holds

=== 4. Oracle becomes stale (30 days pass, no new price posted) ===
  warped forward 30 days; the SAME price accounts are now far past max_price_age_secs (60s)

=== 5. borrow fails closed against the stale oracle ===
  borrow(1 USDC) -> REJECTED: InstructionError(1, Custom(6045))

=== 6. debt-bearing withdraw_collateral also fails closed against the stale oracle ===
  withdraw_collateral(1 SOL) -> REJECTED: InstructionError(1, Custom(6045))

=== 7. repay still succeeds -- no oracle required (INV-REP-01) ===
  repaid 100.000000 USDC while the oracle was stale

=== 8. deposit_collateral still succeeds -- no oracle required (INV-ORA-02) ===
  deposited 1 more SOL while the oracle was stale
  INV-CUS-01: still holds through the whole outage episode

=== 9. Oracle recovers -- SOL now $120.00 ===
  position.collateral_amount: 11000000000 (11.000000000 SOL)
  debt_assets: 800000000  debt_value: $800.00
  collateral_value at the new price: $1320.00
  new health factor: 1.3200

  INV-CUS-01: holds after the full outage-and-recovery episode
  loan_vault.amount: 999199000000

Demo complete. All Phase 5 acceptance criteria exercised above.
```

`Custom(6045)` is `AegisError::OraclePriceStale` (6000 + 45, the Oracle band). Run against the
actual built `aegis.so` and real `PriceUpdateV2` fixture bytes; no network access anywhere.

### 13. Regression — prior-phase guarantees re-run

```
$ cargo fmt --all -- --check         # clean
$ cargo clippy --workspace --all-targets -- -D warnings   # clean, zero warnings
$ for f in scripts/check-*.sh; do bash "$f"; done
check-collateral-transfer-paths: OK — vault token movement goes through exactly the shared
  helpers, from exactly their enumerated call sites  (allowlist updated for real borrow, see
  "Current architectural decisions" above)
check-no-close: OK
check-no-dup: OK
check-no-float: OK
check-no-init-if-needed: OK
check-no-slot-time: OK
check-overflow-checks: OK
$ anchor build                        # succeeds, IDL regenerated
$ cargo test --workspace              # 163 passed, 0 failed (see §11)
```

Every Phase 1-4 test file still passes unmodified in substance — the only edits were (a) two
additional trailing arguments (`collateral_price_update`, `loan_price_update`) threaded through
`withdraw_collateral`'s zero-debt-path call sites in `tests/phase3_collateral.rs`,
`tests/phase3_adversarial.rs` and `examples/phase3_demo.rs` (placeholder `Pubkey::default()` —
never read, since a debt-free withdrawal takes no oracle branch at all), and (b) the removal of
the now-obsolete Phase-3/4-gate tests, replaced with a `NOTE` comment pointing at their Phase 5
successors (see §11 above). No assertion about pre-existing Phase 1-4 behavior was weakened,
removed, or had its expected error changed.

### 14. Deviations

None. No frozen document was edited (the one documentation finding is recorded in "Current
architectural decisions" above, not a deviation). No ADR was written — none was needed.

### 15. Security self-audit

| Question | Answer |
|---|---|
| Can wrong-owner oracle data pass? | No — `A-ORACLE-06` constructs a genuine, correctly-shaped `PriceUpdateV2` owned by a real, wrong program (`spl_token_interface::ID`) and it is rejected with `OracleAccountOwnerMismatch`. |
| Can right owner but wrong feed ID pass? | No — `A-ORACLE-07`, `OracleFeedMismatch`. |
| Can partial verification pass? | No — `A-ORACLE-08`, `OracleVerificationLevelNotFull`. |
| Can stale data pass? | No — `A-ORACLE-03` + boundary, `OraclePriceStale`. |
| Can future data pass? | No — `A-ORACLE-09` + boundary, `OraclePriceInFuture`. |
| Can excessive confidence pass? | No — `A-ORACLE-05` + boundary, `OracleConfidenceTooWide`. |
| Can zero/negative/absurd prices pass? | No — `A-ORACLE-04` (three variants), `OraclePriceNotPositive`/`OraclePriceOutOfBounds`. |
| Can exponent normalization overflow? | No — `scale_to_wad_{floor,ceil}` use `checked_pow`/`checked_mul`/`try_from` throughout; `exponent_driven_overflow_is_a_clean_error_not_a_panic` proves a typed error, never a panic. |
| Can the same account satisfy two feed roles improperly? | No — `A-ORACLE-12`, `OracleDuplicatePriceAccounts`, checked before either feed is even read. |
| Can an oracle failure mutate state before returning? | No — `A-ORACLE-13` (both variants), full before/after byte-exact snapshots. |
| Can deposit be blocked by oracle outage? | No — `A-ORACLE-01`, proven with no price posted anywhere. |
| Can repay be blocked by oracle outage? | No — `A-ORACLE-02`, proven with real debt and a stale oracle. |
| Can borrow bypass oracle validation? | No — oracle validation is the first fallible statement in `handler`, before any other account read/write. |
| Can collateral withdrawal with debt bypass the health check? | No — `debt_bearing_withdraw_rejects_unsafe_post_withdraw_health`. |
| Is post-withdraw health evaluated correctly? | Yes — against `collateral_amount - amount`, never the pre-withdrawal amount; proven by the same test using a case where the pre-withdrawal state would have looked safe. |
| Are collateral and debt priced conservatively? | Yes — `lo` floored for collateral, `hi` ceiled for debt (`U-HEALTH-01/02`, INV-ORA-03). |
| Is feed identity tied to ID rather than arbitrary address? | Yes — `PriceSource::read_price` never receives or checks an "expected pubkey," only `expected_feed_id: &[u8; 32]` compared against `price_message.feed_id`. |
| Did any mock provider sneak into production/test architecture? | No — the only hits for `grep -ri mock` in `programs/aegis/src`/`crates/aegis-test-kit/src` are prose (`oracle/mod.rs`'s own doc comment stating "there is no `Mock` variant"; `market.rs`'s "no mocks, no stubs" doc comment); `PriceSource` has exactly one implementer, `PythPull`, and no `Mock`/`Local` enum variant or struct exists anywhere. |
| Is Hermes/network required anywhere? | No — every required test and the demo inject bytes directly via `LiteSVM::set_account`; `grep -r hermes` across `programs/` and `crates/` returns nothing. |
| Did Phase 6 liquidation logic accidentally enter Phase 5? | No — `grep -ri liquidat programs/aegis/src` hits only pre-existing Phase 2 infrastructure (`PAUSE_LIQUIDATE`'s bit constant and `create_market`'s `liq_bonus`/`liq_threshold` parameter-bound validation, both predating this phase) and this phase's own doc comments; there is no `liquidate` instruction and no `absorb_bad_debt` code (the only hit for either name is a pre-existing Phase 2 doc comment in `create_market.rs` noting that the fee position exists so *Phase 6's future* `absorb_bad_debt` can always require it — no function or account of that name exists). |

Every Phase-5-scoped question above is answered "no attack succeeds" / "yes, correctly" with a
named, currently-passing test — none required a fix during this audit (each was true by the time
the audit was performed, having been built to satisfy exactly these properties from the start).

### 1. Share accounting (`crates/aegis-math/src/shares.rs`)

`to_shares_down`/`to_shares_up`/`to_assets_down`/`to_assets_up` implement `economic-model.md` §3.1
exactly, with `VIRTUAL_SHARES = 1_000_000` and `VIRTUAL_ASSETS = 1` hardcoded (not parameters) so
they can never be weakened at a call site. `to_assets_*` narrow the `mul_div_*` result to `u64`
(an asset amount is always native base units) via a checked, non-truncating `u64::try_from`, never
an `as` cast.

### 2. Rounding — the full 15-row table

`economic-model.md` §1.3's table lists **15** distinct operations, but the document's own closing
sentence says "`U-ROUND-01..14`" — one ID short of the row count. Investigated, not guessed around:
the formulas are unambiguous and are what every test below encodes exactly; the mismatch is in the
document's own count of its rows, recorded here as a documentation finding rather than silently
dropping a row to make the count match.

| # | Operation | Direction | Test | Result |
|---|---|---|---|---|
| 1 | `supply(assets)` → shares minted | floor | `shares::tests::round_01_supply_assets_shares_minted_floors` | ✅ |
| 2 | `withdraw(assets)` → shares burned | ceil | `shares::tests::round_02_withdraw_assets_shares_burned_ceils` | ✅ |
| 3 | `borrow(assets)` → borrow shares minted | ceil | `shares::tests::round_03_borrow_assets_borrow_shares_minted_ceils` | ✅ |
| 4 | `repay(assets)` → borrow shares burned | floor | `shares::tests::round_04_repay_assets_borrow_shares_burned_floors` | ✅ |
| 5 | `supply(shares)` → assets required | ceil | `shares::tests::round_05_supply_shares_assets_required_ceils` | ✅ |
| 6 | `withdraw(shares)` → assets returned | floor | `shares::tests::round_06_withdraw_shares_assets_returned_floors` | ✅ |
| 7 | `borrow(shares)` → assets returned | floor | `shares::tests::round_07_borrow_shares_assets_returned_floors` | ✅ |
| 8 | `repay(shares)` → assets required | ceil | `shares::tests::round_08_repay_shares_assets_required_ceils` | ✅ |
| 9 | Interest accrual → interest added | floor | `irm::tests::round_09_interest_accrual_floors` | ✅ |
| 10 | Protocol fee shares → fee shares | floor | `rounding_law.rs::round_10_protocol_fee_shares_floor` | ✅ |
| 11 | Collateral value → value | floor | `rounding_law.rs::round_11_collateral_value_floor` | ✅ |
| 12 | Debt value → value | ceil | `rounding_law.rs::round_12_debt_value_ceil` | ✅ |
| 13 | Liquidation seize amount → collateral seized | floor | `rounding_law.rs::round_13_liquidation_seize_floor` | ✅ |
| 14 | Liquidation repay (collateral-capped) → repay required | ceil | `rounding_law.rs::round_14_liquidation_clamped_repay_ceil` | ✅ |
| 15 | Liquidation protocol fee → fee taken from bonus | floor | `rounding_law.rs::round_15_liquidation_protocol_fee_floor` | ✅ |

Rows 1–10 are exercised live by Phase 4 instructions; rows 11–15 belong to Phase 5/6 valuation and
liquidation (explicit Phase 4 non-scope: "No oracle... No liquidation"). Those five are pinned
directly against the already-existing Phase 1 `mul_div_floor`/`mul_div_ceil` primitives, applied to
the exact formula shape `economic-model.md` §6–7 defines with representative numbers (several drawn
from the §6.5/§7.5 worked examples, perturbed by a few units where the exact example divides evenly
and would not otherwise exercise floor≠ceil) — this proves the correct primitive-and-direction
choice now without building oracle/health/liquidation modules that are out of scope for this phase.

### 3. Inflation attack (`A-SHARE-01`, `crates/aegis-math/tests/inflation_attack.rs`)

Real Aegis closes the *direct-donation* variant of the attack outright via INV-CUS-08 (a donation
never touches `total_supply_assets` at all — proven for both the collateral and loan vaults, Phase
3's `A-CUS-08` and Phase 4's `loan_vault_direct_donation_is_never_credited`), independent of
virtual offsets. `A-SHARE-01` isolates the share-math defense on its own terms: the identical
attack sequence (1-unit deposit, a `total_assets` inflation standing in for the interest-accrual
variant the offsets exist to defend, then a 1.5e9 victim deposit), run once with the offsets
parameterized to zero and once with Aegis's real, frozen constants:

```
=== WITHOUT virtual offsets (attacker bootstraps 1:1) ===
attacker cost:    1,000,000,001  (1 deposit + 1,000,000,000 "inflation")
attacker redeems: 1,250,000,000
attacker PROFIT:  249,999,999          <- attack SUCCEEDS
victim loss:      249,999,999

=== WITH VIRTUAL_SHARES=1_000_000, VIRTUAL_ASSETS=1 (same capital, same victim deposit) ===
attacker shares for 1 unit: 1,000,000   (not 1 — this is the entire defense)
attacker cost:    1,000,000,001
attacker redeems:   500,000,100
attacker PROFIT:  -499,999,901          <- attack is a net LOSS
victim loss:              199          <- negligible dust, 6 orders of magnitude smaller
```

The attacker doesn't merely fail to profit — they lose roughly half their capital, because the
1,000,000 shares issued for their own 1-unit deposit dilute their claim on the "inflated" assets
almost entirely away to the victim and the pool. Exact figures asserted in
`a_share_01_inflation_attack_without_vs_with_virtual_offsets`.

### 4. IRM and Taylor compounding (`crates/aegis-math/src/irm.rs`)

`utilization`, `borrow_rate`, `taylor_x` and `taylor3` implement `economic-model.md` §4.1–4.2
exactly. `U-IRM-03`'s worked example (§4.4) is pinned bit-exact, independently verified with
Python big-integer arithmetic before writing the Rust test:

```
u = 900e6 * WAD / 1000e6 = 900_000_000_000_000_000   (0.9 WAD)
r = 17_123_287_670                                    (per-second WAD)
x = r * 86_400 = 1_479_452_054_688_000
growth = taylor3(x) = 1_480_546_983_577_839
interest = floor(900e6 * growth / WAD) = 1_332_492
fee_amount = floor(1_332_492 * 0.10) = 133_249
```

`taylor3`'s cubic term is computed as two chained two-factor `mul_div_floor` calls (`mul_div_*`
only takes two factors) rather than a single three-factor product; the extra intermediate floor
this introduces only ever rounds further down (strengthening, never weakening, the
never-over-charges property) and does not change this worked-example result — verified bit-exact
against a true single-shot computation before committing to the two-call decomposition.

`P-IRM-2` (`taylor3(x) <= e^x - 1` for `x >= 0`) is checked against a **non-float** high-precision
reference: a 30-term partial sum of the same non-negative-term Taylor series for `e^x - 1`, computed
with exact `num-bigint` arithmetic. Because every term of that series is non-negative for `x >= 0`,
a 30-term partial sum is provably `>=` a 3-term one — a rigorous proof, not an approximation,
staying entirely float-free (`CI-NOFLOAT`).

### 5. Accrual (`programs/aegis/src/state/market.rs`)

`Market::accrue_view` is pure (`&self`, no mutation) and is the sole computation `Market::accrue_mut`
calls — `accrue_mut` never reimplements the financial formulas.

- **`dt == 0`**: `accrue_with_dt_zero_is_a_no_op` proves `interest == 0`, `fee_amount == 0`, every
  total unchanged, `last_accrual_ts` unchanged, and **no fee shares minted** — a true no-op, not
  merely "no error".
- **`P-ACCRUE-1`** (`p_accrue_1_view_and_mut_agree`, INV-ACC-08): `accrue_view(s, now)`'s totals
  equal what `accrue_mut(s', now)` leaves in `s'` across four cases (typical, empty market, `dt=0`,
  and a near-`u64::MAX`/1-year stress case) — the only permitted divergence, fee shares, is not
  even part of `AccrueOutcome`'s fields, so equality is structural, not coincidental.
- **`P-ACCRUE-2`** (`p_accrue_2_free_liquidity_invariant_under_accrual`, INV-ACC-04):
  `total_supply_assets − total_borrow_assets` is bit-identical before and after accrual.
- **Long-duration overflow safety**: a stress computation (documented in the implementation's own
  comment trail, not asserted as a required test since it uses non-representative inputs) confirmed
  that a full year at `max_rate_ps` against a near-`u64::MAX` `total_borrow_assets` can produce an
  `interest` value that would not fit `u64` — `accrue_view` narrows `interest` to `u64` via a
  checked `u64::try_from` (not `as`) specifically so this fails closed with `ArithmeticOverflow`
  rather than silently truncating; `one_year_dormant_market_accrual` exercises the realistic
  (non-pathological) version of this scenario end-to-end successfully.

### 6. Protocol fees (§4.3, `P-FEE-1`)

`accrue_mut` prices fee shares against `total_supply_assets − fee_amount` — the **pre-fee** base —
exactly as `economic-model.md` §4.3 requires. `p_fee_1_fee_shares_dilute_by_exactly_fee_amount`
proves two things, not one: (a) the fee recipient's claimable assets equal `fee_amount` within 1
unit of rounding, and (b) pricing against the *wrong* (post-fee) denominator would have produced
**strictly fewer** fee shares — i.e. the test would still pass a naive "some shares were minted"
check even with the bug the phase spec calls out ("no obvious test catches it") were the comparison
against the wrong denominator not included explicitly.

### 7. Supply / withdraw (`instructions/lend/{supply,withdraw}.rs`)

Both accrue first, then compute the requested/computed transfer amount using the documented
rounding direction. `supply` transfers the *requested* amount and asserts `credited == requested`
(`VaultAccountingError` on mismatch) rather than trusting the token program's echo — loan assets
are policy-restricted to fee-free mints, so this is verified, never assumed. `withdraw` is bounded
by free liquidity (`total_supply_assets − total_borrow_assets`, the vault-reconciliation identity
itself, not a separate rule) — `withdraw_more_than_free_liquidity_fails` (`U-WD-01`) proves a lender
who owns *enough shares* is still refused with `InsufficientLiquidity` once real debt exists, and
that a withdrawal of exactly the free liquidity succeeds.

### 8. Borrow gate

No oracle-shaped account exists anywhere in `Borrow`'s `#[derive(Accounts)]` struct — there is
nothing a caller could populate with a fake, stale, or assumed price. The handler validates the
exactly-one-of guard and the token program, then unconditionally returns `OracleNotYetAvailable`
before reading or writing any other state:

```
$ (excerpt) tests/phase4_adversarial.rs::borrow_is_hard_gated_returns_oracle_not_yet_available
  borrow(500,000 USDC) -> REJECTED: InstructionError(0, Custom(6040))   # AegisError::OracleNotYetAvailable = 6000+40
  position.borrow_shares after refusal: 0 (unchanged)
  vault.amount unchanged; borrower's ATA received nothing
```

Proven against a market with **real, sufficient liquidity** — the refusal is not "there was nothing
to borrow", it is "the gate fired regardless". A second test
(`borrow_is_hard_gated_regardless_of_form_or_size`) proves the gate fires for both the
assets-given and shares-given forms, and for a 1-unit request as much as a large one.
`scripts/check-collateral-transfer-paths.sh` additionally greps `borrow.rs` and fails CI if it ever
calls either transfer helper — a structural, automated backstop against the gate being weakened by
a future edit that adds a transfer call before the `Err` return.

Everything `borrow` will need *except* the price read and LTV check is implemented and independently
unit-tested as the pure `compute_borrow` function (`instructions/borrow/borrow.rs`) — never called
by the live, gated `handler`, exercised directly by `U-BORROW-01`/`U-BORROW-02`:

```
U-BORROW-01 (INV-BOR-02): free liquidity = 100 (supply 1000, borrow 900).
  request 101 -> InsufficientLiquidity. request 100 -> Ok.
U-BORROW-02 (INV-SOLV-07 / E-25): min_debt = 10.
  borrow(5)  -> DebtBelowMinimum. borrow(10) -> Ok.
```

### 9. Repay (`instructions/borrow/repay.rs`)

No owner signature (`payer: Signer`, no `has_one = owner` on `position`), no oracle account, no
pause check anywhere in the instruction (structural — Phase 12 must never add one, per INV-ADM-04).
Clamped to actual debt: the requested shares are computed and **clamped to
`position.borrow_shares` before** the exact token amount is recomputed from the clamped figure, so
the instruction can never pull more than the debt requires (proved algebraically before writing the
test: for the *unclamped* case, chaining `to_shares_down` then `to_assets_up` on the same numbers is
provably `<=` the original requested amount for any integer `assets`, since `ceil(x) <= a` whenever
`x <= a` and `a` is an integer).

```
U-REPAY-01: debt = 300,000,000. payer requests to repay 1,000,000,000 (>>debt).
  actually pulled: 300,000,000 exactly. position.borrow_shares -> 0.
U-REPAY-02: full repayment via shares drives position.borrow_shares to exactly 0 -- no dust.
repay_by_third_party_succeeds: a stranger with no relationship to the position repays it -- succeeds.
```

### 10. Standalone `accrue_interest`

Permissionless (`accrue_interest`'s caller in the demo and in `i_cus_01_holds_after_every_operation`
is an unrelated keeper, never the admin). Emits `InterestAccrued { interest, fee_amount, fee_shares,
total_borrow_assets, total_supply_assets }`.

### 11. Events

`Supplied`, `Withdrawn`, `Repaid`, `InterestAccrued` are emitted and their fields verified in
integration tests (`Supplied.credited`/`shares_minted` checked against `to_shares_down`;
`Repaid.shares_burned` checked against the clamped figure, etc. — the position/market state
assertions throughout `tests/phase4_lending.rs` are the same numbers the events themselves carry).
`Borrowed` is defined (API completeness against `instruction-catalogue.md`'s event catalogue) but
is **never emitted** — the gated `handler` returns before any `emit!` call could be reached; grep
confirms `programs/aegis/src/instructions/borrow/borrow.rs` contains no `emit!(Borrowed`.

### 12. Exact-one-of guards

`guards::require_exactly_one_amount` is the single shared implementation for all four instructions.
`U-GUARD-01`/`02`/`03` at the `aegis-math`-adjacent unit level (`guards.rs`'s own `#[cfg(test)]`),
plus `supply_rejects_both_zero_and_both_nonzero` and
`withdraw_and_repay_reject_both_zero_and_both_nonzero` exercising it through the real instructions
end-to-end (both invalid forms, on all of `supply`/`withdraw`/`repay`).

### 13. Duplicate mutable accounts (`A-ACC-01`)

`fee_position` is PDA-constrained to `PDA(market, market.fee_recipient)`, never caller-supplied.
The one legitimate scenario where a caller's own `position` coincides with `fee_position` is when
the caller *is* `market.fee_recipient` — `a_acc_01_duplicate_mutable_accounts_rejected` constructs
exactly this coincidence (derives both PDAs, asserts they are equal, then submits `supply` with the
same pubkey passed for both `position` and `fee_position`) and confirms Anchor 1.0's default
duplicate-mutable-account protection rejects it, without any manual dedup code in the program.

### 14. Custody / accounting invariants

`aegis_test_kit::invariants::assert_inv_cus_01` (exact equality, not a bound) and
`assert_all_lending` (INV-CUS-01 + INV-ACC-01/02/03/06) are called after every state-changing step
throughout `tests/phase4_lending.rs`, `tests/phase4_adversarial.rs`, and the demo — see the mapping
table in **Invariant status** above. `loan_vault_direct_donation_is_never_credited` proves the
checker itself is falsifiable (`INV-CUS-01` genuinely fails to hold after a raw donation, exactly
mirroring Phase 3's `A-CUS-08`/`assert_inv_cus_02_detects_uncredited_donation`).

### 15. Tests — commands actually run and results

```
$ cargo test --workspace
   ... (full transcript below is the complete, unedited run) ...

     Running unittests src/lib.rs (target/debug/deps/aegis-...)
running 31 tests
test guards::tests::guard_01_both_zero_is_rejected ... ok
test guards::tests::guard_02_both_nonzero_is_rejected ... ok
test guards::tests::guard_03_exactly_one_nonzero_is_accepted ... ok
test instructions::borrow::borrow::tests::conversions_use_the_documented_rounding_directions ... ok
test instructions::borrow::borrow::tests::u_borrow_01_free_liquidity_bound ... ok
test instructions::borrow::borrow::tests::u_borrow_02_min_debt_floor ... ok
test state::market::tests::accrue_with_dt_zero_is_a_no_op ... ok
test state::market::tests::accrue_view_matches_worked_example ... ok
test state::market::tests::p_accrue_1_view_and_mut_agree ... ok
test state::market::tests::p_accrue_2_free_liquidity_invariant_under_accrual ... ok
test state::market::tests::p_fee_1_fee_shares_dilute_by_exactly_fee_amount ... ok
... (18 Phase 2/3 Market-param tests, unchanged) ...
test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (target/debug/deps/aegis_math-...)
running 26 tests
test shares::tests::first_supply_into_empty_market_applies_virtual_offsets ... ok
test shares::tests::worked_example_alice_then_bob_immediately_yields_zero_tax_exactly ... ok
test shares::tests::later_depositor_receives_fewer_shares_after_ratio_drift ... ok
test shares::tests::round_01_supply_assets_shares_minted_floors ... ok
test shares::tests::round_02_withdraw_assets_shares_burned_ceils ... ok
test shares::tests::round_03_borrow_assets_borrow_shares_minted_ceils ... ok
test shares::tests::round_04_repay_assets_borrow_shares_burned_floors ... ok
test shares::tests::round_05_supply_shares_assets_required_ceils ... ok
test shares::tests::round_06_withdraw_shares_assets_returned_floors ... ok
test shares::tests::round_07_borrow_shares_assets_returned_floors ... ok
test shares::tests::round_08_repay_shares_assets_required_ceils ... ok
test shares::tests::to_assets_survives_maximum_legal_share_asset_state ... ok
test irm::tests::zero_supply_gives_zero_utilization ... ok
test irm::tests::full_utilization_caps_at_wad_and_max_rate ... ok
test irm::tests::worked_example_ninety_percent_utilization_one_day ... ok
test irm::tests::zero_dt_gives_zero_growth ... ok
test irm::tests::taylor_x_is_a_plain_product_of_rate_and_elapsed_seconds ... ok
test irm::tests::round_09_interest_accrual_floors ... ok
test irm::tests::borrow_rate_is_monotone_in_utilization ... ok
test irm::tests::taylor3_never_exceeds_high_precision_reference ... ok
test irm::tests::accrual_over_n_steps_never_exceeds_one_lump_step ... ok
... (5 Phase 1 fixed.rs tests, unchanged) ...
test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/inflation_attack.rs
running 1 test
test a_share_01_inflation_attack_without_vs_with_virtual_offsets ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/property.rs (Phase 1, unchanged)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running tests/rounding_law.rs
running 6 tests
test round_10_protocol_fee_shares_floor ... ok
test round_11_collateral_value_floor ... ok
test round_12_debt_value_ceil ... ok
test round_13_liquidation_seize_floor ... ok
test round_14_liquidation_clamped_repay_ceil ... ok
test round_15_liquidation_protocol_fee_floor ... ok
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/shares_property.rs
running 4 tests
test p_share_1_round_trip_never_creates_value ... ok
test p_share_2_round_trip_never_undercounts_shares ... ok
test p_share_3_supply_then_withdraw_never_profits ... ok
test p_share_4_borrow_then_repay_never_undercollects ... ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.80s

     Running tests/phase2_adversarial.rs / phase2_state.rs / phase2_token_policy.rs (unchanged)
test result: ok. 8 passed ... / ok. 5 passed ... / ok. 9 passed ...

     Running tests/phase3_adversarial.rs / phase3_collateral.rs (unchanged)
test result: ok. 11 passed ... / ok. 5 passed ...

     Running tests/phase4_adversarial.rs
running 10 tests
test lending_instructions_declare_market_writable ... ok
test a_acc_01_duplicate_mutable_accounts_rejected ... ok
test supply_rejects_wrong_token_program ... ok
test supply_rejects_substituted_fee_position ... ok
test supply_rejects_both_zero_and_both_nonzero ... ok
test borrow_is_hard_gated_regardless_of_form_or_size ... ok
test loan_vault_direct_donation_is_never_credited ... ok
test non_owner_cannot_withdraw_someone_elses_supply ... ok
test borrow_is_hard_gated_returns_oracle_not_yet_available ... ok
test withdraw_and_repay_reject_both_zero_and_both_nonzero ... ok
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s

     Running tests/phase4_lending.rs
running 9 tests
test supply_and_withdraw_round_trip ... ok
test full_repayment_via_shares_leaves_no_dust ... ok
test repay_clamps_to_actual_debt_never_pulls_excess ... ok
test repay_by_third_party_succeeds ... ok
test one_year_dormant_market_accrual ... ok
test hundred_percent_utilization ... ok
test multi_user_supply_withdraw_with_interest ... ok
test i_cus_01_holds_after_every_operation ... ok
test withdraw_more_than_free_liquidity_fails ... ok
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s

     Running tests/smoke.rs (unchanged)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s

   Doc-tests aegis / aegis_math / aegis_test_kit — 0 tests each, ok
```

**129 tests total, 0 failures** (31 + 26 + 1 + 3 + 6 + 4 + 0 + 8 + 5 + 9 + 11 + 5 + 10 + 9 + 1 + 0 +
0 + 0 = 129). The full, unedited transcript was captured directly from the command above; ellipses
above elide only test names already documented verbatim in the Phase 1/2/3 evidence sections of
this file, never their pass/fail outcome or counts.

Required test IDs, all passing:

| ID | Test | File |
|---|---|---|
| `U-SHARE-01` | First supply into empty market | `crates/aegis-math/src/shares.rs::first_supply_into_empty_market_applies_virtual_offsets` |
| `U-SHARE-02` | Later-depositor tax (post ratio-drift) | `crates/aegis-math/src/shares.rs::later_depositor_receives_fewer_shares_after_ratio_drift` |
| `U-IRM-01` | `dt=0` → zero growth | `crates/aegis-math/src/irm.rs::zero_dt_gives_zero_growth` |
| `U-IRM-02` | Zero supply → `u=0` | `crates/aegis-math/src/irm.rs::zero_supply_gives_zero_utilization` |
| `U-IRM-03` | Worked example (exact) | `crates/aegis-math/src/irm.rs::worked_example_ninety_percent_utilization_one_day`, `state/market.rs::accrue_view_matches_worked_example` |
| `U-IRM-04` | 100% utilization / rate cap | `crates/aegis-math/src/irm.rs::full_utilization_caps_at_wad_and_max_rate` |
| `U-IRM-05` | Monotonic `last_accrual_ts` (math component) | `crates/aegis-math/src/irm.rs::taylor_x_is_a_plain_product_of_rate_and_elapsed_seconds` |
| `U-ROUND-01..15` | All 15 rounding-law rows | see §2 table above |
| `U-WD-01` | Withdraw exceeds free liquidity | `tests/phase4_lending.rs::withdraw_more_than_free_liquidity_fails` |
| `U-REPAY-01` | Repay clamps to debt | `tests/phase4_lending.rs::repay_clamps_to_actual_debt_never_pulls_excess` |
| `U-REPAY-02` | Full repay leaves no dust | `tests/phase4_lending.rs::full_repayment_via_shares_leaves_no_dust` |
| `U-BORROW-01` | Free-liquidity bound | `instructions/borrow/borrow.rs::u_borrow_01_free_liquidity_bound` |
| `U-BORROW-02` | `min_debt` floor | `instructions/borrow/borrow.rs::u_borrow_02_min_debt_floor` |
| `U-GUARD-01..03` | Exactly-one-of guard | `guards.rs` unit tests + `tests/phase4_adversarial.rs` |
| `P-SHARE-1..4` | Round-trip never creates value | `crates/aegis-math/tests/shares_property.rs` |
| `P-IRM-1` | Rate monotone in `u` | `crates/aegis-math/src/irm.rs::borrow_rate_is_monotone_in_utilization` |
| `P-IRM-2` | `taylor3 <= e^x-1` | `crates/aegis-math/src/irm.rs::taylor3_never_exceeds_high_precision_reference` |
| `P-IRM-3` | Sub-additivity of the discount | `crates/aegis-math/src/irm.rs::accrual_over_n_steps_never_exceeds_one_lump_step` |
| `P-FEE-1` | Fee dilution exact | `state/market.rs::p_fee_1_fee_shares_dilute_by_exactly_fee_amount` |
| `P-ACCRUE-1` | `accrue_view == accrue_mut` | `state/market.rs::p_accrue_1_view_and_mut_agree` |
| `P-ACCRUE-2` | Free liquidity invariant under accrual | `state/market.rs::p_accrue_2_free_liquidity_invariant_under_accrual` |
| `P-ARITH-3` | 256-bit intermediate survives max legal state | `crates/aegis-math/src/shares.rs::to_assets_survives_maximum_legal_share_asset_state` (Phase 4 instance; Phase 1's own remains in `crates/aegis-math/tests/property.rs`) |
| `A-SHARE-01` | Inflation attack, both branches | `crates/aegis-math/tests/inflation_attack.rs` |
| `A-ACC-01` | Duplicate mutable accounts | `tests/phase4_adversarial.rs::a_acc_01_duplicate_mutable_accounts_rejected` |
| `A-CUS-08` (loan side) | Direct donation never credited | `tests/phase4_adversarial.rs::loan_vault_direct_donation_is_never_credited` |
| `I-CUS-01` | INV-CUS-01 after every op | `tests/phase4_lending.rs::i_cus_01_holds_after_every_operation` |
| — | Multi-user supply/withdraw with interest | `tests/phase4_lending.rs::multi_user_supply_withdraw_with_interest` |
| — | One-year dormant market | `tests/phase4_lending.rs::one_year_dormant_market_accrual` |
| — | 100% utilization | `tests/phase4_lending.rs::hundred_percent_utilization` |
| — | Borrow hard gate (2 forms) | `tests/phase4_adversarial.rs::borrow_is_hard_gated_*` |

### 16. Demo

```
$ make demo
anchor build
cargo run -p aegis-test-kit --example phase4_demo
Aegis Protocol — Phase 4 demo (lending, borrowing and interest)
Zero-cost, local, offline: in-process LiteSVM, no devnet, no RPC, no API key.

Deployed program 2GtoBADM175vkjf5UYpbD198Ry1cJadXMGo8sCQvXndh into LiteSVM.
Admin/deployer:  GmaDrppBC7P5ARKV8g3djiwP89vz1jLK23V2GBjuAEGB

=== 1. Protocol and market ===
Market:      FH3ZCzxQmK4LkVoBJi27YBccoSq68FUUDSsYA7GTKsg4
loan_vault:  BvLTnssWjnTZtLbN6gq5wWEfEieUbG1PGjD4x9Mh9gdC
fee_position: DNmsGKwhqzLfiPDBLCZEm2SLzBkAVFqGDdeGJSrrqscJ (owner GyGKxMyg1p9SsHfm15MkNUu1u9TN2JtTspcdmrtGUdse)

=== 2. Lender supplies loan liquidity ===
  supplied:       1000000000000 (1,000,000.000000 USDC)
  supply_shares:  1000000000000000000
  INV-CUS-01 / INV-ACC-01/02/03/06: all hold

=== 3. Borrow is attempted -- and correctly refused ===
  borrow(500,000 USDC) -> REJECTED: InstructionError(0, Custom(6040))
  position.borrow_shares after refusal: 0 (unchanged)

=== 4. Seed debt via TEST-KIT state injection ===
  seeded total_borrow_assets += 900000000000 (900,000.000000 USDC)
  (this is a test fixture, not a real instruction -- borrow remains hard-gated)
  INV-CUS-01: holds immediately after injection

=== 5. Time warped 30 days (sysvar Clock, no real wall-clock waiting) ===
  last_accrual_ts before: 0
  warped forward by:      2592000 seconds (30 days)

=== 6. Utilization and projected APYs (current-rate projection) ===
  utilization: 90.0000%
  borrow APY (projected): 71.2043%
  supply APY (projected, net of 10.0000% protocol fee): 54.7006%

=== 7. accrue_interest (permissionless) ===
  called by: GhFJh9xhWQULf6W1WJLNTViiTWEs4wAj3FevZ616wxL2 (an unrelated keeper, not the admin)
  total_borrow_assets: 900000000000 -> 940844775401
  total_supply_assets: 1000000000000 -> 1040844775401
  last_accrual_ts:     0 -> 2592000
  interest accrued over 30 days: 40844775401 base units (40844.775401 USDC)
  INV-CUS-01 / INV-ACC-01/02/03/06: all hold after accrual

=== 8. Protocol fee shares accrued ===
  fee_position.supply_shares: 3939654661185489
  fee_position's claimable assets: 4084477539 base units

=== 9. Lender withdraws principal plus earned interest ===
  lender's full claim: 1036760297860 (principal 1000000000000 + interest 36760297860)
  free liquidity available: 100000000000
  withdrawing: 100000000000 (bounded by free liquidity: most of the pool is lent out to the borrower)
  lender_ata balance after withdrawal: 100000000000

  INV-CUS-01 / INV-ACC-01/02/03/06: all hold after the full flow

Demo complete. All Phase 4 acceptance criteria exercised above.
```

### 17. Regression

```
$ cargo fmt --all --check
(no output — clean)

$ cargo clippy --workspace --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s)
(zero warnings, all four crates including every test/example target)

$ for s in scripts/check-*.sh; do ./"$s"; done
check-collateral-transfer-paths: OK — vault token movement goes through exactly the shared helpers, from exactly their enumerated call sites (borrow.rs calls neither, as required by the Phase 4 gate)
check-no-close: OK — no close constraint targets Market or Protocol
check-no-dup: OK — no 'dup' constraint in programs/
check-no-float: OK — no f32/f64 in programs/ or crates/aegis-math/
check-no-init-if-needed: OK — no init_if_needed constraint or feature in use
check-no-slot-time: OK — no Clock.slot usage in programs/
check-overflow-checks: OK — overflow-checks = true is set in [profile.release]
```

`scripts/check-collateral-transfer-paths.sh` was updated (not weakened): its allowlist now
enumerates the three new legitimate Phase 4 call sites (`supply.rs`, `withdraw.rs`, `repay.rs`) in
addition to Phase 3's two, and gained an explicit check that `borrow.rs` calls **neither** transfer
helper — a stronger assertion than the script made before, not a relaxed one.

Every Phase 1/2/3 test above continues to pass unchanged (verified in the same `cargo test
--workspace` run, §15). `anchor build` (SBF target) required no new stack-frame workaround beyond
the `Box<Account<'info, Market>>` pattern already established in Phase 2/3, reused unchanged in
`Supply`, `Withdraw`, `Borrow`, and `Repay`.

### 18. Deviations

None requiring an ADR (no frozen formula, account field, seed, or invariant was changed). Three
documentation-accuracy findings, recorded rather than silently worked around:

1. **`economic-model.md` §1.3's rounding table has 15 rows but its closing sentence says
   "`U-ROUND-01..14`".** All 15 rows are implemented and tested (§2 above), numbered
   `U-ROUND-01..15` so none is silently dropped to match the document's own undercount.
2. **`economic-model.md` §3.3's worked example, applied with exact (non-approximate) integer
   arithmetic, produces zero "later-depositor tax"** for the specific numbers it gives (Alice and
   Bob each supplying 1e9 into an otherwise-untouched pool), contradicting its own prose ("Bob
   receives marginally fewer shares... ≈ 999,999,999,000,000"). This is a general fact of the
   formula given `VIRTUAL_ASSETS = 1` (the first deposit into an empty pool is loss-free, landing
   `total_shares:total_assets` exactly on `VIRTUAL_SHARES:VIRTUAL_ASSETS`, so every subsequent
   deposit — of any size — also divides out exactly, until the ratio is perturbed by something
   else, e.g. real interest accrual), not an error in this implementation. The frozen *formula* is
   unambiguous and is exactly what `worked_example_alice_then_bob_immediately_yields_zero_tax_exactly`
   encodes; the real, non-degenerate tax property is separately demonstrated in
   `later_depositor_receives_fewer_shares_after_ratio_drift`, where the ratio has genuinely drifted
   (as it does after real interest accrual).
3. **`aegis_test_kit::reference_market_args` (shared with Phase 2/3) sets every IRM slope to
   zero.** Phase 2/3 never accrue interest, so this was never exercised before. Phase 4's tests
   need the real reference IRM curve from `economic-model.md` §4.1, so the affected fields are
   overridden at each Phase 4 test's own `setup_market`/demo call site (via struct-update syntax)
   rather than changing the shared Phase 2/3 helper — the minimal, lowest-risk fix, leaving every
   passing Phase 2/3 test byte-for-byte unaffected.

One design decision worth recording (not a frozen-document change, following Phase 3's own
precedent exactly): **none of `supply`/`withdraw`/`borrow`/`repay`/`accrue_interest` check a pause
bit.** `set_market_pause`/`set_protocol_pause` are Phase 12 scope, and before they exist no
instruction can ever set a pause bit to nonzero — a check today would be dead code with no way to
exercise it honestly, the identical reasoning Phase 3 recorded for `withdraw_collateral`. `protocol`
is correspondingly omitted from every Phase 4 `Accounts` struct (it is listed in
`instruction-catalogue.md` only for that future pause check), also matching Phase 3's precedent for
`withdraw_collateral`.

### 19. Security self-audit

Performed before declaring Phase 4 complete, per the task's final-audit checklist. Every answer
below is backed by a specific test named in this section, not merely asserted.

| Question | Answer |
|---|---|
| Can rounding create value? | No — `P-SHARE-1..4` prove round-tripping through any pair of the four conversions never returns more than was put in, over tiny/large/near-zero/high-and-low-price states. |
| Can supply shares be manipulated by donation? | No — `total_supply_assets` is a `Market` accounting scalar, never derived from `loan_vault`'s raw balance; `loan_vault_direct_donation_is_never_credited` proves a raw transfer changes the vault balance but not `total_supply_assets`, and that `assert_inv_cus_01` then correctly observes the mismatch. |
| Can virtual offsets be bypassed? | No — `to_shares_*`/`to_assets_*` hardcode `VIRTUAL_SHARES`/`VIRTUAL_ASSETS`; they are not function parameters anywhere in production code. |
| Can the inflation attack become profitable? | No — `A-SHARE-01` proves it is a net *loss* (not merely break-even) for the attacker with the real offsets, for the identical capital and victim deposit that make it profitable without them. |
| Can fee shares be under/over-minted? | No — `P-FEE-1` proves dilution equals `fee_amount` within 1 unit, and separately proves the wrong denominator would under-mint. |
| Is the fee denominator wrong? | No — `accrue_mut` explicitly computes `total_supply_assets.checked_sub(fee_amount)` (the pre-fee base) before pricing fee shares; `P-FEE-1`'s wrong-denominator comparison would fail if this regressed. |
| Can repeated accrual diverge from view computation? | No — `P-ACCRUE-1` asserts exact equality across four states including a stress case; `accrue_mut` calls `accrue_view` rather than reimplementing it, so they cannot structurally diverge. |
| Can `dt == 0` mutate economics? | No — `accrue_with_dt_zero_is_a_no_op` asserts zero interest, zero fee shares, and byte-identical totals. |
| Can withdraw exceed free liquidity? | No — `withdraw_more_than_free_liquidity_fails` proves it fails even when the caller owns sufficient shares. |
| Can raw donated vault tokens permit extra withdrawal? | No — `withdraw`'s free-liquidity check reads `market.total_supply_assets`/`total_borrow_assets` (accounting scalars), never `loan_vault.amount` directly. |
| Can repay pull excess tokens? | No — `repay_clamps_to_actual_debt_never_pulls_excess` proves an overpay request of >3x the debt still pulls exactly the debt. |
| Can third-party repay be incorrectly blocked? | No — `repay_by_third_party_succeeds` proves a stranger with zero relationship to the position can repay it. |
| Can repay be paused? | No — `repay.rs` contains no pause check of any kind; there is no bit to set that would affect it even after Phase 12. |
| Can borrow succeed without oracle? | No — `borrow_is_hard_gated_returns_oracle_not_yet_available` and `borrow_is_hard_gated_regardless_of_form_or_size` prove the unconditional gate against a market with real, sufficient liquidity, for both input forms. |
| Can `Position` and `fee_position` alias? | No — `A-ACC-01` constructs the one legitimate coincidence (caller == `market.fee_recipient`) and proves Anchor 1.0's default protection rejects passing the same pubkey for both. |
| Can overflow occur before `mul_div`? | No — every accumulation into a `mul_div_*` input (`total_borrow_assets + interest`, etc.) uses `checked_add`/`checked_sub` first; `mul_div_floor`/`ceil` themselves use the Phase 1 256-bit intermediate. |
| Is any float present? | No — `check-no-float.sh` passes across `programs/` and `crates/aegis-math/`, including every new file. |
| Is any rounding direction inconsistent with the frozen table? | No — all 15 rows individually tested (§2); the one documentation inconsistency found (14 vs. 15 rows) is in the table's own summary sentence, not in any formula. |
| Did Phase 5 oracle logic accidentally enter Phase 4? | No — `grep -rniE "oracle|pyth|price_update" programs/aegis/src/instructions/{lend,borrow}` returns only doc-comment references to the *absence* of oracle logic; `grep -rniE "pub fn (liquidate|absorb_bad_debt)" programs/aegis/src/` returns nothing. |

No changes were forced by this audit beyond what is already reflected in the code above — every
question was checked against a test that already existed by the time the audit was performed.

Git commit SHA, tag, and remote-verification output are reported in the Phase 4 completion report
(not embedded here, to avoid a self-referencing commit hash inside the commit it would describe).

---

## Phase 3 — evidence

### 1. `deposit_collateral`

`programs/aegis/src/instructions/collateral/deposit_collateral.rs` implements
`instruction-catalogue.md` §10 exactly: `depositor` need not be `position.owner` (INV-AUTH-03);
`market` is `Box<Account<'info, Market>>` **without** `#[account(mut)]`, so Anchor generates a
read-only `AccountMeta` for it (proven by `A-PAR-01`, not inferred); `collateral_vault` is
double-validated (`seeds = [COLLATERAL_VAULT_SEED, market], bump = market.collateral_vault_bump`
**and** `address = market.collateral_vault`); `collateral_mint` is pinned by `address =
market.collateral_mint`; the token program is pinned by an explicit `require_keys_eq!` against
`market.collateral_token_program` (T-11 — the interface type alone accepts either program). No
oracle account, no pause check, no health check exist anywhere in this instruction's accounts or
handler.

### 2. Measured-delta accounting

`programs/aegis/src/token/transfer.rs::transfer_checked_in` implements the mandatory sequence from
`account-model.md` §6.4 and `token-compatibility.md` §5.3 verbatim:

```rust
let before = vault.amount;
token_interface::transfer_checked(/* ... */, amount, decimals)?;
vault.reload()?;                                    // MANDATORY — pre-CPI data is stale
let after = vault.amount;
after.checked_sub(before).ok_or_else(|| error!(AegisError::VaultAccountingError))
```

`deposit_collateral`'s handler credits `position.collateral_amount` by exactly the returned
`credited` value, never by the requested `amount`. Evidence that this actually matters, not just
that the code looks right:

```
U-TOK-01 (SPL, no fee):        requested = 5_000_000_000  credited = 5_000_000_000  (equal)
U-TOK-02 (Token-2022, 5% fee): requested = 1_000_000_000  credited =   950_000_000  (fee = 50_000_000)
```

— both figures read directly from on-chain state (`position.collateral_amount` and the vault's
own `amount` field) after a real CPI through the actual embedded Token-2022 program, never
computed by the test and asserted against itself.

### 3. `token/transfer.rs`

One inbound helper (`transfer_checked_in`, measured-delta, mandatory `reload()`) and one outbound
helper (`transfer_checked_out`, `invoke_signed` via `CpiContext::with_signer`), both built on
`anchor_spl::token_interface::transfer_checked` — which dispatches to whichever token program the
caller's `CpiContext::new(token_program.key(), ...)` names, so one code path serves both SPL Token
and Token-2022 (`token-compatibility.md` §5.1–5.3). Neither helper is called from anywhere except
its one intended collateral instruction — enforced by the new `scripts/check-collateral-transfer-
paths.sh` guard (`A-CUS-04`/INV-CUS-04), which greps for every call site of both helpers and of the
raw `token_interface::transfer_checked` function and fails if either appears outside its expected
home.

### 4. `withdraw_collateral` — the Phase 3 zero-debt path and the debt hard gate

`programs/aegis/src/instructions/collateral/withdraw_collateral.rs` requires `owner` as an actual
transaction `Signer` with `has_one = owner @ AegisError::NotPositionOwner` (INV-AUTH-02) — the
asymmetric counterpart to deposit's no-signer-required depositor. Before touching any balance, it
checks:

```rust
require!(ctx.accounts.position.borrow_shares == 0, AegisError::OracleNotYetAvailable);
```

This is the *only* check on the debt branch — there is no placeholder price, no "assumed healthy"
path, and no oracle account anywhere in the `Accounts` struct (`docs/phase-roadmap.md` "Sequencing
the oracle dependency"). Because no Phase 1-3 instruction can ever set `position.borrow_shares !=
0`, the adversarial test injects that state directly via `svm.set_account` — the same legitimate
fixture technique Phase 2's `attacker_owned_fake_protocol_account_is_rejected` already established
— and proves the instruction refuses it with exactly `OracleNotYetAvailable`, leaving the position
untouched.

On success, the vault-outflow CPI is signed by the `Market` PDA using its own stored, canonical
seeds and bump — never a caller-supplied bump (no instruction in this phase accepts one):

```rust
let signer_seeds: &[&[u8]] = &[
    MARKET_SEED, market.collateral_mint.as_ref(), market.loan_mint.as_ref(),
    &config_id_bytes, &[market.bump],
];
```

`Market` is never written here either — same read-only `Box<Account<'info, Market>>` pattern as
`deposit_collateral`, proven by the same `A-PAR-01` test on this instruction's own generated
account metadata.

### 5. `close_position`

`programs/aegis/src/instructions/position/close_position.rs` requires the **exact** equality
`supply_shares == 0 && borrow_shares == 0 && collateral_amount == 0` (never a dust tolerance) and
uses Anchor's `close = owner` — lamports returned, discriminator zeroed, account reassigned to the
System Program and resized to zero (`common::close` in `anchor-lang` 1.2.0), not the removed
`CLOSED_ACCOUNT_DISCRIMINATOR` pattern. `U-LIFE-01` proves the precondition (a premature close on a
still-funded position fails with `PositionNotEmpty`; the same position closes successfully once
its collateral is withdrawn to zero). `A-LIFE-02` proves revival safety: after close, a `deposit_
collateral` call against the stale address fails (no discriminator left to deserialize), and
`init_position` can recreate the same PDA later — always completely empty.

### 6. `aegis-test-kit::invariants` — the INV-CUS-02 checker

`crates/aegis-test-kit/src/invariants.rs::assert_inv_cus_02` asserts the **exact** integer equality
`collateral_vault.amount == Σ(position.collateral_amount) + market.collateral_fee_accrued` — no
epsilon, no approximate comparison. It is called after every state-changing step in
`tests/phase3_collateral.rs` and in the Phase 3 demo (§8 below). Its own falsifiability is proven,
not assumed: `assert_inv_cus_02_detects_uncredited_donation` (`#[should_panic(expected = "INV-
CUS-02 violated")]`) performs a direct donation to the vault and asserts the checker panics —
exactly the AGENTS.md §8 requirement that "an invariant without a falsifying test is a hope."

### 7. Tests

```
$ cargo test --workspace
running 20 tests
test state::market::tests::close_factor_below_minimum_is_rejected ... ok
test state::market::tests::derived_liquidation_bound_rejects_plausible_but_unsafe_params ... ok
test state::market::tests::fee_above_max_is_rejected ... ok
test state::market::tests::irm_rate_exceeding_max_is_rejected ... ok
test state::market::tests::len_matches_account_model_spec ... ok
test state::market::tests::irm_params_reference_set_is_valid ... ok
test state::market::tests::irm_u_kink_out_of_range_is_rejected ... ok
test state::market::tests::full_liq_hf_zero_is_rejected ... ok
test state::market::tests::liq_bonus_above_max_is_rejected ... ok
test state::market::tests::liq_protocol_fee_above_max_is_rejected ... ok
test state::market::tests::max_ltv_must_be_below_liq_threshold ... ok
test state::market::tests::oracle_config_conf_bps_out_of_range_is_rejected ... ok
test state::market::tests::oracle_config_price_age_out_of_range_is_rejected ... ok
test state::market::tests::oracle_config_reference_is_valid ... ok
test state::market::tests::reference_parameter_set_is_valid ... ok
test state::market::tests::zero_min_debt_is_rejected ... ok
test state::position::tests::len_matches_account_model_spec ... ok
test state::protocol::tests::len_matches_account_model_spec ... ok
test token::policy::tests::transfer_fee_mint_requires_transfer_fee_amount_and_immutable_owner ... ok
test token::policy::tests::vault_extensions_always_include_immutable_owner ... ok
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (target/debug/deps/aegis_math-...)
running 5 tests (fixed::tests::{division_by_zero, ceil_only_rounds_up_on_a_nonzero_remainder,
result_overflow, known_vectors, large_multiplication_survives_256_bit_intermediate})
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/property.rs
running 3 tests (never_panics, floor_le_ceil_le_floor_plus_one, matches_bignum_reference)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running unittests src/lib.rs (target/debug/deps/aegis_test_kit-...)
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/phase2_adversarial.rs
running 8 tests
test reinitializing_protocol_fails ... ok
test attacker_owned_fake_protocol_account_is_rejected ... ok
test non_admin_cannot_create_market ... ok
test non_canonical_bump_is_rejected ... ok
test reinitializing_position_fails ... ok
test reference_parameter_set_is_accepted_on_chain ... ok
test reinitializing_market_fails ... ok
test out_of_bounds_market_parameters_are_rejected ... ok
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s

     Running tests/phase2_state.rs
running 5 tests
test seed_prefixes_are_pairwise_distinct ... ok
test protocol_initializes_with_expected_admin_and_layout ... ok
test create_market_does_not_write_protocol ... ok
test create_market_spl_and_position_lifecycle ... ok
test two_markets_same_asset_pair_different_config_id_coexist ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s

     Running tests/phase2_token_policy.rs
running 9 tests
test transfer_hook_mint_rejected_as_collateral ... ok
test permanent_delegate_mint_rejected ... ok
test default_account_state_frozen_mint_rejected ... ok
test tier_a_extensions_are_accepted_and_recorded ... ok
test mint_close_authority_mint_rejected ... ok
test unrecognized_extension_mint_rejected ... ok
test transfer_fee_mint_accepted_as_collateral_rejected_as_loan_asset ... ok
test freeze_authority_requires_acknowledgement ... ok
test wrong_token_program_for_mint_is_rejected ... ok
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s

     Running tests/phase3_adversarial.rs
running 11 tests
test market_is_not_writable_in_collateral_instructions ... ok
test deposit_rejects_substituted_vault ... ok
test deposit_by_non_owner_succeeds ... ok
test deposit_rejects_wrong_mint ... ok
test direct_donation_is_never_credited ... ok
test withdraw_with_outstanding_debt_returns_oracle_not_yet_available ... ok
test assert_inv_cus_02_detects_uncredited_donation - should panic ... ok
test non_owner_withdraw_fails ... ok
test closed_position_cannot_be_revived_with_stale_data ... ok
test wrong_token_program_for_spl_market_is_rejected ... ok
test wrong_token_program_for_token2022_market_is_rejected ... ok
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s

     Running tests/phase3_collateral.rs
running 5 tests
test spl_deposit_credits_exact_amount ... ok
test withdraw_all_with_zero_debt ... ok
test token2022_transfer_fee_deposit_credits_net_of_fee ... ok
test custody_invariant_holds_across_multiple_positions ... ok
test close_position_requires_exact_zero_balances ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s

     Running tests/smoke.rs
running 1 test
test ping_deploys_and_invokes_offline ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s

   Doc-tests aegis / aegis_math / aegis_test_kit — 0 tests each, ok
```
**67 tests, 0 failures.**

Required test IDs, all passing, and where each lives:

| ID | Test | File |
|---|---|---|
| `U-TOK-01` | SPL deposit: `credited == amount` | `tests/phase3_collateral.rs::spl_deposit_credits_exact_amount` |
| `U-TOK-02` | Transfer-fee deposit: `credited == amount − fee` | `tests/phase3_collateral.rs::token2022_transfer_fee_deposit_credits_net_of_fee` |
| `U-WDC-01` | Withdraw all with zero debt | `tests/phase3_collateral.rs::withdraw_all_with_zero_debt` |
| `U-LIFE-01` | Close requires exact zeros | `tests/phase3_collateral.rs::close_position_requires_exact_zero_balances` |
| `A-LIFE-02` | Revival attempt after close | `tests/phase3_adversarial.rs::closed_position_cannot_be_revived_with_stale_data` |
| `A-CUS-01` | Substituted vault | `tests/phase3_adversarial.rs::deposit_rejects_substituted_vault` |
| `A-CUS-04` | Transfer-path audit (grep) | `scripts/check-collateral-transfer-paths.sh` |
| `A-CUS-06` | Wrong mint | `tests/phase3_adversarial.rs::deposit_rejects_wrong_mint` |
| `A-CUS-08` | Direct donation never credited | `tests/phase3_adversarial.rs::direct_donation_is_never_credited` (+ `assert_inv_cus_02_detects_uncredited_donation`) |
| `A-AUTH-02` | Non-owner withdraw fails | `tests/phase3_adversarial.rs::non_owner_withdraw_fails` |
| `A-AUTH-03` | Deposit by non-owner succeeds | `tests/phase3_adversarial.rs::deposit_by_non_owner_succeeds` |
| `A-TOK-08` | Wrong token program (SPL market) | `tests/phase3_adversarial.rs::wrong_token_program_for_spl_market_is_rejected` |
| `A-TOK-09` | Wrong token program (Token-2022 market) | `tests/phase3_adversarial.rs::wrong_token_program_for_token2022_market_is_rejected` |
| `A-PAR-01` | `Market` not writable | `tests/phase3_adversarial.rs::market_is_not_writable_in_collateral_instructions` |
| `I-CUS-02` | INV-CUS-02 across multiple positions | `tests/phase3_collateral.rs::custody_invariant_holds_across_multiple_positions` |

Also exercised, not on the required list: `withdraw_with_outstanding_debt_returns_oracle_not_yet_available`
(the debt hard-gate, task item 16) and per-step `assert_inv_cus_02` calls throughout
`tests/phase3_collateral.rs` and the demo.

### 8. Adversarial evidence

Every adversarial test asserts a **specific** `AegisError` or, where the rejection is a Anchor
framework check, is_err() on a substitution that cannot syntactically produce a specific `AegisError`
(the same convention Phase 2 established):

| Attack | Result |
|---|---|
| Substituted (non-canonical) collateral vault | Anchor `ConstraintSeeds`/`ConstraintAddress` rejection |
| Wrong collateral mint | `VaultMintMismatch` |
| Wrong token program, SPL market | `TokenProgramMismatch` |
| Wrong token program, Token-2022 market | `TokenProgramMismatch` |
| Direct donation to the vault | Not credited to any position (`position.collateral_amount` unchanged); `assert_inv_cus_02` then panics, proving the checker would catch a real accounting bug of this shape |
| Non-owner signs and claims to be `owner` | `NotPositionOwner` |
| Stranger deposits into someone else's position | **Succeeds** — by design (INV-AUTH-03) |
| Withdraw with `position.borrow_shares > 0` (fixture-injected) | `OracleNotYetAvailable`, position left unchanged |
| Close with nonzero `collateral_amount` | `PositionNotEmpty` |
| Deposit against a closed (stale) position | Anchor account-deserialization rejection (no discriminator left) |
| `deposit_collateral`/`withdraw_collateral` `market` account metadata | `is_writable == false` in both (own account-metas inspection, not source review) |

### 9. Demo

```
$ make demo
anchor build
cargo run -p aegis-test-kit --example phase3_demo
Aegis Protocol — Phase 3 demo (collateral flows)
Zero-cost, local, offline: in-process LiteSVM, no devnet, no RPC, no API key.

Deployed program 2GtoBADM175vkjf5UYpbD198Ry1cJadXMGo8sCQvXndh into LiteSVM.
Admin/deployer:  GmaDrppBC7P5ARKV8g3djiwP89vz1jLK23V2GBjuAEGB

=== 1. Protocol, markets and positions ===
Protocol initialized. admin=GmaDrppBC7P5ARKV8g3djiwP89vz1jLK23V2GBjuAEGB guardian=9hSR6S7WPtxmTojgo6GG3k4yDPecgJY292j7xrsUGWBu
SPL market:         FH3ZCzxQmK4LkVoBJi27YBccoSq68FUUDSsYA7GTKsg4
  collateral_vault: HKyEdmNqhZuWoU5wkcvb5hC6AjkHU2NZ94woJFcfw2cv
Token-2022 market:  BJc8KXjjLDzZe61uZQwgYvNejTy36dnBqAL49gUcyKym  (5% transfer fee on collateral)
  collateral_vault: 7K64vgh7NjgUBFrBYTdRyxUrux3HN5xLGH3kwnAXnpHd
SPL market position:        GzD7si8LgCqKdEbSodFSoQC5FCHNTxvMqn4k6AKhuDqv (owner GhFJh9xhWQULf6W1WJLNTViiTWEs4wAj3FevZ616wxL2)
Token-2022 market position: G7LRN7Km8Ggb4uRD9RRJYDHFEu3JeQQ1kfPgGEiSyKcA (owner HqznL4EpJTbWZmqqetb4sJPftBUN1s6uNdQURBAfAsBr)

=== 2. SPL collateral deposit (no fee) ===
  requested: 5000000000
  credited:  5000000000
  INV-CUS-02: holds exactly (vault == Σ positions + fee_accrued)

=== 3. Token-2022 transfer-fee collateral deposit ===
  requested: 1000000000
  credited:  950000000  (fee = 50000000)
  INV-CUS-02: holds exactly against the credited (not requested) amount

=== 4. Zero-debt withdrawal (SPL market) ===
  withdrawn: 5000000000
  position.collateral_amount now: 0
  INV-CUS-02: holds exactly

=== 5. close_position — rent reclaimed ===
  position rent (lamports):        1900080
  owner balance before close:       9999995000
  owner balance after close:        10001890080
  position account after close:     purged

Demo complete. All Phase 3 acceptance criteria exercised above.
```

### 10. Regression — Phase 1/2 guarantees re-run

```
$ cargo fmt --all --check
(no output — clean)

$ cargo clippy --workspace --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s)
(zero warnings)

$ for s in scripts/check-*.sh; do ./"$s"; done
check-collateral-transfer-paths: OK — vault token movement goes through exactly the two Phase 3 helpers, from exactly their two intended call sites
check-no-close: OK — no close constraint targets Market or Protocol
check-no-dup: OK — no 'dup' constraint in programs/
check-no-float: OK — no f32/f64 in programs/ or crates/aegis-math/
check-no-init-if-needed: OK — no init_if_needed constraint or feature in use
check-no-slot-time: OK — no Clock.slot usage in programs/
check-overflow-checks: OK — overflow-checks = true is set in [profile.release]

$ cargo test --test smoke
test ping_deploys_and_invokes_offline ... ok

$ cargo test --test phase2_state --test phase2_adversarial --test phase2_token_policy
(all 22 Phase 2 tests pass unchanged — see §7 above for the full transcript)
```
`check-no-close.sh`'s own comment already anticipated `close_position` (`Position` *is* closable,
`Market`/`Protocol` are not) — it required no change and correctly does not flag
`close_position.rs`'s `close = owner`.

`anchor build` (SBF target) required no new stack-frame workaround beyond Phase 2's `Box<Account<
'info, Market>>` pattern, which is reused unchanged in `DepositCollateral`, `WithdrawCollateral`
and `ClosePosition`.

### 11. Deviations

None requiring an ADR. One design decision worth recording (not a frozen-document change):
**`withdraw_collateral` does not check any pause bit in Phase 3.** `instruction-catalogue.md` §11's
account list includes `[R][PDA] protocol` — needed only for the eventual pause check — but
`phase-03-collateral.md`'s own scope/test list never mentions pause for either collateral
instruction (unlike `deposit_collateral`, whose scope note explicitly says "no pause"). No
Phase 1-3 instruction can set `Market.paused` or `Protocol.paused` to anything but `0`
(`set_market_pause`/`set_protocol_pause` are Phase 12 scope), so a pause check today would be
dead code with no way to exercise it honestly. Pause enforcement for `withdraw_collateral` is
deferred to Phase 12 alongside those admin instructions, consistent with `INV-ADM-*`'s Phase-12
assignment in `docs/invariants.md`. `Market` is not needed for this either way, and remains
read-only.

---

## Phase 3 self-audit

Performed before declaring Phase 3 complete, per the task's final-audit checklist.

| Question | Answer |
|---|---|
| Can a user credit themselves for a transfer fee they did not receive? | No — `credited` is `after − before` measured post-CPI-`reload()`, never the requested `amount`; `U-TOK-02` proves `credited < requested` on a real 5% fee mint. |
| Is vault state read before CPI and stale after CPI? | No — `before` is read pre-CPI; `vault.reload()` runs immediately after the CPI and before `after` is read. |
| Is `reload()` missing anywhere? | No — `transfer_checked_in` is the only inbound-transfer function in the program (enforced by `scripts/check-collateral-transfer-paths.sh`) and it always reloads; outbound transfers correctly do not reload (the recipient bears the fee, not the protocol's own accounting). |
| Can a direct donation inflate a user's internal balance? | No — `direct_donation_is_never_credited` proves `position.collateral_amount` is unchanged by a raw SPL Token transfer into the vault; `assert_inv_cus_02_detects_uncredited_donation` proves the checker would flag the resulting surplus as a violation if it were ever mistaken for legitimate accounting. |
| Can the wrong vault be substituted? | No — double validation (`seeds`/`bump` **and** `address = market.collateral_vault`); `deposit_rejects_substituted_vault` attempts it with an otherwise-valid token account and is rejected. |
| Can wrong mint/token program pass? | No — `VaultMintMismatch` and `TokenProgramMismatch` respectively, each with a dedicated test (`deposit_rejects_wrong_mint`, `wrong_token_program_for_{spl,token2022}_market_is_rejected`). |
| Can a non-owner withdraw? | No — `has_one = owner @ NotPositionOwner`; `non_owner_withdraw_fails` has an attacker sign and name themselves as `owner`, rejected. |
| Can the owner withdraw with debt before oracle integration? | No — `require!(borrow_shares == 0, OracleNotYetAvailable)` is unconditional and is the first state-dependent check in the handler; `withdraw_with_outstanding_debt_returns_oracle_not_yet_available` proves it against a fixture-injected nonzero `borrow_shares`, since no real instruction can produce one yet. |
| Can `Market` accidentally become writable? | No — `A-PAR-01` inspects the actual `Vec<AccountMeta>` Anchor generates for both `DepositCollateral` and `WithdrawCollateral` and asserts `is_writable == false` on the `market` entry — not inferred from the `#[derive(Accounts)]` source. |
| Can the protocol infer user ownership from vault balance? | No — `assert_inv_cus_02` sums `Position.collateral_amount` fields read from program state; nothing in the program itself ever re-derives a position's balance from the vault's total. |
| Can `Position` be closed with non-zero state? | No — the three-field exact-equality check (`PositionNotEmpty`); `close_position_requires_exact_zero_balances` proves the rejection on a still-funded position and the acceptance once it is empty. |
| Can a closed `Position` be revived improperly? | No — Anchor's `close =` zeroes the discriminator and reassigns the account to the System Program; `closed_position_cannot_be_revived_with_stale_data` proves both that a post-close instruction against the stale address fails, and that a later `init_position` can only recreate it empty. |
| Are PDA signer seeds canonical? | Yes — the outbound CPI's `signer_seeds` are built from `market.collateral_mint`, `market.loan_mint`, `market.config_id`, and `market.bump` — all read from the already-validated `Market` account, never from a caller-supplied argument (no instruction in this phase accepts a bump). |
| Is token authority accidentally user-controlled? | No — the outbound transfer's `authority` is always `market.to_account_info()`; no user `AccountInfo` is ever passed as the CPI authority for vault outflow. |
| Did I implement any Phase 4 lending logic by accident? | No — grep-verified: `grep -rniE "pub fn (supply|borrow|repay|liquidate|accrue|absorb_bad_debt)" programs/aegis/src/` returns nothing. |

No changes were forced by this audit beyond what is already reflected in the code above — every
question was checked against a test that already existed by the time the audit was performed.

---

## Phase 2 — evidence

### 1. Account model

`Protocol`, `Market`, and `Position` (`programs/aegis/src/state/{protocol,market,position}.rs`)
transcribe `account-model.md` §3–5 field-for-field: same order, same types, same `_reserved`
width. Each carries a `LEN` constant computed by summing the documented field groups (not
`size_of::<T>()`, which would reflect Rust's in-memory layout rather than the Borsh-serialized,
Anchor-discriminator-prefixed account size that actually lands on-chain):

| Account | `LEN` (incl. 8-byte discriminator) | `account-model.md` figure |
|---|---|---|
| `Protocol` | 202 | 202 (exact) |
| `Market` | 640 | "~633 ≈ 641" (approximate in the doc; 640 is the exact sum of the same field list) |
| `Position` | 145 | 145 (exact) |

Evidence that these constants match reality, not just each other:

```
$ cargo test -p aegis len_matches_account_model_spec
test state::market::tests::len_matches_account_model_spec ... ok
test state::position::tests::len_matches_account_model_spec ... ok
test state::protocol::tests::len_matches_account_model_spec ... ok
```

And that the account actually produced by `create_market`/`initialize_protocol`/`init_position` is
exactly that size (`U-ACCT-02` — no realloc is ever needed) — from `tests/phase2_state.rs`:
```rust
let account = svm.get_account(&protocol_pubkey).expect("protocol account exists");
assert_eq!(account.data.len(), Protocol::LEN);
...
assert_eq!(market_account.data.len(), Market::LEN);
...
assert_eq!(position_account.data.len(), Position::LEN);
```
All three assertions pass (see the full `cargo test --workspace` transcript in §6 below).

`_reserved` zero (`U-ACCT-01`): every account is constructed with `_reserved: [0u8; N]` explicitly
at initialization (never left uninitialized), and each lifecycle test asserts it directly after
fetch-and-decode, e.g. `assert_eq!(protocol._reserved, [0u8; 64]);`, `assert_eq!(market._reserved,
[0u8; 64]);`, `assert_eq!(position._reserved, [0u8; 32]);` — all in `tests/phase2_state.rs`.

Seeds (`programs/aegis/src/constants.rs`): `PROTOCOL_SEED = b"protocol"`, `MARKET_SEED =
b"market"`, `POSITION_SEED = b"position"`, `COLLATERAL_VAULT_SEED = b"cvault"`, `LOAN_VAULT_SEED =
b"lvault"` — five distinct literal prefixes, asserted pairwise-distinct by
`seed_prefixes_are_pairwise_distinct` (`U-LIFE-02`). Every PDA is derived canonically
(`find_program_address` at creation; `bump = <stored>` on every later read) — never a
caller-supplied bump; proven by `A-LIFE-03` (below).

### 2. Instructions

`initialize_protocol`, `create_market`, `init_position` — implemented exactly to
`instruction-catalogue.md` §1, §6, §9: same accounts, same preconditions, same state transitions,
same events. No `set_*` admin mutation instruction exists (Phase 12 scope); no deposit, withdrawal,
supply, borrow, repay, interest, oracle, or liquidation instruction exists (Phases 3–6 scope) —
confirmed by `grep -rniE "pub fn (deposit|withdraw|borrow|repay|liquidate|supply|accrue)"
programs/aegis/src/`, which returns nothing.

### 3. Custody

Both vaults (`programs/aegis/src/token/vault.rs`) are created by hand — not Anchor's `#[account(init,
token::...)]` sugar — specifically so `ImmutableOwner` can be added to Aegis's own Token-2022
vaults (`token-compatibility.md` §2, §5.4), which that sugar has no attribute for. Order of
operations for a Token-2022 vault: `system_program::create_account` (sized via
`ExtensionType::try_calculate_account_len`, computed from what the mint's own extensions require
via `ExtensionType::get_required_init_account_extensions` plus `ImmutableOwner`) →
`initialize_immutable_owner` → `initialize_account3` (must be last: it marks the account
`Initialized`). A legacy SPL Token vault is always exactly 165 bytes; never hardcoded — the size
is computed by the same function either way, branching only on which token program owns the mint.

Evidence (`A-CUS-03`, INV-ACCT-04/05, from `tests/phase2_state.rs`):
```rust
let cvault_account = svm.get_account(&collateral_vault).expect("collateral vault exists");
assert_eq!(cvault_account.owner, spl_token_interface::ID);
assert_eq!(cvault_account.data.len(), 165, "legacy SPL vault must be exactly 165 bytes");
let cvault_state = fetch_token_account_base(&svm, &collateral_vault);
assert_eq!(cvault_state.mint, collateral_mint);
assert_eq!(cvault_state.owner, market_pubkey, "vault authority must be the Market PDA");
```
And for a Token-2022 transfer-fee collateral vault (`tests/phase2_token_policy.rs`): the vault is
182 bytes (165 + 1 account-type marker + TLV entries for `TransferFeeAmount` and `ImmutableOwner`),
confirmed both by direct assertion (`data.len() > 165`) and printed by `make demo` (§8 below).

Mint/token-program pinning (T-11, `A-TOK-08`-adjacent): `InterfaceAccount<'info, Mint>` only
proves a mint's owner is *one of* SPL Token or Token-2022; `create_market`'s handler additionally
requires `*mint.owner == token_program.key()` for the *specific* program passed for that asset.
`wrong_token_program_for_mint_is_rejected` proves a legacy mint claimed under the Token-2022
program is rejected with `TokenProgramMintMismatch`.

### 4. Token policy

`programs/aegis/src/token/policy.rs` implements the positive allowlist from
`token-compatibility.md` §2 exactly: `evaluate_mint` enumerates a mint's Token-2022 TLV extension
list (empty, trivially, for a classic SPL Token mint — the same code path handles both), matches
each against an explicit `MetadataPointer | TokenMetadata | GroupPointer | TokenGroup |
GroupMemberPointer | TokenGroupMember | InterestBearingConfig | ScaledUiAmount` accept-arm, a
role-gated `TransferFeeConfig` arm, and a catch-all `_ => reject` — so an extension shipped by a
future Token-2022 release that this crate's dependency does not even know how to decode fails
closed automatically (proven by `unrecognized_extension_mint_rejected`, which is rejected with
`InvalidMintAccountData` because the underlying `spl-token-2022-interface` TLV parser itself
cannot decode an unrecognized type code — a byproduct of the library failing closed, not a
misclassification on Aegis's part).

`freeze_authority` (a base-mint field, independent of extensions): `create_market` requires
`ack_freeze_authority == true` whenever either mint has one, and records the fact in
`market.flags` bit 0 — proven by `freeze_authority_requires_acknowledgement` (rejects
unacknowledged, accepts acknowledged, asserts the flag).

`DefaultAccountState` is treated as unconditionally Tier C (rejected regardless of the configured
initial state), not conditionally on `state == Frozen`: `token-compatibility.md` §2's table entry
is a flat Tier C row; reading "DefaultAccountState = Frozen" as a value-conditional carve-out would
require the document to also specify the `Initialized` case, which it does not. This is a reading
of the frozen document, not a deviation from it, and is the more conservative (fail-closed) of the
two readings besides.

### 5. Parameter security

All bounds from `economic-model.md` §5 (`programs/aegis/src/state/market.rs::{validate_risk_params,
validate_irm_params, validate_oracle_config}`), including the derived liquidation-safety bound:

```rust
let bonus_factor = WAD.checked_add(liq_bonus).ok_or(AegisError::ArithmeticOverflow)?;
let threshold_times_bonus = mul_div_floor(liq_threshold, bonus_factor, WAD).map_err(AegisError::from)?;
require!(threshold_times_bonus < WAD, AegisError::LiquidationBonusExceedsThresholdBound);
```
computed through `aegis-math`'s `mul_div_floor` (256-bit intermediate), never a naive multiply.
`liq_bonus` is already bounded to `<= MAX_LIQ_BONUS` (0.25 WAD) before this addition, so
`WAD.checked_add(liq_bonus)` cannot overflow — the bound check ordering itself makes the overflow
path unreachable, rather than merely trapping it.

Evidence of the derived bound firing on an *otherwise-plausible* parameter set (`A-ADM-04`'s
specific requirement): `liq_bonus = 0.24 WAD` (within the flat `MAX_LIQ_BONUS` on its own) combined
with `liq_threshold = 0.85 WAD` gives `0.85 × 1.24 = 1.054 > 1`, rejected with
`LiquidationBonusExceedsThresholdBound` — both as a Tier 1 `aegis-math`-adjacent unit test
(`derived_liquidation_bound_rejects_plausible_but_unsafe_params`, in `state/market.rs`) and as a
full on-chain `create_market` call (`out_of_bounds_market_parameters_are_rejected`). The reference
parameter set from `economic-model.md` §5.1 (`max_ltv=0.75, LT=0.80, b=0.05, ...`) is itself
accepted both at the unit level and on-chain (`reference_parameter_set_is_accepted_on_chain`),
proving the sweep is testing real bounds rather than an over-tight validator that rejects
everything.

### 6. Tests

```
$ cargo test --workspace
running 20 tests
test state::market::tests::close_factor_below_minimum_is_rejected ... ok
test state::market::tests::derived_liquidation_bound_rejects_plausible_but_unsafe_params ... ok
test state::market::tests::fee_above_max_is_rejected ... ok
test state::market::tests::irm_rate_exceeding_max_is_rejected ... ok
test state::market::tests::irm_u_kink_out_of_range_is_rejected ... ok
test state::market::tests::irm_params_reference_set_is_valid ... ok
test state::market::tests::len_matches_account_model_spec ... ok
test state::market::tests::full_liq_hf_zero_is_rejected ... ok
test state::market::tests::liq_bonus_above_max_is_rejected ... ok
test state::market::tests::liq_protocol_fee_above_max_is_rejected ... ok
test state::market::tests::max_ltv_must_be_below_liq_threshold ... ok
test state::market::tests::oracle_config_conf_bps_out_of_range_is_rejected ... ok
test state::market::tests::oracle_config_price_age_out_of_range_is_rejected ... ok
test state::market::tests::oracle_config_reference_is_valid ... ok
test state::market::tests::reference_parameter_set_is_valid ... ok
test state::market::tests::zero_min_debt_is_rejected ... ok
test state::position::tests::len_matches_account_model_spec ... ok
test state::protocol::tests::len_matches_account_model_spec ... ok
test token::policy::tests::transfer_fee_mint_requires_transfer_fee_amount_and_immutable_owner ... ok
test token::policy::tests::vault_extensions_always_include_immutable_owner ... ok
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (target/debug/deps/aegis_math-...)
running 5 tests (fixed::tests::{division_by_zero, ceil_only_rounds_up_on_a_nonzero_remainder,
known_vectors, large_multiplication_survives_256_bit_intermediate, result_overflow})
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/property.rs
running 3 tests (never_panics, floor_le_ceil_le_floor_plus_one, matches_bignum_reference)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running unittests src/lib.rs (target/debug/deps/aegis_test_kit-...)
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/phase2_adversarial.rs
running 8 tests
test reinitializing_protocol_fails ... ok
test attacker_owned_fake_protocol_account_is_rejected ... ok
test non_admin_cannot_create_market ... ok
test reinitializing_market_fails ... ok
test reference_parameter_set_is_accepted_on_chain ... ok
test reinitializing_position_fails ... ok
test non_canonical_bump_is_rejected ... ok
test out_of_bounds_market_parameters_are_rejected ... ok
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s

     Running tests/phase2_state.rs
running 5 tests
test seed_prefixes_are_pairwise_distinct ... ok
test protocol_initializes_with_expected_admin_and_layout ... ok
test create_market_does_not_write_protocol ... ok
test create_market_spl_and_position_lifecycle ... ok
test two_markets_same_asset_pair_different_config_id_coexist ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s

     Running tests/phase2_token_policy.rs
running 9 tests
test transfer_hook_mint_rejected_as_collateral ... ok
test tier_a_extensions_are_accepted_and_recorded ... ok
test mint_close_authority_mint_rejected ... ok
test default_account_state_frozen_mint_rejected ... ok
test permanent_delegate_mint_rejected ... ok
test unrecognized_extension_mint_rejected ... ok
test freeze_authority_requires_acknowledgement ... ok
test transfer_fee_mint_accepted_as_collateral_rejected_as_loan_asset ... ok
test wrong_token_program_for_mint_is_rejected ... ok
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s

     Running tests/smoke.rs
running 1 test
test ping_deploys_and_invokes_offline ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

   Doc-tests aegis / aegis_math / aegis_test_kit — 0 tests each, ok
```
**51 tests, 0 failures.** (The full, un-elided per-dependency compiler output — several hundred
lines of crate names on a from-scratch build — was inspected directly during implementation; it is
not reproduced here for length, matching Phase 1's convention above.)

Required test IDs, all passing, and where each lives:

| ID | Test | File |
|---|---|---|
| `U-ACCT-01` | `_reserved` zero after creation | `tests/phase2_state.rs` (multiple assertions) |
| `U-ACCT-02` | Account size exactly `LEN`, no realloc | `tests/phase2_state.rs` (multiple assertions) |
| `U-LIFE-02` | Seed prefixes pairwise distinct | `tests/phase2_state.rs::seed_prefixes_are_pairwise_distinct` |
| `A-AUTH-01` | Non-admin `create_market` fails | `tests/phase2_adversarial.rs::non_admin_cannot_create_market` |
| `A-AUTH-06` | Attacker-owned fake `Protocol` rejected | `tests/phase2_adversarial.rs::attacker_owned_fake_protocol_account_is_rejected` |
| `A-LIFE-01` | Reinit fails (Protocol, Market, Position) | `tests/phase2_adversarial.rs::reinitializing_{protocol,market,position}_fails` |
| `A-LIFE-03` | Non-canonical bump fails | `tests/phase2_adversarial.rs::non_canonical_bump_is_rejected` |
| `A-ADM-04` | Out-of-bounds parameter sweep incl. derived bound | `tests/phase2_adversarial.rs::out_of_bounds_market_parameters_are_rejected` |
| `A-CUS-03` | Vault authority is the `Market` PDA | `tests/phase2_state.rs::create_market_spl_and_position_lifecycle` |
| `A-TOK-01` | `TransferHook` rejected | `tests/phase2_token_policy.rs::transfer_hook_mint_rejected_as_collateral` |
| `A-TOK-02` | `PermanentDelegate` rejected | `tests/phase2_token_policy.rs::permanent_delegate_mint_rejected` |
| `A-TOK-03` | `MintCloseAuthority` rejected | `tests/phase2_token_policy.rs::mint_close_authority_mint_rejected` |
| `A-TOK-04` | `DefaultAccountState = Frozen` rejected | `tests/phase2_token_policy.rs::default_account_state_frozen_mint_rejected` |
| `A-TOK-05` | Unrecognized extension rejected | `tests/phase2_token_policy.rs::unrecognized_extension_mint_rejected` |
| `A-TOK-07` | Freeze authority ack required, flag recorded | `tests/phase2_token_policy.rs::freeze_authority_requires_acknowledgement` |
| `I-DEPLOY-01` | Post-deploy admin assertion | `tests/phase2_state.rs::protocol_initializes_with_expected_admin_and_layout` |

Not required this phase but exercised anyway because the fixtures were already in hand:
`A-TOK-08`-equivalent (`wrong_token_program_for_mint_is_rejected`), the transfer-fee
collateral-accepted/loan-rejected asymmetry (`transfer_fee_mint_accepted_as_collateral_rejected_as_loan_asset`,
a `token-compatibility.md` §4 acceptance case), and a Tier-A-extension positive-path sanity test
(`tier_a_extensions_are_accepted_and_recorded`).

### 7. Adversarial evidence

Every adversarial test asserts a **specific** `AegisError` (via `assert_aegis_error`, which
decodes `u32::from(AegisError::X)` and compares against the transaction's actual custom error
code) or, where the rejection is Anchor's own framework check rather than Aegis logic (the fake
Protocol account), the specific Anchor `ErrorCode` — never merely "the transaction failed". Attacks
attempted, and their observed rejection:

| Attack | Result |
|---|---|
| Non-admin calls `create_market` | `NotProtocolAdmin` |
| Attacker-owned account at the canonical `Protocol` PDA (owner = System Program) | Anchor `AccountOwnedByWrongProgram` (3007) — caught before any Aegis logic runs |
| Reinitialize `Protocol` / `Market` / `Position` | Anchor `init` rejection (account already in use) in all three cases |
| Non-canonical (but off-curve-valid) bump for `Position` | Anchor `ConstraintSeeds` rejection |
| `max_ltv >= liq_threshold` | `InvalidMaxLtvOrThreshold` |
| `liq_bonus` above the flat 0.25 WAD ceiling | `InvalidLiqBonus` |
| `liq_bonus=0.24, liq_threshold=0.85` (derived bound, INV-LIQ-06) | `LiquidationBonusExceedsThresholdBound` |
| `close_factor` below 0.05 WAD | `InvalidCloseFactor` |
| `full_liq_hf = 0` | `InvalidFullLiqHf` |
| `liq_protocol_fee` above 0.5 WAD | `InvalidLiqProtocolFee` |
| `fee` above 0.25 WAD | `InvalidFee` |
| `min_debt = 0` | `InvalidMinDebt` |
| `u_kink` outside `(0, WAD)` | `InvalidIrmParams` |
| A rate exceeding `max_rate_ps` | `InvalidIrmParams` |
| `max_price_age_secs = 0` | `InvalidMaxPriceAge` |
| `max_conf_bps = 3000` (> 2000) | `InvalidMaxConfBps` |
| `collateral_mint == loan_mint` | `SameCollateralAndLoanMint` |
| `TransferHook` collateral mint | `UnsupportedTokenExtension` |
| `PermanentDelegate` collateral mint | `UnsupportedTokenExtension` |
| `MintCloseAuthority` collateral mint | `UnsupportedTokenExtension` |
| `DefaultAccountState = Frozen` collateral mint | `UnsupportedTokenExtension` |
| Mint with an unrecognized TLV extension code | `InvalidMintAccountData` |
| Transfer-fee mint as the loan asset | `TransferFeeNotAllowedForLoanAsset` |
| Freeze-authority mint, unacknowledged | `FreezeAuthorityNotAcknowledged` |
| Legacy SPL mint claimed under the Token-2022 program | `TokenProgramMintMismatch` |

### 8. Demo

```
$ make demo
anchor build
cargo run -p aegis-test-kit --example phase2_demo
Aegis Protocol — Phase 2 demo (state, PDAs, custody primitives)
Zero-cost, local, offline: in-process LiteSVM, no devnet, no RPC, no API key.

Deployed program 2GtoBADM175vkjf5UYpbD198Ry1cJadXMGo8sCQvXndh into LiteSVM.
Admin/deployer:  GmaDrppBC7P5ARKV8g3djiwP89vz1jLK23V2GBjuAEGB

=== 1. Protocol initialization ===
Protocol account: 3bZsRoC9Uefpd49G2bUBqVDYCTU5ucQRywFcutugH3u8
  admin:         GmaDrppBC7P5ARKV8g3djiwP89vz1jLK23V2GBjuAEGB
  guardian:      9hSR6S7WPtxmTojgo6GG3k4yDPecgJY292j7xrsUGWBu
  fee_recipient: GyGKxMyg1p9SsHfm15MkNUu1u9TN2JtTspcdmrtGUdse
  paused:        0b00

=== 2. Standard SPL market (SOL-like collateral / USDC-like loan) ===
Market account:    FH3ZCzxQmK4LkVoBJi27YBccoSq68FUUDSsYA7GTKsg4
  collateral_mint: 5Z6Ay5NEcbg3xhopc522sBCRXQujkTiuDRnHGfQdcnSf  (decimals 9)
  loan_mint:       7v54NWdBtkjuAFJrLGsS2SXnuk8nKam81mZJeeYxVFi9  (decimals 6)
  config_id:       0
  max_ltv=0.75  liq_threshold=0.80  liq_bonus=0.05  close_factor=0.50
  full_liq_hf=0.95  liq_protocol_fee=0.10  fee=0.10  min_debt=10000000
  total_supply_assets=0 total_borrow_assets=0 (Phase 2: always zero)
  collateral vault: HKyEdmNqhZuWoU5wkcvb5hC6AjkHU2NZ94woJFcfw2cv (canonical: true) authority=FH3ZCzxQmK4LkVoBJi27YBccoSq68FUUDSsYA7GTKsg4 mint=5Z6Ay5NEcbg3xhopc522sBCRXQujkTiuDRnHGfQdcnSf
  loan vault: BvLTnssWjnTZtLbN6gq5wWEfEieUbG1PGjD4x9Mh9gdC (canonical: true) authority=FH3ZCzxQmK4LkVoBJi27YBccoSq68FUUDSsYA7GTKsg4 mint=7v54NWdBtkjuAFJrLGsS2SXnuk8nKam81mZJeeYxVFi9
Fee position:      DNmsGKwhqzLfiPDBLCZEm2SLzBkAVFqGDdeGJSrrqscJ

=== 3. Token-2022 market (transfer-fee collateral / plain SPL loan) ===
Market account:    BJc8KXjjLDzZe61uZQwgYvNejTy36dnBqAL49gUcyKym
  collateral_mint: 3BuW9SR5tG6VFK4MmkQQ3Ak8ny1K1Vv5Uz7is8Aa5pwG  (decimals 9)
  loan_mint:       7v54NWdBtkjuAFJrLGsS2SXnuk8nKam81mZJeeYxVFi9  (decimals 6)
  config_id:       0
  max_ltv=0.75  liq_threshold=0.80  liq_bonus=0.05  close_factor=0.50
  full_liq_hf=0.95  liq_protocol_fee=0.10  fee=0.10  min_debt=10000000
  total_supply_assets=0 total_borrow_assets=0 (Phase 2: always zero)
  collateral_has_transfer_fee flag set: true
  collateral vault: 7K64vgh7NjgUBFrBYTdRyxUrux3HN5xLGH3kwnAXnpHd (canonical: true) authority=BJc8KXjjLDzZe61uZQwgYvNejTy36dnBqAL49gUcyKym mint=3BuW9SR5tG6VFK4MmkQQ3Ak8ny1K1Vv5Uz7is8Aa5pwG
  loan vault: 3q4PNH2hoLKriYoAY9u6vsrndpCCPEKjBvoSc7kwqg2z (canonical: true) authority=BJc8KXjjLDzZe61uZQwgYvNejTy36dnBqAL49gUcyKym mint=7v54NWdBtkjuAFJrLGsS2SXnuk8nKam81mZJeeYxVFi9
Fee position:      222gfyn5HrAhiFtxry22nRca5j6gNGX9YdRaGjbCnEYe
  Token-2022 vault size: 182 bytes (never hardcoded to 165)

=== 4. Position initialization ===
SPL market lender position:      GzD7si8LgCqKdEbSodFSoQC5FCHNTxvMqn4k6AKhuDqv
SPL market borrower position:    9UfCaMnQgxTSHp4qV6ZaR4WDYP8PAoCezVkhjqjakcv
Token-2022 market borrower position: G7LRN7Km8Ggb4uRD9RRJYDHFEu3JeQQ1kfPgGEiSyKcA

=== 5. Rejection table — incompatible mints and parameters ===
Attempt                                       Rejection reason
------------------------------------------------------------------------------------------
TransferHook collateral                       UnsupportedTokenExtension
PermanentDelegate collateral                  UnsupportedTokenExtension
MintCloseAuthority collateral                 UnsupportedTokenExtension
DefaultAccountState=Frozen collateral         UnsupportedTokenExtension
Unrecognized extension collateral             InvalidMintAccountData
Transfer-fee mint as LOAN asset               TransferFeeNotAllowedForLoanAsset
Freeze-authority collateral, unacknowledged   FreezeAuthorityNotAcknowledged
collateral_mint == loan_mint                  SameCollateralAndLoanMint
LT=0.85, bonus=0.24 (derived bound INV-LIQ-06) LiquidationBonusExceedsThresholdBound (INV-LIQ-06)

Demo complete. All Phase 2 acceptance criteria exercised above.
```

### 9. Regression — Phase 1 guarantees re-run

```
$ cargo fmt --all --check
(no output — clean)

$ cargo clippy --workspace --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in ...
(zero warnings)

$ for s in scripts/check-*.sh; do ./"$s"; done
check-no-close: OK — no close constraint targets Market or Protocol
check-no-dup: OK — no 'dup' constraint in programs/
check-no-float: OK — no f32/f64 in programs/ or crates/aegis-math/
check-no-init-if-needed: OK — no init_if_needed constraint or feature in use
check-no-slot-time: OK — no Clock.slot usage in programs/
check-overflow-checks: OK — overflow-checks = true is set in [profile.release]

$ cargo test --test smoke
test ping_deploys_and_invokes_offline ... ok
```
`check-no-close.sh` (CI-NOCLOSE) is new in Phase 2 — it did not exist after Phase 1 (see Invariant
status above). It was proven to actually fire, on a temporary fixture (`close = admin` added to a
scratch Accounts struct referencing `Market`), then reverted with a byte-for-byte diff check
(`diff` against a pre-fixture backup showed no difference) — the same evidence discipline Phase 1
used for its five guards.

`anchor build` (SBF target) also required one fix not present in Phase 1: `CreateMarket`'s
`try_accounts` initially overflowed the SBF stack-frame limit by 192 bytes (`Market` is 640 bytes,
held inline alongside every other account); boxing the `market` field
(`Box<Account<'info, Market>>`) resolved it. This is recorded here as a real, encountered
implementation constraint, not a hypothetical one.

### 10. Deviations

None requiring an ADR. Two implementation-level choices worth recording as design notes (not
frozen-document changes):
- Vaults are created by hand-rolled CPI sequencing rather than Anchor's `#[account(init, token::
  ...)]` sugar, specifically to add `ImmutableOwner` to Token-2022 vaults (§3 above).
- `DefaultAccountState` is rejected unconditionally rather than only when its configured state is
  `Frozen` (§4 above) — the more conservative reading of a table entry that does not specify the
  `Initialized` case.

---

## Phase 2 self-audit

Performed before declaring Phase 2 complete, per the task's final-audit checklist.

| Question | Answer |
|---|---|
| Can a fake Protocol account pass? | No — `attacker_owned_fake_protocol_account_is_rejected` plants a byte-identical, System-Program-owned account at the canonical PDA; Anchor's owner check (`AccountOwnedByWrongProgram`) rejects it before any Aegis logic runs. |
| Can an attacker choose a noncanonical PDA? | No — `non_canonical_bump_is_rejected` submits a real, off-curve-valid PDA for a lower bump than canonical; Anchor's `seeds`/`bump` constraint (which always recomputes the canonical address via `find_program_address`, never accepts a caller-supplied bump) rejects it. |
| Can a bump be manipulated? | No — every PDA field uses bare `bump` (init) or `bump = <stored>` (existing); no instruction accepts a bump as an argument. |
| Can a Market vault point somewhere else? | No — both vaults are PDAs of `(seed, market)`, created once inside `create_market` by the program itself; there is no code path that lets a caller supply an alternative vault address for `init`. |
| Can the wrong token program be substituted? | No — `Interface<'info, TokenInterface>` restricts the account to one of the two known token programs, and the handler additionally pins each mint's actual owner to the *specific* program passed for it (`wrong_token_program_for_mint_is_rejected`). |
| Can an unknown Token-2022 extension slip through? | No — the allowlist is a `match` with an explicit accept-arm list and a `_ => reject` catch-all; `unrecognized_extension_mint_rejected` proves it against a mint carrying a type code this dependency version cannot even decode. |
| Does the extension policy accidentally become a denylist? | No — verified by code inspection: there is no "allow unless in this rejected list" branch anywhere in `token/policy.rs`; the only accept path is the explicit Tier A/B arm list. |
| Can freeze authority be silently accepted? | No — `require!(args.ack_freeze_authority, ...)` fires whenever either mint has one; `freeze_authority_requires_acknowledgement` proves both the rejection and the acceptance-with-recorded-flag paths. |
| Can collateral mint equal loan mint? | No — `require_keys_neq!` is the first check in the handler; proven by the sweep test. |
| Can invalid risk parameters create an unsafe market? | No — every bound in `economic-model.md` §5 is checked, proven individually by the out-of-bounds sweep. |
| Can the liquidation bonus bound overflow or round incorrectly? | No — `liq_bonus` is bounded to `<= 0.25 WAD` *before* `WAD.checked_add(liq_bonus)` runs, so the addition cannot overflow; the multiply-divide goes through `aegis-math`'s 256-bit-intermediate `mul_div_floor`, the same primitive whose overflow behavior Phase 1 exhaustively tested. |
| Can `create_market` omit the fee position? | No — `fee_position` is a non-optional `init`-required account in the `Accounts` struct; the instruction cannot succeed without creating it. |
| Can Market creation be replayed/reinitialized? | No — `reinitializing_market_fails` proves a second `create_market` call with the same `(collateral_mint, loan_mint, config_id)` fails, and the original market's data is untouched. |
| Can two markets unexpectedly share writable custody state? | No — `two_markets_same_asset_pair_different_config_id_coexist` proves distinct PDAs, distinct vaults, and distinct fee positions for two markets differing only in `config_id`. |
| Are reserved bytes deterministic/zero? | Yes — always constructed as `[0u8; N]` explicitly; asserted directly in every lifecycle test. |
| Is any user instruction writing global Protocol state unnecessarily? | No — `create_market_does_not_write_protocol` compares the account's raw bytes before and after a successful `create_market` call and asserts byte-for-byte equality. |
| Did I accidentally implement Phase 3 behavior? | No — grep-verified: no `deposit`/`withdraw`/`borrow`/`repay`/`liquidate`/`supply`/`accrue` function exists anywhere in `programs/aegis/src/`. |
| Did documentation outrun implementation? | No — every claim in this section is backed by a command actually run and output actually observed this session; nothing here describes planned rather than built behavior. |

### Changes forced by this audit

1. **`Market` boxed in `CreateMarket`'s Accounts struct** — found by `anchor build`'s SBF
   stack-frame check, not by review; without it the program does not compile for the on-chain
   target at all (§9 above).
2. **`reinitializing_market_fails` added** — the initial adversarial suite covered `Protocol` and
   `Position` reinitialization but not `Market` itself; added directly from this audit's "Can
   Market creation be replayed?" question.
3. **`create_market_does_not_write_protocol` added** — INV-ACCT-07 had no direct test until this
   audit's "Is any user instruction writing global Protocol state unnecessarily?" question
   prompted one.

---

## Phase 1 — evidence

### 1. Toolchain versions

See **Environment** above for the full raw output and every delta from `docs/ecosystem-research.md`.

### 2. `cargo test --workspace` (offline)

```
$ cargo test --workspace --offline
   Compiling ... (elided — full dependency graph, no network access used)
    Finished `test` profile [unoptimized + debuginfo] target(s)
     Running unittests src/lib.rs (target/debug/deps/aegis-...)
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

     Running unittests src/lib.rs (target/debug/deps/aegis_math-...)
running 5 tests
test fixed::tests::division_by_zero ... ok
test fixed::tests::ceil_only_rounds_up_on_a_nonzero_remainder ... ok
test fixed::tests::large_multiplication_survives_256_bit_intermediate ... ok
test fixed::tests::result_overflow ... ok
test fixed::tests::known_vectors ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

     Running tests/property.rs (target/debug/deps/property-...)
running 3 tests
test never_panics ... ok
test floor_le_ceil_le_floor_plus_one ... ok
test matches_bignum_reference ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

     Running unittests src/lib.rs (target/debug/deps/aegis_test_kit-...)
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

     Running tests/smoke.rs (target/debug/deps/smoke-...)
running 1 test
test ping_deploys_and_invokes_offline ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

   Doc-tests aegis / aegis_math / aegis_test_kit — 0 tests each, ok
```

**`--offline` was passed and the run succeeded** — Cargo would fail immediately if any dependency
resolution needed the network, so this is genuine, checked evidence of NFR-4 for the build+test path,
not an assertion. (The full, un-elided compiler output, several hundred lines of crate names, was
inspected directly during implementation; it is not reproduced here for length.)

Required test IDs, all passing:

| ID | Test | File |
|---|---|---|
| `U-ARITH-01` | Known vectors (`mul_div_floor(3,5,2)==7`, `mul_div_ceil(3,5,2)==8`, more) | `crates/aegis-math/src/fixed.rs::known_vectors` |
| `U-ARITH-02` | `d == 0` → `Err(DivisionByZero)` | `crates/aegis-math/src/fixed.rs::division_by_zero` |
| `U-ARITH-03` | Result overflow → `Err(Overflow)` | `crates/aegis-math/src/fixed.rs::result_overflow` |
| `U-ARITH-04` | `1.8e25 × 1.8e19 / 1e18` succeeds (overflows naive `u128` mul; fits the final `u128` result) | `crates/aegis-math/src/fixed.rs::large_multiplication_survives_256_bit_intermediate` |
| `P-ARITH-1` | `floor ≤ ceil ≤ floor + 1` (proptest, full `u128` domain) | `crates/aegis-math/tests/property.rs::floor_le_ceil_le_floor_plus_one` |
| `P-ARITH-2` | Never panics for any `(a, b, d)`, including `d == 0` (proptest) | `crates/aegis-math/tests/property.rs::never_panics` |
| `P-ARITH-3` | Exact agreement with an independent `num-bigint` reference (proptest) | `crates/aegis-math/tests/property.rs::matches_bignum_reference` |

### 3. `anchor build` (with IDL path)

```
$ anchor build
   Compiling aegis v0.1.0 (/Users/.../aegis-protocol/programs/aegis)
    Finished `release` profile [optimized] target(s) in 6.16s

$ ls -la target/deploy/aegis.so
-rwxr-xr-x  1 vansh  staff  50032  target/deploy/aegis.so

$ cat target/idl/aegis.json
{
  "address": "5emasbxEz9UGdeur6awt71JPE8ptvr716MUoVagaAPa1",
  "metadata": { "name": "aegis", "version": "0.1.0", "spec": "0.1.0",
                "description": "Aegis Protocol on-chain program" },
  "instructions": [
    {
      "name": "ping",
      "docs": ["Does nothing and always succeeds. Proves the program builds, deploys, and is",
               "invocable — the entire Phase 1 acceptance bar for on-chain code."],
      "discriminator": [173, 0, 94, 236, 73, 133, 225, 153],
      "accounts": [],
      "args": []
    }
  ]
}
```

IDL generation confirms `idl-build = ["anchor-lang/idl-build"]` is wired correctly.

### 4. Guard scripts — pass on the repository, and proof each fails on a violation

All five ran clean on the real repository:

```
$ for s in scripts/check-*.sh; do ./"$s"; done
check-no-dup: OK — no 'dup' constraint in programs/
check-no-float: OK — no f32/f64 in programs/ or crates/aegis-math/
check-no-init-if-needed: OK — no init_if_needed constraint or feature in use
check-no-slot-time: OK — no Clock.slot usage in programs/
check-overflow-checks: OK — overflow-checks = true is set in [profile.release]
```

Each was then proven to actually fire, on a temporary fixture, then reverted (no fixture is present in
the final tree — verified via `git status` immediately after):

| Guard | Fixture | Result |
|---|---|---|
| `check-no-float.sh` | Appended `pub const BAD: f64 = 1.0;` to `constants.rs` | `exit 1`, reported the exact line |
| `check-no-init-if-needed.sh` | Appended a comment containing `init_if_needed` to `lib.rs` | `exit 1`, reported the exact line |
| `check-no-dup.sh` | Added `#[account(mut, dup)]` to a scratch struct in `lib.rs` | `exit 1`, reported the exact line |
| `check-no-slot-time.sh` | Added a function reading `clock.slot` to `lib.rs` | `exit 1`, reported the exact line |
| `check-overflow-checks.sh` | Changed `overflow-checks = true` → `false` in the workspace `Cargo.toml` | `exit 1`, named the missing setting |

All five fixtures were reverted with a byte-for-byte restore from a backup copy taken before mutation;
`git status --short` immediately after showed no unexpected diff.

### 5. Smoke test — offline LiteSVM deploy + invoke

```
$ cargo test --test smoke
running 1 test
test ping_deploys_and_invokes_offline ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

`tests/smoke.rs` loads `target/deploy/aegis.so` via `include_bytes!` (a build artifact on local disk),
deploys it into a fresh in-process `litesvm::LiteSVM`, airdrops a deterministic (fixed-seed, not
`Keypair::new()`) payer, builds and signs a `ping` transaction, and asserts `send_transaction` returns
`Ok`. No RPC client, no validator process, no devnet address anywhere in the test or in
`aegis-test-kit`. Re-run with `cargo test --workspace --offline` (see §2) to confirm Cargo itself
needs no network for the full pipeline.

### 6. `fmt` / `clippy`

```
$ cargo fmt --all --check
(no output — clean)

$ cargo clippy --workspace --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s)
(zero warnings)
```

### 7. CI

`.github/workflows/ci.yml` defines four jobs — `fmt`, `clippy`, `guards`, `build-and-test` — covering
every check in `docs/phases/phase-01-foundation.md` §7 and `docs/testing-strategy.md` §9, with no
secrets configured anywhere in the workflow. **The workflow has not been observed running on GitHub
Actions from this environment** (no way to trigger/poll Actions from here); every check it runs was
verified locally instead, with the commands and output reproduced above. This is stated plainly per
`AGENTS.md` §14 rather than assumed green.

---

## Phase 1 self-audit

Performed before declaring Phase 1 complete, per the phase specification §14 / the task's final-audit
checklist.

| Question | Answer |
|---|---|
| Did I accidentally implement Phase 2? | No. `programs/aegis` has exactly one instruction (`ping`) and zero account structs. Grep-verified: no `Protocol`, `Market`, or `Position` identifiers anywhere in `programs/` or `crates/`. |
| Did I introduce protocol/account state? | No. `Ping` is an empty `#[derive(Accounts)]` struct with no fields. |
| Did I use naive `u128` multiplication? | No. `mul_div_floor`/`mul_div_ceil` route through a hand-rolled `U256` two-limb type; `U-ARITH-04` specifically exercises the case that would overflow a naive `u128` multiply. |
| Can any arithmetic panic? | Every division-by-zero and overflow path returns a typed `MathError`; the u256 division routine uses `wrapping_sub`/`overflowing_add` explicitly rather than checked ops that would panic, with the wrapping case proven correct by construction (see the doc comment in `u256.rs`) and cross-checked by the `never_panics` property test over the full `u128` domain. |
| Does `aegis-math` remain `no_std` and Solana-independent? | Yes — `#![cfg_attr(not(test), no_std)]`, and `cargo tree -p aegis-math` shows zero `solana-*`/`anchor-*` dependencies (only `proptest`/`num-bigint`/`num-traits` as dev-dependencies, which never compile into the `no_std` lib target). |
| Are release overflow checks actually enabled? | Yes — `[profile.release] overflow-checks = true` in the workspace `Cargo.toml`, and `scripts/check-overflow-checks.sh` asserts it in CI, proven to fail when the setting is flipped. |
| Could any guard silently never fire? | No — every guard was proven to fail on a real, deliberately-violating fixture (§4 above), not just to pass on the clean tree. |
| Did the smoke test really execute the program? | Yes — it deploys the actual `target/deploy/aegis.so` built by `anchor build` moments earlier into LiteSVM and asserts a real `send_transaction` result, not a mocked call. |
| Does the offline path really avoid external RPC/provider dependency? | Yes — `aegis-test-kit` never constructs an RPC client; LiteSVM is in-process; `cargo test --workspace --offline` (§2) passing is checked, not assumed, evidence. |
| Did I invent any test/build evidence? | No. Every command in this document was actually executed on the implementation machine; several (the `U-ARITH-04` expected value, the `solana-transaction` feature name) were wrong on the first attempt and are shown corrected, not silently fixed and re-presented as first-try. |
| Are docs overstating implementation? | README now states plainly that Phase 1 is toolchain/foundation only, with the specific list of what does not exist yet. |
| Did current dependency changes invalidate any ADR? | No. All deltas (§ Environment above) are toolchain/version facts, not architectural ones. |
| Is there any secret or local key material in Git? | No. `target/deploy/aegis-keypair.json` and `~/.config/solana/id.json` are both outside version control (`target/` is gitignored; the wallet is in the user's home directory, never the repo) — confirmed via `git status` showing no such file staged or tracked. |

---

## Phase 0 self-audit

*(Preserved from Phase 0 for history; unchanged.)*

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

## Phase 9 — evidence

### 0. Phase gate (verified before any code was written)

```
$ git log -1 --format="%H %s" phase-08-composability
16cde79... docs(phase-8): record Phase 8 completion evidence, RV-6/RV-8 resolution, ADR-0013, and README
$ git log -1 --format="%H %s" HEAD
16cde79... docs(phase-8): record Phase 8 completion evidence, RV-6/RV-8 resolution, ADR-0013, and README
```

Tag == HEAD == `origin/main`. Working tree clean. `target/idl/aegis.json` and
`target/deploy/aegis.so` already existed (built earlier the same session) and were used as-is —
`anchor build` was not re-run, since no on-chain source changed at any point in this phase.

### 1. RV-7 — closed before any SDK code was written

Full write-up: `docs/ecosystem-research.md` §17. Summary: SIMD-0296 (the 4096-byte "v1" transaction
format, SIMD-0385) is active on testnet/devnet and has been available for local testing on Surfpool
≥1.5.0 — this workspace's exact pinned version — since 2026-08-24; mainnet activation is scheduled
for epoch 1035 (≈2026-09-15), three days after this research date. `@solana/kit` 8.0.0+ (this
workspace has 8.3.0) supports building/signing/sending v1 transactions. **None of this is used**:
Aegis designs and tests to the classic 1232-byte legacy limit unconditionally (ADR-0011,
`I-TX-01`), so the finding is recorded as closed research, not as a design input.

### 2. SDK package and dependency proof

```
$ cd sdk/ts && npm ls @anchor-lang/core @solana/kit
@aegis/sdk@0.1.0
├── @anchor-lang/core@1.2.0
└── @solana/kit@8.3.0

$ grep -rln "@coral-xyz/anchor" src/ test/
(no output)
$ grep -rln "@solana/web3\.js" src/ test/ | grep -v -- '-- comment'
src/anchorCoder.ts   # doc comment only, explaining the coder's OWN transitive dependency
src/index.ts         # doc comment only, stating the prohibition
$ npm ls @solana/web3.js
@aegis/sdk@0.1.0
└─┬ @anchor-lang/core@1.2.0
  ├── @solana/web3.js@1.99.0 deduped
  └─┬ @anchor-lang/borsh@1.2.0
    └── @solana/web3.js@1.99.0
```

The only `@solana/web3.js` in the dependency tree is transitive, *inside* `@anchor-lang/core`
itself — identical to the situation already documented and accepted in Phase 8
(`bots/liquidator/` shows the exact same `npm ls` shape). Neither `@coral-xyz/anchor` nor
`@solana/web3.js` is imported by any line of this SDK's or the app's own source.

### 3. IDL codegen and stale-detection proof

```
$ cd sdk/ts && npm run codegen
Codegen complete. Wrote .../sdk/ts/src/generated
  15 instructions, 3 accounts, 14 events, 59 errors, 19 types.

$ npm run codegen:check
check-codegen: OK -- sdk/ts/src/generated/ matches the current IDL exactly

# Proof the check actually fires (not merely present):
$ echo "// tamper" >> src/generated/errors.ts && npm run codegen:check; echo "exit=$?"
check-codegen: STALE -- errors.ts differs from current `target/idl/aegis.json`
exit=1
$ git checkout -- src/generated/errors.ts && npm run codegen:check
check-codegen: OK -- sdk/ts/src/generated/ matches the current IDL exactly
```

Every account/instruction/event discriminator and every per-instruction account
name/writable/signer/optional flag in `sdk/ts/src/generated/instructions.ts` is read directly out
of `target/idl/aegis.json` (`sdk/ts/scripts/codegen.mjs`) — none is hand-typed.

### 4. I-SDK-03 — PDA parity

```
$ cargo run -p aegis-test-kit --example phase9_vectors_dump -- tests/vectors
wrote tests/vectors/pdas.json  (+ fixed/shares/irm/health/liquidation.json)

$ cd sdk/ts && npx vitest run test/pda.test.ts
 ✓ test/pda.test.ts (12 tests)
   ✓ protocol PDA matches exactly
   ✓ market/vault PDAs match exactly: market_a_b_config_0 / _1 / _256 / _max / market_b_a_config_0 / market_zero_ff_config_42
   ✓ position PDA matches exactly: position_owner_a / _owner_b / _fee_recipient / _all_zero_owner
   ✓ config_id=1 and config_id=256 derive to DIFFERENT addresses (endian sanity check)
```

`tests/vectors/pdas.json` is generated by calling `crates/aegis-test-kit/src/market.rs`'s own
`protocol_pda`/`market_pda`/`position_pda`/`collateral_vault_pda`/`loan_vault_pda` — the identical
functions every other Rust test in this repository already uses — never a re-derivation.
`config_id` 1 and 256 are the exact mirror-image byte patterns (`[0x01,0x00]` vs. `[0x00,0x01]` in
`u16` little-endian) chosen so a big-endian mistake in `pda.ts`'s seed encoding would produce a
*different* address, not merely an incorrect one; the dedicated test asserts they differ.

### 5. I-SDK-01 — cross-language math vectors

```
$ make vectors
wrote tests/vectors/{fixed,shares,irm,health,liquidation,pdas}.json

$ cd sdk/ts && npx vitest run test/vectors.test.ts
 ✓ test/vectors.test.ts (48 tests)

# Staleness proof:
$ python3 -c "..." # perturb tests/vectors/fixed.json's first expected value
$ ./scripts/check-vectors.sh; echo exit=$?
check-vectors: STALE -- ...
exit=1
$ git checkout -- tests/vectors/fixed.json && ./scripts/check-vectors.sh
check-vectors: OK — tests/vectors/*.json matches current aegis-math output exactly
```

`crates/aegis-test-kit/examples/phase9_vectors_dump.rs` calls `aegis-math`'s real, frozen functions
(`mul_div_floor/ceil`, `to_shares_*`/`to_assets_*`, `utilization`, `borrow_rate`, `taylor3`/
`taylor_x`, `scale_to_wad_*`, `conservative_price_band`, `collateral_value`, `debt_value`,
`health_factor`, `is_within_max_ltv`, `max_repay`, `is_liquidatable`,
`compute_liquidation_by_repay`/`_by_seize`) directly — no reimplementation, no changes to
`aegis-math` itself. `sdk/ts/src/math.ts`'s 48 assertions all pass against these vectors with zero
hand-typed expected values, including the exact `economic-model.md` §7.5 worked liquidation
(repay 900 USDC, seize 9.97035 SOL, protocol cut 0.04748 SOL, ~4.5% net liquidator profit) and the
collateral-clamp path (§7.2).

### 6. I-TX-01 / INV-RES-06 — transaction size

```
$ cd sdk/ts && npx vitest run test/tx-size.test.ts
 ✓ test/tx-size.test.ts (12 tests)
```

| Instruction | Serialized bytes | <=1232? |
|---|---|---|
| `init_position` | 320 | yes |
| `deposit_collateral` | 426 | yes |
| `withdraw_collateral` | 492 | yes |
| `supply` | 475 | yes |
| `withdraw` | 475 | yes |
| `borrow` | 541 | yes |
| `repay` | 475 | yes |
| `accrue_interest` | 285 | yes |
| `close_position` | 286 | yes |
| `liquidate` (no callback) | 639 | yes |
| `liquidate` (callback, realistic account surface) | **801** | yes |

**No architectural blocker.** `liquidate` with a callback — the instruction the phase spec
specifically flagged as the one to watch — has 431 bytes of headroom under the classic 1232-byte
limit. Each transaction includes a real `ComputeBudget` instruction, a real Ed25519 signature, and
a real blockhash-shaped lifetime; sizes are measured via `@solana/transactions`'
`getTransactionSize`, never estimated from account count.

### 7. I-SDK-02 — build/sign/send/confirm/decode against local Surfpool

```
$ make build   # target/deploy/aegis.so, target/idl/aegis.json (unchanged from Phase 8)
$ cd sdk/ts && npm run test:e2e
 ✓ test/e2e.test.ts (1 test) 42993ms
   ✓ runs the full lifecycle: init -> market -> position -> deposit -> borrow -> repay -> withdraw -> liquidate  13008ms
```

The single end-to-end test exercises, against a real local **offline** Surfpool validator it starts
and deploys the real `aegis.so` to itself: `initialize_protocol`, `create_market`, `init_position`
bundled with `supply` (lender) and with `deposit_collateral` (borrower) via
`ix.ts::withInitPositionIfNeeded`, `borrow`, `accrue_interest`, `repay` (full, via shares),
`withdraw_collateral`, `close_position`, a lender `withdraw`, and a scripted-price-drop `liquidate`
— asserting the decoded account state and the decoded event after every step (never merely "did
not throw"). Deterministic Pyth price fixtures come from the real
`aegis_test_kit::PriceFixture`-generated bytes (`phase8_price_fixture_dump`), reused rather than
reimplemented, injected via `surfnet_setAccount`.

### 8. Browser demo — I-UI-01

Run against a real local Surfpool validator (started and deployed to by hand, mirroring
`scripts/run-app.sh`/`make app`) and a real Next.js dev server, driven with the Claude Browser tool
(not merely asserted from source review):

1. **Market list (`/`)** — loads, shows "No markets found yet" with a link to `/demo` when empty
   (a real `getProgramAccounts` round trip to local Surfpool, confirmed via network inspection
   returning `{"result":[]}`), and the connected local wallet's address/balance/airdrop control in
   the header.
2. **`/demo`, step 1** (seed market + position) — real transactions: airdrop, two mint creations,
   `initialize_protocol`, `create_market`, lender `init_position`+`supply`, borrower
   `init_position`+`deposit_collateral`, `borrow`. Log: *"Borrower position opened: 10.000000000
   collateral units, 900.000000 debt units. HF healthy."*
3. **`/demo`, step 2** (warp time + accrue) — `surfnet_timeTravel` advances the validator's clock
   30 days with **no real wall-clock wait**; `accrue_interest` then visibly changes
   `total_borrow_assets` from `900000000` to `900003328` on screen.
4. **`/demo`, step 3** (crash price + liquidate) — injects a crashed Pyth fixture ($150 → $95/SOL),
   then liquidates. Final state: `borrow_shares == 0`, `total_borrow_assets == 0`, a small
   collateral remainder returned to the position — exactly the expected full-liquidation outcome.
5. **`/market/[address]`** — risk parameters (75%/80% LTV/LT, 3600s staleness bound, 1.00% max
   confidence, a green "no freeze authority" pill), market stats (supplied/borrowed/available
   liquidity, utilization/APYs), the connected wallet's own position (health factor rendered as
   `∞ (no debt)`, correctly), and the six action tabs with an amount input.
6. **A real action failure, shown honestly** — submitting a `supply` top-up with an empty token
   balance surfaced `"Transaction simulation failed"` and `status: failed` in the UI, not a false
   success — proving the confirmation-lifecycle and error-surfacing requirements (items 42-43)
   under a genuine on-chain rejection, not merely a client-side validation message.

**Two real bugs found and fixed during this exercise** (recorded as findings, not smoothed over):

- The demo's `initialize_protocol` call originally set `feeRecipient` to the same address as the
  connected wallet, which also acts as lender. Because a market's `fee_position` is
  `PDA(market, market.fee_recipient)` (`account-model.md` §9), this made `supply`'s `position` and
  `fee_position` accounts resolve to the identical address — rejected outright by Anchor 1.0's
  default duplicate-mutable-account protection (`ConstraintDuplicateMutableAccount`, exactly the
  class of bug AGENTS.md §1 describes Anchor 1.0 as closing "by default"). Fixed by generating a
  fee-recipient address distinct from every signing identity in the demo.
- After warping the validator's clock forward with `surfnet_timeTravel`, injecting a price fixture
  timestamped with the *browser's* real wall-clock time made the price appear ~30 days stale to the
  on-chain O-5 staleness check (`OraclePriceStale`), since time-travel advances only the
  validator's `Clock` sysvar, not the host machine's clock. Fixed by adding
  `app/src/lib/surfnet.ts::getOnChainUnixTimestamp`, which reads the `Clock` sysvar's own
  `unix_timestamp` directly, and using it for every fixture injected after a warp.

### 9. Regression — full offline suite

```
$ cargo fmt --all --check
(exit 0, no output)

$ cargo clippy --workspace --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s)
(zero warnings)

$ cargo test --workspace --offline
32 test-result blocks, 0 failed  (identical to Phase 8's count -- programs/aegis and crates/
aegis-math are byte-for-byte unchanged this phase)

$ for s in scripts/check-*.sh; do ./"$s"; done
check-collateral-transfer-paths: OK
check-cpi-allowlist: OK
check-no-close: OK
check-no-dup: OK
check-no-float: OK
check-no-init-if-needed: OK
check-no-slot-time: OK
check-overflow-checks: OK
check-vectors: OK          <-- NEW (Phase 9)

$ cd sdk/ts && npx tsc -p tsconfig.json --noEmit
(exit 0, no output)
$ npm run codegen:check
check-codegen: OK
$ npm run vectors:check
check-vectors: OK
$ npm test
 ✓ test/vectors.test.ts (48 tests)
 ✓ test/pda.test.ts (12 tests)
 ✓ test/tx-size.test.ts (12 tests)
 Test Files  3 passed (3) — Tests  72 passed (72)
$ npm run test:e2e
 ✓ test/e2e.test.ts (1 test) — Tests 1 passed (1)

$ cd app && npx tsc --noEmit -p tsconfig.json
(exit 0, no output)
$ npx next build --webpack
✓ Compiled successfully
✓ Generating static pages (4/4)
```

Every existing Phase 1-8 Rust test passes **unchanged** — the strongest available evidence that
this phase's client-side-only scope was actually respected: `programs/aegis` and `crates/aegis-math`
were not touched.

### 10. INV-RES-06

`docs/invariants.md` §L: *"Every instruction's account count fits a legacy (1232-byte) transaction
without an address-lookup table. Impl: SDK test. Test: I-TX-01. Phase: 9."* Closed exactly as
specified — see §6 above. No ALT is used or required anywhere in `sdk/ts/` or `app/`.

### 11. Security self-audit (item 50's checklist)

| Question | Answer |
|---|---|
| Any private key handling inside SDK? | No. `ix.ts`/`tx.ts` only ever accept a caller-supplied `TransactionSigner`/`KeyPairSigner`; nothing is generated, stored, or persisted by the SDK itself. |
| Hardcoded RPC? | No. `config.ts`'s only default is `http://127.0.0.1:8899` / `ws://127.0.0.1:8900`, overridable via env vars. |
| Hardcoded Hermes? | No. `oracle.ts`'s `HermesOracleClient` takes the endpoint as a required constructor argument with no default. |
| API key? | None anywhere in `sdk/ts/` or `app/`. |
| Legacy `web3.js` direct import? | None in this SDK's/app's own source (grep-verified); the sole occurrence is `@anchor-lang/core`'s own unavoidable transitive dependency (§2). |
| Old Anchor package (`@coral-xyz/anchor`)? | None (grep-verified). |
| `Number` used for critical amount/math? | No. Every occurrence of `Number(...)` in `sdk/ts/src/` and `app/src/` is a UI-only display conversion (APY%, utilization%, LTV%, HF display), never fed back into a builder — grep-verified and listed explicitly in the SDK README. |
| PDA seed/endian mismatch? | Verified absent by `I-SDK-03`, including two `config_id` values specifically chosen to catch exactly this class of bug. |
| Manually duplicated discriminators? | None — all derived from the IDL by `codegen.mjs` (§3). |
| Stale generated IDL? | `check-codegen.mjs` proven to detect it (§3). |
| Stale vectors? | `check-vectors.sh` proven to detect it (§5). |
| Client rounding mismatch? | None found — 48/48 vector assertions pass bit-for-bit. |
| Unsafe decimal parsing? | `app/src/lib/format.ts::parseDecimalToBaseUnits` does no floating-point multiplication and rejects excess fractional precision explicitly. |
| Transaction >1232 bytes? | No instruction exceeds 801 bytes (§6). |
| Hidden ALT use? | None — grep-verified across `sdk/ts/src` and `app/src`. |
| Builder missing an account-constraint assumption? | Account order/flags come from the IDL itself (`ixEngine.ts`), not a hand-maintained list; `I-SDK-02` exercises every builder against the real on-chain constraints and passes. |
| SDK permitting unsupported Token-2022 combinations? | The SDK adds no independent token-extension policy of its own — it relies on (and never bypasses) the on-chain `create_market` policy check; `credited` amounts surfaced by `events.ts` are the real on-chain measured-delta values, never recomputed client-side. |
| App hiding freeze-authority flag? | No — `/market/[address]` shows an explicit labeled pill either way. |
| App hiding oracle risk parameters? | No — max price age and max confidence are shown explicitly in the Risk parameters card. |
| UI treating previews as authoritative? | No — every preview (HF, LTV) is computed client-side for display only; the actual instruction always goes on-chain, which is authoritative (demonstrated live by the honest `supply` failure in §8). |
| Success shown before confirmation? | No — `tx.ts::buildSignSendAndConfirm` only resolves after `sendAndConfirmTransactionFactory` resolves at `commitment: 'confirmed'`; the UI's `status` only reaches `'confirmed'` at that point. |
| App requiring external network? | No — local Surfpool only; oracle fixtures are injected locally, never fetched from Hermes. |
| Callback liquidation incorrectly forwarding signers from frontend? | No — `ixEngine.ts`'s `remainingAccounts` parameter only ever sets `is_signer` from an explicit, caller-provided flag (defaulting to `false`); neither the market's nor the liquidator's signer status is ever attached to a remaining account. |
| Secret wallet committed? | No — every keypair used in tests/demos is either ephemeral (`generateKeyPairSigner`, never written to disk) or a `mktemp`/`os.tmpdir()` file outside the repository; `git status` confirms no keypair-shaped file is staged. |

No findings required a code change beyond what is already reflected above (the two bugs in §8 were
found and fixed as part of this same phase, not left as findings).

### 12. Deviations

- **No browser wallet-extension adapter.** `app/src/lib/localSigner.ts` uses a `@solana/kit`-native
  ephemeral local signer instead of Phantom/Backpack/etc. Reasoned in the SDK/app READMEs: `make
  app`'s clean-clone criterion must not depend on a specific browser extension, and current
  wallet-adapter packages generally carry the same transitive `@solana/web3.js` dependency already
  discussed. This is a scope decision within the phase's own stated flexibility (item 27: "using
  current Solana Kit-compatible frontend patterns"), not a silent omission.
- **Demo price-account addressing is a local-only convenience.** Aegis has no on-chain registry
  mapping a market's `feed_id` to a live price-account address (by design — that resolution is
  inherently off-chain, normally Hermes's job). `app/src/lib/demoRegistry.ts` is a `localStorage`
  registry the `/demo` seed flow populates for this purpose; it is explicitly local-demo-only and
  is not part of `@aegis/sdk` itself.
- No other deviation from the phase specification.

---

## Phase 10 — evidence

**Phase 10 is complete.** Scope per `docs/phases/phase-10-security.md`: complete adversarial
coverage for T-01..T-32, a stateful LiteSVM invariant fuzzer, mutation validation of all nine
`[GLOBAL]` invariants, a value-creation search targeting T-17, invariant/test traceability
enforcement, and transparent security documentation.

### 0. Phase gate (verified before any code was written)

```
$ git log -1 --format="%H %s" phase-09-sdk-ui
8e9cf409fce93fc8560e3a53b2aa432e8ef159f8 docs(phase-9): record Phase 9 completion evidence, RV-7 resolution, and README updates
$ git log -1 --format="%H %s" HEAD
8e9cf409fce93fc8560e3a53b2aa432e8ef159f8 docs(phase-9): record Phase 9 completion evidence, RV-7 resolution, and README updates
```
Tag == HEAD == origin/main. Working tree clean. Baseline `cargo test --workspace --offline`: 236
passed, 0 failed, across 32 test-result blocks. `cargo fmt --all --check` and `cargo clippy
--workspace --all-targets -- -D warnings` both clean. No `tests/adversarial/`, no `tests/fuzz/`;
`docs/security/` held only its README — Phase 10 genuinely not started.

### 1. Threat traceability

Every threat T-01..T-32 in `docs/threat-model.md` §2 was read directly (not from a prior session)
and cross-checked by `grep` against the actual test suite. Full matrix:
`docs/security/threat-traceability.md`. Result: 30 of 32 already had a real, specific-error test
from earlier phases (this repository's established practice of early coverage); T-20 and T-30 are
documented, accepted, not-testable-in-protocol residual risks by design. Two citation gaps closed
(`A-SOLV-01`, `F-INV-01..07` — pre-existing behavior that had never been literally cited by that
exact ID in source) and one new exploit-regression test added (`A-DUST-01`, from the F-10-02
finding below). Three threats (T-01, T-06 twice — `A-LIQ-01` and `A-ORACLE-06`) were spot-checked
this phase via live mitigation removal against the rebuilt on-chain artifact, confirming
non-vacuity directly rather than only trusting the historical record.

### 2. Stateful invariant fuzzer

`tests/fuzz/`: two markets in one `LiteSVM` instance (classic SPL/SPL; Token-2022 transfer-fee
collateral), six actors (two lenders, two borrowers, a liquidator, an attacker), a weighted action
generator covering deposit/withdraw collateral, supply/withdraw, borrow/repay, accrue_interest,
liquidate, absorb_bad_debt, withdraw_collateral_fees, warp_time and move_price as first-class
operations, and biased boundary-adjacent amount sampling (0, 1, dust, exact-balance, near-max,
min_debt±1, and a 15% uniform fallback for coverage). Every action asserts the applicable
**[GLOBAL]** invariants (`crates/aegis-test-kit/src/invariants.rs::assert_all_global`, plus
action-specific INV-SOLV-01/INV-ACC-04 checks) whether it succeeded or failed, and asserts
byte-exact failed-operation atomicity on failure. Seeded and fully reproducible
(`fuzz_determinism_same_seed_same_trace`); a coarse-to-fine ("ddmin") trace shrinker minimizes any
failure to a small reproduction, bounded by a hard replay cap so shrinking itself can never run
unbounded.

```
$ cargo test --test fuzz --offline
test fuzz_shrink_reproduces_on_a_synthetic_failure ... ok
test fuzz_determinism_same_seed_same_trace ... ok
[fuzz-ci] TOTALS seeds=6 ops=1500 succeeded=845 failed=655
test fuzz_ci_bounded_campaign ... ok
test result: ok. 3 passed; 0 failed; 3 ignored (extended campaign + 2 manual mutation probes)
```

### 3. Mutation validation — all nine GLOBAL invariants

Full procedure, results table, and two honestly-recorded findings (a test-harness bug that
initially produced a false "not detected," and two literal mutations proven architecturally inert
by the token program's own CPI-level balance backstop, requiring adjusted mutations) in
`docs/security/mutation-report.md`. **All nine caught within a bounded 20,000-operation budget**
(worst case: 2,001 operations; most caught in under 500).

### 4. A real bug, found and fixed — F-10-02

The extended fuzz campaign found a genuine, previously-unknown, reproducible violation of
INV-ACC-06 against the **correct, unmutated** program (seed 21, step 3628): `repay`'s
`assets`-denominated path could, when a market's `total_borrow_assets` had been repaid down to a
tiny remainder held entirely by one position, leave that position with a small nonzero "dust"
share balance while `total_borrow_assets` reached exactly zero — a permanent inconsistency, since
zero borrow-assets makes `utilization()` read zero and no future interest ever accrues to un-stick
it. Root-caused to the `VIRTUAL_SHARES`/`VIRTUAL_ASSETS`-adjusted floor/ceil round trip between
`to_shares_down` and `to_assets_up` at these tiny magnitudes. Fixed in
`programs/aegis/src/instructions/borrow/repay.rs` (full details, severity assessment, and the
scope-check for the same pattern in `liquidate.rs`: `docs/security/findings.md` F-10-02). Frozen as
a permanent regression, following the required order — reproduce, freeze the failing test, confirm
it fails against the vulnerable code, fix, confirm it passes — in
`tests/adversarial/dust_debt.rs::a_dust_01_repay_of_the_last_borrower_never_strands_shares_without_assets`.

### 5. Value-creation search (T-17)

Conservation model documented in `tests/fuzz/ledger.rs`'s module doc
comment: collateral-side round-tripping is checked as an exact, always-on bound (collateral never
earns yield); loan-side round-tripping is checked against contribution plus the market's entire
lifetime accrued interest, a deliberately conservative upper bound complementing (not replacing)
`aegis-math`'s exact `P-SHARE-1..4` property tests. This search is what found F-10-02.

```
$ make fuzz   # 25 seeds x 4,000 ops = 100,000 operations
[fuzz-extended] seed=21 succeeded=2493 failed=1507   <-- the exact seed that found F-10-02
[fuzz-extended] TOTALS seeds=25 ops=100000 succeeded=64132 failed=35868
test fuzz_extended_campaign ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 742.36s
```
This is the post-fix acceptance run: seed 21 (the exact seed that crashed pre-fix) now completes
cleanly, and zero violations occur across the full 100,000 operations. An earlier run under this
same command, before the fix, is what found F-10-02 in the first place — see
`docs/security/mutation-report.md`'s campaign-statistics section for the full three-run history.

### 6. Traceability enforcement

`scripts/check-traceability.sh` parses `docs/invariants.md` directly (never a hand-maintained
second list), expands range notations (`A-ORACLE-03..11`), skips rows assigned to a phase this
repository has not reached (11+), and asserts every cited test ID exists in source. Already
blocking in CI with no workflow change needed: `.github/workflows/ci.yml`'s `guards` job runs every
`scripts/check-*.sh`, and this script's filename matches that glob. Fail-behavior proven directly:
a temporary fake test ID injected into a copy of the row, script run (fails, names the missing ID
and the offending row), doc restored (`git diff` empty), script re-run (passes).

```
$ ./scripts/check-traceability.sh
check-traceability: OK — 83 test id(s) referenced by docs/invariants.md (phase <= 10) all exist
```

### 7. Manual review log

Systematic, freshly-read (not inherited) review of account constraints, signer boundaries, PDA
seeds, vault ownership, token-program validation, measured-delta paths, accounting totals, rounding
direction, oracle validation order, interest accrual, health/LTV, liquidation, bad debt, Token-2022
policy, CPI callback safety, post-CPI reload, admin authority, pause behavior, resource/DoS surface,
and SDK numeric-precision assumptions, plus an explicit panic search (zero `.unwrap()`/`.expect()`
in production code paths). Full log: `docs/security/review-log.md`.

### 8. Full regression

```
$ cargo fmt --all --check
(exit 0, no output)

$ cargo clippy --workspace --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 10.15s
(zero warnings)

$ cargo test --workspace --offline
test result: ok. 240 passed; 0 failed ... (34 test-result blocks total, up from 32 at the Phase 9
baseline: +1 for tests/fuzz.rs, +1 for tests/adversarial.rs)

$ for s in scripts/check-*.sh; do ./"$s"; done
(all OK, including the new check-traceability.sh)
```

### 9. Deviations

- **Two of the nine literal mutations in `phase-10-security.md`'s own table** ("skip the
  free-liquidity check in `withdraw`"; "remove the borrow liquidity check") were found, on
  direct rebuild-and-test analysis, to be architecturally inert given this codebase's CPI-level
  balance backstop — not a fuzzer gap. Both were replaced with an adjusted mutation that
  genuinely exercises the same invariant (direct accounting desync rather than removing a
  redundant precondition). Recorded as `docs/security/mutation-report.md`'s "Finding 2," per
  `phase-10-security.md` #5's explicit instruction not to force a match through a contrived
  scenario.
- **Not all 32 threats received a fresh, live mitigation-removal cycle this phase** — three were
  spot-checked directly (T-01, and T-06 twice); the remaining 29 rely on the historical record that
  each was proven non-vacuous when its test was originally written, under this repository's own
  standing AGENTS.md §8 discipline ("an invariant without a falsifying test is a hope"), verified
  this phase only via traceability (the test exists and currently passes), not via a fresh removal
  cycle for every one of the 32. Stated plainly as a scope decision, not a silent gap.
- **The liquidation callback path (`labs/hostile-callback`, Phase 8) was not re-fuzzed inside the
  stateful campaign** — building full callback-CPI support into the generic action generator was
  judged disproportionate additional complexity given `A-CPI-01..04` already provides dedicated,
  atomic-rollback-proven coverage of that surface (Phase 8). Recorded as a scope decision.
- No ADR was written this phase: no frozen document's formula, invariant, or account model changed
  — the F-10-02 fix corrects the *code* to actually implement the already-frozen INV-ACC-06, it
  does not change what INV-ACC-06 or any economic formula says.

---

## Phase 11 — evidence

**Phase 11 is complete.** Scope per `docs/phases/phase-11-performance.md`: the Mollusk CU benchmark
harness, a committed CU baseline for every current production instruction (SPL and Token-2022
variants), PERF-I1..I6 investigated with real measurements (PERF-I6 first), PERF-C1..C3 contention
verification, the three `labs/` custody implementations plus `labs/cu-bench`'s comparison, the CU
regression CI gate (with a proven deliberate-failure cycle), and measurement-justified optimization
only where justified.

### 0. Phase gate (verified before any code was written)

Phases 0–10 complete and tagged (`phase-10-security` = HEAD at session start); working tree clean;
baseline `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and
`cargo test --workspace --offline` (240 passed, 34 test-result blocks, matching Phase 10's own
recorded baseline exactly) all green; no `tests/bench/`, no `benchmarks/`, no `labs/vault-*` —
Phase 11 genuinely not started. Toolchain versions re-verified directly against crates.io rather
than assumed (`docs/ecosystem-research.md` §18): `mollusk-svm` 0.15.1, `mollusk-svm-programs-token`
0.15.1, `pinocchio` 0.11.2, `pinocchio-token` 0.7.0, `pinocchio-system` 0.6.1.

### 1. PERF-I6, first — and a real finding (T-27)

`tests/bench.rs::perf_i6_liquidate_worst_case`: a market with Token-2022 on both sides (2%
transfer-fee collateral; fee-free Token-2022 loan side, the only configuration `create_market`
accepts for a loan asset), a maximal real borrow, then a severe price crash driving the position
into the collateral-clamp liquidation branch — built entirely through real `supply`/
`deposit_collateral`/`borrow`/`liquidate` instructions, measured through Mollusk against the real
compiled `aegis.so`.

**First honest measurement, before any optimization: 469,137 CU — 2.3× over the 200,000 CU
budget.** Every scenario in the full suite that touches real interest accrual was also over or
dangerously close to budget (`repay` dt=30d: ~203–205k CU; `borrow`/`supply`/`withdraw`: 160k–184k
CU). This is a genuine T-27-class correctness/resource-safety finding, investigated (§2 below,
PERF-I2) and fixed (§3, OPT-01) rather than hidden by raising the compute limit, per the phase
spec's explicit instruction.

**After the fix: 109,687 CU — a 45.2% margin under budget**, and now the worst-case instruction in
the whole benchmark suite.

### 2. PERF-I1..I6 — investigated with measurements

Full write-up with exact numbers and methodology: `docs/performance-strategy.md` §6. Summary:

| ID | Question | Result |
|---|---|---|
| PERF-I6 | Does `liquidate` fit 200k CU with Token-2022 on both sides? | **Initially NO** (469,137 CU) — root-caused and fixed; **after OPT-01: YES** (109,687 CU, 45.2% margin) |
| PERF-I2 | Cost of `mul_div` with 256-bit intermediates? | Phase 0's ~100–300 CU/call estimate was **wrong by ~2 orders of magnitude** for the as-written implementation: a 256-iteration bit-serial division loop ran on every call regardless of whether the value needed 256 bits. Root cause of PERF-I6's finding. Fixed by OPT-01. |
| PERF-I1 | What fraction of `borrow`'s CU is oracle validation + valuation? | ~25,434 CU (~53% of `borrow`'s 48,360 CU total, post-OPT-01) — real, but not dominant with 76% margin remaining; no further action justified |
| PERF-I3 | Token-2022 vs SPL transfer cost? | +1.7k–3k CU per transfer — small, bounded, as anticipated; document only |
| PERF-I4 | Does the stored-bump optimization save meaningfully? | Already adopted architecturally since Phase 2 (`grep -rn find_program_address programs/aegis/src/` — zero matches); nothing to measure a delta against |
| PERF-I5 | Cost of the mandatory post-CPI `reload()`? | Folded into every transfer-bearing benchmark; required for correctness (INV-CUS-05) regardless of cost; not isolated further given comfortable margins everywhere |

### 3. OPT-01 — the one optimization, in full BEFORE/CHANGE/AFTER/DELTA/RISK form

Full entry: `docs/performance-strategy.md` §7. Summary: a 4-line fast path in
`crates/aegis-math/src/u256.rs::div_u128` (native `u128` division when the true value fits in the
low limb — provably identical output to the existing 256-iteration bit-serial loop for that case,
not an approximation). `accrue_interest` (dt=30d): 151,016 → 12,118 CU (−92.0%). Worst-case
`liquidate`: 469,137 → 109,687 CU (−76.6%). Verified via the full `aegis-math` test suite
(including the bignum-reference property test) and the full workspace suite, both unchanged and
green after rebuilding. No invariant, economic formula, or account layout changed.

No other optimization was found to be justified by measurement — every scenario has comfortable
margin after OPT-01 alone (`AGENTS.md` §17: optimize only if measurement justifies it).

### 4. CU baseline — every current production instruction

`benchmarks/cu.json` (committed) / `benchmarks/README.md` (human table + methodology + limitations).
29 scenarios across all 13 current production instructions (`ping` excluded as a Phase 1 toolchain
proof, not production; the Phase 12 governance/pause instructions do not exist in this program yet).
Worst case: `liquidate`, Token-2022 both sides, clamped/full-repay — 109,687 CU, 45.2% margin to the
200,000 CU budget.

```
$ make bench
[... full 29-scenario table, see benchmarks/README.md §5 ...]
wrote benchmarks/cu.json
```

### 5. PERF-C1..C3 — contention, verified two ways

Full evidence: `docs/performance-strategy.md` §2. Static half — `tests/bench/contention.rs`:

```
$ cargo test --test bench --offline perf_c -- --nocapture
=== PERF-C1: disjoint writable sets across two markets (supply, two users) ===
market A writable set: {...6 pubkeys...}
market B writable set: {...6 pubkeys...}
PERF-C1 confirmed: zero overlap between the two markets' writable account sets.
test contention::perf_c1_disjoint_markets_static ... ok

=== PERF-C3: write-set enumeration ===
instruction                    market   position   fee_position   protocol
...
PERF-C3 confirmed: among instructions that write Market, the only OTHER writable non-vault,
non-user-ATA account is the per-user Position — Market is the sole intra-market contention point.
test contention::perf_c3_write_set_enumeration ... ok

test result: ok. 2 passed; 0 failed
```

PERF-C2 is `A-PAR-01` (Phase 3, `tests/phase3_adversarial.rs`), re-confirmed here as part of the
same write-set enumeration rather than duplicated as a second, independently-maintained check.

Dynamic half — `bots/liquidator/src/perfC1ConcurrentDemo.ts` (`make bench-contention`, or
`npm run perf-c1` in `bots/liquidator/`), a real local (offline, no fork) Surfpool run: two
`supply` transactions on two different markets fired via `Promise.all` (neither awaited before the
other is sent) both confirmed successfully, and a same-market contrast pair (two lenders supplying
into the SAME market, also concurrent) also both succeeded — the expected result, since Solana's
runtime queues rather than rejects concurrent writers to one account; the same-market pair rules
out "Surfpool doesn't contend accounts at all" as a confound for the cross-market result.

```
$ npm run perf-c1
[7] PERF-C1 static check: writable account sets from the COMPILED `supply`
    instruction metadata (AccountMeta[], not the Rust struct)...
    Market A writable set (6 accounts): 4VMGLc2ZETmsR8iMumwg9vNCzxjjhRF13pGadRbWbzGy,
      HQGNLQhQRHSKf7PW1i4qDfQQBCP8TSNKwYL28U3xD5ar, x4of7nhs74dptQVrFYEwLzvSFYCjT6bWLuAXs6KXZ33,
      Ft195nK6uQHGYEL2rJU2h8pmMyosbdUHojeNTGM7mo37, 6WKDnFEXMXX6TScZkrdzCxCMZPRundGrFvoH1gyiRXQx,
      6Z8AiZMMC6zyinnrpXPCguMKnndqk5R7fCknsYoLdvDo
    Market B writable set (6 accounts): 82UQxqs2bdnQWdBP9uAF3SoNhMVxrKDvEkvjDk8jK5pe,
      yfqLjPCFBecHE3kj3PzhLNrrynY3bv9RwwoYJpFwZFv, HuAMFRhurvr4AniBAQf4LRwtxzqRoQeWYYBPvhMRd1g3,
      7X2XDNtmZjzDP3zZzXmfGU3tRVwwHHJatHPXi6VY8o95, 2HVzKPMwF9gvrKFrNewew9ogYd2yVEAXTjt2MQnnSXUq,
      GdJy1N9GLRVPUKD6Vs1sumGes6dXRZYy9EysDv5yPz1f
    PASS: the two writable sets are disjoint -- zero shared addresses.

[8] PERF-C1 dynamic check: submitting BOTH transactions via Promise.all so
    neither is awaited before the other is sent...
    slot before submission: 397
    Market A supply: signature=5zrzfVASuRhrnLVwhBp2MQFMyZsH2yYT1uNsq7gJXiD5a4rbU4Mi2eek2fwf9N12BVbqmG8Yx7Xg9udx6kWYJufd
      confirmationStatus=confirmed err=null slot=398
    Market B supply: signature=xQRVGTsCQ6Wcr1N4uV8JKnj6V6k4DY7ABcwhzi14RPEtMst8odJAY4XDvLbxVZmtj4uFgUxbMAzhgZoVvhiL7sT
      confirmationStatus=confirmed err=null slot=398
    PASS: both cross-market transactions confirmed with no error, submitted
    concurrently (neither awaited before the other was sent).

[9] Contrast case: two DIFFERENT depositors supplying into the SAME market
    (Market A), also fired concurrently via Promise.all...
    shared writable accounts between the two same-market supplies: 3
      HQGNLQhQRHSKf7PW1i4qDfQQBCP8TSNKwYL28U3xD5ar, Ft195nK6uQHGYEL2rJU2h8pmMyosbdUHojeNTGM7mo37,
      6WKDnFEXMXX6TScZkrdzCxCMZPRundGrFvoH1gyiRXQx
    Market A supply (depositor 2): signature=L9jEySqHd3tbwu4TsV1wg3ubXY5dTBZnSkHD9QsJ6qR7XrgcG3Q2CeuFWn7L89B1YaUj4FimcrnTowXd5a5PZ6S
      confirmationStatus=confirmed err=null slot=408
    Market A supply (depositor 3): signature=5mN7P1i29djPPUeTCJb4UWcnAEDGLbear5Hgzc1hjDsovH7ACXUKXvBdUG3wH8UkYELFCEc7ZJ52JKRia67TqcR6
      confirmationStatus=confirmed err=null slot=408
    PASS: both same-market transactions ALSO confirmed successfully -- expected
    (Solana queues concurrent writers to a shared account rather than rejecting
    them); PERF-C1's claim is about avoiding a shared bottleneck ACROSS markets,
    not about same-market transactions failing.

=== PERF-C1 VERIFIED ===
```

Note: the deployed `aegis.so` used for this live run was compiled for SBPFv1
(`cargo build-sbf --arch v1`, into an isolated scratch build directory) rather than the default
SBPFv3 this workspace's other artifacts use — a real, separately-verified finding this same work
surfaced: the locally installed `solana` CLI (3.1.10) cannot deploy an SBPFv3 binary at all
(`ELF error: Detected sbpf_version required by the executable which are not enabled`), confirmed
directly (including with `--skip-feature-verify`), so this script builds its own SBPFv1 copy
specifically for real on-chain deployment. This has no bearing on `tests/bench/`'s own CU
measurements, which run through Mollusk's own BPF loader directly against the default-built
artifact and never go through `solana`/`surfpool` deploy at all.

### 6. `labs/` — three custody implementations, measured

Full evidence and written conclusion: `docs/performance-strategy.md` §8. `vault-anchor`,
`vault-native`, `vault-pinocchio` all implement the identical custody primitive (initialize a vault
PDA, deposit, withdraw via a PDA-signed CPI) with equivalent security checks (signer, mint, token
program, vault/PDA, authority, owner — verified directly by reading each lab's source, not just
its own self-report). Each has its own `tests/basic.rs` proving the happy path and a non-owner
withdrawal rejection.

```
$ cargo test -p vault-anchor -p vault-native -p vault-pinocchio --offline
... 2 tests per lab, all passing ...

$ cargo test -p cu-bench --offline -- --nocapture
=== labs/cu-bench: custody primitive CU comparison ===
lab                  initialize      deposit     withdraw
vault-anchor              15206         8441         8541
vault-native              12411         6134         6181
vault-pinocchio            5263         1837         1874

Native Δ vs Anchor: initialize=2795 deposit=2307 withdraw=2360
Pinocchio Δ vs Anchor: initialize=9943 deposit=6604 withdraw=6667
test cu_comparison ... ok
```

**Written conclusion (full reasoning in `docs/performance-strategy.md` §8): not yet.** Native is
18–28% cheaper than Anchor; Pinocchio is 65–78% cheaper than Anchor — confirming the hypothesis and
matching the magnitude of Anza's own `p-token` result. But Aegis's binding constraints are
correctness (an audit surface Anchor's automatic validation makes reviewable) and contention
(`Market` write-locking, unaffected by framework choice), not CU — every instruction has
comfortable margin after OPT-01 alone. ADR-0003 stands: Anchor for production, native/Pinocchio
remain lab-scoped.

### 7. CU regression CI gate — built, wired into CI, and proven to fire

`scripts/check-cu-regression.sh`: compares a fresh measurement (via `cu_benchmark_suite`, written
to a scratch file, never overwriting the committed baseline) against `benchmarks/cu.json`; fails on
a regression exceeding 10% on any scenario (exactly +10.00% passes — integer arithmetic, no
floating point). Already wired into CI: `.github/workflows/ci.yml`'s `guards` job matches every
`scripts/check-*.sh`, and now depends on `build-and-test` (reusing its cache) so the compiled
program artifact this check needs is available.

**Proven to actually fire**, not just implemented (`scripts/prove-cu-regression-gate.sh`):

```
$ ./scripts/prove-cu-regression-gate.sh
=== Step 1: deliberately regress (disable OPT-01's fast path) ===
=== Step 2: rebuild the on-chain program with the regression ===
=== Step 3: run check-cu-regression.sh -- this MUST FAIL ===
check-cu-regression: FAILED -- cu_benchmark_suite did not run successfully:
[... 6 scenarios at/over the 200,000 CU budget, exactly matching the pre-OPT-01 baseline ...]
prove-cu-regression-gate: gate correctly FAILED on the deliberate regression (expected, see output above)
=== Step 4: restore the real code and rebuild ===
=== Step 5: run check-cu-regression.sh again -- this MUST PASS ===
check-cu-regression: OK — no benchmarked scenario regressed by more than 10% against the committed baseline (29 scenarios checked)
prove-cu-regression-gate: OK — gate fired on the deliberate regression and passed after restoration
```

The deliberate regression was never committed — `crates/aegis-math/src/u256.rs` is restored from
a backup inside the script's own `trap cleanup EXIT`, verified identical to the pre-run file after
every invocation.

### 8. INV-RES-01..07

| ID | Test | Status |
|---|---|---|
| INV-RES-01 | `B-CU-*` (`B-CU-LIQUIDATE-WORST-CASE`, `B-CU-ALL`) | **NEW, Phase 11** — every scenario in `benchmarks/cu.json` asserted under 200,000 CU |
| INV-RES-02 | `A-PAR-01` | Phase 3, pre-existing; re-confirmed via `tests/bench/contention.rs::perf_c3_write_set_enumeration` |
| INV-RES-03 | `A-PAR-02` | Phase 6, pre-existing (`tests/phase6_admin.rs`) |
| INV-RES-04 | `CI-NOLOOP` | **NEW, Phase 11** — `scripts/check-no-loop.sh`, an allowlist of the two reviewed, structurally-bounded production loops; proven to fire on an injected unreviewed loop and pass after removal |
| INV-RES-05 | `U-ACCT-02` | Phase 2, pre-existing |
| INV-RES-06 | `I-TX-01` | Phase 9, pre-existing |
| INV-RES-07 | `A-CPI-01` | Phase 8, pre-existing |

### 9. Full regression

```
$ cargo fmt --all --check
(exit 0, no output)

$ cargo clippy --workspace --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s)
(zero warnings)

$ cargo test --workspace --offline
test result: ok ... (45 test-result blocks total, up from 34 at the Phase 10 baseline: +1 for
tests/bench.rs, +2 for the three new labs' own test binaries, +2 for cu-bench/vault-* unit-test
scaffolding, +remainder from labs/vault-native and labs/vault-pinocchio's doc-tests)

$ for s in scripts/check-*.sh; do ./"$s"; done
(all OK, including the two new Phase 11 scripts)

$ cargo test -p vault-anchor -p vault-native -p vault-pinocchio -p cu-bench --offline
(all passing, see §6 above)
```

### 10. Deviations

- **`tests/bench/`'s Mollusk methodology builds pre-state through real `LiteSVM` instructions, then
  snapshots for the single measured call** — a refinement of `testing-strategy.md` §3's original
  "hand-constructed account set" phrasing, made explicit and documented there, per the phase spec's
  own requirement to "execute real instruction paths, not isolated helper functions."
- **`liquidate`'s callback branch (Phase 8) is excluded from the worst-case CU figure** — its own
  internal cost is the callback program's responsibility, not Aegis's, matching
  `performance-strategy.md`'s own target-table treatment of `liquidate` as "2 CPIs" (no callback).
  Not separately re-measured this phase as a distinct scenario; noted as a limitation in
  `benchmarks/README.md` §8.
- **`labs/` is classic SPL Token only**, no Token-2022 variant — a deliberate scope bound (ADR-0003:
  "one primitive, three implementations... cannot grow into a second protocol"), matching what
  `vault-anchor` established first.
- No ADR was written this phase: OPT-01 is a pure algorithmic substitution inside a private helper
  function with an unchanged public contract — no invariant, economic formula, account layout, or
  frozen document changed. `docs/performance-strategy.md` itself was updated extensively (as its
  own header always anticipated Phase 11 would), which is expected maintenance of a
  hypothesis-tracking document, not a frozen-document *decision* change.

---

## Phase 12 — Governance, Upgrades and Migrations

**Status: COMPLETE.** Two-step admin transfer, guardian-only pause-setting, `set_market_params`
with the tighten/loosen timelock asymmetry, one real Anchor 1.2.0 `Migration<'info, From, To>`
account schema migration, and a genuine devnet verifiable build.

### 1. Admin/governance instructions (IMPLEMENTED, TESTED)

`set_pending_admin`, `accept_admin`, `set_guardian`, `set_protocol_pause`, `set_market_pause`,
`set_market_params`, `commit_pending_params`, `migrate_protocol_v2` — all eight new instructions in
`programs/aegis/src/instructions/admin/`, wired in `lib.rs`.

```
$ cargo test -p aegis --lib
test result: ok. 50 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test --test phase12_admin_transfer
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test --test phase12_pause
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test --test phase12_params
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test --test phase12_migration
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### 2. Two-step admin transfer

`set_pending_admin` writes only `pending_admin`; `accept_admin` requires `signer ==
protocol.pending_admin` exactly (`A-AUTH-05`), sets `admin`, and clears `pending_admin` in the same
instruction — which is also what makes a replayed `accept_admin` fail (the old pending admin no
longer matches the cleared field). There is no single-transaction `set_admin` instruction anywhere
in `lib.rs` (`no_single_step_set_admin_instruction_exists`, a structural grep-based test).

### 3. Pause architecture and the guardian asymmetry

`guards::require_authorized_pause_change` implements the rule directly: undefined bits are rejected
unconditionally (`A-ADM-05`); the admin may set or clear any combination of the four defined bits;
the guardian may only add bits (`new_paused` must be a superset of `old_paused`) — clearing even
one already-set bit as the guardian fails with the exact `GuardianCannotClearPause` error
(`A-AUTH-04`).

**INV-ADM-04, the load-bearing property of this phase:** `repay`, `deposit_collateral`,
`absorb_bad_debt`, and `close_position` do not import `guards::require_pause_bit_clear` at all, and
their `Accounts` structs carry no `protocol` field — there is no code path through which pause state
could reach these handlers (ADR-0014 §5). `A-ADM-01` proves this with real on-chain state: a lender
supplies, a borrower deposits collateral and borrows, a second position is seeded with real bad debt
(`seed_borrow_state`, the same legitimate injection technique Phase 4/6 already established), every
protocol pause bit AND every market pause bit are set, and all four safety exits still succeed —
while `supply`/`borrow` fail with `OperationPaused` in the same paused state. `A-ADM-03` and a
companion test separately confirm `borrow`/`supply`/`withdraw`/`withdraw_collateral`/`liquidate`
each fail with the exact same error under their own specific pause bit.

### 4. `set_market_params` / `commit_pending_params`

`accrue_mut` runs under the current (about-to-be-superseded) parameters before anything else reads
or writes a parameter this instruction might change (INV-ADM-07, `U-ADM-01` — both the immediate
and delayed-commit paths are tested with real nonzero elapsed time and nonzero utilization, each
checked against `Market::accrue_view`'s own prediction). The canonical bounds validators
(`Market::validate_risk_params`/`validate_irm_params`/`validate_oracle_config` — the exact same
functions `create_market` uses) are re-run on every write, immediate or staged, including the
derived liquidation bound (`A-ADM-04`, tested on both paths). Identity fields (mints, token
programs, vaults, decimals, `config_id`) have no field in `SetMarketParamsArgs` at all — not merely
rejected at runtime, structurally absent (`A-ADM-06`, both a byte-level struct-size assertion and a
before/after on-chain field comparison).

Risk-increasing ("loosening") changes are staged in a standalone `PendingMarketParams` account
(`PDA([b"pending_params", market])`) behind a 48-hour timelock (`constants::PARAM_TIMELOCK_SECS`);
risk-reducing ("tightening") changes apply immediately. `I-ADM-01` runs the full lifecycle: tighten
→ immediate; loosen → active params unchanged, pending populated; commit one second before
`effective_at` → `PendingParamsNotYetEffective`; commit at `effective_at` → succeeds, bounds
re-validated again, pending state cleared. A second loosening proposal while one is already pending
is rejected (`PendingParamsAlreadyStaged`) rather than silently overwritten or merged (ADR-0014 §6).

### 5. Account migration: `migrate_protocol_v2`

Verified against the actual installed `anchor-lang 1.2.0` source
(`~/.cargo/registry/.../anchor-lang-1.2.0/src/accounts/migration.rs`), not training-data memory of
pre-1.0 Anchor, which had no such primitive. `Protocol` gained `schema_version: u8`, carved out of
`_reserved` (`64 → 63` bytes; `Protocol::LEN` unchanged at 202 — no realloc). The pre-Phase-12
layout is kept as `ProtocolV1`, used only as the `Migration<'info, ProtocolV1, Protocol>`'s `From`
half.

```
$ cargo test --test phase12_migration
test i_upg_01_migration_preserves_every_field_and_initializes_schema_version ... ok
test i_upg_02_second_migration_attempt_is_rejected ... ok
test migration_requires_the_real_admin_signer ... ok
test migration_rejects_account_owned_by_the_wrong_program ... ok
test migration_rejects_corrupted_old_data ... ok
test migration_rejects_unsupported_version_garbage_discriminator ... ok
test migration_rejects_a_nonexistent_account ... ok
test migration_accounts_struct_has_no_token_or_vault_fields ... ok
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

`I-UPG-01`: a real `ProtocolV1` account is injected (`svm.set_account`, the same legitimate
technique `state_injection.rs` established for otherwise-unreachable prior-schema states — no real
transaction in an already-Phase-12 codebase can produce a `ProtocolV1` account any other way), then
migrated via a real transaction; every preserved field is checked byte-for-byte, `schema_version` is
initialized, the PDA address and program ownership are unchanged, and the account size is identical
before and after (no realloc). `I-UPG-02`: a second migration attempt on the now-migrated account
fails — Anchor's own `AccountDiscriminatorMismatch`, because the account now carries `Protocol`'s
discriminator, not `ProtocolV1`'s — not a silent no-op.

### 6. Verifiable build (INV-UPG-05, I-UPG-03)

Tooling re-verified directly (`docs/ecosystem-research.md` §19.2), not assumed: `solana-verify
0.5.1`, installed via `cargo install solana-verify --locked`. `apr.dev` is not referenced anywhere
in this workflow — every command targets a local Docker build or the OtterSec `verify.osec.io`
infrastructure.

**Two real defects the Docker build's stricter toolchain caught that this repository's own local
build did not, both fixed before the final deployment below:**

1. `Liquidate::try_accounts` exceeded the SBF stack-frame limit (4096 bytes) by 448 bytes after
   `protocol` was added unboxed, a genuine "may cause undefined behavior during execution" compiler
   error. Fixed by boxing `protocol` and (still 64 bytes over) `position` in
   `programs/aegis/src/instructions/liquidate/liquidate.rs` — the only instruction of the five
   pausable ones that needed it. Full before/after CU accounting: `benchmarks/README.md` §9.
2. `set_market_params`'s `PendingMarketParams` account creation used a raw
   `anchor_lang::solana_program::program::invoke_signed` call — caught not by the Docker build
   itself but by `scripts/check-cpi-allowlist.sh` (INV-RES-07/A-CPI-01) run immediately afterward,
   which asserts no raw `invoke_signed` exists anywhere in `programs/aegis/src` outside the audited
   `CpiContext`-based helpers. Fixed by switching to `anchor_lang::system_program::create_account`
   with `CpiContext::new(...).with_signer(...)` — the exact pattern `token/vault.rs`'s
   `create_vault` already uses for its own PDA-signed account creation, not a new one.

Both fixes required a second, then a third, Docker rebuild — the hash below is from the final
build, after both fixes, and is what is actually deployed.

```
$ cargo install solana-verify --locked
    Installed package `solana-verify v0.5.1` (executable `solana-verify`)

$ solana-verify build --library-name aegis --arch v1
...
    Finished `release` profile [optimized] target(s) in 19.72s
Finished building program
Program Solana version: v4.0.0
Docker image Solana version: v4.0.0
d204bb910eecead8a0b87321f294daaf021bb596e0699a3ffb8dd3bfce8ed099

$ solana-verify get-executable-hash target/deploy/aegis.so
d204bb910eecead8a0b87321f294daaf021bb596e0699a3ffb8dd3bfce8ed099

$ solana-verify get-program-hash DbRhjkZV1QSxMj5AvrYdgVsyEz8nKhoCLnSLGSKsqaF9 --url https://api.devnet.solana.com
d204bb910eecead8a0b87321f294daaf021bb596e0699a3ffb8dd3bfce8ed099
```

**The local reproducible build and the live devnet program hash byte-for-byte identically.** This
is real, not fabricated: a program is actually deployed at `DbRhjkZV1QSxMj5AvrYdgVsyEz8nKhoCLnSLGSKsqaF9`
on devnet (see §7), and its on-chain bytes were independently hashed via `solana-verify
get-program-hash` against a live RPC call, matching the locally-built artifact's own hash exactly.
`scripts/verify-build.sh` reproduces this entire check in one command (I-UPG-03, tagged
network-dependent/optional-tier, never part of `make test` — `docs/zero-cost-demo.md` §8):

```
$ ./scripts/verify-build.sh
verify-build: building deterministically (solana-verify build --arch v1)...
...
verify-build: local reproducible-build hash: d204bb910eecead8a0b87321f294daaf021bb596e0699a3ffb8dd3bfce8ed099
verify-build: fetching the deployed program's on-chain hash (DbRhjkZV1QSxMj5AvrYdgVsyEz8nKhoCLnSLGSKsqaF9 @ https://api.devnet.solana.com)...
verify-build: on-chain program hash: d204bb910eecead8a0b87321f294daaf021bb596e0699a3ffb8dd3bfce8ed099
verify-build: OK -- local reproducible build matches the deployed devnet program exactly (d204bb910eecead8a0b87321f294daaf021bb596e0699a3ffb8dd3bfce8ed099)
```

**I-UPG-03 / INV-UPG-05: satisfied**, against a real devnet deployment — not a local-only build
with the live-verification step honestly marked blocked (the original fallback plan, superseded
once devnet SOL became available; see §7 for how that was obtained).

### 7. Deployment and upgrade authority

**Stage 0 → a real Stage-1-shaped devnet deployment** (`governance.md` §5). Honest characterization:
the upgrade authority below is a plain software keypair generated for this deployment, not a
hardware wallet — this deployment demonstrates the "first devnet deploy" milestone and the
verifiable-build workflow end-to-end; it does **not** claim true Stage-1 key-custody hardening.
Real Stage 1 (a hardware wallet) and Stages 2-4 remain future work, exactly as `governance.md` §5
already states.

| Field | Value |
|---|---|
| Network | Solana devnet (`https://api.devnet.solana.com`) |
| Program ID | `DbRhjkZV1QSxMj5AvrYdgVsyEz8nKhoCLnSLGSKsqaF9` (matches `declare_id!` in `programs/aegis/src/lib.rs` exactly — no identity mismatch) |
| Upgrade authority | `ALjq2DN6nipDE31uadKrSHnvpYwm54sH7LyM2VMp2vBE` (a plain devnet-only software keypair, generated for this deployment; not committed to this repository, per AGENTS.md §19) |
| Initial deploy transaction | `373TuyccCxrkT4V5oJEUiUwPJ9KrAv9gWpmkMQMmTE3dycMXbMYsvRXAUHrgpDyYQXaHDiGqhoE9v3KTnFnEDcwD` (pre-CPI-fix build) |
| Final upgrade transaction | `4Cg5wgHbshWGzMMBZfB8yJ3HKPUwXM68ZxGwta6ZHCUvZPS93rFc8P4EMZ3A3z93ommzP5V91cwHZCaimS4e7XR4` (after both fixes in §6 — this is what is live now) |
| Data length | 704,616 bytes |
| Program-data account | `9861ArMX2CQE4SA2iPk3hBZzvM2Xu52RooriGdvHrLXs` |
| Build architecture | SBPFv1 (`solana-verify build --arch v1`) — this machine's installed `solana` CLI (3.1.10) cannot deploy an SBPFv3 binary at all (`ELF error: Detected sbpf_version required by the executable which are not enabled`), an independently-reproduced instance of the exact same finding `docs/project-status.md`'s Phase 11 section already recorded for `cargo build-sbf`'s own default target |
| Verified hash | `d204bb910eecead8a0b87321f294daaf021bb596e0699a3ffb8dd3bfce8ed099` (matches the local deterministic build exactly, post-fix — see §6) |

No mainnet deployment exists or was created. `docs/governance.md` §5's upgrade-authority
progression table has been updated to reflect this real devnet deployment, not merely the
hypothetical version that existed before Phase 12.

### 8. Documentation updated

`docs/governance.md` (full pause matrix §3.1, concrete timelock/storage/overwrite rules §4,
concrete migration record §6.1, real deployment/upgrade-authority record §5), `docs/adr/0014-*.md`
(new — the implementation decisions the phase spec left open), `docs/adr/README.md`,
`docs/ecosystem-research.md` (§19, OtterSec/`solana-verify` re-verification), `docs/invariants.md`
(evidence column), `docs/threat-model.md` (evidence for T-29), `benchmarks/README.md`/`cu.json` (new
instructions benchmarked).

### 9. Security and performance regression

```
$ ./scripts/check-traceability.sh
check-traceability: OK — 97 test id(s) referenced by docs/invariants.md (phase <= 12) all exist

$ ./scripts/check-cu-regression.sh
check-cu-regression: OK — no benchmarked scenario regressed by more than 10% against the committed baseline (37 scenarios checked)

$ cargo test --workspace
test result: ok ... (49 test-result blocks, 308 individual tests passed, 0 failed, 4 ignored [network-tagged/optional-tier])

$ cargo fmt --all --check
(exit 0, no output)

$ cargo clippy --workspace --all-targets -- -D warnings
(zero warnings)
```

### 10. Non-vacuity (targeted mutation, reverted before commit)

Five targeted mutations, each applied, confirmed to make the intended test fail with real command
output, then reverted (`git status` clean before and after; no mutation was ever committed):

| # | Mutation | Result |
|---|---|---|
| 1 | Removed the guardian's superset check in `guards::require_authorized_pause_change` | `a_auth_04_guardian_can_set_but_not_clear` (unit) and both `a_auth_04_*` integration tests FAILED — guardian could clear pause bits |
| 2 | Wired a market-pause check into `repay`'s handler | `a_adm_01_safety_exits_succeed_with_every_pause_bit_set` FAILED — `repay` returned `OperationPaused` |
| 3 | Commented out `Market::validate_risk_params` in `set_market_params` | `a_adm_04_invalid_params_rejected_with_exact_errors` and `a_adm_04_derived_bound_rejected_on_staged_path_too` FAILED — invalid/unsafe params were silently accepted |
| 4 | Added a `collateral_mint: Pubkey` field to `SetMarketParamsArgs` and applied it to `market.collateral_mint` | `a_adm_06_identity_fields_have_no_field_in_set_market_params_args` (struct-size assertion, 303→335 bytes) and `a_adm_06_identity_fields_are_unchanged_after_set_market_params` (runtime field comparison) both FAILED |
| 5 | Forced the loosening branch's condition to `false` (`if false && is_loosening(...)`) | `i_adm_01_full_tighten_loosen_timelock_lifecycle` FAILED — a loosening change applied immediately instead of staging |

Every mutation was reverted via a byte-identical restore from a pre-mutation backup (`diff` run
against the backup after restoring, confirmed identical) before the next mutation was attempted;
the final `git status` after all five shows only the legitimate Phase 12 changes, no mutation
residue.

### 11. Deviations

- `instruction-catalogue.md`'s `repay` account-list row lists a `[R][PDA] protocol` account that
  `repay`'s actual `Accounts` struct does not have — a deliberate, documented narrowing (ADR-0014
  §5) in the direction INV-ADM-04 requires, not an oversight. `deposit_collateral`/
  `absorb_bad_debt`/`close_position`'s documented rows never included `protocol` in the first
  place.
- The exact 48-hour timelock duration, the `PendingMarketParams`-as-separate-account storage shape,
  the conservative "unclassified field ⇒ loosening" default, and the pending-proposal
  reject-not-overwrite rule are all Phase 12 implementation decisions filling gaps the frozen
  documents left open — recorded in ADR-0014, not silently guessed.
- No mainnet deployment; devnet only, per the phase's own "do not spend real funds unnecessarily"
  instruction and `governance.md` §5's own roadmap (Aegis v1 targets Stage 1).

### 12. Next action

**Phase 12 is complete. Phase 13 has NOT been started.**

---

## Phase 13 — Integration, Security Review and Release

**Status: COMPLETE.** Phase gate verified first (HEAD at `phase-12-governance`, zero commits since,
clean tree, baseline suite/CU regression/traceability all green — see §0 below); scope held to
demo, self-review, runbooks, documentation reconciliation, and release, per
`docs/phases/phase-13-release.md`'s explicit non-scope (no new instructions, no new features, no
refactor except to fix a real defect).

### 0. Phase gate (verified before any doc/code change)

```
$ git log -1 --format='%H' HEAD
4cfca022076b5b79547edf8f60d56d7799d28a39
$ git log -1 --format='%H' phase-12-governance
4cfca022076b5b79547edf8f60d56d7799d28a39        # identical -- HEAD IS the tag
$ git log phase-12-governance..HEAD --oneline    # (empty -- zero commits since)
$ git status --short                             # (empty -- clean tree)
$ cargo test --workspace
test result: ok ... 49 test-result blocks, 308 individual tests, 0 failed, 4 ignored
$ ./scripts/check-traceability.sh
check-traceability: OK — 97 test id(s) referenced by docs/invariants.md (phase <= 12) all exist
$ ./scripts/check-cu-regression.sh
check-cu-regression: OK — no benchmarked scenario regressed by more than 10% (37 scenarios)
```

All ten guard scripts (`check-collateral-transfer-paths`, `check-cpi-allowlist`, `check-no-close`,
`check-no-dup`, `check-no-float`, `check-no-init-if-needed`, `check-no-loop`, `check-no-slot-time`,
`check-overflow-checks`, `check-traceability`) pass. `cargo fmt --all --check` and
`cargo clippy --workspace --all-targets -- -D warnings` are both clean. Phase gate: **satisfied.**

### 1. The complete offline demo (`zero-cost-demo.md` §5)

New: `crates/aegis-test-kit/examples/phase13_demo.rs`, wired to `make demo` (replacing the Phase 8
demo `make demo` ran previously — that demo remains directly runnable via
`cargo run -p aegis-test-kit --example phase8_demo`). Implements all 16 steps of the canonical
scenario against a single market and a single borrower position, using a real Token-2022
transfer-fee collateral mint (step 1) and real, nonzero IRM rates so interest and protocol fees are
genuine, not fixture-zeroed: mints/protocol/market → inject prices → lender supplies → borrower
deposits (fee-aware, measured-delta credited) → borrows → 30-day warp + real accrual with printed
utilization/APR → a beyond-LTV borrow rejected (`ExceedsMaxLtv`) → a stale-oracle borrow rejected
(`OraclePriceStale`) → the SAME stale oracle still permits repay + collateral top-up → a fresh
crashed price makes the position liquidatable → a partial liquidation → a deeper crash and a
clamped liquidation into real bad debt → `absorb_bad_debt` (protocol fee shares burned first) →
the lender's full withdrawal realizing the socialized loss → a final invariant report and a
per-instruction CU ledger built from THIS run's own measured `compute_units_consumed`, never
copied from `benchmarks/cu.json`. Every "EXPECT FAILURE" step asserts the exact error code AND that
the position's `borrow_shares`/`collateral_amount` are byte-unchanged, not merely that the call
failed.

```
$ make demo
... (full transcript; see the evidence bundle) ...
=== 16. Final invariant report and per-instruction compute-unit ledger (this run's own measurements) ===
  INV-CUS-01: holds
  INV-CUS-02: holds
  INV-ACC-03: holds (total_supply_assets >= total_borrow_assets)
  INV-ACC-06: VIOLATED as expected -- total_supply_shares=22 > 0 while total_supply_assets=0.
  This is F-13-01 (docs/security/findings.md), a genuine Phase 13 finding, not a demo defect.

instruction                      compute units
----------------------------------------------
supply                                  22430
deposit_collateral                      15718
borrow                                  49011
accrue_interest                         14425
repay                                   24780
deposit_collateral (stale oracle)       15718
liquidate (partial)                    107119
liquidate (clamped, bad debt)           69408
absorb_bad_debt                         12640
withdraw                                22425
```

The demo's own final step is where **F-13-01** (below) was actually discovered — a real, if
harmless, invariant edge case surfaced by running the mandated full scenario, not invented for this
report.

### 2. Self-review — F-13-01, and the panic-search correction

`docs/security/review-log.md`'s new "Phase 13" section records the systematic pass. One genuine
finding (F-13-01: a bad-debt event can leave `fee_position` holding supply-share "dust" that,
combined with the market's one remaining lender withdrawing in full, can zero `total_supply_assets`
while `total_supply_shares` stays nonzero — INV-ACC-06's literal text violated). Proven, not
assumed, to carry zero economic consequence at any future deposit size, including `u64::MAX`
(`tests/adversarial/orphaned_fee_shares.rs`). No code fix — matching the F-10-01 precedent for a
confirmed-harmless finding; the recommendation and rationale are in `docs/security/findings.md`.

Separately, this review found and corrected an inaccurate Phase 10 claim: the panic-search section
of `docs/security/review-log.md` said "zero `.unwrap()`/`.expect()` matches in production code
paths," which was already false at the time it was written — `liquidate.rs`'s Phase 8 callback
branch has two, both provably unreachable-to-panic (guarded by a preceding `require_eq!`).
Corrected in place rather than silently.

### 3. Token-movement paths — six vs. seven, resolved

`docs/account-model.md` §6.3 already documents **seven** paths (Phase 8 updated it when the
liquidation callback added a new destination on the existing seizure leg) — not six. Verified this
is exhaustive and current: `grep`-enumerated every `transfer_checked_in`/`transfer_checked_out`
call site (4 inbound call sites across 2 logical paths, 6 outbound call sites across 5 logical
paths = 7 total), confirmed no raw `token_interface::transfer_checked` call exists outside
`token/transfer.rs`, and re-ran `scripts/check-collateral-transfer-paths.sh` (passes: an
allowlisted-caller-set assertion, not just a count) and `scripts/check-cpi-allowlist.sh`. **No
eighth, undocumented path exists.**

### 4. `liquidate` deep review

Read end to end, both branches (no-callback and Phase 8 callback), against
`instruction-catalogue.md` §17 and ADR-0013. Ordering confirmed security-critical and correct:
pause guard → reentrancy guard → input/token-program checks → oracle validation (before any state
write, INV-ORA-07) → liquidatability (`HF < WAD` strict) → accounting mutation → (callback branch
only) seize into the callback account → set guard, flush to the account buffer, dispatch `invoke`
(never `invoke_signed`) with a minimal, `is_signer: false`-only account list → clear guard →
**re-read** `loan_vault` and require the measured delta `>= repay_assets` → recompute `hf_after`
from the same validated price bands. The two `.unwrap()`s in the callback branch are reachable only
after a preceding `require_eq!` guarantees both `Option`s are `Some` — provably panic-free, not
merely untested. No gap found in any `require!`/`require_eq!`/`require_keys_eq!` in this file.

### 5. Callback / reentrancy review

Re-verified directly against source, not from memory of the Phase 8/10 record: no `Market` or
`liquidator` `AccountInfo` ever appears in the callback's constructed `Vec<AccountMeta>`
(`build_callback_instruction`, unit-testable and unconditionally `is_signer: false`); the guard
(`market.liquidation_guard`) is set and flushed via `ctx.accounts.market.exit(&crate::ID)?` **before**
the CPI, checked unconditionally at the top of `handler` in both branches; `A-CPI-01..04` remain
present and, per the baseline run above, passing. No new Phase 12 account was omitted from
protection — the four unpausable instructions (`repay`, `deposit_collateral`, `absorb_bad_debt`,
`close_position`) still carry no `protocol` field at all, structurally.

### 6. Custody source-of-truth audit

Grepped every `.amount` read on `loan_vault`/`collateral_vault` across `programs/aegis/src`. The
only crediting path is `token/transfer.rs::transfer_checked_in`'s measured `after - before` delta
following a mandatory `reload()`; no instruction reads a vault's raw balance as an accounting
source of truth anywhere. **No forbidden path found.**

### 7. Feature-gate / build-mode review

`grep -rn 'cfg(feature' programs/aegis/src crates/aegis-math/src` — zero matches. `overflow-checks
= true` confirmed in `[profile.release]` and by `scripts/check-overflow-checks.sh`. `lto = "fat"`,
`codegen-units = 1` unchanged. Program ID (`declare_id!`) matches the deployed devnet program
recorded in Phase 12 §7. No canonical-build-vs-tested-artifact divergence found.

### 8. Fresh mutation-gate spot-check (3 of 9 [GLOBAL] invariants, representative subset)

Full re-run of all nine gates was judged not to fit this session's remaining budget after the
demo/review/documentation work above; instead, three were re-run fresh end to end — chosen to
cover the two files Phase 12 materially changed since the original Phase 10 mutation-validation
(`withdraw.rs`, `borrow.rs`) plus the accrual core (`market.rs`) — using the exact
edit→build→probe→revert→rebuild procedure `docs/security/mutation-report.md` established:

| Invariant | File mutated | Detected | Evidence |
|---|---|:--:|---|
| INV-CUS-01 | `lend/withdraw.rs` (skip `total_supply_assets` decrement) | ✅ | `mutation_probe` caught in 115 ops (2-step minimized trace: `Supply` → `Withdraw`); `mutation_probe_bad_debt` caught in 55 tail steps |
| INV-SOLV-01 | `borrow/borrow.rs` (skip the post-borrow LTV `require!`) | ✅ | `mutation_probe` caught in 174 ops (2-step trace: `Supply` → `Borrow`); `mutation_probe_bad_debt` caught in 5 tail steps |
| INV-ACC-04 | `state/market.rs::accrue_view` (credit interest to borrow side only) | ✅ | `mutation_probe` caught in 624 ops (fired via the downstream INV-CUS-01 check, a legitimate cross-invariant catch, same class as the original INV-ACC-02 mutation's result); `mutation_probe_bad_debt` caught in 200 tail steps |

Each mutation was reverted via `git checkout --` and confirmed byte-identical against `HEAD`
before proceeding to the next; the full workspace suite was re-run clean afterward (§0's baseline,
re-confirmed: 309 passed [308 + this phase's new `orphaned_fee_shares` test], 0 failed). The
remaining six gates (INV-CUS-02, INV-ACC-01, INV-ACC-02, INV-ACC-03, INV-ACC-06, INV-SOLV-04) were
**not** re-mutated this phase; their Phase 10 evidence (`docs/security/mutation-report.md`) stands,
and `token/transfer.rs`/`liquidate.rs`/`absorb_bad_debt.rs` — three of the other six gates' target
files — have had zero commits since Phase 10 (`git log phase-10-security..HEAD --`), which is
itself evidence those exact enforcement lines are unchanged, though it is not a fresh execution and
is reported as such rather than rounded up to "all nine re-verified."

### 9. Documentation reconciliation

- **`docs/instruction-catalogue.md`**: added the two instructions missing since Phase 12
  (`commit_pending_params`, `migrate_protocol_v2` — real, tested, event-emitting since Phase 12; the
  catalogue simply never listed them) and corrected `repay`'s account list (a stale `[R][PDA]
  protocol` row predating Phase 12, per ADR-0014 §5's own "negative consequence" note). Program and
  docs now match exactly: `grep -n 'pub fn ' programs/aegis/src/lib.rs` lists 22 entrypoints (`ping`
  plus 21 production instructions); all 21 non-`ping` instructions are now documented.
- **`docs/invariants.md`**: INV-ACC-06's row annotated with the F-13-01 evidence (no enforcement
  change; a documentation-of-reality addition, matching the precedent INV-CUS-01's own row already
  set for its Phase 8 callback exception).
- **`docs/security/review-log.md` / `findings.md`**: see §2 above.
- **`sdk/ts/src/generated/`**: found and fixed a real, live staleness — the committed IDL/generated
  code lagged a doc-comment wording change in `set_market_params.rs` (comment-only; no functional
  API difference). `npm run codegen` regenerated it; `npm run codegen:check` now passes
  (`check-codegen: OK`); `./scripts/check-vectors.sh` independently confirms `tests/vectors/*.json`
  is not stale.
- **`README.md`**: full rewrite (§12 below) — the prior version was still headed "PHASE 9" and
  described `make fuzz`/`make bench` as non-functional stubs.
- **Component status table** (this document, above): two stale rows corrected (Governance &
  migrations; Liquidator bot) — see the note directly under that table.
- **`docs/runbooks/`** (new): R-1..R-5, per `governance.md` §7 — see §11 below.

### 10. Phase 0 final self-audit — re-answered against the final Phase 13 codebase

Every question from the historical **Phase 0 self-audit** (above, preserved unchanged) re-answered
fresh, against the code and evidence as they exist today — not copied forward:

| Question | Phase 13 answer |
|---|---|
| Is this a coherent lending protocol? | Yes, unchanged in shape and now fully evidenced end to end: the Phase 13 demo (§1) runs the complete supply→borrow→accrue→liquidate→bad-debt→socialize loop against real transactions, not a description of one. |
| Is any feature present solely for resume coverage? | Re-examined with the full, final feature set in view (governance, migrations, Token-2022, composability all now shipped): every one has a stated product or security reason in its own phase's ADR/spec; `labs/` remains explicitly labeled non-production (`coverage-matrix.md` §2/§3). No new coverage-only feature was added in Phase 13 — none was in scope. |
| Can the account model parallelize? | Yes, re-confirmed structurally this phase (§3-§6 above) with zero new writable-account sharing introduced since Phase 0; PERF-C1..C3 (Phase 11) remain the measured evidence, unchanged. |
| Is shared writable state minimized? | Yes. `Protocol` gained one field in Phase 12 (`schema_version`, carved from `_reserved`, no realloc) and remains read-only in every user instruction — confirmed by `grep` this phase finding no new `Protocol`-writing user instruction. |
| Are authorities unambiguous? | Yes. Still exactly one signer PDA (`Market`); the Phase 8 callback branch is confirmed (§5) to add a new destination, never a new authority. |
| Could user-provided accounts redirect assets? | No — the double-validation pattern (canonical PDA + `has_one`) was spot-checked this phase in `liquidate.rs`, `absorb_bad_debt.rs`, `set_market_params.rs`/`commit_pending_params.rs` (the two instructions the catalogue was missing) and holds in all of them. |
| Could the wrong token program be accepted? | No — `require_keys_eq!` against `market.*_token_program` confirmed present at every token-touching call site read this phase, including the two Phase 12 admin instructions that don't touch tokens at all (structurally exempt, not merely unchecked). |
| Could Token-2022 semantics invalidate accounting? | No — the Phase 13 demo (§1) exercises a real transfer-fee collateral mint through the full lifecycle including liquidation and bad debt, with measured-delta accounting holding at every step; RV-5's Phase 7 closure stands unchanged (no new extension shipped or discovered this phase). |
| Could vault balances diverge from internal accounting? | Answered precisely, not just "no": INV-CUS-01/02 hold throughout the fresh full-suite run and the demo; the ONE reachable divergence-adjacent state found this phase (F-13-01) is on the SHARE side, proven to carry zero token/value consequence, and is fully documented rather than glossed over. |
| Could rounding be exploited? | The 14 rounding directions are unchanged; this phase's own investigation of F-13-01 is itself a rounding-boundary analysis, concluding no value creation, extending the T-17 evidence base rather than contradicting it. |
| What happens when oracle data is unavailable? | Re-verified live, not just re-read: the Phase 13 demo's steps 8-9 make the SAME price account stale by clock warp alone (no re-publish) and show borrow failing closed while repay/deposit succeed, in one continuous run. |
| What happens during extreme volatility? | Re-exercised in the demo (a $150 → $95 → $40 SOL crash across two liquidations); `max_conf_bps` and the LTV/LT gap are unchanged and were not re-derived this phase (no reason to). |
| How does bad debt arise? | The demo produces a real instance via mechanism #1 (gap risk: a price crash outrunning what a single partial liquidation can collect) — one of the five named mechanisms, observed directly rather than only described. |
| How does liquidation fail? | Unchanged; this phase's `liquidate` review (§4) found no new failure mode and no gap in the existing ones. |
| Which admin action could cause catastrophic damage? | Re-confirmed: none involving funds (INV-ADM-01, and now also covering the two Phase 12 instructions the catalogue was missing, §9) — the upgrade authority (T-30) remains the stated largest residual risk, documented in `governance.md` §5 with the real Phase 12 devnet deployment's own honest Stage-0.5-not-Stage-1 characterization, unchanged. |
| Which assumptions would be unacceptable for real money? | Unchanged list (`economic-model.md` §11, `threat-model.md` §4), plus F-13-01 is now an explicit, named addendum to "assumptions a real audit should re-examine," rather than an unstated blind spot. |
| Are tests capable of falsifying important invariants? | Re-tested, not re-asserted: three [GLOBAL] mutation gates were freshly re-run this phase (§8) and all three were caught; the remaining six rest on Phase 10's original, still-largely-unmodified-file evidence, reported honestly as not re-run rather than implied to be. |
| Is every portfolio claim backed by future observable evidence? | The README rewrite (§12) is the concrete test of this claim for the final time — every number in it traces to a command in this document or a file in this repository. |
| Could a Sonnet session execute the phases without inventing architecture? | Retrospectively yes, borne out over 13 phases; Phase 13 itself invented no architecture (F-13-01 was investigated and documented, not "fixed" with a new formula, precisely to honor this). |
| Have unnecessary technologies been rejected explicitly? | Unchanged; Phase 13 added no new technology (no new dependency, no new instruction, no new integration). |

### 11. Operational runbooks R-1..R-5

New: `docs/runbooks/` (`README.md` index + `R-1-oracle-degradation.md` ..
`R-5-hostile-market-params.md`), expanding `governance.md` §7's trigger/immediate/decide/recovery
sketches into executable procedures: exact authority, exact SDK calls (verified against the real
generated function signatures in `sdk/ts/src/generated/instructions.ts` — a real, if minor,
correction was needed mid-drafting when the first draft assumed a single-options-object call shape
that does not match the actual generated `(programId, accounts, args)` signature; a further real
gap was found and disclosed: `@aegis/sdk`'s pause bitflag constants exist only as private,
unexported constants in `read.ts`, so the runbooks define them locally rather than claim an import
that would not compile), validation before and after, rollback where one exists, logging, and
explicit "do not" items, plus a shared "explicit forbidden actions" section.

### 12. README

Full rewrite. Real numbers only: 309 deterministic Rust/TS tests in the release suite (308 from
`cargo test --workspace` + 1 new this phase), 87 invariants, 32 threats, the 100,000-operation
extended fuzz campaign figure (Phase 10, cited not re-run at full scale this phase — see §13), the
`benchmarks/cu.json` CU table, the transaction-size table (`sdk/ts` §12 above, max 834 bytes).
Removed: the stale "PHASE 9" status banner, the "not yet functional" claim about `make fuzz`/
`make bench` (both are real and passing). No forbidden marketing term
(`production-ready`/`battle-tested`/`secure`/`audited`/etc.) appears describing Aegis positively.
The non-audit/non-deployment warning is in the second visible section, not the footer.

### 13. Extended fuzz campaign and full release regression

`make fuzz` was re-run fresh against the final Phase 13 codebase:

```
$ make fuzz
[fuzz-extended] TOTALS seeds=25 ops=100000 succeeded=64429 failed=35571
test fuzz_extended_campaign ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 323.36s
```

Zero invariant violations across all 25 seeds / 100,000 operations, including seed 21 (the exact
seed that found F-10-02 in Phase 10), which now completes cleanly. Full command list and output:
`docs/evidence/release-regression.txt`; the campaign's own result:
`docs/evidence/extended-fuzz-campaign-result.txt`.

Full release-candidate regression (fmt, clippy, all 10 guard scripts, `anchor build`, CU
regression, `cargo test --workspace`, SDK codegen/vectors/unit tests, app typecheck/build,
liquidator bot compile, `make demo`, `make fuzz`) — every command and its real output:
`docs/evidence/release-regression.txt`. Final tally: **309 Rust tests + 72 TypeScript tests, 0
failed**, CU regression clean (37 scenarios), all guard scripts pass, app and SDK build clean.

### 14. Deviations

- No protocol logic was changed. F-13-01 was investigated and documented, not fixed, per its own
  stated rationale (fixing it would touch the frozen `economic-model.md` §8.2 settlement formula
  for a purely cosmetic gain — out of Phase 13's reconciliation-not-redesign mandate).
- A full re-run of all nine [GLOBAL] mutation gates was scoped down to three representative ones
  (§8) for session time-budget reasons, disclosed rather than rounded up.
- The extended (100,000-op) fuzz campaign's Phase 13 status is recorded in the final report, not
  here, for the reason stated in §13.

### 15. Next action

**Phase 13 is complete pending remote tag verification (`phase-13-release`, `v0.1.0` resolving to
the release commit on `origin`). This is the final planned phase. No Phase 14.**

---

## Next action

**Phase 13 is complete — see "Phase 13 — Integration, Security Review and Release" above for full
evidence. This is the final planned phase of Aegis Protocol. No Phase 14 exists or is planned.**
