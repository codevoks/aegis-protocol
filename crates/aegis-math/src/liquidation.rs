//! Liquidation math (`docs/economic-model.md` §7): close-factor/dust-rule `max_repay`, seizure,
//! bonus, protocol cut, and the collateral clamp with upward-rounded repay recomputation.
//! `no_std`, float-free, integer-only — like the rest of this crate (ADR-0009).
//!
//! **Liquidatability is strict** (`is_liquidatable`, E-12, INV-LIQ-01): `HF == WAD` is *not*
//! liquidatable. Every comparison in this module that gates eligibility uses `<`, never `<=` —
//! grep this file for `<=` against `hf`/`full_liq_hf` if that claim is ever in doubt.
//!
//! Two liquidator-facing entry points mirror `instruction-catalogue.md` §17's dual input form
//! (`liquidate(repay_assets, seize_collateral)`, exactly one nonzero):
//! - [`compute_liquidation_by_repay`] — the primary form, §7.1-§7.3 verbatim.
//! - [`compute_liquidation_by_seize`] — the liquidator names a target collateral amount instead;
//!   internally this uses the **same** conservative (ceil-rounded) inversion formula §7.2 already
//!   specifies for the collateral-clamp recomputation, so there is exactly one "seize -> repay"
//!   formula in this module, used both when the *protocol* clamps (collateral insufficient) and
//!   when the *caller* specifies a seize target directly.

use crate::constants::WAD;
use crate::fixed::{mul_div_ceil, mul_div_floor, MathError};

/// Errors from this module — a superset of [`MathError`] plus the liquidation-specific
/// preconditions (`economic-model.md` §7.1, E-12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiquidationError {
    Overflow,
    DivisionByZero,
    /// `repay_assets` (or the `seize_collateral`-derived repay) is zero.
    ZeroRepay,
    /// `repay_assets` exceeds `max_repay` (close factor / dust rule / full-liquidation bound).
    RepayExceedsMaxRepay,
    /// A caller-specified `seize_collateral` exceeds `position.collateral_amount` — this form
    /// takes no clamp path of its own; the caller chose the amount and must choose one that fits.
    SeizeExceedsCollateral,
}

impl From<MathError> for LiquidationError {
    fn from(e: MathError) -> Self {
        match e {
            MathError::Overflow => LiquidationError::Overflow,
            MathError::DivisionByZero => LiquidationError::DivisionByZero,
        }
    }
}

/// The market/position parameters `max_repay` and seizure math need, gathered into one struct
/// because of the sheer count (`docs/implementation-handoff.md` explicitly leaves "internal
/// function decomposition" to the implementer) — every field here is read directly off `Market`
/// (WAD risk params, cached decimals) or the already-validated conservative oracle bands, never
/// computed inside this module.
#[derive(Debug, Clone, Copy)]
pub struct LiquidationParams {
    pub collateral_decimals: u8,
    pub loan_decimals: u8,
    /// Confidence-lower-bound collateral price, WAD (`oracle-design.md`, INV-ORA-03).
    pub price_c_lo: u128,
    /// Confidence-upper-bound loan price, WAD.
    pub price_l_hi: u128,
    pub liq_bonus: u128,
    pub liq_protocol_fee: u128,
    pub close_factor: u128,
    pub full_liq_hf: u128,
    pub min_debt: u64,
}

/// Everything a successful `liquidate` needs to settle (`economic-model.md` §7.3) plus whether
/// the collateral clamp fired, for event/test observability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiquidationOutcome {
    pub repay_assets: u64,
    pub total_seize: u64,
    pub base_seize: u64,
    pub bonus_amount: u64,
    pub protocol_cut: u64,
    pub to_liquidator: u64,
    /// `true` iff the naive (unclamped) seizure exceeded `position.collateral_amount` and the
    /// collateral-clamp path (`economic-model.md` §7.2 "Collateral cap") recomputed `repay_assets`
    /// upward-rounded from the clamped seizure.
    pub clamped: bool,
}

fn pow10(exp: u8) -> Result<u128, LiquidationError> {
    10u128
        .checked_pow(exp as u32)
        .ok_or(LiquidationError::Overflow)
}

