//! Conservative valuation and health-factor math (`docs/economic-model.md` §6,
//! `docs/oracle-design.md` §2-3). `no_std`, float-free, and free of any `solana-*`/`anchor-*`
//! dependency, like the rest of this crate (ADR-0009).
//!
//! The oracle *account* reading and identity/staleness/verification-level validation (O-1..O-6,
//! O-11) live in `programs/aegis/src/oracle`, which calls into this module for the pure numeric
//! checks (O-7..O-10: price positivity, confidence bound, sanity bounds, exponent scaling) and for
//! `collateral_value`/`debt_value`/`health_factor`. Splitting it this way keeps every numeric edge
//! case testable at Tier 1 (`AGENTS.md` §9: "do not push them into the SVM").

use crate::constants::WAD;
use crate::fixed::{mul_div_ceil, mul_div_floor, MathError};

/// O-9 sanity bound (`economic-model.md` §6.1): the smallest admissible confidence-adjusted WAD
/// price, `1e-12` USD per whole token. Anything smaller is rejected as an absurd oracle value
/// instead of being propagated into downstream valuation arithmetic.
pub const MIN_PRICE_WAD: u128 = 1_000_000;
/// O-9 sanity bound: the largest admissible confidence-adjusted WAD price, `1e12` USD per whole
/// token.
pub const MAX_PRICE_WAD: u128 = 1_000_000_000_000_000_000_000_000_000_000;

/// Errors from this module — a superset of [`MathError`] plus the oracle-specific numeric policy
/// checks (O-7, O-8, O-9/O-10) that turn an absurd or hostile oracle value into a typed error
/// instead of an arithmetic abort (`oracle-design.md` §2: "Return clean Aegis errors rather than
/// allowing arithmetic aborts").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthError {
    Overflow,
    DivisionByZero,
    /// O-7: `price <= 0`.
    PriceNotPositive,
    /// O-8: `conf > price * max_conf_bps / 10_000`.
    ConfidenceTooWide,
    /// O-9: the confidence-adjusted WAD price falls outside `[MIN_PRICE_WAD, MAX_PRICE_WAD]`.
    PriceOutOfBounds,
}

impl From<MathError> for HealthError {
    fn from(e: MathError) -> Self {
        match e {
            MathError::Overflow => HealthError::Overflow,
            MathError::DivisionByZero => HealthError::DivisionByZero,
        }
    }
}

/// A confidence-adjusted price band, already normalized to WAD (`oracle-design.md` §1): `lo`
/// values collateral (rounded down), `hi` values debt (rounded up).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PriceBandWad {
    pub lo: u128,
    pub hi: u128,
}

fn pow10(exp: u32) -> Result<u128, HealthError> {
    10u128.checked_pow(exp).ok_or(HealthError::Overflow)
}

/// Scales a raw Pyth integer to WAD given the feed's `expo`, rounding down
/// (`economic-model.md` §6.1: `raw * 10^(18+expo)` when `18+expo >= 0`, else `raw /
/// 10^(-(18+expo))`). Every exponent is `checked`/`try_from`-guarded, so an absurd `expo` returns
/// [`HealthError::Overflow`] instead of overflowing or panicking (O-10).
pub fn scale_to_wad_floor(raw: u128, expo: i32) -> Result<u128, HealthError> {
    let shift: i64 = 18i64 + expo as i64;
    if shift >= 0 {
        let factor = pow10(u32::try_from(shift).map_err(|_| HealthError::Overflow)?)?;
        raw.checked_mul(factor).ok_or(HealthError::Overflow)
    } else {
        let divisor = pow10(u32::try_from(-shift).map_err(|_| HealthError::Overflow)?)?;
        if divisor == 0 {
            return Err(HealthError::DivisionByZero);
        }
        Ok(raw / divisor)
    }
}

