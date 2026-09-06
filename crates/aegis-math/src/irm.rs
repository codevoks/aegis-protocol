//! Stateless piecewise-linear interest-rate model and Taylor compounding (`economic-model.md` §4,
//! ADR-0007). Pure functions of `(state, params)` — no `Market` account, no IRM state, no floats.

use crate::constants::WAD;
use crate::fixed::{mul_div_floor, MathError};

/// `u = min(WAD, floor(total_borrow_assets * WAD / total_supply_assets))`, or `0` when the market
/// has no supply at all (E-03) — never a division by zero.
pub fn utilization(total_borrow_assets: u64, total_supply_assets: u64) -> Result<u128, MathError> {
    if total_supply_assets == 0 {
        return Ok(0);
    }
    let u = mul_div_floor(
        total_borrow_assets as u128,
        WAD,
        total_supply_assets as u128,
    )?;
    Ok(u.min(WAD))
}

/// The piecewise-linear per-second borrow rate, capped at `max_rate_ps` (economic-model.md §4.1).
/// Every rate parameter is already validated by `Market::validate_irm_params` at creation
/// (`0 < u_kink < WAD`, `max_rate_ps > 0`), so the two divisors below (`u_kink`, `WAD - u_kink`)
/// are never zero for any state reachable through the program — but this function still returns a
/// typed `Result` rather than assuming it, because `aegis-math` never trusts a precondition it did
/// not itself just check (mirrors `mul_div_*`'s own contract).
#[allow(clippy::too_many_arguments)]
pub fn borrow_rate(
    u: u128,
    base_rate_ps: u128,
    slope1_ps: u128,
    slope2_ps: u128,
    u_kink: u128,
    max_rate_ps: u128,
) -> Result<u128, MathError> {
    let r = if u <= u_kink {
        let slope_component = mul_div_floor(slope1_ps, u, u_kink)?;
        base_rate_ps
            .checked_add(slope_component)
            .ok_or(MathError::Overflow)?
    } else {
        let above_kink = u.checked_sub(u_kink).ok_or(MathError::Overflow)?;
        let wad_minus_kink = WAD.checked_sub(u_kink).ok_or(MathError::Overflow)?;
        let slope_component = mul_div_floor(slope2_ps, above_kink, wad_minus_kink)?;
        base_rate_ps
            .checked_add(slope1_ps)
            .ok_or(MathError::Overflow)?
            .checked_add(slope_component)
            .ok_or(MathError::Overflow)?
    };
    Ok(r.min(max_rate_ps))
}

/// Third-order Taylor expansion of `e^x - 1`: `growth = x + x^2/(2*WAD) + x^3/(6*WAD^2)`, all
/// WAD-scaled (economic-model.md §4.2, ADR-0007). Under-approximates `e^x - 1` for `x >= 0`
/// (`P-IRM-2`), so it can never over-charge a borrower.
///
/// The cubic term is computed as two chained `mul_div_floor` calls — `t = floor(x*x/WAD)` then
/// `term3 = floor(t*x/(6*WAD))` — rather than a single three-factor product (`mul_div_*` only
/// takes two factors), which introduces one additional floor versus a true single-shot
/// `floor(x^3/(6*WAD^2))`. That extra floor only ever rounds *further down*, so it strengthens
/// (never weakens) the "never over-charges" property; it does not change the worked-example
/// result in `economic-model.md` §4.4, which is bit-for-bit identical either way (verified).
pub fn taylor3(x: u128) -> Result<u128, MathError> {
    let term2 = mul_div_floor(x, x, 2 * WAD)?;
    let x_squared_over_wad = mul_div_floor(x, x, WAD)?;
    let term3 = mul_div_floor(x_squared_over_wad, x, 6 * WAD)?;

    x.checked_add(term2)
        .and_then(|s| s.checked_add(term3))
        .ok_or(MathError::Overflow)
}

