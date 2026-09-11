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

## Phase 6 demo: SOL crashes to $95.00, a position becomes liquidatable and is liquidated for the
## exact economic-model.md §7.5 figures (seizure, bonus, protocol cut); a second position is
## crashed to $40.00, its collateral fully seized by the clamp with debt remaining, and the
## resulting bad debt is absorbed with real protocol fee shares burned FIRST before the residual
## is socialized — a lender then withdraws and realizes the loss directly. See
## docs/phases/phase-06-liquidation.md "Demo". Zero-cost, offline, in-process LiteSVM; byte-exact
## PriceUpdateV2 fixtures via the real pyth-solana-receiver-sdk, no Hermes, no Pyth program deploy.
## Earlier phase demos remain runnable directly:
## `cargo run -p aegis-test-kit --example phase2_demo`
## `cargo run -p aegis-test-kit --example phase3_demo`
## `cargo run -p aegis-test-kit --example phase4_demo`
## `cargo run -p aegis-test-kit --example phase5_demo`
demo: build
	cargo run -p aegis-test-kit --example phase6_demo

## UI against local Surfpool — Phase 9.
app:
	@echo "not implemented until Phase 9 (SDK, client & UI)"