/// **E-12 / INV-LIQ-01, strict.** `HF == WAD` is never liquidatable — this is the one and only
/// comparison in the protocol that decides eligibility, and it must never be relaxed to `<=`.
#[inline]
pub fn is_liquidatable(hf: u128) -> bool {
    hf < WAD
}

/// `economic-model.md` §7.1: the maximum debt (in loan-asset base units) a single liquidation may
/// repay, applying the close-factor/full-liquidation split and then the dust rule.
///
/// - `hf < full_liq_hf` -> 100% (`debt_assets`).
/// - otherwise -> `floor(debt_assets * close_factor / WAD)`.
/// - **Dust rule** (`E-14`, `U-LIQ-04`): if the resulting remaining debt would be `(0,
///   min_debt)` — strictly positive but below the dust floor — force full repayment instead,
///   because a position `min_debt` itself forbids that state from existing at rest.
pub fn max_repay(
    debt_assets: u64,
    hf: u128,
    params: &LiquidationParams,
) -> Result<u64, LiquidationError> {
    let mut repay = if hf < params.full_liq_hf {
        debt_assets
    } else {
        let r = mul_div_floor(debt_assets as u128, params.close_factor, WAD)?;
        u64::try_from(r).map_err(|_| LiquidationError::Overflow)?
    };

    // close_factor <= WAD (create_market/set_market_params bound), so `repay <= debt_assets`
    // always holds here and this subtraction cannot underflow.
    let remaining = debt_assets
        .checked_sub(repay)
        .ok_or(LiquidationError::Overflow)?;
    if remaining > 0 && remaining < params.min_debt {
        repay = debt_assets;
    }
    Ok(repay)
}

/// `economic-model.md` §7.2's primary (non-clamped) leg: `repay_assets` -> `(repay_value,
/// base_seize, total_seize, bonus_amount)`. `repay_value` is ceiled (the liquidator's credit is
/// valued conservatively low for them); `base_seize`/`total_seize` are floored (the liquidator
/// never receives more collateral than exactly earned, INV-LIQ-07's seizure-side counterpart).
fn repay_to_seize(
    repay_assets: u64,
    params: &LiquidationParams,
) -> Result<(u128, u64, u64, u64), LiquidationError> {
    let loan_scale = pow10(params.loan_decimals)?;
    let collateral_scale = pow10(params.collateral_decimals)?;

    let repay_value = mul_div_ceil(repay_assets as u128, params.price_l_hi, loan_scale)?;

    let base_seize_u128 = mul_div_floor(repay_value, collateral_scale, params.price_c_lo)?;
    let base_seize = u64::try_from(base_seize_u128).map_err(|_| LiquidationError::Overflow)?;

    let bonus_factor = WAD
        .checked_add(params.liq_bonus)
        .ok_or(LiquidationError::Overflow)?;
    let bonus_repay_value = mul_div_floor(repay_value, bonus_factor, WAD)?;
    let total_seize_u128 = mul_div_floor(bonus_repay_value, collateral_scale, params.price_c_lo)?;
    let total_seize = u64::try_from(total_seize_u128).map_err(|_| LiquidationError::Overflow)?;

    let bonus_amount = total_seize.saturating_sub(base_seize);
    Ok((repay_value, base_seize, total_seize, bonus_amount))
}

/// The inversion of `repay_to_seize`, used both by the collateral-clamp recomputation and by
/// `compute_liquidation_by_seize` (`economic-model.md` §7.2 "Collateral cap"): a target
/// `seize_collateral` -> the `repay_assets` required to earn exactly that much, rounded so the
/// liquidator **pays no less** than owed (every step ceiled).
fn seize_to_repay(
    seize_collateral: u64,
    params: &LiquidationParams,
) -> Result<u64, LiquidationError> {
    let collateral_scale = pow10(params.collateral_decimals)?;
    let loan_scale = pow10(params.loan_decimals)?;

    let seize_value = mul_div_ceil(
        seize_collateral as u128,
        params.price_c_lo,
        collateral_scale,
    )?;
    let bonus_factor = WAD
        .checked_add(params.liq_bonus)
        .ok_or(LiquidationError::Overflow)?;
    let repay_value = mul_div_ceil(seize_value, WAD, bonus_factor)?;
    let repay_assets_u128 = mul_div_ceil(repay_value, loan_scale, params.price_l_hi)?;
    u64::try_from(repay_assets_u128).map_err(|_| LiquidationError::Overflow)
}