/// As [`scale_to_wad_floor`], rounding up.
pub fn scale_to_wad_ceil(raw: u128, expo: i32) -> Result<u128, HealthError> {
    let shift: i64 = 18i64 + expo as i64;
    if shift >= 0 {
        let factor = pow10(u32::try_from(shift).map_err(|_| HealthError::Overflow)?)?;
        raw.checked_mul(factor).ok_or(HealthError::Overflow)
    } else {
        let divisor = pow10(u32::try_from(-shift).map_err(|_| HealthError::Overflow)?)?;
        if divisor == 0 {
            return Err(HealthError::DivisionByZero);
        }
        let numerator = raw.checked_add(divisor - 1).ok_or(HealthError::Overflow)?;
        Ok(numerator / divisor)
    }
}

/// Computes the confidence-adjusted, WAD-normalized `(lo, hi)` band from a raw Pyth
/// `(price, conf, expo)` triple, enforcing O-7 (`price > 0`), O-8 (`conf <= price *
/// max_conf_bps / 10_000`) and O-9 (sanity bounds) — `economic-model.md` §6.1 exactly. No
/// floating point anywhere in this computation.
pub fn conservative_price_band(
    price: i64,
    conf: u64,
    expo: i32,
    max_conf_bps: u16,
) -> Result<PriceBandWad, HealthError> {
    // O-7
    if price <= 0 {
        return Err(HealthError::PriceNotPositive);
    }
    let price_u128 = price as u128;

    // O-8
    let max_conf = mul_div_floor(price_u128, max_conf_bps as u128, 10_000)?;
    if conf as u128 > max_conf {
        return Err(HealthError::ConfidenceTooWide);
    }

    // raw_lo saturates at 1 rather than 0 (economic-model.md §6.1: "saturating at 1") so a
    // maximal, still-admissible confidence interval can never produce a zero or negative raw
    // lower bound.
    let raw_lo = price_u128.saturating_sub(conf as u128).max(1);
    let raw_hi = price_u128
        .checked_add(conf as u128)
        .ok_or(HealthError::Overflow)?;

    let lo = scale_to_wad_floor(raw_lo, expo)?;
    let hi = scale_to_wad_ceil(raw_hi, expo)?;

    // O-9
    if lo < MIN_PRICE_WAD || hi > MAX_PRICE_WAD {
        return Err(HealthError::PriceOutOfBounds);
    }

    Ok(PriceBandWad { lo, hi })
}

/// `economic-model.md` §6.2: `collateral_value = floor(collateral_amount * price_lo /
/// 10^collateral_decimals)` — understates collateral (INV-ORA-03).
pub fn collateral_value(
    collateral_amount: u64,
    price_lo_wad: u128,
    collateral_decimals: u8,
) -> Result<u128, HealthError> {
    let scale = pow10(collateral_decimals as u32)?;
    Ok(mul_div_floor(
        collateral_amount as u128,
        price_lo_wad,
        scale,
    )?)
}

/// `economic-model.md` §6.2: `debt_value = ceil(debt_assets * price_hi / 10^loan_decimals)` —
/// overstates debt (INV-ORA-03).
pub fn debt_value(
    debt_assets: u64,
    price_hi_wad: u128,
    loan_decimals: u8,
) -> Result<u128, HealthError> {
    let scale = pow10(loan_decimals as u32)?;
    Ok(mul_div_ceil(debt_assets as u128, price_hi_wad, scale)?)
}

/// `economic-model.md` §6.3: `HF = floor(collateral_value * liq_threshold / debt_value)`, or
/// `u128::MAX` when `debt_value == 0` (no debt is always maximally healthy).
pub fn health_factor(
    collateral_value: u128,
    liq_threshold: u128,
    debt_value: u128,
) -> Result<u128, HealthError> {
    if debt_value == 0 {
        return Ok(u128::MAX);
    }
    Ok(mul_div_floor(collateral_value, liq_threshold, debt_value)?)
}

