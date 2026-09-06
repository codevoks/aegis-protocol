# Aegis — Current Ecosystem & Tooling Research

**Research date: 2026-09-04. Re-verified: 2026-09-05/06 (Phase 1); 2026-09-06 (Phase 2, Phase 5).**
**Status: FROZEN for Phase 0. Phase 1 re-verification is in §12; Phase 2's is in §14; Phase 5's
(RV-3/RV-4) is in §15.**

This document records what was verified about the Solana ecosystem *at the research date*, which
sources were used, and which Aegis design decisions depend on each finding. Nothing in Aegis may be
built on a tutorial or a memory of "how Solana worked"; it must trace to an entry here or to a newer
verified entry added by a later phase.

---

## 0. Verification protocol

Every version number below is either (a) read from a primary source at the research date, or
(b) marked `UNVERIFIED` / `ASSUMED`. Implementation phases MUST re-run the verification commands in
§8 before pinning anything, because this file will drift.

**Local machine state at research date** (measured, not assumed):

| Tool | Local version | Status |
|---|---|---|
| `solana` (Agave CLI) | 2.2.21 | **STALE — must upgrade in Phase 1** |
| `rustc` / `cargo` | 1.88.0 | Adequate; re-check against Anchor's MSRV |
| `node` | v22.12.0 | Adequate |
| `anchor` / `avm` | not installed | **Must install in Phase 1** |
| `surfpool` | not installed | **Must install in Phase 1** |

---

## 1. Anchor — MAJOR finding

**Anchor reached its first stable major release, v1.0.0, on 2026-04-02. Latest at research date: v1.1.2 (2026-06-26).**

This is the single most consequential finding of Phase 0. Almost every Anchor tutorial, blog post and
LLM-memorized pattern predates it and is describing 0.29/0.30/0.31 semantics.

Breaking changes that directly affect Aegis:

| Change | Impact on Aegis |
|---|---|
| TS package renamed `@coral-xyz/anchor` → `@anchor-lang/core` | SDK and app must import `@anchor-lang/core`. Any generated code referencing `@coral-xyz/anchor` is wrong. |
| **Duplicate mutable accounts disallowed by default**; opt in via `dup` constraint | Removes an entire historical vulnerability class *by default*. Aegis must never use `dup`. Recorded as a mitigation in the threat model (T-13). |
| LiteSVM test template is the `anchor init` default | Aligns with our test strategy; LiteSVM is the primary harness. |
| Surfpool is the default for `anchor test` / `anchor localnet` | Replaces `solana-test-validator`. |
| External `solana` CLI dependency removed (native build/deploy/airdrop/balance) | Simplifies toolchain; CLI still needed for keygen/config ergonomics. |
| Legacy IDL instructions removed; **Program Metadata** used for IDL management | IDL publication story changed. SDK codegen must target the new layout. |
| `idl-build` feature now required in the program's `Cargo.toml` | Phase 1 must set this or IDL generation silently breaks. |
| `Migration<'info, From, To>` account type added | Directly serves Phase 12 (account schema upgrades) — a real migration primitive exists; do not hand-roll one. |
| Optional-account bumps are `Option<u8>` not `u8` | Affects any bump handling code. |
| `CLOSED_ACCOUNT_DISCRIMINATOR` removed | Close-account patterns must use current Anchor `close =` semantics, not the old manual discriminator trick. |
| `#[interface]` attribute and `interface-instructions` feature removed | Do not use them. |
| Multiple `#[error_code]` enums in one program disallowed | Aegis uses exactly one error enum. |
| Anchor updated to the Solana 3.0 crate line | Affects crate compatibility for every dependency (see Pyth, §4). |
| `verifiedBuild` now uses the OtterSec registry (`verify.osec.io`); `apr.dev` is defunct | Verifiable-build story for Phase 12 targets OtterSec. |
| `[registry]` section removed from `Anchor.toml`; `login` command removed | Phase 1 config must not include them. |

Additional guidance confirmed: **do not add `solana-program` as a direct dependency of an Anchor
program**; use the crate re-exported by `anchor-lang` to avoid v1/v2/v3 crate conflicts. Anchor warns
on builds that do.

Sources:
- <https://github.com/solana-foundation/anchor/releases>
- <https://raw.githubusercontent.com/solana-foundation/anchor/master/CHANGELOG.md>
- <https://www.anchor-lang.com/docs/installation>
- <https://www.anchor-lang.com/docs>

**Decisions affected:** ADR-0001 (Anchor as production framework), ADR-0002 (test stack),
Phase 1 toolchain pinning, Phase 12 (migrations, verifiable builds).