/// Recomputes `base_seize` consistent with a given `repay_assets` (`economic-model.md` §7.2's
/// clamp-path formula: `floor(ceil(repay_assets * price_l_hi / loan_scale) * collateral_scale /
/// price_c_lo)`) — shared by the clamp path and the seize-specified form so both settle on
/// exactly the same `bonus_amount = total_seize - base_seize` computation.
fn base_seize_for_repay(
    repay_assets: u64,
    params: &LiquidationParams,
) -> Result<u64, LiquidationError> {
    let loan_scale = pow10(params.loan_decimals)?;
    let collateral_scale = pow10(params.collateral_decimals)?;
    let repay_value = mul_div_ceil(repay_assets as u128, params.price_l_hi, loan_scale)?;
    let base = mul_div_floor(repay_value, collateral_scale, params.price_c_lo)?;
    u64::try_from(base).map_err(|_| LiquidationError::Overflow)
}

/// `economic-model.md` §7.3: `protocol_cut = floor(bonus_amount * liq_protocol_fee / WAD)`,
/// taken from the bonus **only** (INV-LIQ-07) — never from `base_seize`, the principal-equivalent
/// seizure.
fn protocol_cut_and_payout(
    total_seize: u64,
    bonus_amount: u64,
    liq_protocol_fee: u128,
) -> Result<(u64, u64), LiquidationError> {
    let cut_u128 = mul_div_floor(bonus_amount as u128, liq_protocol_fee, WAD)?;
    let protocol_cut = u64::try_from(cut_u128).map_err(|_| LiquidationError::Overflow)?;
    let to_liquidator = total_seize
        .checked_sub(protocol_cut)
        .ok_or(LiquidationError::Overflow)?;
    Ok((protocol_cut, to_liquidator))
}

/// Primary form: liquidator specifies `repay_assets` (`instruction-catalogue.md` §17,
/// `economic-model.md` §7.1-§7.3 in full, including the collateral clamp §7.2).
///
/// Preconditions enforced here (the caller must separately enforce `is_liquidatable(hf)` and
/// oracle validity, which are not this function's concern): `repay_assets > 0` and `repay_assets
/// <= max_repay(debt_assets, hf, params)`.
pub fn compute_liquidation_by_repay(
    params: &LiquidationParams,
    debt_assets: u64,
    collateral_amount: u64,
    hf: u128,
    repay_assets: u64,
) -> Result<LiquidationOutcome, LiquidationError> {
    if repay_assets == 0 {
        return Err(LiquidationError::ZeroRepay);
    }
    let mr = max_repay(debt_assets, hf, params)?;
    if repay_assets > mr {
        return Err(LiquidationError::RepayExceedsMaxRepay);
    }

    let (_repay_value, base_seize, total_seize, bonus_amount) =
        repay_to_seize(repay_assets, params)?;

    let (repay_assets, total_seize, base_seize, bonus_amount, clamped) =
        if total_seize > collateral_amount {
            // Collateral cap (economic-model.md §7.2): clamp seizure to exactly what exists and
            // recompute repay UPWARD-rounded -- the liquidator pays more, never less, and never
            // receives more collateral than exists (INV-LIQ-02/03).
            let clamped_total_seize = collateral_amount;
            let mut new_repay = seize_to_repay(clamped_total_seize, params)?;
            new_repay = new_repay.min(debt_assets);
            let new_base = base_seize_for_repay(new_repay, params)?;
            let new_bonus = clamped_total_seize.saturating_sub(new_base);
            (new_repay, clamped_total_seize, new_base, new_bonus, true)
        } else {
            (repay_assets, total_seize, base_seize, bonus_amount, false)
        };

    if repay_assets == 0 {
        // Only reachable via an adversarial/degenerate clamp input (e.g. collateral_amount == 0,
        // which `liquidate`'s own precondition (debt_assets, HF < WAD implies collateral was
        // nonzero at read time) should already exclude) -- fail closed rather than settle a
        // zero-repay "liquidation".
        return Err(LiquidationError::ZeroRepay);
    }

    let (protocol_cut, to_liquidator) =
        protocol_cut_and_payout(total_seize, bonus_amount, params.liq_protocol_fee)?;

    Ok(LiquidationOutcome {
        repay_assets,
        total_seize,
        base_seize,
        bonus_amount,
        protocol_cut,
        to_liquidator,
        clamped,
    })
}

