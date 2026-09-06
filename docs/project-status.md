# Aegis — Project Status

**Last updated: 2026-09-06**
**Current phase: Phase 2 — State, PDAs & Custody Primitives — COMPLETE**
**Next phase: Phase 3 — Collateral Flows — NOT STARTED**

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

**Phase 2 is complete.** `Protocol`, `Market`, `Position`, both custody vaults, and the Token-2022
extension policy engine exist on-chain, exactly as frozen in `account-model.md` and
`token-compatibility.md`. No token transfer, deposit, withdrawal, supply, borrow, repay, interest,
oracle, or liquidation logic exists yet — those begin at Phases 3–6.

## Component status

| Component | IMPL | TEST | DEMO | DOC | COMMIT |
|---|:--:|:--:|:--:|:--:|:--:|
| `aegis-math` — arithmetic (`mul_div_floor`/`mul_div_ceil`) | ✅ | ✅ | ⬜ | ✅ | ⬜ |
| `aegis-math` — shares | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| `aegis-math` — IRM/accrual | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| `aegis-math` — health | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| `aegis-math` — liquidation | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| `programs/aegis` — `ping` (toolchain proof only) | ✅ | ✅ | ⬜ | ✅ | ⬜ |
| `Protocol` / `Market` / `Position` | ✅ | ✅ | ✅ | ✅ | ⬜ |
| `initialize_protocol` / `create_market` / `init_position` | ✅ | ✅ | ✅ | ✅ | ⬜ |
| Vaults & custody | ✅ | ✅ | ✅ | ✅ | ⬜ |
| Token-2022 policy engine | ✅ | ✅ | ✅ | ✅ | ⬜ |
| Collateral instructions | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| Lend/borrow instructions | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| Oracle (Pyth adapter) | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| Liquidation & bad debt | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| Governance & migrations | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| `aegis-test-kit` (mints, market/position lifecycle, account decoding) | ✅ | ✅ | ✅ | ✅ | ⬜ |
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

**INV-ACCT-08** (`deposit_collateral`/`withdraw_collateral` do not declare `Market` writable) is
**not yet testable**: its subject instructions are Phase 3 scope and do not exist yet. This is
recorded here explicitly rather than silently skipped; it will be tested when Phase 3 ships those
instructions.

**Naming note:** `account-model.md` §11 defines a 9-item "Account-model invariant summary" using an
`INV-ACCT-*` prefix, distinct from `invariants.md`'s own master `INV-ACC-*` series (accounting,
§C) — the two documents use similar-looking prefixes for different, only partially overlapping
content (e.g. `INV-ACCT-09` and `INV-ACC-11` both concern `_reserved` bytes being zero). Phase 2's
task instructions referenced "`INV-ACCT-01..09`", which only exist in `account-model.md` §11; all
nine are addressed above/below. Nothing in either document was edited to resolve this — it is
noted here as a cross-reference clarification, not a frozen-document conflict.

Still **0 of the 87 numbered `invariants.md` invariants assigned to later phases** are implemented
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

**Phase 2 is complete. Hand Phase 3 (collateral flows) to the implementation model when the
maintainer explicitly authorizes it. Phase 3 has NOT been started.**