**Note on repository identity:** the canonical Anchor repo is now under `solana-foundation/anchor`,
maintained with OtterSec involvement. `coral-xyz/anchor` links are historical.

---

## 2. Runtime / validator — Agave 4.2

Agave 4.2 was recommended for mainnet adoption in August 2026, with feature activations starting the
week of 2026-08-17. Relevant, feature-gated changes:

| SIMD | Change | Relevance to Aegis |
|---|---|---|
| SIMD-0437 | **~90% rent reduction**: `lamports_per_byte` 6960 → 696, phased across five feature gates | Account-size cost pressure drops sharply. This *weakens* the argument for aggressive account-size micro-optimization and *strengthens* the argument for explicit, readable state. Recorded in the performance strategy. |
| SIMD-0296 | **Max transaction size 1232 → 4096 bytes** via a new v1 transaction format; opt-in, older versions still valid | Relieves pressure on account counts per transaction, but Aegis must NOT assume it — the zero-cost local path and older clients may not have it. Design to the 1232-byte budget; treat 4096 as headroom. |
| SIMD-0525 | Slot time 400ms → 200ms in four 50ms steps, gated on skip rates | Halves the real-time meaning of any slot-based staleness window. **Aegis must express all staleness in seconds (unix timestamp), never in slots.** |

Alpenglow (TowerBFT → Votor, ~150ms finality) is *not* activated in 4.2; targeted for Agave 4.3
around October 2026. Aegis makes no assumption about finality timing.

Sources:
- <https://solana.com/upgrades/agave-4-2-release-overview>
- <https://solana.com/news/solana-changelog-august-6-2026>
- <https://solana.com/news/solana-changelog-agave-v4-1-0-rpc-2-0-and-alpenglow>

**Decisions affected:** ADR-0009 (fixed-point + no slot-based time), oracle staleness design,
performance strategy, Phase 11 acceptance criteria.

---

## 3. Solana CLI / crate versions

Anchor 1.0.2's installation documentation shows verified toolchains including Agave CLI **3.1.10**
and Rust **1.85.0**. Note the two independent version lines that are easy to confuse:

- **Agave validator client** releases: 4.1, 4.2, 4.3-alpha (network software).
- **Solana CLI / SDK crate** line: 3.x (developer tooling and `solana-*` crates).

The local machine's `solana-cli 2.2.21` predates both and **must be upgraded in Phase 1**.

`UNVERIFIED`: the exact `solana-*` crate versions that `anchor-lang 1.1.2` resolves to were not read
from a lockfile during Phase 0. Phase 1 must record the resolved versions from `Cargo.lock`.

Sources: <https://www.anchor-lang.com/docs/installation>, <https://github.com/anza-xyz/solana-sdk>

---

## 4. Oracle — Pyth

- Current crate: **`pyth-solana-receiver-sdk` v2.0.0 (2026-06-15)**, which depends on
  `anchor-lang ^1.0.2`. **This resolves the main compatibility risk**: Pyth's Rust SDK supports the
  Anchor 1.x line. (The narrative documentation still lists compatibility only up to Anchor v0.31.1
  and is stale relative to the crate.)
- A legacy crate `pyth-solana-receiver-sdk-legacy` (0.3.3) exists for pre-1.x consumers. **Do not use it.**
- **Pyth Core was upgraded on 2026-08-26**; new integrations must target the upgraded Solana
  contracts. Existing integrations were auto-upgraded by the DAO on 2026-08-18.
- The legacy `hermes.pyth.network` endpoint now requires an API key. The recommended endpoint is
  `pyth.dourolabs.app/hermes`. **This is an off-chain price-fetching concern only** and therefore
  sits entirely outside the zero-cost core path.
- Pull model: price updates are posted as `PriceUpdateV2` accounts. Consuming them is an **account
  read, not a CPI** — critical for the zero-cost design (see §6).
- Official best practices confirmed and adopted verbatim into Aegis's oracle design:
  - Fixed-point: value = `price × 10^expo`; confidence shares the exponent.
  - **Value collateral at the lower bound `μ − σ`; value liabilities at the upper bound `μ + σ`.**
  - **When `σ/μ` exceeds a protocol threshold, pause activity dependent on that price.**
  - Use `get_price_no_older_than()`; "highly latency-sensitive protocols may wish to reduce the
    threshold to a few seconds."
  - Named risks for lending/liquidation: same-block exploitation, staleness selection, wide
    confidence intervals, liquidity impact, availability gaps.

Sources:
- <https://docs.rs/crate/pyth-solana-receiver-sdk/latest>
- <https://docs.pyth.network/price-feeds/core/best-practices>
- <https://docs.pyth.network/price-feeds/core/use-real-time-data/pull-integration/solana>

