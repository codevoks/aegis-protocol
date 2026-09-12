//! Biased amount sampling (`docs/phases/phase-10-security.md` #11): uniform random amounts mostly
//! bounce off preconditions and waste the fuzz budget, so every amount the campaign generates is
//! drawn from a small, weighted menu of boundary-adjacent values relative to whatever bound is
//! currently relevant (free liquidity, a position's own debt/collateral, `min_debt`, ...).

use rand::Rng;

/// Draws one amount, heavily weighted toward the exact boundary of `bound` and its immediate
/// neighbors, with a small share of genuinely large/uniform draws for coverage.
///
/// `bound` is the tightest currently-relevant ceiling (e.g. free liquidity for a borrow, the
/// position's own collateral for a withdraw). `extra` is an additional boundary worth weighting
/// (e.g. `min_debt`), or `None` when not applicable.
pub fn biased_amount<R: Rng + ?Sized>(rng: &mut R, bound: u64, extra: Option<u64>) -> u64 {
    // A pool of candidate values, built fresh each call so it can react to the current bound.
    let mut candidates: Vec<u64> = vec![
        0,
        1,
        2, // dust
        bound,
        bound.saturating_sub(1),
        bound.saturating_add(1),
        bound / 2,
        bound.saturating_mul(99) / 100, // near-max-safe
    ];
    if let Some(e) = extra {
        candidates.push(e);
        candidates.push(e.saturating_sub(1));
        candidates.push(e.saturating_add(1));
    }
    // 15% of the time, fall back to a genuinely uniform draw up to 2x the bound (or a fixed large
    // constant when bound is 0) -- coverage for values the boundary menu would never produce.
    if rng.gen_bool(0.15) {
        let hi = bound.saturating_mul(2).max(1_000_000);
        return rng.gen_range(0..=hi);
    }
    let idx = rng.gen_range(0..candidates.len());
    candidates[idx]
}

/// As [`biased_amount`], but for a `u128` share-space bound. Not currently called by any
/// generated action (every action in this campaign drives assets, never a raw share amount), kept
/// as part of the sampler's public surface for a future share-denominated action.
#[allow(dead_code)]
pub fn biased_amount_u128<R: Rng + ?Sized>(rng: &mut R, bound: u128) -> u128 {
    let candidates: Vec<u128> = vec![
        0,
        1,
        2,
        bound,
        bound.saturating_sub(1),
        bound.saturating_add(1),
        bound / 2,
    ];
    if rng.gen_bool(0.15) {
        let hi = bound.saturating_mul(2).max(1_000_000);
        return rng.gen_range(0..=hi);
    }
    let idx = rng.gen_range(0..candidates.len());
    candidates[idx]
}

/// Biased `warp_time` delta, in seconds (`phase-10-security.md` #9): zero, one second, a small
/// delta, an interest-relevant delta (an hour/day), and an occasional long warp.
pub fn biased_time_delta<R: Rng + ?Sized>(rng: &mut R) -> i64 {
    const CANDIDATES: [i64; 7] = [
        0,
        1,
        30,           // small dt
        3_600,        // 1 hour -- interest-relevant
        86_400,       // 1 day
        7 * 86_400,   // 1 week
        365 * 86_400, // long dt -- a full year, the dormant-market boundary
    ];
    if rng.gen_bool(0.1) {
        return rng.gen_range(0..=(2 * 365 * 86_400_i64));
    }
    CANDIDATES[rng.gen_range(0..CANDIDATES.len())]
}

/// Biased price-move multiplier, expressed in basis points applied to the current price
/// (`phase-10-security.md` #10): unchanged, small moves, sharp drops/rises, and values chosen to
/// land near HF == WAD when combined with a market's current LTV/threshold parameters.
pub fn biased_price_move_bps<R: Rng + ?Sized>(rng: &mut R) -> i64 {
    const CANDIDATES: [i64; 9] = [
        0,      // unchanged
        -100,   // -1%
        100,    // +1%
        -1_000, // -10%
        1_000,  // +10%
        -3_000, // -30% -- sharp drop, likely liquidatable
        -500,   // -5% -- near max_ltv=0.75/LT=0.80 boundary for the reference market
        -2_000, // -20%
        -9_000, // -90% -- near-zero-collateral-value trajectory
    ];
    if rng.gen_bool(0.1) {
        return rng.gen_range(-9_900..=10_000);
    }
    CANDIDATES[rng.gen_range(0..CANDIDATES.len())]
}