/// `x = r * dt` (WAD-scaled): `r` is already a WAD per-second rate, `dt` is a plain count of
/// elapsed seconds, so this is a plain checked multiply (no division, hence no `mul_div_*` call —
/// AGENTS.md's "every multiply-*divide*" rule does not apply to a bare multiply, but the result
/// still goes through `checked_mul`, never a wrapping one).
pub fn taylor_x(rate_per_second_wad: u128, dt_seconds: u64) -> Result<u128, MathError> {
    rate_per_second_wad
        .checked_mul(dt_seconds as u128)
        .ok_or(MathError::Overflow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixed::mul_div_ceil;
    use num_bigint::BigUint;

    const REF_SLOPE1: u128 = 1_268_391_679;
    const REF_SLOPE2: u128 = 31_709_791_983;
    const REF_U_KINK: u128 = 800_000_000_000_000_000;
    const REF_MAX_RATE: u128 = 317_097_919_837;

    // U-IRM-02 / E-03: total_supply_assets == 0 => u == 0, r == base_rate, interest == 0.
    #[test]
    fn zero_supply_gives_zero_utilization() {
        let u = utilization(0, 0).unwrap();
        assert_eq!(u, 0);
        let u2 = utilization(500, 0).unwrap();
        assert_eq!(
            u2, 0,
            "zero supply must yield u = 0 regardless of borrow amount"
        );
    }

    // U-IRM-04 / E-04: 100% utilization saturates u at WAD; with the reference IRM params, r at
    // u=100% is base+slope1+slope2 (~104% APR), below max_rate_ps -- this pins that value exactly.
    // The separate assertion below uses steeper slopes specifically to prove the `max_rate_ps` cap
    // itself fires (E-04's "r capped at max_rate_ps" requirement), which the reference params
    // alone do not reach.
    #[test]
    fn full_utilization_caps_at_wad_and_max_rate() {
        let u = utilization(1_000_000_000, 1_000_000_000).unwrap();
        assert_eq!(u, WAD);
        let r = borrow_rate(u, 0, REF_SLOPE1, REF_SLOPE2, REF_U_KINK, REF_MAX_RATE).unwrap();
        assert_eq!(
            r,
            REF_SLOPE1 + REF_SLOPE2,
            "at u=100% with the reference params, r = base + slope1 + slope2"
        );

        // Steep slopes that would exceed max_rate_ps if uncapped -- the cap must fire.
        let steep_slope1 = REF_MAX_RATE; // already at the cap
        let steep_slope2 = REF_MAX_RATE; // pushes well past it above the kink
        let r_capped =
            borrow_rate(u, 0, steep_slope1, steep_slope2, REF_U_KINK, REF_MAX_RATE).unwrap();
        assert_eq!(
            r_capped, REF_MAX_RATE,
            "rate at u=100% must be capped at max_rate_ps"
        );
    }

    // U-IRM-03: the exact worked example from economic-model.md §4.4.
    #[test]
    fn worked_example_ninety_percent_utilization_one_day() {
        let total_supply_assets: u64 = 1_000_000_000; // 1,000 USDC @ 6dp
        let total_borrow_assets: u64 = 900_000_000; // 900 USDC

        let u = utilization(total_borrow_assets, total_supply_assets).unwrap();
        assert_eq!(u, 900_000_000_000_000_000); // 0.9 WAD

        let r = borrow_rate(u, 0, REF_SLOPE1, REF_SLOPE2, REF_U_KINK, REF_MAX_RATE).unwrap();
        assert_eq!(r, 17_123_287_670);

        let dt = 86_400u64;
        let x = taylor_x(r, dt).unwrap();
        assert_eq!(x, 1_479_452_054_688_000);

        let growth = taylor3(x).unwrap();
        assert_eq!(growth, 1_480_546_983_577_839);

        let interest = mul_div_floor(total_borrow_assets as u128, growth, WAD).unwrap();
        assert_eq!(interest, 1_332_492);

        let fee = 100_000_000_000_000_000u128; // 0.10 WAD
        let fee_amount = mul_div_floor(interest, fee, WAD).unwrap();
        assert_eq!(fee_amount, 133_249);
    }

    // U-IRM-01 / E-02: dt = 0 => x = 0 => growth = 0 (the accrual no-op is asserted at the
    // Market::accrue_view level in `state/market.rs`; this pins the underlying math).
    #[test]
    fn zero_dt_gives_zero_growth() {
        let x = taylor_x(REF_MAX_RATE, 0).unwrap();
        assert_eq!(x, 0);
        assert_eq!(taylor3(x).unwrap(), 0);
    }

    // U-IRM-05 / INV-ACC-07 (math-level component): last_accrual_ts monotonicity is enforced by
    // the caller (`Market::accrue_mut` only ever sets it to `now >= last_accrual_ts`); this test
    // pins that `taylor_x` treats `dt` as a plain non-negative elapsed-seconds count with no
    // special-casing that could let a caller smuggle a negative interval through as a large u64.
    #[test]
    fn taylor_x_is_a_plain_product_of_rate_and_elapsed_seconds() {
        let x = taylor_x(1_000_000_000_000_000_000, 2).unwrap(); // r = 1 WAD/s, dt = 2s
        assert_eq!(x, 2_000_000_000_000_000_000); // 2 WAD
    }

    // U-ROUND-09: interest accrual -> interest added, floor (pinned directly on the primitive used
    // by `Market::accrue_view`; economic-model.md §4.2: `interest = mul_div_floor(total_borrow_assets, growth, WAD)`).
    #[test]
    fn round_09_interest_accrual_floors() {
        let total_borrow_assets = 7u128;
        let growth = 5 * WAD / 3; // chosen so the product has a nonzero remainder mod WAD
        let floor = mul_div_floor(total_borrow_assets, growth, WAD).unwrap();
        let ceil = mul_div_ceil(total_borrow_assets, growth, WAD).unwrap();
        assert!(ceil > floor, "fixture must have a nonzero remainder");
    }

    // P-IRM-1: r is monotone non-decreasing in u, holding all other params fixed.
    #[test]
    fn borrow_rate_is_monotone_in_utilization() {
        let mut prev = borrow_rate(0, 0, REF_SLOPE1, REF_SLOPE2, REF_U_KINK, REF_MAX_RATE).unwrap();
        let mut u = 0u128;
        while u < WAD {
            u = (u + WAD / 200).min(WAD); // 200 steps across [0, WAD]
            let r = borrow_rate(u, 0, REF_SLOPE1, REF_SLOPE2, REF_U_KINK, REF_MAX_RATE).unwrap();
            assert!(
                r >= prev,
                "rate must be non-decreasing: u={u} r={r} prev={prev}"
            );
            prev = r;
        }
    }

    // P-IRM-2: taylor3(x) <= e^x - 1 for x >= 0, checked against a high-precision (non-float)
    // reference: a much-higher-order (30-term) partial sum of the same non-negative-term Taylor
    // series for e^x - 1, computed with exact big-integer division. Because every term of that
    // series is non-negative for x >= 0, a partial sum with MORE terms is provably >= a partial
    // sum with fewer terms (taylor3 keeps only 3) — so this comparison is a rigorous lower bound
    // proof, not an approximate one, and it stays entirely float-free (CI-NOFLOAT).
    fn reference_taylor_n(x: u128, n: u32) -> BigUint {
        let x_big = BigUint::from(x);
        let mut sum = BigUint::from(0u32);
        let mut term = x_big.clone(); // term_1 = x
        for k in 1..=n {
            if k > 1 {
                // term_k = term_{k-1} * x / (k * WAD): one division per step (not two chained
                // ones), so no precision is lost prematurely for x < WAD -- each floor still only
                // rounds the term further down, so "more terms => provably >= taylor3" still holds.
                let denom = BigUint::from(k) * BigUint::from(WAD);
                term = (&term * &x_big) / &denom;
            }
            sum += &term;
        }
        sum
    }

    #[test]
    fn taylor3_never_exceeds_high_precision_reference() {
        // A representative sweep including the documented ~0.1 "full day at ~3650% APR" boundary
        // and values well beyond it (a dormant-market regime where the approximation is known to
        // diverge from e^x - 1, but must still never exceed it -- E-24 / P-IRM-3 territory).
        let xs = [
            0u128,
            1,
            WAD / 1_000_000, // tiny
            WAD / 10,        // the documented ~0.1 boundary
            WAD,             // x = 1
            5 * WAD,         // x = 5
            10 * WAD,        // x = 10 (~1000% APR sustained for a year at max_rate_ps)
        ];
        for x in xs {
            let approx = taylor3(x).unwrap();
            let reference = reference_taylor_n(x, 30);
            let approx_big = BigUint::from(approx);
            assert!(
                approx_big <= reference,
                "taylor3({x}) = {approx} must never exceed the high-precision reference {reference}"
            );
        }
    }

    // P-IRM-3: accrual over n steps of dt is <= accrual over one step of n*dt (sub-additivity of
    // the discount -- compounding more frequently at the same total elapsed time and average rate
    // can only ever under-accrue relative to one lump compounding step, since taylor3 discounts
    // more per call at larger x than the sum of the same discount applied piecewise). Checked here
    // at the x-domain level (growth is monotone increasing and convex in x for x >= 0, so
    // n * taylor3(x) <= taylor3(n * x)).
    #[test]
    fn accrual_over_n_steps_never_exceeds_one_lump_step() {
        let dt_step = 3600u64; // 1 hour
        let n = 24u128; // 24 steps = 1 day
        let r = REF_SLOPE1; // a representative, nonzero per-second rate

        let x_step = taylor_x(r, dt_step).unwrap();
        let growth_step = taylor3(x_step).unwrap();
        let n_steps_growth = growth_step.checked_mul(n).unwrap();

        let x_lump = taylor_x(r, dt_step * (n as u64)).unwrap();
        let lump_growth = taylor3(x_lump).unwrap();

        assert!(
            n_steps_growth <= lump_growth,
            "n * taylor3(x) ({n_steps_growth}) must be <= taylor3(n*x) ({lump_growth})"
        );
    }
}
