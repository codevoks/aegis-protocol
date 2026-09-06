//! `Market` — one isolated lending venue (`account-model.md` §4). Field order, types and
//! `_reserved` width are transcribed verbatim from the frozen spec; do not reorder.
//!
//! Parameter-bound validation lives here (not in the instruction handler) per
//! `architecture.md` §2 ("`state/market.rs` // Market account + accrual + param validation").

use crate::constants::{
    MAX_CONF_BPS, MAX_FEE, MAX_LIQ_BONUS, MAX_LIQ_PROTOCOL_FEE, MAX_PRICE_AGE_SECS,
    MIN_CLOSE_FACTOR, MIN_CONF_BPS, MIN_PRICE_AGE_SECS,
};
use crate::error::AegisError;
use aegis_math::{mul_div_floor, WAD};
use anchor_lang::prelude::*;

#[account]
pub struct Market {
    // --- identity (immutable after creation) ---
    pub collateral_mint: Pubkey,
    pub loan_mint: Pubkey,
    pub collateral_token_program: Pubkey,
    pub loan_token_program: Pubkey,
    pub collateral_vault: Pubkey,
    pub loan_vault: Pubkey,
    pub fee_recipient: Pubkey,
    pub config_id: u16,
    pub collateral_decimals: u8,
    pub loan_decimals: u8,

    // --- oracle config (admin-mutable, market-local) ---
    pub oracle_kind: u8,
    pub collateral_feed_id: [u8; 32],
    pub loan_feed_id: [u8; 32],
    pub max_price_age_secs: u32,
    pub max_conf_bps: u16,

    // --- risk params (admin-mutable, bounds-checked) ---
    pub max_ltv: u128,
    pub liq_threshold: u128,
    pub liq_bonus: u128,
    pub close_factor: u128,
    pub full_liq_hf: u128,
    pub liq_protocol_fee: u128,
    pub fee: u128,
    pub min_debt: u64,

    // --- IRM params (stateless) ---
    pub base_rate_ps: u128,
    pub slope1_ps: u128,
    pub slope2_ps: u128,
    pub u_kink: u128,
    pub max_rate_ps: u128,

    // --- accounting (hot) ---
    pub total_supply_assets: u64,
    pub total_supply_shares: u128,
    pub total_borrow_assets: u64,
    pub total_borrow_shares: u128,
    pub collateral_fee_accrued: u64,
    pub last_accrual_ts: i64,

    // --- flags / bumps ---
    pub paused: u8,
    pub flags: u8,
    pub bump: u8,
    pub collateral_vault_bump: u8,
    pub loan_vault_bump: u8,
    pub _reserved: [u8; 64],
}

impl Market {
    /// Discriminator (8) + identity (7 `Pubkey` + `u16` + 2 `u8` = 228) + oracle config (71) +
    /// risk params (120) + IRM params (80) + accounting (64) + flags/bumps (69) = 640.
    ///
    /// `account-model.md` §4 states this size approximately ("~633 ... ≈ 641"); this constant is
    /// the exact figure derived by summing the field list in that same section field-by-field,
    /// and is pinned by `U-ACCT-02` against the account actually produced by `create_market`.
    pub const LEN: usize = 8
        + (32 * 7 + 2 + 1 + 1) // identity: 228
        + (1 + 32 + 32 + 4 + 2) // oracle config: 71
        + (16 * 7 + 8) // risk params: 120
        + (16 * 5) // IRM params: 80
        + (8 + 16 + 8 + 16 + 8 + 8) // accounting: 64
        + (1 + 1 + 1 + 1 + 1 + 64); // flags/bumps: 69

    /// Risk-parameter bounds from `economic-model.md` §5, including the derived liquidation
    /// bound `liq_threshold * (WAD + liq_bonus) / WAD < WAD` (INV-LIQ-06).
    #[allow(clippy::too_many_arguments)]
    pub fn validate_risk_params(
        max_ltv: u128,
        liq_threshold: u128,
        liq_bonus: u128,
        close_factor: u128,
        full_liq_hf: u128,
        liq_protocol_fee: u128,
        fee: u128,
        min_debt: u64,
    ) -> Result<()> {
        require!(
            max_ltv > 0 && max_ltv < liq_threshold && liq_threshold < WAD,
            AegisError::InvalidMaxLtvOrThreshold
        );
        require!(liq_bonus <= MAX_LIQ_BONUS, AegisError::InvalidLiqBonus);

        // Derived bound: liq_bonus is already <= MAX_LIQ_BONUS (0.25 WAD), so WAD + liq_bonus
        // cannot overflow u128, and mul_div_floor's 256-bit intermediate cannot overflow either.
        let bonus_factor = WAD
            .checked_add(liq_bonus)
            .ok_or(AegisError::ArithmeticOverflow)?;
        let threshold_times_bonus =
            mul_div_floor(liq_threshold, bonus_factor, WAD).map_err(AegisError::from)?;
        require!(
            threshold_times_bonus < WAD,
            AegisError::LiquidationBonusExceedsThresholdBound
        );

        require!(
            (MIN_CLOSE_FACTOR..=WAD).contains(&close_factor),
            AegisError::InvalidCloseFactor
        );
        require!(
            full_liq_hf > 0 && full_liq_hf <= WAD,
            AegisError::InvalidFullLiqHf
        );
        require!(
            liq_protocol_fee <= MAX_LIQ_PROTOCOL_FEE,
            AegisError::InvalidLiqProtocolFee
        );
        require!(fee <= MAX_FEE, AegisError::InvalidFee);
        require!(min_debt > 0, AegisError::InvalidMinDebt);
        Ok(())
    }

