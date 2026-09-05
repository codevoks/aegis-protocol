//! Local, offline LiteSVM test fixtures for Aegis Protocol.
//!
//! Phase 1 ships only the bootstrap needed by `tests/smoke.rs`: deploying the built
//! program into LiteSVM with a funded, deterministic payer. Mint, oracle, market and
//! invariant fixtures are added as later phases need them
//! (see `docs/zero-cost-demo.md` §6).

pub mod svm;

pub use svm::{deploy, deterministic_payer};
