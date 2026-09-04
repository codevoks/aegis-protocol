# Phase 1 — Toolchain, Repository and CI Foundation

**Status: NOT STARTED. This phase has not been started.**
**Prerequisite: Phase 0 complete and frozen.**

> Phase 1 builds no protocol logic. Its entire purpose is to make every later phase verifiable,
> reproducible and offline. Do not be tempted to "get started on the program" — a shaky foundation
> costs more in phases 4–10 than it saves here.

---

## 1. Scope

1. Verify and record the **actual current** toolchain versions (research gates RV-1, RV-2).
2. Initialize the Git repository and the Cargo/Anchor workspace.
3. Create the crate skeletons: `programs/aegis`, `crates/aegis-math`, `crates/aegis-test-kit`.
4. Implement **`aegis-math`'s arithmetic primitives only** — `mul_div_floor`, `mul_div_ceil` with
   256-bit intermediates — with full unit and property tests.
5. Establish CI with every guard from `testing-strategy.md` §9.
6. Establish the `Makefile` targets from `zero-cost-demo.md` §4.
7. Prove the zero-cost path works end to end: a trivial program builds, deploys to LiteSVM, and is
   invoked, offline.
8. Copy the Phase 0 documentation into the repository and create `docs/project-status.md`.

## 2. Explicit NON-scope

**Do not** implement, in this phase:

- Any account struct (`Protocol`, `Market`, `Position`) — that is Phase 2.
- Any instruction beyond a trivial no-op used to prove the toolchain works.
- Share math, IRM, health, or liquidation math — Phase 1 ships **only** `mul_div_*`.
- The oracle, tokens, or vaults.
- The SDK, the app, or the liquidator bot.
- Any `labs/` implementation (Phase 11).
- Devnet deployment of any kind.

If the toolchain fights back, **fix the toolchain**; do not work around it by deferring configuration
into a later phase.

---

## 3. Step 1 — Version verification (do this first, before writing anything)

Run and record the real output:

```bash
rustc --version && cargo --version
solana --version
avm --version && anchor --version
surfpool --version
npm view @solana/kit version
npm view @anchor-lang/core version
cargo search litesvm --limit 1
cargo search mollusk-svm --limit 1
cargo search pyth-solana-receiver-sdk --limit 1
```

**Known starting state** (measured 2026-09-04): `solana-cli 2.2.21` (**stale — must be upgraded**),
`rustc 1.88.0`, `node v22.12.0`, **no `anchor`, no `avm`, no `surfpool`**.

**Installation:**

```bash
# Agave CLI (Anchor 1.0.2 docs reference 3.1.10; install the current stable 3.x/4.x line)
sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"

# Anchor via avm
cargo install --git https://github.com/solana-foundation/anchor avm --force
avm install latest && avm use latest

# Surfpool — follow the current official instructions at solana.com/docs/tools/surfpool
```

**Expected targets** (from `docs/ecosystem-research.md` — *verify, do not assume*):
`anchor ≈ 1.1.2+`, `litesvm ≈ 0.16.0+`, `@solana/kit ≈ 8.2.0+`, `surfpool ≈ 1.5.0+`.

**If any version differs materially from the research document:**
1. Update `docs/ecosystem-research.md` with the new finding and today's date.
2. Note the delta in `docs/project-status.md`.
3. If a Phase 0 *decision* is invalidated, **STOP and report** — do not redesign.

Paste the real command output into `docs/project-status.md`. "Versions verified" without the output is
not evidence.

---

## 4. Repository structure to create

```
/
├── .github/workflows/ci.yml
├── .gitignore
├── Anchor.toml
├── Cargo.toml                    # workspace
├── Makefile
├── rust-toolchain.toml
├── README.md
├── AGENTS.md
├── CLAUDE.md
├── LICENSE                       # MIT or Apache-2.0
├── programs/aegis/
│   ├── Cargo.toml
│   └── src/lib.rs                # #[program] with ONE no-op instruction (`ping`)
├── crates/
│   ├── aegis-math/
│   │   ├── Cargo.toml            # no_std-compatible, zero solana deps
│   │   └── src/{lib.rs, fixed.rs, constants.rs}
│   └── aegis-test-kit/
│       ├── Cargo.toml
│       └── src/{lib.rs, svm.rs}
├── tests/
│   └── smoke.rs                  # LiteSVM: deploy + invoke `ping`
├── scripts/
│   ├── check-no-float.sh
│   ├── check-no-init-if-needed.sh
│   ├── check-no-dup.sh
│   ├── check-no-slot-time.sh
│   └── check-overflow-checks.sh
├── benchmarks/README.md
└── docs/                         # the Phase 0 tree, copied in
```