    /// IRM bounds from `instruction-catalogue.md` §6 precondition 7:
    /// `0 < u_kink < WAD`, `max_rate_ps > 0`, every rate `<= max_rate_ps`.
    pub fn validate_irm_params(
        base_rate_ps: u128,
        slope1_ps: u128,
        slope2_ps: u128,
        u_kink: u128,
        max_rate_ps: u128,
    ) -> Result<()> {
        require!(
            u_kink > 0 && u_kink < WAD && max_rate_ps > 0,
            AegisError::InvalidIrmParams
        );
        require!(
            base_rate_ps <= max_rate_ps && slope1_ps <= max_rate_ps && slope2_ps <= max_rate_ps,
            AegisError::InvalidIrmParams
        );
        Ok(())
    }

    /// Oracle config bounds from `instruction-catalogue.md` §6 precondition 9.
    pub fn validate_oracle_config(max_price_age_secs: u32, max_conf_bps: u16) -> Result<()> {
        require!(
            (MIN_PRICE_AGE_SECS..=MAX_PRICE_AGE_SECS).contains(&max_price_age_secs),
            AegisError::InvalidMaxPriceAge
        );
        require!(
            (MIN_CONF_BPS..=MAX_CONF_BPS).contains(&max_conf_bps),
            AegisError::InvalidMaxConfBps
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn len_matches_account_model_spec() {
        // account-model.md §4 states "8 + ~633 ~= 641" (approximate); 640 is the exact sum of
        // the field list in that same section.
        assert_eq!(Market::LEN, 640);
    }

    const REF_MAX_LTV: u128 = 750_000_000_000_000_000; // 0.75 WAD
    const REF_LT: u128 = 800_000_000_000_000_000; // 0.80 WAD
    const REF_BONUS: u128 = 50_000_000_000_000_000; // 0.05 WAD
    const REF_CLOSE_FACTOR: u128 = 500_000_000_000_000_000; // 0.5 WAD
    const REF_FULL_LIQ_HF: u128 = 950_000_000_000_000_000; // 0.95 WAD
    const REF_LIQ_PROTOCOL_FEE: u128 = 100_000_000_000_000_000; // 0.10 WAD
    const REF_FEE: u128 = 100_000_000_000_000_000; // 0.10 WAD
    const REF_MIN_DEBT: u64 = 10_000_000; // 10 USDC at 6dp

    // U-ADM-00 (informal): economic-model.md §5.1's own reference parameter set must validate.
    #[test]
    fn reference_parameter_set_is_valid() {
        assert!(Market::validate_risk_params(
            REF_MAX_LTV,
            REF_LT,
            REF_BONUS,
            REF_CLOSE_FACTOR,
            REF_FULL_LIQ_HF,
            REF_LIQ_PROTOCOL_FEE,
            REF_FEE,
            REF_MIN_DEBT,
        )
        .is_ok());
    }

    // A-ADM-04 (math-level component): an otherwise-plausible parameter set that specifically
    // violates the derived liquidation bound LT*(1+b) < WAD must be rejected, distinctly from the
    // flat liq_bonus bound (0.24 WAD alone is within MAX_LIQ_BONUS).
    #[test]
    fn derived_liquidation_bound_rejects_plausible_but_unsafe_params() {
        let unsafe_bonus = 240_000_000_000_000_000u128; // 0.24 WAD, <= MAX_LIQ_BONUS
        let high_threshold = 850_000_000_000_000_000u128; // 0.85 WAD: 0.85 * 1.24 = 1.054 > 1
        let err = Market::validate_risk_params(
            REF_MAX_LTV,
            high_threshold,
            unsafe_bonus,
            REF_CLOSE_FACTOR,
            REF_FULL_LIQ_HF,
            REF_LIQ_PROTOCOL_FEE,
            REF_FEE,
            REF_MIN_DEBT,
        )
        .unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::LiquidationBonusExceedsThresholdBound)
        );
    }

