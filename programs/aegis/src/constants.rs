//! Seeds and bounds shared across the Aegis program (architecture.md §2).
//!
//! `WAD` and the other numeric scale constants live in `aegis-math`, not here — this module
//! holds only the on-chain-specific constants: PDA seed prefixes, pause bitflags, `Market.flags`
//! bit layout, and the risk/oracle parameter bounds from `economic-model.md` §5 and
//! `instruction-catalogue.md` §6.

use aegis_math::WAD;

/// `Protocol` seed (account-model.md §3). Distinct from every other prefix (INV-LIFE-06).
pub const PROTOCOL_SEED: &[u8] = b"protocol";
/// `Market` seed (account-model.md §4).
pub const MARKET_SEED: &[u8] = b"market";
/// `Position` seed (account-model.md §5).
pub const POSITION_SEED: &[u8] = b"position";
/// Collateral vault seed (account-model.md §6).
pub const COLLATERAL_VAULT_SEED: &[u8] = b"cvault";
/// Loan vault seed (account-model.md §6).
pub const LOAN_VAULT_SEED: &[u8] = b"lvault";

/// Pause bitflags shared by `Protocol.paused` and `Market.paused` (account-model.md §3).
pub const PAUSE_SUPPLY: u8 = 0b0001;
pub const PAUSE_BORROW: u8 = 0b0010;
pub const PAUSE_WITHDRAW: u8 = 0b0100;
pub const PAUSE_LIQUIDATE: u8 = 0b1000;
/// Every bit any pause instruction may ever set (INV-ADM-03).
pub const PAUSE_ALL_BITS: u8 = PAUSE_SUPPLY | PAUSE_BORROW | PAUSE_WITHDRAW | PAUSE_LIQUIDATE;

/// `Market.flags` bit layout (account-model.md §4).
pub const FLAG_ACK_FREEZE_AUTHORITY: u8 = 0b0000_0001;
pub const FLAG_COLLATERAL_HAS_TRANSFER_FEE: u8 = 0b0000_0010;

/// Liquidation bonus upper bound (economic-model.md §5): `0 <= b <= 0.25 WAD`. The tighter,
/// *derived* bound `liq_threshold * (WAD + b) / WAD < WAD` (INV-LIQ-06) is checked separately
/// because it depends on `liq_threshold` and cannot be expressed as a constant.
pub const MAX_LIQ_BONUS: u128 = WAD / 4;
/// `close_factor` lower bound (economic-model.md §5): `0.05 WAD <= cf <= WAD`.
pub const MIN_CLOSE_FACTOR: u128 = WAD / 20;
/// `liq_protocol_fee` upper bound (economic-model.md §5): `0 <= f <= 0.5 WAD`.
pub const MAX_LIQ_PROTOCOL_FEE: u128 = WAD / 2;
/// Interest `fee` upper bound (economic-model.md §5): `0 <= fee <= 0.25 WAD`.
pub const MAX_FEE: u128 = WAD / 4;

/// Oracle staleness/confidence config bounds (instruction-catalogue.md §6 precondition 9).
pub const MIN_PRICE_AGE_SECS: u32 = 1;
pub const MAX_PRICE_AGE_SECS: u32 = 3600;
pub const MIN_CONF_BPS: u16 = 1;
pub const MAX_CONF_BPS: u16 = 2000;