**`UNVERIFIED` / Phase 5 gate:** the exact upgraded Pyth receiver **program ID** post-2026-08-26, and
whether `PriceUpdateV2` remains the account type name in the upgraded contracts. Phase 5 must verify
against `docs.pyth.network` before writing the adapter.

**Decisions affected:** ADR-0008 (oracle abstraction & deterministic local prices), oracle design doc,
Phase 5.

---

## 5. Testing stack

| Tool | Version at research date | Role |
|---|---|---|
| **LiteSVM** | 0.16.0 (2026-08-24) | Primary integration harness. In-process SVM, ~10× faster per test than a validator. Default Anchor test template. Supports arbitrary account injection. |
| **Surfpool** | 1.5.0 (July 2026) | Drop-in `solana-test-validator` replacement; Anchor 1.0's default for `anchor test`/`anchor localnet`. Full JSON-RPC surface. Can fetch mainnet accounts just-in-time. |
| **Mollusk** | `UNVERIFIED` version | Isolated single-instruction execution and CU measurement. |
| `solana-test-validator` | legacy | Superseded by Surfpool for our purposes. |

The community-standard pyramid — LiteSVM for fast unit/integration, Mollusk for isolated instruction
and CU checks, Surfpool where a real RPC endpoint / realistic state / full JSON-RPC surface is needed —
matches the Aegis test architecture.

**Zero-cost caveat:** Surfpool's headline feature (JIT mainnet account fetching) requires an RPC
endpoint. Aegis therefore uses Surfpool in **pure local mode** for all required tests, and mainnet-fork
mode only in an explicitly optional, network-tagged test tier.

Sources:
- <https://crates.io/api/v1/crates/litesvm>
- <https://solana.com/docs/tools/surfpool>
- <https://solana.com/docs/intro/installation/surfpool-cli-basics>
- <https://github.com/otter-sec/anchor/pull/4106>

**Decisions affected:** ADR-0002, testing strategy, ADR-0010 (zero-cost architecture).

---

## 6. Native Rust and Pinocchio

**Pinocchio has crossed into production maturity.** The decisive evidence: Anza rewrote the SPL Token
program in Pinocchio (`p-token`), reducing a token transfer from ~4,645 CU to ~76 CU, and it went live
on mainnet in spring 2026. Pinocchio is `no_std`, zero-dependency, avoids copying accounts into owned
memory, and can drop the heap allocator.

This finding cuts **both ways** for Aegis and the distinction matters:

- It proves Pinocchio is a legitimate, current, non-toy skill — so a Pinocchio artifact is worth
  building and is not resume theatre.
- It does **not** imply Aegis's production program should be written in Pinocchio. Aegis is
  security-first and account-validation-heavy; Anchor's declarative constraints eliminate whole
  vulnerability classes (and 1.0 now blocks duplicate mutable accounts by default). Hand-rolling that
  validation in `no_std` for a lending protocol trades a large security budget for CU savings that are
  not the binding constraint.

**Decision:** Anchor for production; Pinocchio and native `solana-program` appear in a scoped,
benchmarked lab that reimplements the *actual* Aegis custody primitive three ways. See ADR-0003.

Sources:
- <https://github.com/anza-xyz/pinocchio>
- <https://docs.rs/pinocchio/latest/pinocchio/>
- <https://www.helius.dev/blog/pinocchio>
- <https://orbitflare.com/blog/fundamentals/p-token>

---

## 7. TypeScript client stack

- **`@solana/kit` v8.2.0** (published ~2026-08-31) is the current recommended SDK. `@solana/web3.js`
  is explicitly legacy; new applications should use Kit.
- Kit is modular and tree-shakeable: `@solana/accounts`, `@solana/codecs`, `@solana/errors`, etc.
- `gill` and `kite` are third-party ergonomic wrappers over Kit. Aegis uses **Kit directly** in the
  SDK to keep the dependency surface minimal and the demonstrated knowledge first-party; wrappers may
  be used in the app layer if they earn their place.
- Anchor 1.x's TS package is **`@anchor-lang/core`**.

Sources:
- <https://www.npmjs.com/package/@solana/kit>
- <https://solana.com/docs/clients/official/javascript>
- <https://github.com/anza-xyz/kit>

**Decisions affected:** ADR-0011 (client stack), Phase 9.

---

## 8. Token programs

- SPL Token (legacy) — program ID unchanged; implementation now `p-token` (Pinocchio) on mainnet.
  Interface and program ID are unchanged, so this is transparent to Aegis.