`sdk/`, `app/`, `bots/`, `labs/` are **not** created in Phase 1. Empty directories for future phases
are clutter that implies work that does not exist.

---

## 5. Critical configuration (get these exactly right)

### 5.1 `Cargo.toml` (workspace) — release profile

```toml
[profile.release]
overflow-checks = true      # MANDATORY (T-16). Release builds do NOT check overflow by default.
lto = "fat"
codegen-units = 1
```

`overflow-checks = true` is the single most important line in the repository's configuration. Without
it, every arithmetic safety argument in `economic-model.md` is void in the deployed artifact.
`scripts/check-overflow-checks.sh` asserts its presence in CI.

### 5.2 `programs/aegis/Cargo.toml`

```toml
[dependencies]
anchor-lang = "<pinned 1.x>"            # NO `init-if-needed` feature — banned by INV-LIFE-01
anchor-spl  = "<pinned 1.x>"            # for token_interface (added in Phase 2)

[features]
idl-build = ["anchor-lang/idl-build"]   # REQUIRED in Anchor 1.0 or IDL generation silently breaks
```

The `init-if-needed` feature must never be enabled: it is the canonical reinitialization footgun and
`scripts/check-no-init-if-needed.sh` exists to keep it out.

**Do not** add `solana-program` as a direct dependency — Anchor 1.x re-exports what is needed, and a
direct dependency causes crate-version conflicts and emits a build warning.

### 5.3 `Anchor.toml`
- No `[registry]` section (removed in Anchor 1.0).
- Test runner: LiteSVM template default; Surfpool for `anchor localnet`.
- Pin `anchor_version` and the Solana version explicitly.

### 5.4 `rust-toolchain.toml`
Pin the exact channel verified in step 1. Reproducibility is the point.

---

## 6. `aegis-math` — the only logic in this phase

```rust
// crates/aegis-math/src/fixed.rs
pub fn mul_div_floor(a: u128, b: u128, d: u128) -> Result<u128, MathError>;
pub fn mul_div_ceil (a: u128, b: u128, d: u128) -> Result<u128, MathError>;
```

Requirements:
- `#![no_std]`; **no** `solana-*` or `anchor-*` dependency; **no** floats.
- 256-bit intermediate for `a·b`. Either a hand-rolled `u256` (two `u128` limbs) or a vetted crate —
  if a crate, justify the choice in the phase report and confirm it is `no_std` and float-free.
- `d == 0` returns `Err(MathError::DivisionByZero)` — never panics.
- A result exceeding `u128::MAX` returns `Err(MathError::Overflow)`.
- `mul_div_ceil` computed as `(a·b + d − 1)/d` in 256-bit space.

Also ship `constants.rs`: `WAD`, `VIRTUAL_SHARES`, `VIRTUAL_ASSETS`, `SECONDS_PER_YEAR`.

**Tests (all required to complete the phase):**

| ID | Test |
|---|---|
| `U-ARITH-01` | Known vectors, including `mul_div_floor(3,5,2) == 7` and `mul_div_ceil(3,5,2) == 8` |
| `U-ARITH-02` | `d == 0` → `Err(DivisionByZero)`, no panic |
| `U-ARITH-03` | Overflow of the result → `Err(Overflow)`, no panic |
| `U-ARITH-04` | **`mul_div_floor(1.8e25 as u128, 1.8e19 as u128, 1e18 as u128)` succeeds** — the case that overflows a naive `u128` implementation. This test is the entire justification for 256-bit intermediates and must be present. |
| `P-ARITH-1` | `floor ≤ ceil ≤ floor + 1` for all inputs (proptest) |
| `P-ARITH-2` | Never panics for any `(a, b, d)` (proptest) |
| `P-ARITH-3` | Exact against a reference bignum computation (proptest) |

---

## 7. CI

`.github/workflows/ci.yml`, all jobs blocking, **running with no secrets configured**:

| Job | Command |
|---|---|
| fmt | `cargo fmt --all --check` |
| clippy | `cargo clippy --all-targets -- -D warnings` |
| math tests | `cargo test -p aegis-math` |
| build | `anchor build` |
| smoke | `cargo test --test smoke` |
| guards | `scripts/check-*.sh` |

Guard scripts (grep-based, deliberately crude — cheap, unambiguous, and they defend rules that reviewer
memory otherwise enforces):

| Script | Asserts | Invariant |
|---|---|---|
| `check-no-float.sh` | No `f32`/`f64` in `programs/` or `crates/aegis-math/` | INV-ACC-10 |
| `check-no-init-if-needed.sh` | No `init_if_needed` anywhere | INV-LIFE-01 |
| `check-no-dup.sh` | No `dup` Anchor constraint | T-11 |
| `check-no-slot-time.sh` | No `Clock::slot` used for time; only `unix_timestamp` | INV-ORA-06 |
| `check-overflow-checks.sh` | `overflow-checks = true` present in the release profile | T-16 |

