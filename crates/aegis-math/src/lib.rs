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
pub mod health;
pub mod irm;
pub mod shares;
mod u256;

pub use constants::{SECONDS_PER_YEAR, VIRTUAL_ASSETS, VIRTUAL_SHARES, WAD};
pub use fixed::{mul_div_ceil, mul_div_floor, MathError};
pub use health::{
    collateral_value, conservative_price_band, debt_value, health_factor, is_within_max_ltv,
    scale_to_wad_ceil, scale_to_wad_floor, HealthError, PriceBandWad, MAX_PRICE_WAD, MIN_PRICE_WAD,
};
pub use irm::{borrow_rate, taylor3, taylor_x, utilization};
pub use shares::{to_assets_down, to_assets_up, to_shares_down, to_shares_up};