- **Token-2022 / Token Extensions** — live on mainnet since January 2024, 20+ optional extensions.
- Anchor 1.0 added token-extensions support; `anchor-spl`'s `token_interface` types are the correct
  way to accept either token program.
- Security guidance adopted from Neodyme's extension analysis (see `docs/token-compatibility.md`) —
  in particular that transfer fees are deducted from the *recipient's* amount (so escrow accounting
  must use measured balance deltas), that a permanent delegate can drain a vault outright, and that a
  mint with a close authority can be closed and reinitialized with different extensions at the same
  address.

Sources:
- <https://neodyme.io/en/blog/token-2022/>
- <https://chainstack.com/solana-token-2022-fee-transfer-hooks/>
- <https://www.quillaudits.com/research/rwa-development/non-evm-standards/solana-token-2022>

---

## 9. Security guidance

Consolidated current guidance, all of which is reflected in `docs/threat-model.md`:

- **Authentication failures are the leading cause of Solana exploits.** The canonical case remains
  Wormhole ($320M), which checked a pubkey without verifying `is_signer`.
- Core recurring classes: missing signer checks; unvalidated account ownership; unsafe CPI patterns
  that forward user signers to untrusted programs; integer overflow (release builds do not check by
  default); PDA seed collisions across users or functions.
- **Transfer hooks reintroduce control-flow risk** that the Solana account model otherwise avoids;
  extra accounts arrive via `ExtraAccountMetaList` and lax seed validation lets an attacker inject
  accounts.
- Current performance/security guidance explicitly recommends moving away from a single global-state
  PDA toward sharded, seed-derived PDAs to minimize write-lock contention. **Aegis's isolated-market
  architecture is the direct expression of this guidance.**

Sources:
- <https://www.helius.dev/blog/a-hitchhikers-guide-to-solana-program-security>
- <https://www.zealynx.io/research/smart-contracts/solana-security-checklist>
- <https://www.zealynx.io/blogs/solana-2026-security>

---

## 10. Known-unstable / deprecated — do not use

| Item | Status |
|---|---|
| `@coral-xyz/anchor` | Renamed. Use `@anchor-lang/core`. |
| `@solana/web3.js` (v1 style) | Legacy. Use `@solana/kit`. |
| `solana-test-validator` | Superseded by Surfpool for Anchor 1.x workflows. |
| `apr.dev` IDL registry | Defunct. Verified builds via `verify.osec.io`. |
| Anchor `#[interface]` / `interface-instructions` | Removed in 1.0. |
| `CLOSED_ACCOUNT_DISCRIMINATOR` | Removed in 1.0. |
| `pyth-solana-receiver-sdk-legacy` | Legacy shim. Use `pyth-solana-receiver-sdk` 2.x. |
| `hermes.pyth.network` (keyless) | Now requires an API key; use `pyth.dourolabs.app/hermes`. Off-chain only. |
| Anchor `[registry]` in `Anchor.toml`, `anchor login` | Removed in 1.0. |
| Direct `solana-program` dependency in an Anchor program | Discouraged; causes crate-version conflicts. |
| Slot-based staleness windows | Unsafe under SIMD-0525 variable slot times. Use unix seconds. |

---

## 11. Phase 1 re-verification commands (mandatory)

Phase 1 must run these, paste real output into `docs/project-status.md`, and update this file if
anything has moved:

```bash
rustc --version && cargo --version
solana --version
avm --version && anchor --version
surfpool --version
cargo search litesvm --limit 1
cargo search mollusk-svm --limit 1
cargo search pyth-solana-receiver-sdk --limit 1
npm view @solana/kit version
npm view @anchor-lang/core version
```

Anything that contradicts this document is a **research finding**, not an inconvenience: update this
file, note the delta in `docs/project-status.md`, and open an ADR if a decision changes.

---

## 12. Phase 1 re-verification (2026-09-05/06) — RV-1 and RV-2 resolved

Every command below was actually run on the implementation machine (macOS/aarch64); raw output is
also pasted into `docs/project-status.md`. Nothing here is copied from a tutorial.

### 13.1 Installed toolchain (before → after)

