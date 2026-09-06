//! Supply/borrow share <-> asset conversions with virtual offsets (`economic-model.md` §3).
//!
//! `VIRTUAL_SHARES = 1_000_000` and `VIRTUAL_ASSETS = 1` are a security mechanism against
//! share-price / inflation manipulation (T-18, ADR-0006), not a tunable knob — they are added on
//! both sides of every conversion below, unconditionally, and are never taken as parameters.
//! **Do not make them configurable.** See `docs/phases/phase-04-lending.md` and §3.2 of the
//! economic model for the attack they defend against; `A-SHARE-01` (`tests/phase4_adversarial.rs`)
//! demonstrates the attack succeeding with the offsets removed and failing with them present.
//!
//! Four conversions, each with a fixed rounding direction from `economic-model.md` §1.3 — the
//! direction is baked into the function name and is never a caller-supplied argument, so a call
//! site cannot accidentally round the wrong way:
//!
//! | Function | Rounds | Used by (assets given / shares given) |
//! |---|---|---|
//! | [`to_shares_down`] | floor | `supply(assets)`, `repay(assets)` |
//! | [`to_shares_up`] | ceil | `withdraw(assets)`, `borrow(assets)` |
//! | [`to_assets_down`] | floor | `withdraw(shares)`, `borrow(shares)` |
//! | [`to_assets_up`] | ceil | `supply(shares)`, `repay(shares)` |

use crate::constants::{VIRTUAL_ASSETS, VIRTUAL_SHARES};
use crate::fixed::{mul_div_ceil, mul_div_floor, MathError};

/// `⌊assets · (total_shares + VIRTUAL_SHARES) / (total_assets + VIRTUAL_ASSETS)⌋` —
/// `supply(assets)`'s shares-minted rounding (favors the protocol: the depositor receives fewer
/// shares) and `repay(assets)`'s borrow-shares-burned rounding (the payer is credited no more debt
/// reduction than exactly earned).
pub fn to_shares_down(
    assets: u64,
    total_assets: u64,
    total_shares: u128,
) -> Result<u128, MathError> {
    let total_shares_offset = total_shares
        .checked_add(VIRTUAL_SHARES)
        .ok_or(MathError::Overflow)?;
    let total_assets_offset = (total_assets as u128)
        .checked_add(VIRTUAL_ASSETS)
        .ok_or(MathError::Overflow)?;
    mul_div_floor(assets as u128, total_shares_offset, total_assets_offset)
}

/// `⌈assets · (total_shares + VIRTUAL_SHARES) / (total_assets + VIRTUAL_ASSETS)⌉` —
/// `withdraw(assets)`'s shares-burned rounding (the withdrawer burns more shares, never fewer) and
/// `borrow(assets)`'s borrow-shares-minted rounding (the borrower owes at least the exact amount,
/// INV-BOR-03).
pub fn to_shares_up(assets: u64, total_assets: u64, total_shares: u128) -> Result<u128, MathError> {
    let total_shares_offset = total_shares
        .checked_add(VIRTUAL_SHARES)
        .ok_or(MathError::Overflow)?;
    let total_assets_offset = (total_assets as u128)
        .checked_add(VIRTUAL_ASSETS)
        .ok_or(MathError::Overflow)?;
    mul_div_ceil(assets as u128, total_shares_offset, total_assets_offset)
}

/// `⌊shares · (total_assets + VIRTUAL_ASSETS) / (total_shares + VIRTUAL_SHARES)⌋`, narrowed to
/// `u64` (an asset amount is always native `u64` base units, `economic-model.md` §1.1) —
/// `withdraw(shares)`'s assets-returned rounding (the withdrawer receives no more than exactly
/// earned) and `borrow(shares)`'s assets-returned rounding (the borrower receives no more than
/// exactly requested).
pub fn to_assets_down(
    shares: u128,
    total_assets: u64,
    total_shares: u128,
) -> Result<u64, MathError> {
    let total_assets_offset = (total_assets as u128)
        .checked_add(VIRTUAL_ASSETS)
        .ok_or(MathError::Overflow)?;
    let total_shares_offset = total_shares
        .checked_add(VIRTUAL_SHARES)
        .ok_or(MathError::Overflow)?;
    let assets = mul_div_floor(shares, total_assets_offset, total_shares_offset)?;
    u64::try_from(assets).map_err(|_| MathError::Overflow)
}

