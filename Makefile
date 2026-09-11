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

## anchor build: compiles the program and generates its IDL.
build:
	anchor build

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
demo: build
	cargo run -p aegis-test-kit --example phase7_demo

## UI against local Surfpool — Phase 9.
app:
	@echo "not implemented until Phase 9 (SDK, client & UI)"