| Tool | §0 recorded (2026-09-04) | Actually installed (2026-09-05/06) |
|---|---|---|
| `rustc` / `cargo` | 1.88.0 | **1.98.1** (upgraded — see §13.4) |
| `solana` (Agave CLI) | 2.2.21 | **4.2.2** installed via the stable installer; **`anchor build` itself then switched the active release to 3.1.10** (Anchor 1.0.2's own documented verified toolchain) — see §13.4 |
| `avm` | not installed | **1.1.2** |
| `anchor` | not installed | **1.2.0** (newer than the 1.1.2 this document assumed — see §13.2) |
| `surfpool` | not installed | **1.5.0** — matches §5 exactly |
| `node` | v22.12.0 | v22.12.0 (unchanged) |

### 13.2 Anchor / crates.io deltas from §1 and §5

crates.io's own metadata (`max_stable_version` / `default_version`, fetched directly, not assumed)
as of 2026-09-05:

| Crate | This doc (§1/§5) said | Actually resolves to |
|---|---|---|
| `anchor-lang` | 1.1.2 | **1.2.0** (max_stable_version; a `2.0.0-rc.1` pre-release also exists — not used) |
| `anchor-spl` | (implied 1.1.2) | **1.2.0** |
| `litesvm` | 0.16.0 | **0.16.0** — confirmed exact |
| `mollusk-svm` | `UNVERIFIED` | **0.15.1** — RV-2 resolved: the crate is named `mollusk-svm` (not `mollusk`), current stable 0.15.1, repo `github.com/anza-xyz/mollusk` |
| `pyth-solana-receiver-sdk` | 2.0.0 | **2.0.0** — confirmed exact |
| `@solana/kit` | 8.2.0 | **8.2.0** — confirmed exact |
| `@anchor-lang/core` | (unspecified) | **1.2.0** |

None of these deltas are architecturally significant — Anchor is still the 1.x line with the same
breaking changes already recorded in §1 (dup-by-default, `idl-build` required, `@anchor-lang/core`,
etc.). This is a version-number update, not a finding that invalidates a Phase 0 decision.

One repository-identity delta worth recording: `anchor-lang`'s crates.io metadata now lists
`repository = "https://github.com/otter-sec/anchor"` (not `solana-foundation/anchor`). The install
command in `docs/phases/phase-01-foundation.md` (`cargo install --git
https://github.com/solana-foundation/anchor avm --force`) **still works** — `solana-foundation/anchor`
resolved correctly during installation — so no change was needed, but a future phase should not be
surprised if the canonical URL moves again.

### 13.3 RV-1 resolved — `solana-*` crate versions under `anchor-lang 1.2.0`

Read directly from this repository's own `Cargo.lock` after `anchor build` + `cargo test --workspace`
resolved the full dependency graph (see `docs/project-status.md` for the extraction command and full
list). The headline finding: **the dependency graph genuinely contains two coexisting major-version
lines for the same logical types**, mid-rename:

- `solana-pubkey` resolves to **both 3.0.0 and 4.2.1** simultaneously. In 4.2.1, `solana-pubkey` is a
  thin compatibility shim: `pub use solana_address::Address as Pubkey;` — i.e. `Pubkey` is now
  literally a type alias for `solana_address::Address`, not a distinct type.
- `solana-address` resolves to **both 1.1.0 and 2.6.1**.
- `solana-transaction` resolves to **4.1.6**; the feature that gates `VersionedTransaction::try_new`
  was renamed from `bincode` (used in older 3.x-line examples, including what `anchor init`'s own
  generated `programs/*/Cargo.toml` dev-dependency comment implies) to **`wincode`** in the 4.x line.
- `solana-message` resolves to **4.4.1** in the workspace's resolution (litesvm 0.16.0 itself declares
  `solana-message = "4.2.4"`).
- `solana-keypair` (**3.1.2**) and `solana-signer` (**3.0.1**) stayed on the 3.x line litesvm expects.

**Practical implication, recorded so the next phase does not rediscover it the hard way:** any crate
that talks to LiteSVM 0.16.0's public API must depend on the *same major line LiteSVM itself declares*
for `solana-message`, `solana-transaction`, `solana-pubkey`/`solana-address`, `solana-keypair`, and
`solana-signer` — read from litesvm's own `Cargo.toml` in the local registry cache, not assumed from
an example. Depending on a different major version of the same crate produces two structurally
identical but nominally distinct Rust types, and the compiler error only says "no associated function"
or "trait not implemented," not "your dependency versions are misaligned." `crates/aegis-test-kit`'s
and the workspace root's `Cargo.toml` pin exactly the versions verified this way (see their inline
comments).

### 13.4 Two additional real findings, not previously recorded

1. **`avm` (and therefore `anchor-cli`, built from source) requires `rustc >= 1.91`.** The
   `rustc 1.88.0` this document called "Adequate" in §0 does **not** satisfy this — `cargo install
   --git ... avm` fails outright with `cargo-platform@0.3.3 requires rustc 1.91`. Phase 1 upgraded the
   host toolchain via `rustup update stable` to **1.98.1**, which is what this repository's
   `rust-toolchain.toml` now pins.
