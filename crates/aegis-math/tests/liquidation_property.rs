//! Liquidation properties `P-LIQ-1..4` (`docs/economic-model.md` §5.1/§7/§10,
//! `docs/invariants.md` INV-LIQ-02/05/08, `docs/phases/phase-06-liquidation.md`).
//!
//! **Construction technique:** every generated state uses `price_c_lo = price_l_hi = WAD` and
//! `collateral_decimals = loan_decimals = 18`, which makes `collateral_value == collateral_amount`
//! and `debt_value == debt_assets` **exactly** (`mul_div_*(amount, WAD, WAD) == amount`, no
//! rounding introduced by the price/decimals normalization itself). This isolates the properties
//! under test — HF-improvement and profitability, both statements about *value* — from
//! `aegis_math::health`'s own price/decimals normalization, which `P-VAL-1` already proves exact
//! for every `(decimals, expo)` pair independently. It also makes collateral and loan units
//! directly comparable as values for the profitability check (`P-LIQ-3`), since both assets are
//! worth exactly 1 value-unit per base unit in this construction.

use aegis_math::{
    compute_liquidation_by_repay, health_factor, is_liquidatable, max_repay, LiquidationParams, WAD,
};
use proptest::prelude::*;

/// One representative `(liq_threshold, liq_bonus)` pair satisfying the derived config bound
/// `liq_threshold * (WAD + liq_bonus) / WAD < WAD` (INV-LIQ-06, `economic-model.md` §5.1) — the
/// precondition every `create_market`/`set_market_params` call already enforces on-chain, so a
/// state violating it is not a state the protocol can ever reach.
fn lt_bonus_strategy() -> impl Strategy<Value = (u128, u128)> {
    prop_oneof![
        Just((500_000_000_000_000_000u128, 0u128)), // LT 0.50, b 0
        Just((600_000_000_000_000_000u128, 50_000_000_000_000_000)), // LT 0.60, b 0.05
        Just((700_000_000_000_000_000u128, 100_000_000_000_000_000)), // LT 0.70, b 0.10
        Just((750_000_000_000_000_000u128, 50_000_000_000_000_000)), // LT 0.75, b 0.05
        Just((800_000_000_000_000_000u128, 50_000_000_000_000_000)), // reference: LT 0.80, b 0.05
        Just((800_000_000_000_000_000u128, 200_000_000_000_000_000)), // LT 0.80, b 0.20
        Just((900_000_000_000_000_000u128, 100_000_000_000_000_000)), // LT 0.90, b 0.10
    ]
}

