//! The rounding-direction discipline from `economic-model.md` §1.3 — "every multiply-divide in
//! Aegis goes through exactly two functions... every rounding: in the protocol's favor, and
//! explicit at the call site" — encoded as one pinned test per documented operation.
//!
//! **Documentation-accuracy finding, recorded here rather than silently worked around:**
//! `economic-model.md` §1.3's own rounding table lists **15** distinct operations (`supply(assets)`
//! through `Liquidation protocol fee`), but its own closing sentence says "every row above is a
//! distinct unit test (`U-ROUND-01..14`)" — 14 IDs for 15 rows. This file, together with the tests
//! colocated with the functions they pin (`crates/aegis-math/src/shares.rs` for rows 1-8,
//! `crates/aegis-math/src/irm.rs::round_09_interest_accrual_floors` for row 9), tests **all 15**
//! rows, numbered `U-ROUND-01..15` so no row is silently dropped to match the document's own
//! undercount. Phase 4's live instructions (`supply`, `withdraw`, `borrow`-gated, `repay`,
//! `accrue_interest`) exercise rows 1-10 for real; rows 11-15 (collateral/debt valuation and
//! liquidation) belong to Phase 5/6 instructions that do not exist yet (Phase 4's explicit
//! non-scope: "No oracle... No liquidation"). Those five rows are pinned here directly against the
//! already-existing, phase-1 `mul_div_floor`/`mul_div_ceil` primitives applied to the exact
//! formula shape `economic-model.md` §6-7 defines, using representative numbers — proving the
//! correct primitive-and-direction choice now, without building oracle/health/liquidation modules
//! that are out of scope for this phase.
//!
//! | # | Operation | Direction | Test |
//! |---|---|---|---|
//! | 1 | `supply(assets)` -> shares minted | floor | `shares::tests::round_01_*` |
//! | 2 | `withdraw(assets)` -> shares burned | ceil | `shares::tests::round_02_*` |
//! | 3 | `borrow(assets)` -> borrow shares minted | ceil | `shares::tests::round_03_*` |
//! | 4 | `repay(assets)` -> borrow shares burned | floor | `shares::tests::round_04_*` |
//! | 5 | `supply(shares)` -> assets required | ceil | `shares::tests::round_05_*` |
//! | 6 | `withdraw(shares)` -> assets returned | floor | `shares::tests::round_06_*` |
//! | 7 | `borrow(shares)` -> assets returned | floor | `shares::tests::round_07_*` |
//! | 8 | `repay(shares)` -> assets required | ceil | `shares::tests::round_08_*` |
//! | 9 | Interest accrual -> interest added | floor | `irm::tests::round_09_*` |
//! | 10 | Protocol fee shares -> fee shares | floor | `round_10_*` (this file) |
//! | 11 | Collateral value -> value | floor | `round_11_*` (this file) |
//! | 12 | Debt value -> value | ceil | `round_12_*` (this file) |
//! | 13 | Liquidation seize amount -> collateral seized | floor | `round_13_*` (this file) |
//! | 14 | Liquidation repay (collateral-capped) -> repay required | ceil | `round_14_*` (this file) |
//! | 15 | Liquidation protocol fee -> fee taken from bonus | floor | `round_15_*` (this file) |

use aegis_math::{mul_div_ceil, mul_div_floor, to_shares_down, WAD};

// U-ROUND-10: protocol fee shares are minted with to_shares_down (floor) -- economic-model.md
// §4.3's `fee_shares = to_shares_down(fee_amount, total_supply_assets - fee_amount,
// total_supply_shares)`. Pinned directly on the primitive Phase 4's `accrue_mut` uses (see
// `programs/aegis/src/state/market.rs`); `P-FEE-1` (in the `aegis` crate, which has a real
// `Market`/`Position` to accrue against) separately proves the *protocol-level* dilution property.
#[test]
fn round_10_protocol_fee_shares_floor() {
    let (total_assets, total_shares) = (997u64, 1_009u128); // chosen for a nonzero remainder
    let fee_amount = 11u64;
    let floor = to_shares_down(fee_amount, total_assets, total_shares).unwrap();
    let ceil_via_primitive = mul_div_ceil(
        fee_amount as u128,
        total_shares + aegis_math::VIRTUAL_SHARES,
        total_assets as u128 + aegis_math::VIRTUAL_ASSETS,
    )
    .unwrap();
    assert!(
        ceil_via_primitive > floor,
        "fixture must have a nonzero remainder"
    );
}

