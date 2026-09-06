//! `P-SHARE-1..4` (`economic-model.md` §10) — round-tripping through the share/asset conversions
//! must never manufacture value, over a wide sweep of tiny, large, near-zero, high-share-price and
//! low-share-price states (`docs/phases/phase-04-lending.md` §3 "boundary/property testing").

use aegis_math::{to_assets_down, to_assets_up, to_shares_down, to_shares_up};
use proptest::prelude::*;

// Bounded so total_assets + total_shares combinations stay within what a real market could ever
// reach (u64::MAX assets, and shares bounded well below the point where the *inputs themselves*
// already overflow u128 -- P-ARITH-3 / `to_assets_survives_maximum_legal_share_asset_state`
// separately proves the 256-bit intermediate survives the true maximum legal state).
fn total_assets_strategy() -> impl Strategy<Value = u64> {
    prop_oneof![
        Just(0u64),
        1..=1_000u64,
        1_000_000..=1_000_000_000_000u64,
        (u64::MAX - 1_000)..=u64::MAX,
    ]
}

fn total_shares_strategy() -> impl Strategy<Value = u128> {
    prop_oneof![
        Just(0u128),
        1..=1_000u128,
        1_000_000..=1_000_000_000_000_000_000u128,
    ]
}

fn amount_strategy() -> impl Strategy<Value = u64> {
    prop_oneof![
        Just(0u64),
        Just(1u64),
        1..=1_000_000_000u64,
        (u64::MAX - 100)..=u64::MAX,
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(4096))]

    // P-SHARE-1: to_assets_down(to_shares_down(a, T, S), T, S) <= a -- round-tripping through the
    // floor direction on both legs never creates value for the depositor.
    #[test]
    fn p_share_1_round_trip_never_creates_value(
        a in amount_strategy(),
        total_assets in total_assets_strategy(),
        total_shares in total_shares_strategy(),
    ) {
        if let Ok(shares) = to_shares_down(a, total_assets, total_shares) {
            let total_assets_after = total_assets.checked_add(a);
            let total_shares_after = total_shares.checked_add(shares);
            if let (Some(ta), Some(ts)) = (total_assets_after, total_shares_after) {
                if let Ok(assets_back) = to_assets_down(shares, ta, ts) {
                    prop_assert!(assets_back <= a, "round-trip must never return more than {a}, got {assets_back}");
                }
            }
        }
    }

    // P-SHARE-2: to_shares_up(to_assets_up(s, T, S), T, S) >= s -- round-tripping the other
    // direction never lets a caller redeem more value than their share count actually represents.
    #[test]
    fn p_share_2_round_trip_never_undercounts_shares(
        s in 0u128..=1_000_000_000_000_000_000u128,
        total_assets in total_assets_strategy(),
        total_shares in total_shares_strategy(),
    ) {
        if let Ok(assets) = to_assets_up(s, total_assets, total_shares) {
            if let Ok(shares_back) = to_shares_up(assets, total_assets, total_shares) {
                prop_assert!(shares_back >= s, "round-trip must never require fewer than {s} shares, got {shares_back}");
            }
        }
    }

    // P-SHARE-3: supply(assets=a) then immediately withdraw(shares=<all just-minted shares>) never
    // returns more than was supplied.
    #[test]
    fn p_share_3_supply_then_withdraw_never_profits(
        a in amount_strategy(),
        total_assets in total_assets_strategy(),
        total_shares in total_shares_strategy(),
    ) {
        if let Ok(minted_shares) = to_shares_down(a, total_assets, total_shares) {
            if let (Some(ta), Some(ts)) = (total_assets.checked_add(a), total_shares.checked_add(minted_shares)) {
                // withdraw(shares) rounds down (economic-model.md §1.3 row 6).
                if let Ok(returned) = to_assets_down(minted_shares, ta, ts) {
                    prop_assert!(returned <= a, "supply-then-withdraw must never return more than supplied ({a}), got {returned}");
                }
            }
        }
    }

    // P-SHARE-4: borrow(assets=a) then immediately repay(shares=<all just-minted borrow shares>)
    // never requires less than was borrowed (the debt is never under-collected).
    #[test]
    fn p_share_4_borrow_then_repay_never_undercollects(
        a in amount_strategy(),
        total_borrow_assets in total_assets_strategy(),
        total_borrow_shares in total_shares_strategy(),
    ) {
        // borrow(assets) mints shares with ceil (row 3).
        if let Ok(minted_shares) = to_shares_up(a, total_borrow_assets, total_borrow_shares) {
            if let (Some(ta), Some(ts)) = (
                total_borrow_assets.checked_add(a),
                total_borrow_shares.checked_add(minted_shares),
            ) {
                // repay(shares) requires assets with ceil (row 8).
                if let Ok(required) = to_assets_up(minted_shares, ta, ts) {
                    prop_assert!(required >= a, "borrow-then-repay must never require less than borrowed ({a}), got {required}");
                }
            }
        }
    }
}