/// Alternate form: liquidator specifies a target `seize_collateral` instead of `repay_assets`
/// (`instruction-catalogue.md` §17: "lets a liquidator size the trade against available swap
/// liquidity"). Uses the identical conservative inversion the collateral clamp uses, so there is
/// exactly one "seize -> repay" formula in this crate.
///
/// This form takes **no clamp path of its own**: the caller chose `seize_collateral`, so a value
/// exceeding `position.collateral_amount` is simply rejected (`LiquidationError::
/// SeizeExceedsCollateral`) rather than silently reduced — the clamp exists to protect the
/// *protocol* when a debt-driven seizure overruns collateral, not to rescue an over-specified
/// caller input.
pub fn compute_liquidation_by_seize(
    params: &LiquidationParams,
    debt_assets: u64,
    collateral_amount: u64,
    hf: u128,
    seize_collateral: u64,
) -> Result<LiquidationOutcome, LiquidationError> {
    if seize_collateral == 0 {
        return Err(LiquidationError::ZeroRepay);
    }
    if seize_collateral > collateral_amount {
        return Err(LiquidationError::SeizeExceedsCollateral);
    }

    let mut repay_assets = seize_to_repay(seize_collateral, params)?;
    repay_assets = repay_assets.min(debt_assets);
    if repay_assets == 0 {
        return Err(LiquidationError::ZeroRepay);
    }

    let mr = max_repay(debt_assets, hf, params)?;
    if repay_assets > mr {
        return Err(LiquidationError::RepayExceedsMaxRepay);
    }

    let base_seize = base_seize_for_repay(repay_assets, params)?;
    let bonus_amount = seize_collateral.saturating_sub(base_seize);
    let (protocol_cut, to_liquidator) =
        protocol_cut_and_payout(seize_collateral, bonus_amount, params.liq_protocol_fee)?;

    Ok(LiquidationOutcome {
        repay_assets,
        total_seize: seize_collateral,
        base_seize,
        bonus_amount,
        protocol_cut,
        to_liquidator,
        clamped: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const WAD_: u128 = WAD;

    fn ref_params() -> LiquidationParams {
        LiquidationParams {
            collateral_decimals: 9,
            loan_decimals: 6,
            price_c_lo: 94_800_000_000_000_000_000, // (95.00 - 0.20) WAD, economic-model.md §7.5
            price_l_hi: 1_000_200_000_000_000_000,  // 1.0002 WAD
            liq_bonus: 50_000_000_000_000_000,      // 0.05 WAD
            liq_protocol_fee: 100_000_000_000_000_000, // 0.10 WAD
            close_factor: 500_000_000_000_000_000,  // 0.50 WAD
            full_liq_hf: 950_000_000_000_000_000,   // 0.95 WAD
            min_debt: 10_000_000,                   // 10 USDC
        }
    }

    // --- E-12 / INV-LIQ-01: strict HF < WAD ---

    #[test]
    fn u_liq_02_hf_equal_to_wad_is_not_liquidatable() {
        assert!(!is_liquidatable(WAD_));
        assert!(is_liquidatable(WAD_ - 1));
        assert!(!is_liquidatable(WAD_ + 1));
    }

    // Grep-style self-check: nothing in this module's public eligibility function uses `<=`.
    #[test]
    fn is_liquidatable_never_uses_inclusive_comparison() {
        assert!(is_liquidatable(0));
        assert!(!is_liquidatable(WAD_));
    }

    // --- U-LIQ-01: economic-model.md §7.5 worked example, exact figures ---

    #[test]
    fn u_liq_01_worked_liquidation_example() {
        let params = ref_params();
        let debt_assets = 900_000_000u64; // 900 USDC
        let collateral_amount = 10_000_000_000u64; // 10 SOL
        let hf = 842_498_167_033_260_014u128; // from health.rs's u_health_02 vector (< full_liq_hf)
        assert!(
            hf < params.full_liq_hf,
            "fixture must be in the full-liquidation band"
        );

        let mr = max_repay(debt_assets, hf, &params).unwrap();
        assert_eq!(
            mr, debt_assets,
            "HF < full_liq_hf must allow full repayment"
        );

        let outcome =
            compute_liquidation_by_repay(&params, debt_assets, collateral_amount, hf, debt_assets)
                .unwrap();

        assert!(!outcome.clamped, "collateral must be sufficient: no clamp");
        assert_eq!(outcome.repay_assets, 900_000_000);
        assert_eq!(outcome.base_seize, 9_495_569_620);
        assert_eq!(outcome.total_seize, 9_970_348_101);
        assert_eq!(outcome.bonus_amount, 474_778_481);
        assert_eq!(outcome.protocol_cut, 47_477_848);
        assert_eq!(outcome.to_liquidator, 9_922_870_253);

        // Post-conditions (economic-model.md §7.4).
        assert!(outcome.total_seize <= collateral_amount);
        assert!(outcome.repay_assets <= debt_assets);
        let remaining_collateral = collateral_amount - outcome.total_seize;
        assert_eq!(remaining_collateral, 29_651_899); // ~0.02965 SOL, matches doc prose
    }

    // --- U-LIQ-03 / U-LIQ-05: collateral clamp, recomputed repay rounded up, full seizure with
    // remaining debt (independently verified via `scripts` python cross-check during authoring). ---

    #[test]
    fn u_liq_03_and_05_collateral_clamp_recomputes_repay_upward_and_leaves_remaining_debt() {
        let params = ref_params();
        let debt_assets = 900_000_000u64;
        let collateral_amount = 9_000_000_000u64; // insufficient for the naive 9_970_348_101 seize
        let hf = 842_498_167_033_260_014u128; // full-liquidation band

        // Unclamped, the naive seizure this repay would earn exceeds available collateral.
        let (_rv, _base, naive_total_seize, _bonus) = repay_to_seize(debt_assets, &params).unwrap();
        assert!(naive_total_seize > collateral_amount);

        let outcome =
            compute_liquidation_by_repay(&params, debt_assets, collateral_amount, hf, debt_assets)
                .unwrap();

        assert!(outcome.clamped, "must take the collateral-clamp path");
        // INV-LIQ-02: seizure never exceeds what the position holds -- exact equality at the clamp.
        assert_eq!(outcome.total_seize, collateral_amount);
        // The clamp recomputes repay DOWNWARD from the naive 900_000_000 (less collateral => less
        // debt can be justified), but always rounded UP within that recomputation (ceil at every
        // step of `seize_to_repay`) so the liquidator never pays less than the clamped seizure
        // actually earns.
        assert_eq!(outcome.repay_assets, 812_408_947);
        assert!(outcome.repay_assets < debt_assets);
        assert_eq!(outcome.base_seize, 8_571_428_573);
        assert_eq!(outcome.bonus_amount, 428_571_427);
        assert_eq!(outcome.protocol_cut, 42_857_142);
        assert_eq!(outcome.to_liquidator, 8_957_142_858);

        // U-LIQ-05: full seizure (100% of position.collateral_amount) with debt remaining --
        // exactly the bridge into bad-debt eligibility (economic-model.md §8).
        assert_eq!(
            outcome.total_seize, collateral_amount,
            "all collateral seized"
        );
        let remaining_debt = debt_assets - outcome.repay_assets;
        assert!(remaining_debt > 0, "debt must remain after full seizure");
    }

    // Liquidator must never pay less because of favorable rounding: an independent re-derivation
    // via `seize_to_repay` from the clamp's own `total_seize` must agree exactly with the
    // `repay_assets` the clamp produced (both apply the identical ceil-rounded formula).
    #[test]
    fn u_liq_03_clamp_repay_matches_independent_reinversion() {
        let params = ref_params();
        let debt_assets = 900_000_000u64;
        let collateral_amount = 9_000_000_000u64;
        let hf = 842_498_167_033_260_014u128;

        let outcome =
            compute_liquidation_by_repay(&params, debt_assets, collateral_amount, hf, debt_assets)
                .unwrap();
        let reinverted = seize_to_repay(outcome.total_seize, &params).unwrap();
        assert_eq!(outcome.repay_assets, reinverted.min(debt_assets));
    }

    // --- U-LIQ-04: dust rule forces full repayment when a partial repay would leave dust debt ---

    #[test]
    fn u_liq_04_dust_rule_forces_full_repayment() {
        let mut params = ref_params();
        params.close_factor = 500_000_000_000_000_000; // 50%
        params.min_debt = 60_000_000; // 60 USDC
        let debt_assets = 100_000_000u64; // 100 USDC
        let hf = 960_000_000_000_000_000u128; // above full_liq_hf (0.95) but < WAD: partial branch

        // Naive close-factor repay would be 50 USDC, leaving 50 USDC remaining -- below the 60
        // USDC dust floor, so the dust rule must force full repayment instead.
        let naive = crate::fixed::mul_div_floor(debt_assets as u128, params.close_factor, WAD_)
            .unwrap() as u64;
        assert_eq!(naive, 50_000_000);
        assert!(debt_assets - naive < params.min_debt);

        let mr = max_repay(debt_assets, hf, &params).unwrap();
        assert_eq!(mr, debt_assets, "dust rule must force full repayment");
    }

    // The same fixture with a min_debt low enough that 50 USDC of remaining debt is NOT dust must
    // NOT force full repayment -- pins the dust-rule transition in both directions.
    #[test]
    fn u_liq_04_dust_rule_transition_no_dust_case() {
        let mut params = ref_params();
        params.close_factor = 500_000_000_000_000_000;
        params.min_debt = 10_000_000; // 10 USDC: 50 USDC remaining is comfortably above dust
        let debt_assets = 100_000_000u64;
        let hf = 960_000_000_000_000_000u128;

        let mr = max_repay(debt_assets, hf, &params).unwrap();
        assert_eq!(
            mr, 50_000_000,
            "close-factor bound applies; no dust forcing"
        );
    }

    // Boundary: remaining debt exactly at min_debt must NOT trigger the dust rule (the rule is
    // "0 < remaining < min_debt", strictly); remaining == min_debt - 1 must trigger it.
    #[test]
    fn dust_rule_boundary_exactly_at_min_debt_vs_one_below() {
        let mut params = ref_params();
        params.close_factor = 500_000_000_000_000_000;
        let debt_assets = 100_000_000u64;
        let hf = 960_000_000_000_000_000u128;

        // remaining = 50_000_000 exactly == min_debt -> no dust forcing.
        params.min_debt = 50_000_000;
        assert_eq!(max_repay(debt_assets, hf, &params).unwrap(), 50_000_000);

        // remaining = 50_000_000 < min_debt (50_000_001) -> dust forcing.
        params.min_debt = 50_000_001;
        assert_eq!(max_repay(debt_assets, hf, &params).unwrap(), debt_assets);
    }

    // --- E-13 boundary: total_seize exactly equal to collateral_amount must NOT clamp (only a
    // strict excess clamps). ---

    #[test]
    fn clamp_boundary_exact_equality_does_not_clamp() {
        let params = ref_params();
        let debt_assets = 900_000_000u64;
        let hf = 842_498_167_033_260_014u128;
        let (_rv, _base, naive_total_seize, _bonus) = repay_to_seize(debt_assets, &params).unwrap();

        let outcome = compute_liquidation_by_repay(
            &params,
            debt_assets,
            naive_total_seize, // exactly enough, no more
            hf,
            debt_assets,
        )
        .unwrap();
        assert!(!outcome.clamped);
        assert_eq!(outcome.total_seize, naive_total_seize);

        // One unit less must clamp.
        let outcome_clamped = compute_liquidation_by_repay(
            &params,
            debt_assets,
            naive_total_seize - 1,
            hf,
            debt_assets,
        )
        .unwrap();
        assert!(outcome_clamped.clamped);
        assert_eq!(outcome_clamped.total_seize, naive_total_seize - 1);
    }

    // --- INV-LIQ-02 / P-LIQ-2: seizure never exceeds collateral, across many repay sizes ---

    #[test]
    fn seizure_never_exceeds_collateral_amount() {
        let params = ref_params();
        let debt_assets = 900_000_000u64;
        let hf = 842_498_167_033_260_014u128;
        for collateral_amount in [
            1u64,
            100,
            1_000_000,
            5_000_000_000,
            9_970_348_101,
            20_000_000_000,
        ] {
            let outcome = compute_liquidation_by_repay(
                &params,
                debt_assets,
                collateral_amount,
                hf,
                debt_assets,
            )
            .unwrap();
            assert!(
                outcome.total_seize <= collateral_amount,
                "seizure {} exceeded collateral {collateral_amount}",
                outcome.total_seize
            );
            assert!(outcome.repay_assets <= debt_assets);
        }
    }

    // --- INV-LIQ-03: repay never exceeds outstanding debt, including the clamp path ---

    #[test]
    fn repay_never_exceeds_debt() {
        let params = ref_params();
        let debt_assets = 900_000_000u64;
        let hf = 842_498_167_033_260_014u128;
        for collateral_amount in [1u64, 1_000, 5_000_000_000, 9_970_348_101] {
            let outcome = compute_liquidation_by_repay(
                &params,
                debt_assets,
                collateral_amount,
                hf,
                debt_assets,
            )
            .unwrap();
            assert!(outcome.repay_assets <= debt_assets);
        }
    }

    // --- INV-LIQ-07: protocol cut is taken from the bonus only, never from base_seize ---

    #[test]
    fn protocol_cut_never_exceeds_bonus_and_never_touches_base_seize() {
        let params = ref_params();
        let debt_assets = 900_000_000u64;
        let collateral_amount = 10_000_000_000u64;
        let hf = 842_498_167_033_260_014u128;
        let outcome =
            compute_liquidation_by_repay(&params, debt_assets, collateral_amount, hf, debt_assets)
                .unwrap();
        assert!(outcome.protocol_cut <= outcome.bonus_amount);
        assert_eq!(
            outcome.to_liquidator,
            outcome.total_seize - outcome.protocol_cut
        );
        assert!(
            outcome.to_liquidator >= outcome.base_seize,
            "liquidator must receive at least the principal-equivalent seizure"
        );
    }

    // --- E-13/E-14 exercised via the alternate seize-specified input form ---

    #[test]
    fn compute_liquidation_by_seize_matches_repay_form_at_the_same_point() {
        let params = ref_params();
        let debt_assets = 900_000_000u64;
        let collateral_amount = 10_000_000_000u64;
        let hf = 842_498_167_033_260_014u128;

        let by_repay =
            compute_liquidation_by_repay(&params, debt_assets, collateral_amount, hf, debt_assets)
                .unwrap();
        let by_seize = compute_liquidation_by_seize(
            &params,
            debt_assets,
            collateral_amount,
            hf,
            by_repay.total_seize,
        )
        .unwrap();
        assert_eq!(by_seize.total_seize, by_repay.total_seize);
        assert_eq!(by_seize.repay_assets, by_repay.repay_assets);
        assert_eq!(by_seize.protocol_cut, by_repay.protocol_cut);
        assert_eq!(by_seize.to_liquidator, by_repay.to_liquidator);
    }

    #[test]
    fn compute_liquidation_by_seize_rejects_seize_above_collateral() {
        let params = ref_params();
        let err = compute_liquidation_by_seize(
            &params,
            900_000_000,
            10_000_000_000,
            842_498_167_033_260_014,
            10_000_000_001,
        )
        .unwrap_err();
        assert_eq!(err, LiquidationError::SeizeExceedsCollateral);
    }

    #[test]
    fn zero_repay_and_zero_seize_are_rejected() {
        let params = ref_params();
        let hf = 842_498_167_033_260_014u128;
        assert_eq!(
            compute_liquidation_by_repay(&params, 900_000_000, 10_000_000_000, hf, 0).unwrap_err(),
            LiquidationError::ZeroRepay
        );
        assert_eq!(
            compute_liquidation_by_seize(&params, 900_000_000, 10_000_000_000, hf, 0).unwrap_err(),
            LiquidationError::ZeroRepay
        );
    }

    #[test]
    fn repay_exceeding_max_repay_is_rejected() {
        let params = ref_params();
        let hf = 960_000_000_000_000_000u128; // above full_liq_hf: close-factor branch, 50%
        let debt_assets = 100_000_000u64;
        let mr = max_repay(debt_assets, hf, &params).unwrap();
        assert_eq!(mr, 50_000_000);
        let err = compute_liquidation_by_repay(&params, debt_assets, 10_000_000_000, hf, mr + 1)
            .unwrap_err();
        assert_eq!(err, LiquidationError::RepayExceedsMaxRepay);
        // Exactly at the bound must succeed.
        assert!(compute_liquidation_by_repay(&params, debt_assets, 10_000_000_000, hf, mr).is_ok());
    }
}