    #[test]
    fn max_ltv_must_be_below_liq_threshold() {
        let err = Market::validate_risk_params(
            REF_LT,
            REF_MAX_LTV,
            REF_BONUS,
            REF_CLOSE_FACTOR,
            REF_FULL_LIQ_HF,
            REF_LIQ_PROTOCOL_FEE,
            REF_FEE,
            REF_MIN_DEBT,
        )
        .unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::InvalidMaxLtvOrThreshold)
        );
    }

    #[test]
    fn liq_bonus_above_max_is_rejected() {
        let err = Market::validate_risk_params(
            REF_MAX_LTV,
            REF_LT,
            MAX_LIQ_BONUS + 1,
            REF_CLOSE_FACTOR,
            REF_FULL_LIQ_HF,
            REF_LIQ_PROTOCOL_FEE,
            REF_FEE,
            REF_MIN_DEBT,
        )
        .unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::InvalidLiqBonus)
        );
    }

    #[test]
    fn close_factor_below_minimum_is_rejected() {
        let err = Market::validate_risk_params(
            REF_MAX_LTV,
            REF_LT,
            REF_BONUS,
            MIN_CLOSE_FACTOR - 1,
            REF_FULL_LIQ_HF,
            REF_LIQ_PROTOCOL_FEE,
            REF_FEE,
            REF_MIN_DEBT,
        )
        .unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::InvalidCloseFactor)
        );
    }

    #[test]
    fn full_liq_hf_zero_is_rejected() {
        let err = Market::validate_risk_params(
            REF_MAX_LTV,
            REF_LT,
            REF_BONUS,
            REF_CLOSE_FACTOR,
            0,
            REF_LIQ_PROTOCOL_FEE,
            REF_FEE,
            REF_MIN_DEBT,
        )
        .unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::InvalidFullLiqHf)
        );
    }

    #[test]
    fn liq_protocol_fee_above_max_is_rejected() {
        let err = Market::validate_risk_params(
            REF_MAX_LTV,
            REF_LT,
            REF_BONUS,
            REF_CLOSE_FACTOR,
            REF_FULL_LIQ_HF,
            MAX_LIQ_PROTOCOL_FEE + 1,
            REF_FEE,
            REF_MIN_DEBT,
        )
        .unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::InvalidLiqProtocolFee)
        );
    }

    #[test]
    fn fee_above_max_is_rejected() {
        let err = Market::validate_risk_params(
            REF_MAX_LTV,
            REF_LT,
            REF_BONUS,
            REF_CLOSE_FACTOR,
            REF_FULL_LIQ_HF,
            REF_LIQ_PROTOCOL_FEE,
            MAX_FEE + 1,
            REF_MIN_DEBT,
        )
        .unwrap_err();
        assert_eq!(err, anchor_lang::error::Error::from(AegisError::InvalidFee));
    }

    #[test]
    fn zero_min_debt_is_rejected() {
        let err = Market::validate_risk_params(
            REF_MAX_LTV,
            REF_LT,
            REF_BONUS,
            REF_CLOSE_FACTOR,
            REF_FULL_LIQ_HF,
            REF_LIQ_PROTOCOL_FEE,
            REF_FEE,
            0,
        )
        .unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::InvalidMinDebt)
        );
    }

    #[test]
    fn irm_params_reference_set_is_valid() {
        assert!(Market::validate_irm_params(0, WAD / 20, WAD / 2, WAD / 2, WAD).is_ok());
    }

    #[test]
    fn irm_u_kink_out_of_range_is_rejected() {
        let err = Market::validate_irm_params(0, WAD / 20, WAD / 2, WAD, WAD).unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::InvalidIrmParams)
        );
    }

    #[test]
    fn irm_rate_exceeding_max_is_rejected() {
        let err = Market::validate_irm_params(0, WAD / 20, WAD + 1, WAD / 2, WAD).unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::InvalidIrmParams)
        );
    }

    #[test]
    fn oracle_config_reference_is_valid() {
        assert!(Market::validate_oracle_config(60, 100).is_ok());
    }

    #[test]
    fn oracle_config_price_age_out_of_range_is_rejected() {
        let err = Market::validate_oracle_config(0, 100).unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::InvalidMaxPriceAge)
        );
        let err = Market::validate_oracle_config(MAX_PRICE_AGE_SECS + 1, 100).unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::InvalidMaxPriceAge)
        );
    }

    #[test]
    fn oracle_config_conf_bps_out_of_range_is_rejected() {
        let err = Market::validate_oracle_config(60, 0).unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::InvalidMaxConfBps)
        );
        let err = Market::validate_oracle_config(60, MAX_CONF_BPS + 1).unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::InvalidMaxConfBps)
        );
    }
}