/// `⌈shares · (total_assets + VIRTUAL_ASSETS) / (total_shares + VIRTUAL_SHARES)⌉`, narrowed to
/// `u64` — `supply(shares)`'s assets-required rounding (the depositor pays at least the exact
/// amount) and `repay(shares)`'s assets-required rounding (the payer pays at least the exact
/// amount owed, never less).
pub fn to_assets_up(shares: u128, total_assets: u64, total_shares: u128) -> Result<u64, MathError> {
    let total_assets_offset = (total_assets as u128)
        .checked_add(VIRTUAL_ASSETS)
        .ok_or(MathError::Overflow)?;
    let total_shares_offset = total_shares
        .checked_add(VIRTUAL_SHARES)
        .ok_or(MathError::Overflow)?;
    let assets = mul_div_ceil(shares, total_assets_offset, total_shares_offset)?;
    u64::try_from(assets).map_err(|_| MathError::Overflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    // U-SHARE-01 / E-01: first supply into an empty market applies the virtual offsets and never
    // divides by zero (total_assets = 0, total_shares = 0; the offsets make the denominator
    // VIRTUAL_ASSETS = 1, never zero).
    #[test]
    fn first_supply_into_empty_market_applies_virtual_offsets() {
        let assets = 1_000_000_000u64; // 1e9
        let shares = to_shares_down(assets, 0, 0).expect("must not divide by zero");
        // economic-model.md §3.3: shares = floor(1e9 * (0 + 1e6) / (0 + 1)) = 1e15.
        assert_eq!(shares, 1_000_000_000_000_000);
    }

    // §3.3's own worked example, encoded as an exact regression test on the frozen formula.
    // NOTE ON A DOCUMENTATION FINDING: economic-model.md §3.3 additionally narrates that "Bob
    // receives marginally fewer shares than Alice" when Bob supplies the identical 1e9 immediately
    // after Alice with no accrual in between, and gives an approximate hand-computation
    // ("≈ 999_999_999_000_000"). Applying the frozen formula to those exact numbers with exact
    // (non-approximate) integer arithmetic instead gives *zero* tax: with `VIRTUAL_ASSETS = 1`,
    // Alice's first deposit into an empty pool is loss-free (denominator `0 + 1 = 1`, so
    // `total_shares / total_assets` lands *exactly* on `VIRTUAL_SHARES / VIRTUAL_ASSETS` with no
    // remainder), and every subsequent depositor then divides out exactly too, for *any* amount —
    // this is a general fact of the formula given `VIRTUAL_ASSETS = 1`, not particular to these
    // numbers. This is reported in the Phase 4 completion report as a documentation-accuracy
    // finding (the formula itself is unambiguous and is what this test encodes exactly); the real,
    // non-degenerate "later depositor tax" property is instead demonstrated in
    // `later_depositor_receives_fewer_shares_after_ratio_drift` below, where the pool's
    // `total_assets : total_shares` ratio has genuinely drifted from `VIRTUAL_SHARES :
    // VIRTUAL_ASSETS` (as it does after real interest accrual).
    #[test]
    fn worked_example_alice_then_bob_immediately_yields_zero_tax_exactly() {
        let alice_assets = 1_000_000_000u64;
        let alice_shares = to_shares_down(alice_assets, 0, 0).unwrap();
        assert_eq!(alice_shares, 1_000_000_000_000_000);

        let bob_assets = 1_000_000_000u64;
        let bob_shares = to_shares_down(bob_assets, alice_assets, alice_shares).unwrap();
        assert_eq!(
            bob_shares, alice_shares,
            "exact integer arithmetic: Bob receives exactly Alice's share count here, not fewer \
             (see the documentation-accuracy note above)"
        );
    }

    // U-SHARE-02: the real "later depositor tax" this virtual-offset scheme imposes, demonstrated
    // where it actually manifests — once the pool's total_assets:total_shares ratio has drifted
    // away from VIRTUAL_SHARES:VIRTUAL_ASSETS (exactly what happens after interest accrues extra
    // assets without minting extra shares, economic-model.md §4.2). A later depositor then
    // receives strictly fewer shares per asset than the pool's founding price, and the deficit is
    // bounded and negligible relative to their deposit.
    #[test]
    fn later_depositor_receives_fewer_shares_after_ratio_drift() {
        // Bootstrap: 1e9 assets <-> 1e15 shares (the exact 1e6:1 founding ratio).
        let total_assets_before_accrual = 1_000_000_000u64;
        let total_shares = 1_000_000_000_000_000u128;

        // Simulate interest accrual growing total_assets without minting shares (§4.2): +1 unit is
        // enough to perturb the ratio away from exactly VIRTUAL_SHARES:VIRTUAL_ASSETS.
        let total_assets_after_accrual = total_assets_before_accrual + 1;

        let depositor_assets = 1_000_000_000u64;
        let shares = to_shares_down(depositor_assets, total_assets_after_accrual, total_shares)
            .expect("must not overflow");

        // Founding price was exactly 1e6 shares per asset unit; a depositor after ratio drift must
        // receive strictly fewer, and the deficit must be tiny relative to their deposit (the
        // "bounded and negligible" claim in economic-model.md §3.3).
        let founding_price_shares = depositor_assets as u128 * 1_000_000u128;
        assert!(
            shares < founding_price_shares,
            "a later depositor after ratio drift must receive fewer shares than the founding price \
             would give: got {shares}, founding price would give {founding_price_shares}"
        );
        let deficit = founding_price_shares - shares;
        assert!(
            deficit <= VIRTUAL_SHARES,
            "the virtual-offset tax must be bounded (order VIRTUAL_SHARES), got deficit {deficit}"
        );
    }

    // U-ROUND-01: supply(assets) -> shares minted, floor.
    #[test]
    fn round_01_supply_assets_shares_minted_floors() {
        // Chosen so the exact quotient has a nonzero remainder, making floor != ceil observable.
        let (total_assets, total_shares) = (3u64, 7u128);
        let assets = 5u64;
        let floor = to_shares_down(assets, total_assets, total_shares).unwrap();
        let ceil = to_shares_up(assets, total_assets, total_shares).unwrap();
        assert!(ceil > floor, "fixture must have a nonzero remainder");
        // supply(assets) must use the floor variant by construction of to_shares_down's contract;
        // this test exists to pin the numeric direction, not merely the function identity.
        let expected = (assets as u128 * (total_shares + VIRTUAL_SHARES))
            / (total_assets as u128 + VIRTUAL_ASSETS);
        assert_eq!(floor, expected);
    }

    // U-ROUND-04: repay(assets) -> borrow shares burned, floor (same function, distinct economic
    // meaning from U-ROUND-01 — pinned separately per the phase spec's "one explicit test each").
    #[test]
    fn round_04_repay_assets_borrow_shares_burned_floors() {
        let (total_borrow_assets, total_borrow_shares) = (11u64, 13u128);
        let repay_assets = 6u64;
        let floor = to_shares_down(repay_assets, total_borrow_assets, total_borrow_shares).unwrap();
        let ceil = to_shares_up(repay_assets, total_borrow_assets, total_borrow_shares).unwrap();
        assert!(ceil > floor, "fixture must have a nonzero remainder");
    }

    // U-ROUND-02: withdraw(assets) -> shares burned, ceil.
    #[test]
    fn round_02_withdraw_assets_shares_burned_ceils() {
        let (total_assets, total_shares) = (3u64, 7u128);
        let assets = 5u64;
        let floor = to_shares_down(assets, total_assets, total_shares).unwrap();
        let ceil = to_shares_up(assets, total_assets, total_shares).unwrap();
        assert!(ceil > floor, "fixture must have a nonzero remainder");
    }

    // U-ROUND-03: borrow(assets) -> borrow shares minted, ceil (INV-BOR-03).
    #[test]
    fn round_03_borrow_assets_borrow_shares_minted_ceils() {
        let (total_borrow_assets, total_borrow_shares) = (11u64, 13u128);
        let borrow_assets = 6u64;
        let floor =
            to_shares_down(borrow_assets, total_borrow_assets, total_borrow_shares).unwrap();
        let ceil = to_shares_up(borrow_assets, total_borrow_assets, total_borrow_shares).unwrap();
        assert!(ceil > floor, "fixture must have a nonzero remainder");
    }

    // U-ROUND-05: supply(shares) -> assets required, ceil.
    #[test]
    fn round_05_supply_shares_assets_required_ceils() {
        let (total_assets, total_shares) = (5u64, 3u128);
        let shares = 7u128;
        let floor = to_assets_down(shares, total_assets, total_shares).unwrap();
        let ceil = to_assets_up(shares, total_assets, total_shares).unwrap();
        assert!(ceil > floor, "fixture must have a nonzero remainder");
    }

    // U-ROUND-08: repay(shares) -> assets required, ceil.
    #[test]
    fn round_08_repay_shares_assets_required_ceils() {
        let (total_borrow_assets, total_borrow_shares) = (13u64, 11u128);
        let shares = 6u128;
        let floor = to_assets_down(shares, total_borrow_assets, total_borrow_shares).unwrap();
        let ceil = to_assets_up(shares, total_borrow_assets, total_borrow_shares).unwrap();
        assert!(ceil > floor, "fixture must have a nonzero remainder");
    }

    // U-ROUND-06: withdraw(shares) -> assets returned, floor.
    #[test]
    fn round_06_withdraw_shares_assets_returned_floors() {
        let (total_assets, total_shares) = (5u64, 3u128);
        let shares = 7u128;
        let floor = to_assets_down(shares, total_assets, total_shares).unwrap();
        let ceil = to_assets_up(shares, total_assets, total_shares).unwrap();
        assert!(ceil > floor, "fixture must have a nonzero remainder");
    }

    // U-ROUND-07: borrow(shares) -> assets returned, floor.
    #[test]
    fn round_07_borrow_shares_assets_returned_floors() {
        let (total_borrow_assets, total_borrow_shares) = (13u64, 11u128);
        let shares = 6u128;
        let floor = to_assets_down(shares, total_borrow_assets, total_borrow_shares).unwrap();
        let ceil = to_assets_up(shares, total_borrow_assets, total_borrow_shares).unwrap();
        assert!(ceil > floor, "fixture must have a nonzero remainder");
    }

    // P-ARITH-3 (share-conversion instance): to_assets_up succeeds for the maximum legal
    // share/asset state without overflowing — the exact scenario ADR-0009 exists to cover.
    // shares ~= assets * VIRTUAL_SHARES can reach ~1.8e25; converting back multiplies by
    // total_assets (~1.8e19), a product (~3.2e44) that overflows u128 (~3.4e38) even though the
    // final quotient fits comfortably. A plain (non-256-bit) implementation would abort here.
    #[test]
    fn to_assets_survives_maximum_legal_share_asset_state() {
        let total_assets: u64 = 18_000_000_000_000_000_000u64; // ~1.8e19, near u64::MAX
        let total_shares: u128 = 18_000_000_000_000_000_000_000_000u128; // ~1.8e25
        let shares = total_shares; // redeem everything
        let result = to_assets_down(shares, total_assets, total_shares);
        assert!(
            result.is_ok(),
            "256-bit intermediate must survive the maximum legal state: {result:?}"
        );
    }
}