2. **The workspace's declared `rust-version` must stay below whatever rustc ships inside Solana's
   bundled platform-tools**, not below the host rustc. `cargo-build-sbf` cross-compiles on-chain code
   with a separate, older rustc bundled in platform-tools (`1.95.0-dev` at the time of writing);
   declaring `rust-version = "1.98.1"` (the *host* compiler) in `[workspace.package]` makes `anchor
   build` fail with `rustc 1.95.0-dev is not supported ... requires rustc 1.98.1`, even though nothing
   is actually wrong with the code. This repository's workspace `Cargo.toml` deliberately pins
   `rust-version = "1.85.0"` — comfortably under the bundled compiler — with a comment explaining why.
   This is a toolchain-plumbing fact, not an architectural one, but it is exactly the kind of thing
   "trust the installed CLI's own output over any blog post" (§0) is warning about.

### 13.5 Real target triple used for the on-chain build

`anchor build` (Anchor 1.2.0, Solana CLI/platform-tools resolved to 3.1.10) compiles the program for
target `sbpfv3-solana-solana`, not the historically-remembered `bpfel-unknown-unknown`. Recorded here
because it is exactly the kind of detail a memorized pattern gets wrong silently.

---

## 13. Open verification items carried into later phases

| ID | Question | Gate | Status |
|---|---|---|---|
| RV-1 | Resolved `solana-*` crate versions under `anchor-lang 1.1.2` | Phase 1 | ✅ RESOLVED (§12.3) |
| RV-2 | Current Mollusk crate name/version and CU-measurement API | Phase 1 | ✅ RESOLVED (§12.2) |
| RV-3 | Upgraded Pyth receiver program ID and whether `PriceUpdateV2` is still the account type | Phase 5 | ✅ **RESOLVED** — see §15.1: address unchanged at `rec5EKMGg6MxZYaMdyBfgwp4d5rB9T1VQH5pJv5LtFJ`; `PriceUpdateV2` unchanged |
| RV-4 | Exact `VerificationLevel` enum shape in `pyth-solana-receiver-sdk` 2.x | Phase 5 | ✅ **RESOLVED** — see §15.2: `enum VerificationLevel { Partial { num_signatures: u8 }, Full }` |
| RV-5 | Complete current Token-2022 extension list, including any added after Jan 2024 (e.g. `Pausable`, `ScaledUiAmount`) and their discriminants | Phase 7 | OPEN |
| RV-6 | **Whether the Solana runtime permits `A → B → A` CPI reentrancy** (non-self-recursive). Aegis must not depend on the answer, but Phase 8's callback design must state it correctly. | Phase 8 | OPEN |
| RV-7 | Whether SIMD-0296 (4096-byte transactions) is active on the target cluster and supported by `@solana/kit` | Phase 9 | OPEN |
| RV-8 | Current Jupiter API/program surface for liquidation routing | Phase 8 | OPEN |

---

## 14. Phase 2 re-verification (2026-09-06) — `anchor-spl` / Token-2022 API surface

Every finding below was hit directly during implementation (compile errors, `anchor build` SBF
warnings, or direct inspection of the fetched crate sources under
`~/.cargo/registry/src/`), not assumed from prior knowledge.

1. **`anchor-spl` 1.2.0's Token-2022 support depends on `spl-token-2022-interface` 2.1.0, not a
   crate literally named `spl-token-2022`.** Its legacy counterpart is `spl-token-interface`
   2.0.0. `anchor_spl::token_2022::spl_token_2022` and `anchor_spl::token::spl_token` are aliasing
   re-exports of these interface crates, so code written against the documented `anchor_spl`
   module paths is unaffected — only a direct Cargo dependency on the underlying crate needs the
   `-interface` suffixed name.
