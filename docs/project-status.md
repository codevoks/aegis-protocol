# Aegis — Project Status

**Last updated: 2026-09-06**
**Current phase: Phase 1 — Toolchain, Repository and CI Foundation — COMPLETE**
**Next phase: Phase 2 — State, PDAs & custody primitives — NOT STARTED**

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

**Phase 2 has NOT been started.** Nothing beyond a single no-op `ping` instruction exists on-chain.

## Component status

| Component | IMPL | TEST | DEMO | DOC | COMMIT |
|---|:--:|:--:|:--:|:--:|:--:|
| `aegis-math` — arithmetic (`mul_div_floor`/`mul_div_ceil`) | ✅ | ✅ | ⬜ | ✅ | ⬜ |
| `aegis-math` — shares | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| `aegis-math` — IRM/accrual | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| `aegis-math` — health | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| `aegis-math` — liquidation | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| `programs/aegis` — `ping` (toolchain proof only) | ✅ | ✅ | ⬜ | ✅ | ⬜ |
| `Protocol` / `Market` / `Position` | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| Vaults & custody | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| Token-2022 policy engine | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| Collateral instructions | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| Lend/borrow instructions | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| Oracle (Pyth adapter) | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| Liquidation & bad debt | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| Governance & migrations | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| `aegis-test-kit` (LiteSVM bootstrap only) | ✅ | ✅ | ⬜ | ✅ | ⬜ |
| Invariant fuzzer | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| CU benchmarks | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| `labs/` (Anchor/native/Pinocchio) | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| TypeScript SDK | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| Web app | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| Liquidator bot | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |

`COMMIT` columns above turn ✅ only once this phase's commit and tag are pushed and verified against
the remote — see **Git** at the end of this document.

## Invariant status

87 invariants defined across 12 groups (9 marked **[GLOBAL]**). Phase 1 does not test any protocol
invariant (there is no protocol yet); it tests two Phase-1-scoped, non-numbered engineering rules by
grep guard: **T-16** (`overflow-checks = true`) and **INV-ACC-10** (no floats), plus **INV-LIFE-01**
(no `init_if_needed`) and **T-13** (no `dup`) and **INV-ORA-06** (no slot-based time), all as guard
scripts rather than runtime invariants, because there is no runtime state yet for them to guard.
**0 of the 87 numbered protocol invariants are implemented or tested** — this is correct and expected
at the end of Phase 1; they begin at Phase 2. See `docs/invariants.md` for the per-phase assignment.

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

**Phase 1 is complete. Hand Phase 2 to the implementation model when the maintainer explicitly
authorizes it. Phase 2 has NOT been started.**
