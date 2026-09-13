.PHONY: setup build test fmt lint clean bench fuzz traceability demo app \
        sdk-install app-install codegen codegen-check vectors vectors-check \
        sdk-test sdk-test-e2e app-build app-typecheck

## Verify and print the exact toolchain versions this repository was built against.
setup:
	@echo "--- rustc / cargo ---"
	@rustc --version
	@cargo --version
	@echo "--- solana (Agave CLI) ---"
	@solana --version
	@echo "--- avm / anchor ---"
	@avm --version
	@anchor --version
	@echo "--- surfpool ---"
	@surfpool --version
	@echo "--- node ---"
	@node --version

## anchor build: compiles the aegis program and generates its IDL. Anchor CLI's own notion of
## "the workspace" only discovers programs under programs/ (verified directly: `anchor build -p
## example_liquidator` fails with "is not part of the workspace" even though the crate is a normal
## Cargo workspace member and is listed in Anchor.toml's [programs.localnet]) -- so the two Phase 8
## labs/ programs are built with `cargo build-sbf` directly, which is the same underlying command
## `anchor build` itself shells out to per program.
##
## NOTE (found during Phase 8, pre-existing, unrelated to this phase's own code): this local
## checkout's target/deploy/aegis-keypair.json does not match `declare_id!`'s
## 2GtoBADM175vkjf5UYpbD198Ry1cJadXMGo8sCQvXndh (target/ is gitignored, so this keypair was never
## committed and its local history is unknown). This has no effect on `make test` (LiteSVM loads
## the compiled .so's bytes directly, independent of the local deploy keypair) but WOULD block a
## real `anchor deploy`. Not fixed here: resolving it means either regenerating and re-pinning the
## program's real identity or running `anchor keys sync`, both of which are identity decisions for
## the maintainer, not a Phase 8 code change. `anchor build` is left plain below so this warning
## stays visible; run `anchor build --ignore-keys` manually if you need to build past it locally.
build:
	anchor build
	cargo build-sbf --manifest-path labs/example-liquidator/Cargo.toml
	cargo build-sbf --manifest-path labs/hostile-callback/Cargo.toml

## cargo test --workspace: offline, no secrets, no network. The one load-bearing
## command (docs/zero-cost-demo.md §4).
test:
	cargo test --workspace

fmt:
	cargo fmt --all

lint:
	cargo clippy --workspace --all-targets -- -D warnings

clean:
	cargo clean
	rm -rf .anchor

## Extended invariant fuzz campaign — Phase 10. Larger and slower than the bounded campaign
## `make test` already runs as part of `tests/fuzz.rs`'s default (non-`#[ignore]`d) tests; this
## target is the manual/nightly-scale run docs/security/mutation-report.md's campaign-statistics
## section reports evidence from (testing-strategy.md §9: "Extended fuzz campaign | nightly /
## pre-tag | no (reported)"). Takes on the order of minutes, not seconds -- deliberately not part
## of the default `make test`/CI path.
fuzz:
	cargo test --test fuzz --offline -- --ignored fuzz_extended_campaign --nocapture

## Phase 10 traceability check, runnable on its own. Already blocking in CI without any workflow
## change: `.github/workflows/ci.yml`'s `guards` job runs every `scripts/check-*.sh`, and this
## script's filename matches that glob.
traceability:
	./scripts/check-traceability.sh

## CU benchmarks -> benchmarks/cu.json — Phase 11. Real instructions through LiteSVM, measured
## through Mollusk against the actual compiled aegis.so (docs/performance-strategy.md §5,
## benchmarks/README.md). Requires `anchor build` to have produced target/deploy/aegis.so first.
bench:
	AEGIS_BENCH_WRITE=1 cargo test --test bench --offline cu_benchmark_suite -- --nocapture

## Phase 11 CU regression gate: compares tests/bench's live measurements against the committed
## benchmarks/cu.json baseline. Fails on >10% regression on any benchmarked instruction.
bench-check:
	./scripts/check-cu-regression.sh

## Phase 11 contention verification (PERF-C1..C3): the static Rust suite asserting compiled
## account metadata (PERF-C1's disjoint-writable-set claim, PERF-C2's A-PAR-01 regression guard,
## PERF-C3's write-set enumeration), plus a real, local, offline Surfpool run that actually
## executes two markets' transactions concurrently via Promise.all (bots/liquidator/src/
## perfC1ConcurrentDemo.ts) -- the static suite proves the design permits parallelism; this run
## proves the validator actually exploits it. Requires `make build` first (target/deploy/aegis.so,
## target/idl/aegis.json).
bench-contention:
	cargo test --test bench --offline perf_c -- --nocapture
	cd bots/liquidator && npm run perf-c1