2. **`anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface}` plus
   `find_mint_account_size`/`get_mint_extension_data`** are exactly the types `create_market`
   needs: `InterfaceAccount<'info, Mint>` derefs to `spl_token_2022::state::Mint` and validates
   the account owner is one of the two token programs (not the *specific* one pinned per market —
   callers must still check that explicitly, per `token-compatibility.md` §5.1's own warning).
3. **`ExtensionType::get_required_init_account_extensions(&mint_extension_types) ->
   Vec<ExtensionType>`** (in `spl-token-2022-interface`) is the correct, current API for computing
   which *account*-level extensions a vault needs from a mint's *mint*-level extensions (e.g.
   `TransferFeeConfig` → `TransferFeeAmount`) — this is exactly the "appropriate equivalent" the
   phase spec asked to find for `ExtensionType::try_calculate_account_len`. Anchor's own `#[account(init,
   token::mint = ...)]` codegen (`anchor-syn`'s `generate_get_token_account_space`) uses precisely
   this function, confirming it against Anchor's own reference implementation rather than only this
   repository's usage of it.
4. **`anchor_lang::context::CpiContext::new` takes `program_id: Pubkey`, not an `AccountInfo`.**
   The invoked program's `AccountInfo` does not need to appear in the CPI's account list at all —
   the Solana runtime resolves the callee from the *transaction's* full account list, not from the
   `invoke`/`invoke_signed` call's own slice. Every CPI helper in `anchor_spl::token_2022`
   confirms this (none of them include the token-program account in their internal
   `invoke`/`invoke_signed` calls).
5. **LiteSVM 0.16.0 embeds real SPL program bytecode** (`spl_token-3.5.0.so`,
   `spl_token_2022-11.0.0.so`, `spl_memo-1.0.0.so`/`4.0.0.so`,
   `spl_associated_token_account-1.1.1.so`, `address_lookup_table.so`, a Pinocchio token program
   used when a specific feature-gate is active) inside its own crate, loaded automatically by
   `LiteSVM::new()` (via `.with_default_programs()`). This means test fixtures can create real SPL
   Token and Token-2022 mints — including with real extensions — via ordinary CPI-building
   instructions and `send_transaction`, with zero network access and zero hand-rolled account
   bytes, except for the one deliberately synthetic fixture that simulates an extension type this
   repository's dependency does not define at all (`aegis-test-kit::create_token_2022_mint_with_unrecognized_extension`).
6. **`#[error_code]`'s discriminant handling** (`anchor-syn` 1.2.0's `parser/error.rs`): explicit
   integer discriminants (`Variant = N`) on an `AegisError` enum variant are fully supported and
   are exactly what banded error codes (`architecture.md` §8) need — `anchor-attribute-error`
   preserves the enum's own Rust discriminants verbatim (`#[repr(u32)] #error_enum`) and computes
   the final on-chain code as `variant as u32 + ERROR_CODE_OFFSET` (6000 by default). A variant
   without an explicit discriminant continues from the previous one, so only the first variant in
   each band needs its discriminant spelled out.

None of these are architectural findings — every Phase 0 decision they touch (Anchor as the
production framework, `anchor_spl::token_interface` for dual-token-program support, LiteSVM as the
Tier 3 harness) still holds exactly as ADR-0001/0002 state it. They are the version-plumbing and
current-API facts a from-scratch Phase 2 session needs and that this document's Phase 1 research
could not yet have (Token-2022 policy code did not exist until this phase).

---

## 15. Phase 5 re-verification (2026-09-06) — RV-3 and RV-4 resolved

Both gates were closed **before any oracle code was written**, per this document's own §0
verification protocol and `docs/phases/phase-05-oracle.md`'s explicit research gate. Every finding
below traces to a primary source actually fetched during this phase — crates.io's own registry
API, the real `pyth-solana-receiver-sdk` 2.0.0 crate source downloaded from static.crates.io, and
`docs.pyth.network` directly — never memory or a tutorial.

### 15.1 RV-3 — upgraded Pyth receiver program ID and account type

**Resolved: the receiver program address did NOT change across the 2026-08-26 Pyth Core
upgrade.**

- `docs.pyth.network/price-feeds/core/contract-addresses/solana` (fetched directly, 2026-09-06)
  states verbatim: *"The Solana receiver program is deployed at
  `rec5EKMGg6MxZYaMdyBfgwp4d5rB9T1VQH5pJv5LtFJ` across all supported networks"* and *"Pyth Core
  upgrade completed successfully on August 26, 2026 ... Existing integrations using the current
  addresses were automatically upgraded by the DAO on August 26, 2026"* — new integrations are
  told to "use the upgraded Solana contracts," but **the receiver program address itself remained
  consistent across the upgrade**.
- Independently confirmed by reading the actual crate source: `pyth-solana-receiver-sdk` 2.0.0's
  `src/lib.rs` (`declare_id!("rec5EKMGg6MxZYaMdyBfgwp4d5rB9T1VQH5pJv5LtFJ")`, the default —
  non-`pro-compatible` — build) matches the docs.pyth.network address exactly. A second,
  feature-gated `pro-compatible` ID (`rec2HHDDnjLfj4kE7VyEtFA1HPGQLK33259532cRyHp`) also exists in
  the crate but is **not** used here — Aegis depends on the crate with default features, so
  `pyth_solana_receiver_sdk::ID` resolves to the address above.
