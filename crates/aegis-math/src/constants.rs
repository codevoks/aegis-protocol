/// WAD fixed-point scale used for every fraction, rate, price and value in Aegis.
pub const WAD: u128 = 1_000_000_000_000_000_000;

/// Virtual shares added to the share/asset ratio to defend the first-depositor share price
/// against inflation manipulation.
pub const VIRTUAL_SHARES: u128 = 1_000_000;

/// Virtual assets added alongside `VIRTUAL_SHARES`.
pub const VIRTUAL_ASSETS: u128 = 1;

/// Length of a fixed, non-leap 365-day year in seconds.
pub const SECONDS_PER_YEAR: u128 = 31_536_000;