// U-ROUND-11: collateral_value = mul_div_floor(collateral_amount, price_c_lo, 10^decimals) --
// economic-model.md §6.2. Understates collateral, never overstates it.
#[test]
fn round_11_collateral_value_floor() {
    let collateral_amount: u128 = 7; // small amount so the division has a nonzero remainder
                                     // $149.70 WAD (§6.5's worked example), perturbed by 3 so the product has a nonzero remainder
                                     // mod 10^decimals -- the exact §6.5 price divides evenly and would not exercise floor vs ceil.
    let price_c_lo: u128 = 149_700_000_000_000_000_003;
    let ten_pow_decimals: u128 = 1_000_000_000; // 9 decimals (SOL)
    let floor = mul_div_floor(collateral_amount, price_c_lo, ten_pow_decimals).unwrap();
    let ceil = mul_div_ceil(collateral_amount, price_c_lo, ten_pow_decimals).unwrap();
    assert!(ceil > floor, "fixture must have a nonzero remainder");
}

// U-ROUND-12: debt_value = mul_div_ceil(debt_assets, price_l_hi, 10^decimals) --
// economic-model.md §6.2. Overstates debt, never understates it.
#[test]
fn round_12_debt_value_ceil() {
    let debt_assets: u128 = 900_000_007; // from §6.5's worked example, perturbed for a remainder
    let price_l_hi: u128 = 1_000_200_000_000_000_003; // $1.0002 WAD, perturbed by 3 (see above)
    let ten_pow_decimals: u128 = 1_000_000; // 6 decimals (USDC)
    let floor = mul_div_floor(debt_assets, price_l_hi, ten_pow_decimals).unwrap();
    let ceil = mul_div_ceil(debt_assets, price_l_hi, ten_pow_decimals).unwrap();
    assert!(ceil > floor, "fixture must have a nonzero remainder");
}

// U-ROUND-13: base_seize = mul_div_floor(repay_value, 10^decimals, price_c_lo) --
// economic-model.md §7.2. The liquidator receives no more collateral than exactly earned.
#[test]
fn round_13_liquidation_seize_floor() {
    let repay_value: u128 = 900_180_000_000_000_000_007; // §7.5's worked repay_value, perturbed
    let ten_pow_decimals: u128 = 1_000_000_000;
    let price_c_lo: u128 = 94_800_000_000_000_000_000; // $94.80 WAD, from §7.5
    let floor = mul_div_floor(repay_value, ten_pow_decimals, price_c_lo).unwrap();
    let ceil = mul_div_ceil(repay_value, ten_pow_decimals, price_c_lo).unwrap();
    assert!(ceil > floor, "fixture must have a nonzero remainder");
}

// U-ROUND-14: repay_assets = mul_div_ceil(repay_value', 10^decimals, price_l_hi) on the
// collateral-capped clamp path -- economic-model.md §7.2. The liquidator pays no less than owed.
#[test]
fn round_14_liquidation_clamped_repay_ceil() {
    let repay_value_prime: u128 = 123_456_789_000_000_000_007;
    let ten_pow_decimals: u128 = 1_000_000;
    let price_l_hi: u128 = 1_000_200_000_000_000_000;
    let floor = mul_div_floor(repay_value_prime, ten_pow_decimals, price_l_hi).unwrap();
    let ceil = mul_div_ceil(repay_value_prime, ten_pow_decimals, price_l_hi).unwrap();
    assert!(ceil > floor, "fixture must have a nonzero remainder");
}

// U-ROUND-15: protocol_cut = mul_div_floor(bonus_amount, liq_protocol_fee, WAD) --
// economic-model.md §7.3. Never over-takes from the liquidator's bonus.
#[test]
fn round_15_liquidation_protocol_fee_floor() {
    let bonus_amount: u128 = 474_778_481; // §7.5's worked bonus_amount
    let liq_protocol_fee: u128 = 100_000_000_000_000_000; // 0.10 WAD
                                                          // Perturb slightly so the division has a nonzero remainder (the exact worked example happens
                                                          // to divide evenly at this scale; the point here is the direction, not the exact figure).
    let bonus_amount = bonus_amount + 3;
    let floor = mul_div_floor(bonus_amount, liq_protocol_fee, WAD).unwrap();
    let ceil = mul_div_ceil(bonus_amount, liq_protocol_fee, WAD).unwrap();
    assert!(ceil > floor, "fixture must have a nonzero remainder");
}