## Phase 11 labs: build and benchmark the vault-anchor/vault-native/vault-pinocchio custody
## primitives (docs/adr/0003-native-pinocchio-as-labs.md).
labs-build:
	anchor build -p vault-anchor --ignore-keys
	cargo build-sbf --manifest-path labs/vault-native/Cargo.toml
	cargo build-sbf --manifest-path labs/vault-pinocchio/Cargo.toml

labs-test:
	cargo test -p vault-anchor -p vault-native -p vault-pinocchio --offline

labs-bench:
	cargo test -p cu-bench --offline -- --nocapture

## Phase 7 demo: two side-by-side markets, one classic SPL collateral, one Token-2022
## transfer-fee (2%) collateral, run through an identical supply/deposit/borrow/accrue/liquidate/
## withdraw lifecycle, printing requested vs. credited (and the fee) at every collateral transfer
## so the reconciliation is visible; ends with the verified extension-policy rejection table
## (real create_market attempts against TransferHook/PermanentDelegate/MintCloseAuthority/
## DefaultAccountState=Frozen/Pausable/NonTransferable/an unrecognized discriminant/a transfer-fee
## loan asset). See docs/phases/phase-07-token2022.md "Demo". Zero-cost, offline, in-process
## LiteSVM. Earlier phase demos remain runnable directly:
## `cargo run -p aegis-test-kit --example phase2_demo`
## `cargo run -p aegis-test-kit --example phase3_demo`
## `cargo run -p aegis-test-kit --example phase4_demo`
## `cargo run -p aegis-test-kit --example phase5_demo`
## `cargo run -p aegis-test-kit --example phase6_demo`
## `cargo run -p aegis-test-kit --example phase7_demo`
##
## Phase 8 demo: an under-funded liquidator (zero loan-asset balance) liquidates via the
## example-liquidator callback -- seize, deterministic local swap, repay, all in one transaction --
## then, side by side, an ordinary pre-funded liquidator liquidates a second position with no
## callback at all, proving I-LIQ-CB-02. See docs/phases/phase-08-composability.md "Demo". The
## companion TypeScript keeper demo lives in bots/liquidator/ (see its own README) and is run
## separately, against a local, non-forking Surfpool validator.
demo: build
	cargo run -p aegis-test-kit --example phase8_demo

## --- Phase 9: SDK, client & UI ---

## Installs sdk/ts's npm dependencies.
sdk-install:
	cd sdk/ts && npm install

## Installs app/'s npm dependencies (which resolves @aegis/sdk via a local file: path to sdk/ts).
app-install:
	cd app && npm install

## Regenerates sdk/ts/src/generated/ from the current target/idl/aegis.json (run `make build`
## first). Deterministic: given an unchanged IDL, output is byte-identical.
codegen:
	cd sdk/ts && npm run codegen

## CI stale-generated-code guard (docs/phases/phase-09-sdk-ui.md item 3): regenerates into a temp
## dir and diffs against the committed sdk/ts/src/generated/. Never rewrites the committed files.
codegen-check:
	cd sdk/ts && npm run codegen:check

## Regenerates tests/vectors/*.json from the real aegis-math implementation (I-SDK-01).
vectors:
	cargo run -p aegis-test-kit --example phase9_vectors_dump -- tests/vectors

## CI stale-vector guard: regenerates into a temp dir and diffs against the committed
## tests/vectors/. Never rewrites the committed files on a pure check.
vectors-check:
	./scripts/check-vectors.sh

## SDK unit tests: cross-language math vectors (I-SDK-01), PDA parity (I-SDK-03), and transaction
## size (I-TX-01 / INV-RES-06). No network, no running validator required.
sdk-test:
	cd sdk/ts && npm test

## SDK end-to-end test (I-SDK-02): build -> sign -> send -> confirm -> decode for every
## instruction, against a real local Surfpool validator this test starts and deploys to itself.
## Requires `make build` first.
sdk-test-e2e:
	cd sdk/ts && npm run test:e2e

## Typechecks the Next.js app (does not require a running validator).
app-typecheck:
	cd app && npm run typecheck

## Production build of the Next.js app (does not require a running validator).
app-build:
	cd app && npm run build

## Local Surfpool + the aegis program deployed to it + the Next.js dev server, all in one command
## (docs/phases/phase-09-sdk-ui.md items 25-27, 46: "make app works on a clean clone"). Ctrl+C
## stops the dev server and the background validator together. Prerequisites: `make build`,
## `make sdk-install`, `make app-install`.
app:
	./scripts/run-app.sh
