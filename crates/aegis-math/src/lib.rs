//! Aegis Protocol fixed-point arithmetic primitives.
//!
//! `no_std`, float-free, and free of any `solana-*` / `anchor-*` dependency (see
//! `docs/phases/phase-01-foundation.md` §6 and ADR-0009). Every economic multiply-divide
//! in the protocol goes through [`mul_div_floor`] / [`mul_div_ceil`], which compute the
//! product in a 256-bit intermediate so legal-but-large intermediate values cannot
//! overflow a `u128` even though the final result fits.
#![cfg_attr(not(test), no_std)]

pub mod constants;
pub mod fixed;
mod u256;

pub use constants::{SECONDS_PER_YEAR, VIRTUAL_ASSETS, VIRTUAL_SHARES, WAD};
pub use fixed::{mul_div_ceil, mul_div_floor, MathError};