- **Ownership semantics**: `PriceUpdateV2` is an Anchor `#[account]` struct defined inside this
  crate, so its `Owner` impl (generated by the `#[account]` macro) returns `crate::ID` — i.e. the
  same receiver program ID. `oracle::pyth::PythPull::read_price` asserts this explicitly via
  `require_keys_eq!(*account.owner, pyth_solana_receiver_sdk::ID, ...)` (O-1), in addition to
  whatever an `Account<'info, T>` wrapper would have checked automatically, per the phase spec's
  explicit-owner-validation requirement.
- **Account type**: `PriceUpdateV2` is unchanged — same crate, same version, same struct layout
  (`write_authority: Pubkey`, `verification_level: VerificationLevel`,
  `price_message: PriceFeedMessage`, `posted_slot: u64`).
- **Exact crate version**: `pyth-solana-receiver-sdk` **2.0.0** (published 2026-06-15T20:22:15Z) —
  confirmed as both `max_stable_version` and `newest_version` via crates.io's own registry API
  (`GET /api/v1/crates/pyth-solana-receiver-sdk`, fetched 2026-09-06; no newer version has been
  published since, including after the 2026-08-26 Core upgrade — the upgrade did not require an
  SDK bump). Matches this document's §4 finding exactly; nothing had gone stale.
- **`anchor-lang` compatibility**: `pyth-solana-receiver-sdk` 2.0.0's own `Cargo.toml` declares
  `anchor-lang = "1.0.2"` (a `^1.0.2` caret requirement), satisfied by this workspace's
  `anchor-lang = "1.2.0"` (resolved and building cleanly — verified by `cargo build`/`anchor
  build` actually succeeding with both crates in the dependency graph).

Sources (fetched 2026-09-06):
- <https://docs.pyth.network/price-feeds/core/contract-addresses/solana>
- <https://crates.io/api/v1/crates/pyth-solana-receiver-sdk> (registry API, `max_stable_version`/`newest_version`/`updated_at`)
- <https://crates.io/api/v1/crates/pyth-solana-receiver-sdk/2.0.0/dependencies>
- `pyth-solana-receiver-sdk-2.0.0.crate` (downloaded from static.crates.io and extracted directly)
  — `src/lib.rs`, `src/program.rs`, `src/price_update.rs`, `Cargo.toml`

### 15.2 RV-4 — `VerificationLevel` shape in `pyth-solana-receiver-sdk` 2.x

**Resolved directly from the crate's own source** (`src/price_update.rs`):

```rust
pub enum VerificationLevel {
    Partial {
        #[allow(unused)]
        num_signatures: u8,
    },
    Full,
}
```

with a `gte` method (`Full` is always `>=` everything; `Partial{n}` is `>=` `Partial{m}` iff `n >=
m`) used internally by `PriceUpdateV2::get_price_no_older_than`, which **hardcodes a `Full`
requirement**:

```rust
pub fn get_price_no_older_than(&self, clock: &Clock, maximum_age: u64, feed_id: &FeedId)
    -> Result<Price, GetPriceError> {
    self.get_price_no_older_than_with_custom_verification_level(
        clock, maximum_age, feed_id, VerificationLevel::Full,
    )
}
```

This is unchanged in shape from what this document's §4 already recorded as the SDK's documented
behavior — no enum-shape drift across versions was found. `oracle::pyth::PythPull::read_price` (1)
calls `get_price_no_older_than` (enforcing O-3/O-4/O-5 together, mapping its `GetPriceError`
variants to specific `AegisError`s) **and** (2) separately, explicitly asserts
`price_update.verification_level == VerificationLevel::Full` before that call — the phase spec's
"still assert it explicitly ... easy to omit and rarely tested" requirement, so the check survives
even if a future SDK version changed `get_price_no_older_than`'s internal default.

Source: `pyth-solana-receiver-sdk-2.0.0/src/price_update.rs` (downloaded and read directly, as
above) — the same file both findings come from.

### 15.3 Contradiction check

Neither finding invalidates any Phase 0 architectural decision. ADR-0008's architecture (oracle
abstraction, one real Pyth implementer, deterministic fixture injection, no mock program, account
read not CPI) holds exactly as written — the receiver program ID is unchanged, `PriceUpdateV2` is
unchanged, and `VerificationLevel` is unchanged. Only the exact address/shape needed confirming
against a primary source before writing code that depends on them; both are now pinned with
evidence above. No ADR was required.

### 15.4 Environment at Phase 5 (2026-09-06)

Unchanged from Phase 1/2 (`rustc`/`cargo` 1.98.1, Agave CLI 3.1.10, `anchor-cli` 1.2.0, `avm`
1.1.2) — re-verified by `anchor build` and `cargo test --workspace` succeeding. No toolchain delta
this phase.