Write these scripts so they **fail** when given a deliberately-violating fixture — a guard that never
fires is not a guard. Verify each one manually before declaring the phase complete.

---

## 8. `Makefile`

```make
setup:  ## verify + print toolchain versions
build:  ## anchor build
test:   ## cargo test --workspace   (offline, no secrets)
fmt lint clean
```

`bench`, `fuzz`, `demo`, `app` are declared as stubs that print "not implemented until phase NN" —
so the interface is stable from Phase 1 and later phases fill it in.

---

## 9. Smoke test (proves the zero-cost path)

`tests/smoke.rs`:
1. Build the program.
2. Load it into LiteSVM.
3. Invoke `ping`.
4. Assert success.

**With no network access.** Run it with networking disabled to genuinely verify NFR-4 rather than
assuming it.

---

## 10. Concepts demonstrated

Rust workspace and crate design · `no_std` library design · checked fixed-point arithmetic with
widening intermediates · property-based testing · Anchor 1.x project configuration · reproducible
toolchain pinning · CI as an enforcement mechanism · offline SVM testing.

---

## 11. Security work

- `overflow-checks = true` verified in the release profile (T-16).
- All five guard scripts implemented **and verified to fail on violating fixtures**.
- `.gitignore` covers `target/`, `.anchor/`, `node_modules/`, `*.json` keypairs, `.env*` (NFR-11).
- Confirm no keypair or secret is committed; the deploy keypair is generated locally and ignored.

---

## 12. Documentation deliverables

- All Phase 0 documents present under `docs/`.
- `AGENTS.md` and `CLAUDE.md` at the repository root.
- `README.md`: what Aegis is, current status (**Phase 1 — foundation only**), how to build and test.
  It must **not** describe features that do not exist. State plainly: *"Aegis is under construction;
  Phase 1 establishes the toolchain only."*
- `docs/project-status.md` created and filled in.
- `docs/ecosystem-research.md` updated with real verified versions.

---

## 13. Acceptance criteria

- [ ] Every version verified with **pasted real output** in `project-status.md`; RV-1 and RV-2 closed.
- [ ] `solana` CLI upgraded from 2.2.21 to the current stable line.
- [ ] `anchor`, `avm`, `surfpool` installed; versions recorded.
- [ ] `anchor build` succeeds; an IDL is generated (confirms the `idl-build` feature is set correctly).
- [ ] `cargo test --workspace` passes **offline**.
- [ ] `U-ARITH-04` (the 256-bit overflow case) passes.
- [ ] All seven `aegis-math` tests pass.
- [ ] `cargo clippy -- -D warnings` and `cargo fmt --check` clean.
- [ ] All five guard scripts pass on the repository **and fail** on a violating fixture.
- [ ] `overflow-checks = true` confirmed in the release profile.
- [ ] Smoke test deploys and invokes `ping` in LiteSVM with networking disabled.
- [ ] CI green with **no secrets configured**.
- [ ] No `sdk/`, `app/`, `bots/`, `labs/` directories created.
- [ ] No account structs and no protocol instructions implemented.
- [ ] `project-status.md` complete.
- [ ] Git tag `phase-01-foundation`.

## 14. Required evidence in `project-status.md`

1. Raw output of every version command.
2. Raw output of `cargo test --workspace`.
3. Raw output of `anchor build` (with the IDL path).
4. Output of each guard script, plus proof each fails on a violating fixture.
5. Confirmation the smoke test ran offline.
6. Any delta from `ecosystem-research.md`, with the document updated.

## 15. Known risks for this phase

| Risk | Response |
|---|---|
| Anchor 1.x is recent; templates or docs may not match | Trust the installed CLI's own `anchor init` output over any blog post. Record what it actually generates. |
| `avm`/Anchor install may need a specific Rust version | Adjust `rust-toolchain.toml`; record the requirement. |
| Surfpool install may differ from the documented path | Follow the current official docs; record the actual procedure. |
| LiteSVM 0.16 API may differ from older examples | Read the crate docs for the installed version, not tutorials. |
| A 256-bit crate may not be `no_std`/float-free | Prefer a hand-rolled two-limb `u256`; it is ~50 lines and removes the dependency question entirely. |

---

## 16. On completion

Update `project-status.md`, create the tag, then:

> **STOP.** Report Phase 1 complete with evidence. Do not begin Phase 2 without explicit instruction.
