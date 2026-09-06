//! `A-SHARE-01` — the first-depositor share-inflation attack (T-18, `economic-model.md` §3.2,
//! ADR-0006), run against the *raw share-math formula* with the virtual offsets parameterized to
//! zero (simulating "no defense") and then with Aegis's real, frozen constants
//! (`VIRTUAL_SHARES = 1_000_000`, `VIRTUAL_ASSETS = 1`) — never by weakening
//! `crates/aegis-math/src/shares.rs`'s production functions, which hardcode the real constants and
//! are not parameterized (by design; see that module's doc comment).
//!
//! Real Aegis additionally closes the *direct-donation* variant of this attack entirely via
//! INV-CUS-08 (unsolicited vault transfers are never credited to `total_supply_assets` at all —
//! proven in Phase 3's `A-CUS-08`/`direct_donation_is_never_credited`), independent of virtual
//! offsets. This test isolates the share-math defense on its own terms by asking: *if* an attacker
//! could grow `total_assets` by some amount without minting shares for it (exactly what real
//! interest accrual on a real borrow legitimately does — the variant the offsets exist to defend,
//! `economic-model.md` §3.2's closing sentence), what happens to their economics with and without
//! the offsets?

/// A local, test-only reimplementation of the share formula parameterized by virtual offsets, so
/// this file can compare "offsets disabled" vs "real offsets" without making the production
/// `to_shares_down`/`to_assets_down` functions configurable (which the frozen design forbids).
fn to_shares_down_parameterized(
    assets: u128,
    total_assets: u128,
    total_shares: u128,
    virtual_shares: u128,
    virtual_assets: u128,
) -> u128 {
    let numerator = assets * (total_shares + virtual_shares);
    let denominator = total_assets + virtual_assets;
    numerator / denominator
}

fn to_assets_down_parameterized(
    shares: u128,
    total_assets: u128,
    total_shares: u128,
    virtual_shares: u128,
    virtual_assets: u128,
) -> u128 {
    let numerator = shares * (total_assets + virtual_assets);
    let denominator = total_shares + virtual_shares;
    numerator / denominator
}

struct AttackResult {
    attacker_profit_i128: i128,
    victim_loss: u128,
}

/// Runs the identical attack sequence — attacker supplies 1 unit, "inflates" `total_assets` by
/// `donation` (standing in for the interest-accrual variant, since the literal donation path is
/// already closed by INV-CUS-08 regardless of offsets), then a victim supplies `victim_supply`,
/// then the attacker redeems their shares — under the given virtual-offset parameters.
fn run_attack(
    virtual_shares: u128,
    virtual_assets: u128,
    donation: u128,
    victim_supply: u128,
    bootstrap_1_to_1_when_offsets_are_zero: bool,
) -> AttackResult {
    let attacker_supply: u128 = 1;

    let attacker_shares = if bootstrap_1_to_1_when_offsets_are_zero {
        // With true zero offsets, the very first deposit into an empty market is 0/0
        // (economic-model.md §3.2: "without virtual offsets, an empty market is attackable" --
        // the classic ERC4626-style attack conventionally bootstraps the first deposit 1:1).
        attacker_supply
    } else {
        to_shares_down_parameterized(attacker_supply, 0, 0, virtual_shares, virtual_assets)
    };
    let mut total_assets = attacker_supply;
    let mut total_shares = attacker_shares;

    // Inflate total_assets without minting shares (the interest-accrual variant).
    total_assets += donation;

    let victim_shares = to_shares_down_parameterized(
        victim_supply,
        total_assets,
        total_shares,
        virtual_shares,
        virtual_assets,
    );
    total_assets += victim_supply;
    total_shares += victim_shares;

    let attacker_redeem = to_assets_down_parameterized(
        attacker_shares,
        total_assets,
        total_shares,
        virtual_shares,
        virtual_assets,
    );
    let attacker_cost = attacker_supply + donation;
    let attacker_profit_i128 = attacker_redeem as i128 - attacker_cost as i128;

    let remaining_assets = total_assets - attacker_redeem;
    let remaining_shares = total_shares - attacker_shares;
    let victim_value = if remaining_shares > 0 {
        to_assets_down_parameterized(
            victim_shares,
            remaining_assets,
            remaining_shares,
            virtual_shares,
            virtual_assets,
        )
    } else {
        0
    };
    let victim_loss = victim_supply.saturating_sub(victim_value);

    AttackResult {
        attacker_profit_i128,
        victim_loss,
    }
}

// A-SHARE-01: the attack succeeds (strictly profitable) with virtual offsets disabled, and is
// unprofitable (a net loss for the attacker) with Aegis's real, frozen offsets -- same attacker
// capital, same victim deposit, in both branches.
#[test]
fn a_share_01_inflation_attack_without_vs_with_virtual_offsets() {
    let donation: u128 = 1_000_000_000; // 1e9 base units, economic-model.md §3.2's own figure
    let victim_supply: u128 = 1_500_000_000; // 1.5e9, §3.2's own figure

    // --- Branch 1: no virtual offsets ---
    let without_offsets = run_attack(0, 0, donation, victim_supply, true);
    assert_eq!(
        without_offsets.attacker_profit_i128, 249_999_999,
        "without virtual offsets, the attacker must profit by stealing (almost) exactly the \
         victim's loss -- matches economic-model.md §3.2's narrative ('stealing 0.25e9')"
    );
    assert_eq!(without_offsets.victim_loss, 249_999_999);
    assert!(
        without_offsets.attacker_profit_i128 > 0,
        "A-SHARE-01: the attack must SUCCEED (be strictly profitable) with offsets disabled"
    );

    // --- Branch 2: Aegis's real, frozen virtual offsets ---
    let with_offsets = run_attack(
        aegis_math::VIRTUAL_SHARES,
        aegis_math::VIRTUAL_ASSETS,
        donation,
        victim_supply,
        false,
    );
    assert_eq!(
        with_offsets.attacker_profit_i128, -499_999_901,
        "with the real virtual offsets, the identical attack must be a net LOSS for the attacker"
    );
    assert_eq!(
        with_offsets.victim_loss, 199,
        "the victim's loss collapses to negligible dust (199 base units) with the real offsets, \
         versus 249_999_999 without them"
    );
    assert!(
        with_offsets.attacker_profit_i128 < 0,
        "A-SHARE-01: the attack must be UNPROFITABLE with the real virtual offsets"
    );

    // The core comparative claim: the attacker's outcome with offsets is dramatically worse than
    // without them, for the identical capital outlay and victim deposit.
    assert!(with_offsets.attacker_profit_i128 < without_offsets.attacker_profit_i128);
}