fn threshold_of(lt: u128, bonus: u128) -> u128 {
    aegis_math::mul_div_floor(lt, WAD + bonus, WAD).unwrap()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    // --- P-LIQ-1 / INV-LIQ-05: if HF_before > LT*(1+b), then HF_after > HF_before, for ANY
    // non-clamped liquidation amount (not merely a full liquidation) -- the derivation in
    // economic-model.md §5.1 is independent of how much of the position is repaid. ---
    #[test]
    fn p_liq_1_health_improves_above_the_derived_threshold(
        (lt, bonus) in lt_bonus_strategy(),
        debt_assets in 1_000u64..=1_000_000_000_000u64,
        hf_frac in 1u128..999u128, // strictly inside the (threshold, WAD) open interval
        repay_frac in 1u128..=100u128,
        close_factor in 100_000_000_000_000_000u128..=1_000_000_000_000_000_000u128, // 10%..100%
        protocol_fee in 0u128..=500_000_000_000_000_000u128, // 0..=0.5 WAD (config bound)
    ) {
        let threshold = threshold_of(lt, bonus);
        prop_assume!(threshold < WAD); // guaranteed by construction, defensive

        // Target an HF strictly between threshold and WAD, biased upward (ceil) so the health
        // module's own floor-rounding doesn't pull it back below threshold.
        let span = WAD - threshold;
        let hf_target = threshold + (span * hf_frac) / 1000;
        let collateral_value = aegis_math::mul_div_ceil(hf_target, debt_assets as u128, lt).unwrap()
            + 1;
        prop_assume!(collateral_value <= u64::MAX as u128);
        let collateral_amount = collateral_value as u64;

        let hf_before = health_factor(collateral_value, lt, debt_assets as u128).unwrap();
        prop_assume!(hf_before > threshold && hf_before < WAD);
        prop_assert!(is_liquidatable(hf_before));

        let full_liq_hf = threshold; // isolates the close-factor (partial) branch below WAD
        let params = LiquidationParams {
            collateral_decimals: 18,
            loan_decimals: 18,
            price_c_lo: WAD,
            price_l_hi: WAD,
            liq_bonus: bonus,
            liq_protocol_fee: protocol_fee,
            close_factor,
            full_liq_hf,
            min_debt: 1,
        };

        let mr = max_repay(debt_assets, hf_before, &params).unwrap();
        prop_assume!(mr > 0);
        let repay_assets = (((mr as u128) * repay_frac) / 100).max(1).min(mr as u128) as u64;
        prop_assume!(repay_assets > 0);

        let outcome = compute_liquidation_by_repay(
            &params,
            debt_assets,
            collateral_amount,
            hf_before,
            repay_assets,
        )
        .unwrap();
        // P-LIQ-1's claim is about the non-clamped derivation; see this file's module doc and
        // `u_liq_03_and_05_*` in `liquidation.rs` for the (excluded-by-construction-here, but
        // separately tested) full-seizure case where HF can legitimately fall to 0.
        prop_assume!(!outcome.clamped);

        let collateral_after = collateral_amount - outcome.total_seize;
        let debt_after = debt_assets - outcome.repay_assets;
        let hf_after = health_factor(collateral_after as u128, lt, debt_after as u128).unwrap();

        prop_assert!(
            hf_after > hf_before,
            "HF must improve: before={hf_before} after={hf_after} \
             (lt={lt} bonus={bonus} debt={debt_assets} collateral={collateral_amount} \
             repay={repay_assets} seize={})",
            outcome.total_seize
        );
    }

    // --- P-LIQ-2 / INV-LIQ-02: seizure never exceeds position.collateral_amount, over random
    // (possibly clamp-triggering) states -- broader than P-LIQ-1's improving-band filter. ---
    #[test]
    fn p_liq_2_seizure_never_exceeds_collateral(
        (_lt, bonus) in lt_bonus_strategy(),
        debt_assets in 1u64..=1_000_000_000_000u64,
        collateral_amount in 0u64..=1_000_000_000_000u64,
        hf_num in 1u128..=2_000_000_000_000_000_000u128, // 0..2 WAD, spans healthy and liquidatable
        close_factor in 50_000_000_000_000_000u128..=1_000_000_000_000_000_000u128,
        full_liq_hf in 1u128..=WAD,
        protocol_fee in 0u128..=500_000_000_000_000_000u128,
    ) {
        let params = LiquidationParams {
            collateral_decimals: 18,
            loan_decimals: 18,
            price_c_lo: WAD,
            price_l_hi: WAD,
            liq_bonus: bonus,
            liq_protocol_fee: protocol_fee,
            close_factor,
            full_liq_hf,
            min_debt: 1,
        };
        let hf = hf_num;
        prop_assume!(is_liquidatable(hf));
        let mr = match max_repay(debt_assets, hf, &params) {
            Ok(v) => v,
            Err(_) => return Ok(()),
        };
        prop_assume!(mr > 0);
        let repay_assets = mr; // largest permitted -> largest seizure, the binding case

        if let Ok(outcome) = compute_liquidation_by_repay(
            &params,
            debt_assets,
            collateral_amount,
            hf,
            repay_assets,
        ) {
            prop_assert!(outcome.total_seize <= collateral_amount);
            prop_assert!(outcome.repay_assets <= debt_assets);
            prop_assert!(outcome.protocol_cut <= outcome.bonus_amount);
        }
    }

    // --- P-LIQ-3 / INV-LIQ-08: liquidation is profitable for the liquidator whenever HF < WAD
    // and the collateral clamp is NOT hit -- profit measured in value terms (this file's
    // WAD-price/18-decimals construction makes collateral and loan base units directly
    // value-comparable). Assumption stated per the phase brief: liq_bonus > 0 (a zero-bonus
    // market has no liquidation incentive by design and is not claimed profitable).
    //
    // **Construction, by direct bound rather than an HF target:** the collateral range that is
    // simultaneously liquidatable (`HF < WAD`, i.e. `collateral < debt/LT`) AND large enough that
    // a full-debt repay's seizure never clamps (`collateral >= debt*(1+b)`) is non-empty exactly
    // because `LT*(1+b) < WAD` (INV-LIQ-06) -- `debt*(1+b) < debt/LT`. Picking `collateral_amount`
    // from inside that band by construction (rather than deriving it from an HF target and hoping
    // it lands there) makes "not clamped" the common case instead of a rare accept, without
    // narrowing the parameter space the property claims to hold over.
    #[test]
    fn p_liq_3_liquidation_is_profitable_when_not_clamped(
        (lt, bonus) in lt_bonus_strategy(),
        debt_assets in 1_000u64..=1_000_000_000_000u64,
        band_frac in 1u128..999u128,
        close_factor in 100_000_000_000_000_000u128..=1_000_000_000_000_000_000u128,
        protocol_fee in 0u128..=500_000_000_000_000_000u128, // < WAD always: config bound is 0.5
    ) {
        prop_assume!(bonus > 0, "a zero-bonus market has no liquidation incentive by design");
        let threshold = threshold_of(lt, bonus);
        prop_assume!(threshold < WAD);

        let low = aegis_math::mul_div_ceil(debt_assets as u128, WAD + bonus, WAD).unwrap() + 1;
        let high = aegis_math::mul_div_floor(debt_assets as u128, WAD, lt).unwrap();
        prop_assume!(low < high); // guaranteed by LT*(1+b) < WAD except at tiny debt_assets
        let collateral_value = low + ((high - low) * band_frac) / 1000;
        prop_assume!(collateral_value <= u64::MAX as u128);
        let collateral_amount = collateral_value as u64;

        let hf_before = health_factor(collateral_value, lt, debt_assets as u128).unwrap();
        prop_assume!(is_liquidatable(hf_before));

        let params = LiquidationParams {
            collateral_decimals: 18,
            loan_decimals: 18,
            price_c_lo: WAD,
            price_l_hi: WAD,
            liq_bonus: bonus,
            liq_protocol_fee: protocol_fee,
            close_factor,
            full_liq_hf: threshold,
            min_debt: 1,
        };
        let mr = max_repay(debt_assets, hf_before, &params).unwrap();
        prop_assume!(mr > 0);

        let outcome = compute_liquidation_by_repay(
            &params,
            debt_assets,
            collateral_amount,
            hf_before,
            mr,
        )
        .unwrap();
        prop_assume!(!outcome.clamped);
        prop_assume!(outcome.bonus_amount > 0); // tiny-repay rounding can floor the bonus to 0

        // Value-domain profit: value(to_liquidator) - value(repay) -- both directly comparable
        // here since price == WAD and decimals == 18 on both sides (see module doc comment).
        let profit = outcome.to_liquidator as i128 - outcome.repay_assets as i128;
        prop_assert!(
            profit > 0,
            "liquidation must be profitable: to_liquidator={} repay_assets={} profit={profit}",
            outcome.to_liquidator,
            outcome.repay_assets
        );
    }
}

// --- P-LIQ-4: for the reference parameter set (economic-model.md §5.1), full_liq_hf >= LT*(1+b)
// -- a recommended (not hard on-chain) bound, checked here directly per the phase brief. ---
#[test]
fn p_liq_4_reference_params_satisfy_full_liq_hf_bound() {
    let lt = 800_000_000_000_000_000u128; // 0.80 WAD
    let bonus = 50_000_000_000_000_000u128; // 0.05 WAD
    let full_liq_hf = 950_000_000_000_000_000u128; // 0.95 WAD

    let threshold = threshold_of(lt, bonus);
    assert_eq!(threshold, 840_000_000_000_000_000); // 0.84 WAD, matches economic-model.md §5.1
    assert!(
        full_liq_hf >= threshold,
        "full_liq_hf ({full_liq_hf}) must be >= LT*(1+b) ({threshold})"
    );
}