/// `economic-model.md` §6.3: the borrow / withdraw-collateral admissibility check, expressed
/// against `max_ltv` directly (`debt_value <= collateral_value * max_ltv / WAD`) rather than as a
/// second health factor, to avoid a second division and its rounding ambiguity.
pub fn is_within_max_ltv(
    collateral_value: u128,
    debt_value: u128,
    max_ltv: u128,
) -> Result<bool, HealthError> {
    let max_debt_value = mul_div_floor(collateral_value, max_ltv, WAD)?;
    Ok(debt_value <= max_debt_value)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- O-7/O-8/O-9 numeric policy checks ---

    #[test]
    fn zero_and_negative_price_are_rejected() {
        assert_eq!(
            conservative_price_band(0, 0, -8, 100),
            Err(HealthError::PriceNotPositive)
        );
        assert_eq!(
            conservative_price_band(-1, 0, -8, 100),
            Err(HealthError::PriceNotPositive)
        );
    }

    // Boundary: confidence exactly at max_conf_bps must pass; +1 must fail.
    #[test]
    fn confidence_boundary() {
        // price = 10_000, max_conf_bps = 100 (1%) -> max_conf = 100 exactly.
        assert!(conservative_price_band(10_000, 100, 0, 100).is_ok());
        assert_eq!(
            conservative_price_band(10_000, 101, 0, 100),
            Err(HealthError::ConfidenceTooWide)
        );
    }

    #[test]
    fn absurdly_small_and_large_prices_are_rejected() {
        // A tiny raw price with a very negative expo scales below MIN_PRICE_WAD.
        assert_eq!(
            conservative_price_band(1, 0, -30, 0),
            Err(HealthError::PriceOutOfBounds)
        );
        // A price that scales above MAX_PRICE_WAD without overflowing the u128 multiplication
        // itself (a case that overflows the multiplication, e.g. i64::MAX at expo=20, correctly
        // returns Overflow instead -- both are typed errors, never a panic; see
        // `exponent_driven_overflow_is_a_clean_error_not_a_panic`).
        assert_eq!(
            conservative_price_band(1_000_000, 0, 10, 0),
            Err(HealthError::PriceOutOfBounds)
        );
    }

    #[test]
    fn exponent_driven_overflow_is_a_clean_error_not_a_panic() {
        // expo so large that 10^(18+expo) cannot be represented -- must be a typed Overflow, not
        // a panic.
        let err = scale_to_wad_floor(1, i32::MAX).unwrap_err();
        assert_eq!(err, HealthError::Overflow);
        let err = scale_to_wad_ceil(1, i32::MAX).unwrap_err();
        assert_eq!(err, HealthError::Overflow);
        // expo so negative that -(18+expo) cannot be represented either.
        let err = scale_to_wad_floor(1, i32::MIN).unwrap_err();
        assert_eq!(err, HealthError::Overflow);
    }

    #[test]
    fn typical_band_matches_hand_computation() {
        // SOL @ $150.00 +/- $0.30, expo = -8 -> raw price 15_000_000_000, conf 30_000_000.
        let band = conservative_price_band(15_000_000_000, 30_000_000, -8, 100).unwrap();
        assert_eq!(band.lo, 149_700_000_000_000_000_000); // 149.70 WAD
        assert_eq!(band.hi, 150_300_000_000_000_000_000); // 150.30 WAD
    }

    // --- U-HEALTH-01/02: economic-model.md §6.5 worked examples ---
    //
    // The document's own prose states the health factors as "1.330838..." and "0.842495...".
    // Investigated per AGENTS.md ("if implementation disagrees with the frozen worked example,
    // investigate formula/rounding first"): collateral_value/debt_value below are computed
    // exactly per economic-model.md §6.2's formulas from the document's own §6.5 input numbers,
    // and independently cross-checked with exact rational arithmetic (Python's `fractions.
    // Fraction`) outside this repository. The true value of 1197.6/900.18 is
    // 1.330400586549357..., and of 758.4/900.18 is 0.84249816703326... — the document's prose
    // contains a manual-arithmetic transcription slip in the last few digits, not a
    // formula/rounding disagreement; the qualitative conclusions it draws (healthy at the first
    // price, liquidatable and eligible for full liquidation at the second) are unaffected and
    // still hold exactly. This is recorded as a documentation finding (`docs/project-status.md`),
    // not silently worked around, matching Phase 4's precedent for the U-ROUND-01..14 count slip.
    #[test]
    fn u_health_01_healthy_position() {
        let price_c_lo = 149_700_000_000_000_000_000u128; // 149.70 WAD
        let price_l_hi = 1_000_200_000_000_000_000u128; // 1.0002 WAD
        let collateral_amount = 10_000_000_000u64; // 10 SOL @ 9dp
        let debt_assets = 900_000_000u64; // 900 USDC @ 6dp
        let liq_threshold = 800_000_000_000_000_000u128; // 0.80 WAD
        let max_ltv = 750_000_000_000_000_000u128; // 0.75 WAD

        let cv = collateral_value(collateral_amount, price_c_lo, 9).unwrap();
        let dv = debt_value(debt_assets, price_l_hi, 6).unwrap();
        assert_eq!(cv, 1_497_000_000_000_000_000_000); // $1,497.00
        assert_eq!(dv, 900_180_000_000_000_000_000); // $900.18

        let hf = health_factor(cv, liq_threshold, dv).unwrap();
        assert_eq!(hf, 1_330_400_586_549_356_795);
        assert!(hf > WAD, "position must be healthy (HF > 1)");

        assert!(is_within_max_ltv(cv, dv, max_ltv).unwrap());
        // Additional borrowing capacity ~= $222.57 of debt value (economic-model.md §6.5).
        let max_debt_value = 1_122_750_000_000_000_000_000u128;
        assert_eq!(max_debt_value - dv, 222_570_000_000_000_000_000);
    }

    #[test]
    fn u_health_02_price_drop_to_liquidatable() {
        let price_c_lo = 94_800_000_000_000_000_000u128; // (95.00 - 0.20) WAD
        let price_l_hi = 1_000_200_000_000_000_000u128; // unchanged
        let collateral_amount = 10_000_000_000u64;
        let debt_assets = 900_000_000u64;
        let liq_threshold = 800_000_000_000_000_000u128;
        let full_liq_hf = 950_000_000_000_000_000u128; // 0.95 WAD

        let cv = collateral_value(collateral_amount, price_c_lo, 9).unwrap();
        let dv = debt_value(debt_assets, price_l_hi, 6).unwrap();
        assert_eq!(cv, 948_000_000_000_000_000_000); // $948.00

        let hf = health_factor(cv, liq_threshold, dv).unwrap();
        assert_eq!(hf, 842_498_167_033_260_014);
        assert!(hf < WAD, "position must be liquidatable (HF < 1)");
        assert!(
            hf < full_liq_hf,
            "HF must be below full_liq_hf -- full liquidation permitted"
        );
    }

    // --- P-VAL-2 (economic-model.md §10): collateral_value is monotone non-decreasing in
    // collateral_amount and in price_c_lo. ---
    #[test]
    fn p_val_2_collateral_value_monotone_in_amount() {
        let price = 149_700_000_000_000_000_000u128;
        let mut prev = 0u128;
        for amount in [0u64, 1, 1_000, 1_000_000, 1_000_000_000, 10_000_000_000] {
            let v = collateral_value(amount, price, 9).unwrap();
            assert!(
                v >= prev,
                "collateral_value must be non-decreasing in amount"
            );
            prev = v;
        }
    }

    #[test]
    fn p_val_2_collateral_value_monotone_in_price() {
        let amount = 10_000_000_000u64;
        let mut prev = 0u128;
        for price in [
            0u128,
            1_000_000,
            1_000_000_000_000_000_000,
            149_700_000_000_000_000_000,
            MAX_PRICE_WAD,
        ] {
            let v = collateral_value(amount, price, 9).unwrap();
            assert!(
                v >= prev,
                "collateral_value must be non-decreasing in price"
            );
            prev = v;
        }
    }

    // debt_value must be non-decreasing in debt_assets and in price_hi (mirrors P-VAL-2 for the
    // debt side, task #17's broader monotonicity ask).
    #[test]
    fn debt_value_monotone_in_assets_and_price() {
        let price = 1_000_200_000_000_000_000u128;
        let mut prev = 0u128;
        for assets in [0u64, 1, 1_000, 900_000_000, 10_000_000_000] {
            let v = debt_value(assets, price, 6).unwrap();
            assert!(v >= prev);
            prev = v;
        }
        let assets = 900_000_000u64;
        let mut prev = 0u128;
        for price in [0u128, 1_000_000, 1_000_200_000_000_000_000, MAX_PRICE_WAD] {
            let v = debt_value(assets, price, 6).unwrap();
            assert!(v >= prev);
            prev = v;
        }
    }

    // health_factor must be non-decreasing in collateral_value and non-increasing in debt_value
    // (task #17: "higher collateral value should not worsen health; higher debt value should not
    // improve health").
    #[test]
    fn health_factor_monotone_in_collateral_and_debt() {
        let liq_threshold = 800_000_000_000_000_000u128;
        let debt = 900_180_000_000_000_000_000u128;
        let mut prev = 0u128;
        for cv in [
            0u128,
            100,
            500_000_000_000_000_000_000,
            1_497_000_000_000_000_000_000,
        ] {
            let hf = health_factor(cv, liq_threshold, debt).unwrap();
            assert!(hf >= prev, "HF must be non-decreasing in collateral_value");
            prev = hf;
        }

        let cv = 1_497_000_000_000_000_000_000u128;
        let mut prev = u128::MAX;
        // Starts at 1 WAD, not 1 base unit: `mul_div_floor(cv, liq_threshold, dv)` for a dv this
        // small against this cv is a legitimately overflowing quotient (>u128::MAX), which
        // `mul_div_floor` correctly rejects as a typed `Overflow` rather than silently capping --
        // not a bug in `health_factor`, just an unrealistic input for this monotonicity check.
        for dv in [
            1_000_000_000_000_000_000u128,
            500_000_000_000_000_000_000,
            900_180_000_000_000_000_000,
            2_000_000_000_000_000_000_000,
        ] {
            let hf = health_factor(cv, liq_threshold, dv).unwrap();
            assert!(hf <= prev, "HF must be non-increasing in debt_value");
            prev = hf;
        }
    }

    // --- P-VAL-1 (economic-model.md §10): valuation correct for every decimals pair in 0..=12
    // crossed with expo in -12..=0 -- proves normalization does not silently shift economic
    // scale. A token worth exactly $1.00, at any decimals count and any oracle exponent
    // resolution, must value to exactly 1 WAD -- no float, no approximation, exact equality.
    #[test]
    fn p_val_1_decimals_x_expo_matrix_preserves_scale() {
        for decimals in 0u8..=12 {
            let amount: u64 = 10u64.pow(decimals as u32); // "1 whole token" in base units
            for expo in -12i32..=0 {
                // raw price representing exactly $1.00 at this exponent resolution.
                let raw_price: u128 = 10u128.pow((-expo) as u32);
                let price_wad = scale_to_wad_floor(raw_price, expo).unwrap();
                assert_eq!(
                    price_wad, WAD,
                    "decimals={decimals} expo={expo}: $1.00 must scale to exactly 1 WAD"
                );

                let value = collateral_value(amount, price_wad, decimals).unwrap();
                assert_eq!(
                    value, WAD,
                    "decimals={decimals} expo={expo}: 1 whole token worth $1.00 must value to exactly 1 WAD"
                );

                let dvalue = debt_value(amount, price_wad, decimals).unwrap();
                assert_eq!(
                    dvalue, WAD,
                    "decimals={decimals} expo={expo}: debt side must agree exactly"
                );
            }
        }
    }

    // Boundary values of the P-VAL-1 matrix, called out individually per the phase brief.
    #[test]
    fn p_val_1_boundary_values() {
        for (decimals, expo) in [(0u8, -12i32), (0, 0), (12, -12), (12, 0)] {
            let amount: u64 = 10u64.pow(decimals as u32);
            let raw_price: u128 = 10u128.pow((-expo) as u32);
            let price_wad = scale_to_wad_floor(raw_price, expo).unwrap();
            let value = collateral_value(amount, price_wad, decimals).unwrap();
            assert_eq!(value, WAD, "decimals={decimals} expo={expo} boundary case");
        }
    }
}
