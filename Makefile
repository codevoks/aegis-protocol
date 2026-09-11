.PHONY: setup build test fmt lint clean bench fuzz demo app

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

## Extended invariant fuzz campaign — Phase 10.
fuzz:
	@echo "not implemented until Phase 10 (security campaign)"

## CU benchmarks -> benchmarks/cu.json — Phase 11.
bench:
	@echo "not implemented until Phase 11 (performance)"

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

## UI against local Surfpool — Phase 9.
app:
	@echo "not implemented until Phase 9 (SDK, client & UI)"
