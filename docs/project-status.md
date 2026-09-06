# Aegis — Project Status

**Last updated: 2026-09-06**
**Current phase: Phase 4 — Lending, Borrowing & Interest — COMPLETE**
**Next phase: Phase 5 — Oracle — NOT STARTED**

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
| 5 | Oracle | ⬜ NOT STARTED | — |
| 6 | Health, liquidation & bad debt | ⬜ NOT STARTED | — |
| 7 | Token-2022 | ⬜ NOT STARTED | — |
| 8 | Composability | ⬜ NOT STARTED | — |
| 9 | SDK, client & UI | ⬜ NOT STARTED | — |
| 10 | Security campaign | ⬜ NOT STARTED | — |
| 11 | Performance | ⬜ NOT STARTED | — |
| 12 | Governance & upgrades | ⬜ NOT STARTED | — |
| 13 | Integration & release | ⬜ NOT STARTED | — |

**Phase 3 is complete.** `deposit_collateral`, `withdraw_collateral` (zero-debt path only), and
`close_position` exist on-chain, exactly as frozen in `instruction-catalogue.md` §10/11/20 and
scoped by `docs/phases/phase-03-collateral.md`. Real token custody now moves through the protocol
for the first time — with measured-delta accounting on both SPL Token and Token-2022 transfer-fee
mints. `Market` remains read-only in both collateral instructions, preserving the intra-market
collateral parallelism claim (C2).

**Phase 4 is complete.** `supply`, `withdraw`, `repay` and `accrue_interest` exist on-chain and are
fully functional; `borrow` exists structurally but is hard-gated to always fail with
`OracleNotYetAvailable` until Phase 5 (`docs/phase-roadmap.md` "Sequencing the oracle
dependency") — no oracle account exists anywhere in its account list, so there is no code path
that could permit an actual borrow without a price check. `aegis-math` gained `shares.rs` (virtual
-offset share/asset conversions) and `irm.rs` (utilization, the piecewise-linear rate curve, and
third-order Taylor compounding); `state/market.rs` gained `accrue_view`/`accrue_mut`. Full evidence
is in **Phase 4 — evidence** below.

## Component status

| Component | IMPL | TEST | DEMO | DOC | COMMIT |
|---|:--:|:--:|:--:|:--:|:--:|
| `aegis-math` — arithmetic (`mul_div_floor`/`mul_div_ceil`) | ✅ | ✅ | ⬜ | ✅ | ⬜ |
| `aegis-math` — shares | ✅ | ✅ | ✅ | ✅ | ⬜ |
| `aegis-math` — IRM/accrual | ✅ | ✅ | ✅ | ✅ | ⬜ |
| `aegis-math` — health | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| `aegis-math` — liquidation | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| `programs/aegis` — `ping` (toolchain proof only) | ✅ | ✅ | ⬜ | ✅ | ⬜ |
| `Protocol` / `Market` / `Position` | ✅ | ✅ | ✅ | ✅ | ⬜ |
| `initialize_protocol` / `create_market` / `init_position` | ✅ | ✅ | ✅ | ✅ | ⬜ |
| Vaults & custody | ✅ | ✅ | ✅ | ✅ | ⬜ |
| Token-2022 policy engine | ✅ | ✅ | ✅ | ✅ | ⬜ |
| Collateral instructions (`deposit_collateral`, `withdraw_collateral`, `close_position`) | ✅ | ✅ | ✅ | ✅ | ⬜ |
| Lend/borrow instructions (`supply`, `withdraw`, `repay`, `accrue_interest`; `borrow` hard-gated) | ✅ | ✅ | ✅ | ✅ | ⬜ |
| Oracle (Pyth adapter) | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| Liquidation & bad debt | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| Governance & migrations | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| `aegis-test-kit` (mints, market/position lifecycle, user token accounts, invariant checker, borrow-state injection) | ✅ | ✅ | ✅ | ✅ | ⬜ |
| Invariant fuzzer | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| CU benchmarks | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| `labs/` (Anchor/native/Pinocchio) | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| TypeScript SDK | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| Web app | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| Liquidator bot | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |

`COMMIT` columns above turn ✅ only once this phase's commit and tag are pushed and verified against
the remote — see **Git** at the end of this document.

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
| INV-REP-01 | `borrow_is_hard_gated_*` tests plus `repay`'s account list containing no price account at all |
| INV-REP-02 | `repay.rs` declares no pause check anywhere (structural; Phase 12 must not add one) |
| INV-REP-03 | `U-REPAY-01` (`repay_clamps_to_actual_debt_never_pulls_excess`) |
| INV-REP-04 | `U-ROUND-04` (`round_04_repay_assets_borrow_shares_burned_floors`, `aegis-math`) |
| INV-REP-05 | `U-REPAY-02` (`full_repayment_via_shares_leaves_no_dust`) |

`INV-CUS-08`, formally assigned to Phase 4 in `docs/invariants.md`'s per-phase column, is tested on
the loan side by `loan_vault_direct_donation_is_never_credited` (`tests/phase4_adversarial.rs`),
mirroring Phase 3's collateral-side `A-CUS-08`.

Still **0 of the 87 numbered `invariants.md` invariants assigned to Phases 5-13** are implemented
or tested — expected at this point; see `docs/invariants.md` for the full per-phase assignment.

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
| RV-3 | Upgraded Pyth receiver program ID (post 2026-08-26) | 5 | OPEN |
| RV-4 | `VerificationLevel` shape in `pyth-solana-receiver-sdk` 2.x | 5 | OPEN |
| RV-5 | Complete current Token-2022 extension list and discriminants | 7 | OPEN |
| RV-6 | Does the runtime permit `A → B → A` CPI reentrancy? | 8 | OPEN |
| RV-7 | SIMD-0296 (4096-byte tx) availability and `@solana/kit` support | 9 | OPEN |
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

---

## Phase 4 — evidence

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

## Next action

**Phase 4 is complete. Hand Phase 5 (oracle) to the implementation model when the maintainer
explicitly authorizes it. Phase 5 has NOT been started.**
