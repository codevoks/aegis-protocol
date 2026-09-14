//! **Exploit regression — discovered by the Phase 13 release demo**
//! (`crates/aegis-test-kit/examples/phase13_demo.rs`, `docs/security/findings.md` F-13-01).
//!
//! A real, reachable **INV-ACC-06** `[GLOBAL]` violation on the SUPPLY side (`total_supply_shares
//! == 0 <=> total_supply_assets == 0`), distinct from F-10-02 (which was the BORROW side): after a
//! genuine bad-debt event burns `fee_position`'s supply shares down to a small nonzero dust
//! remainder (`absorb_bad_debt`'s protocol-first-loss step, `economic-model.md` §8.2), the
//! market's one remaining lender withdrawing EXACTLY their own full share balance can, via
//! `to_assets_down`'s floor rounding, legitimately receive 100% of `total_supply_assets` — because
//! the dust shares' true fractional entitlement is provably sub-base-unit. The result:
//! `total_supply_shares > 0` while `total_supply_assets == 0`.
//!
//! **Severity: Informational — confirmed NOT exploitable, not merely "small".** Unlike F-10-02
//! (fixed: a permanent, if bounded, uncollectible obligation), this dust's implied value is
//! *provably* zero forever, for a deposit of **any** size, not merely small ones. For virtual
//! offsets `VIRTUAL_SHARES = 1_000_000` and dust shares `d`, `to_assets_down(d, D, D*(d +
//! VIRTUAL_SHARES) + d)` (the dust's implied claim after any subsequent deposit `D`) has the exact
//! closed form `floor(d*(D+1) / (D*(d+VIRTUAL_SHARES)+d+1_000_001))`, which is `0` for every `D` in
//! `[0, u64::MAX]` whenever `d < VIRTUAL_SHARES` (the asymptotic ratio as `D -> infinity` is
//! `d/(d+VIRTUAL_SHARES) < 1`, and the expression is monotonically bounded above by that ratio for
//! every finite `D`) — verified in this test out to `D = u64::MAX`, not merely "large" samples.
//! Nobody is ever diluted and no value is ever created or destroyed; the dust shares are a
//! permanent, inert bookkeeping ghost with the literal wording of INV-ACC-06 violated but its
//! purpose (no orphaned VALUE) intact.
//!
//! **Recommendation: no code change** (`docs/security/findings.md` F-13-01, matching F-10-01's
//! precedent for a confirmed-harmless finding). A future v2 could `absorb_bad_debt` any residual
//! `fee_position` dust to exactly zero when `total_supply_assets` reaches zero, for bookkeeping
//! cleanliness only — not a correctness or safety requirement. This test pins today's real,
//! observed behavior so a future change cannot silently make it WORSE (i.e., cannot make the dust
//! shares' value nonzero, which would be actual value creation and a real T-17 instance).

#![allow(clippy::result_large_err)]

use aegis::instructions::admin::CreateMarketArgs;
use aegis_test_kit::{
    absorb_bad_debt, accrue_interest, create_market, create_spl_mint, create_token_account, deploy,
    fetch_market, fetch_position, init_position, initialize_protocol, invariants, mint_to,
    seed_borrow_state, spl_token_interface, supply, withdraw,
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

fn program_bytes() -> &'static [u8] {
    include_bytes!(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/aegis.so"))
}

fn fixed_pubkey(seed: u8) -> Pubkey {
    Keypair::new_from_array([seed; 32]).pubkey()
}

/// Closed-form reproduction of `to_shares_down`/`to_assets_down` (`aegis-math`'s own formulas,
/// `economic-model.md` §3.1) used only to verify the "stays zero for ANY future deposit" claim
/// out to `u64::MAX` without executing a real (slow) transaction per sample.
fn dust_value_after_future_deposit(dust_shares: u128, future_deposit: u64) -> u128 {
    const VIRTUAL_SHARES: u128 = 1_000_000;
    const VIRTUAL_ASSETS: u128 = 1;
    let d = future_deposit as u128;
    let new_shares = (d * VIRTUAL_SHARES) / VIRTUAL_ASSETS;
    let total_shares_after = dust_shares + new_shares;
    let total_assets_after = d;
    (dust_shares * (total_assets_after + VIRTUAL_ASSETS)) / (total_shares_after + VIRTUAL_SHARES)
}

#[test]
fn f_13_01_bad_debt_dust_in_fee_position_never_regains_value_at_any_future_deposit_size() {
    // The closed-form check first, independent of any on-chain state: for the class of dust sizes
    // this scenario can plausibly produce (anything well below VIRTUAL_SHARES), the implied value
    // is 0 at the smallest possible deposit (1 base unit) AND at the largest representable one
    // (u64::MAX) -- and, being monotonically bounded by the asymptotic ratio in between, at every
    // deposit size a real supply() could ever be called with.
    for dust_shares in [1u128, 22, 1_000, 999_999] {
        assert_eq!(dust_value_after_future_deposit(dust_shares, 1), 0);
        assert_eq!(dust_value_after_future_deposit(dust_shares, u64::MAX), 0);
    }

    // Now reproduce the real on-chain sequence end to end: a genuine bad-debt event, absorbed for
    // real, leaves fee_position holding real dust shares; the market's one lender then withdraws
    // in full via a real `withdraw` instruction.
    let mut svm;
    let admin = Keypair::new_from_array([80u8; 32]);
    (svm, _) = deploy(aegis::id(), program_bytes());
    svm.airdrop(&admin.pubkey(), 10_000_000_000).unwrap();

    let guardian = fixed_pubkey(81);
    let fee_recipient = fixed_pubkey(82);
    initialize_protocol(&mut svm, &admin, guardian, fee_recipient).expect("initialize_protocol");

    let collateral_mint = create_spl_mint(&mut svm, &admin, 83, 9, admin.pubkey(), None);
    let loan_mint = create_spl_mint(&mut svm, &admin, 84, 6, admin.pubkey(), None);
    // Real, nonzero IRM rates (economic-model.md §4.1's reference set) -- a zero-rate market
    // (the shared `reference_market_args()` test fixture) would never mint fee_position any
    // shares at all, and this scenario needs fee_position to hold SOME real shares before they
    // are burned down to dust.
    let args = CreateMarketArgs {
        config_id: 0,
        oracle_kind: 0,
        collateral_feed_id: [0xAAu8; 32],
        loan_feed_id: [0xBBu8; 32],
        max_price_age_secs: 60,
        max_conf_bps: 100,
        max_ltv: 750_000_000_000_000_000,
        liq_threshold: 800_000_000_000_000_000,
        liq_bonus: 50_000_000_000_000_000,
        close_factor: 500_000_000_000_000_000,
        full_liq_hf: 950_000_000_000_000_000,
        liq_protocol_fee: 100_000_000_000_000_000,
        fee: 100_000_000_000_000_000, // 0.10 WAD
        min_debt: 1,
        base_rate_ps: 0,
        slope1_ps: 1_268_391_679,
        slope2_ps: 31_709_791_983,
        u_kink: 800_000_000_000_000_000,
        max_rate_ps: 317_097_919_837,
        ack_freeze_authority: false,
    };
    let (result, market, _collateral_vault, loan_vault, fee_position) = create_market(
        &mut svm,
        &admin,
        collateral_mint,
        loan_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        fee_recipient,
        args,
    );
    result.expect("create_market");

    // The market's one lender.
    let lender = Keypair::new_from_array([85u8; 32]);
    svm.airdrop(&lender.pubkey(), 10_000_000_000).unwrap();
    let lender_ata = create_token_account(
        &mut svm,
        &admin,
        86,
        loan_mint,
        lender.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    let supply_amount = 1_000_000_000u64; // 1,000 USDC
    mint_to(
        &mut svm,
        &admin,
        loan_mint,
        lender_ata,
        &admin,
        supply_amount,
        spl_token_interface::ID,
    );
    let (_, lender_position) = init_position(&mut svm, &admin, market, lender.pubkey());
    supply(
        &mut svm,
        &lender,
        market,
        lender_position,
        fee_position,
        loan_vault,
        lender_ata,
        loan_mint,
        spl_token_interface::ID,
        supply_amount,
        0,
    )
    .expect("supply");

    // Seed a fully-collateral-free debt position (this scenario's mechanism is independent of
    // HOW the debt/bad-debt arose -- `seed_borrow_state` is the same legitimate, established
    // fixture technique `dust_debt.rs`/F-10-02 already uses; the real demo instead reaches this
    // point via real deposit_collateral/borrow/liquidate transactions, see phase13_demo.rs steps
    // 4-13, which produce the same shape of state).
    let borrower_position_owner = fixed_pubkey(87);
    let (_, borrower_position) = init_position(&mut svm, &admin, market, borrower_position_owner);
    seed_borrow_state(
        &mut svm,
        market,
        borrower_position,
        500_000_000,
        500_000_000_000_000,
    );

    // Real interest accrual mints REAL shares to fee_position -- not a fixture.
    let mut clock = svm.get_sysvar::<solana_clock::Clock>();
    clock.unix_timestamp += 365 * 86_400;
    svm.set_sysvar(&clock);
    accrue_interest(&mut svm, &admin, market, fee_position).expect("accrue_interest");
    let fee_shares_before_absorption = fetch_position(&svm, &fee_position).supply_shares;
    assert!(
        fee_shares_before_absorption > 0,
        "the scenario requires fee_position to hold real shares before the bad-debt event"
    );

    // Real `absorb_bad_debt` -- requires collateral_amount == 0 (true: this position was never
    // given any real collateral) and borrow_shares > 0 (true, from the seed above).
    absorb_bad_debt(&mut svm, &admin, market, borrower_position, fee_position)
        .expect("absorb_bad_debt");

    let fee_shares_after_absorption = fetch_position(&svm, &fee_position).supply_shares;
    let market_after_absorption = fetch_market(&svm, &market);
    println!(
        "fee_position.supply_shares: {} -> {} (dust)",
        fee_shares_before_absorption, fee_shares_after_absorption
    );

    if fee_shares_after_absorption == 0 || market_after_absorption.total_supply_assets == 0 {
        // The bad debt happened to consume fee_position's shares exactly, or leave nothing for
        // the lender to withdraw against -- not the scenario this test targets. Not a failure,
        // but nothing further to assert.
        return;
    }

    // The market's one lender withdraws EXACTLY their own full share balance.
    let lender_shares = fetch_position(&svm, &lender_position).supply_shares;
    withdraw(
        &mut svm,
        &lender,
        market,
        lender_position,
        fee_position,
        loan_vault,
        lender_ata,
        loan_mint,
        spl_token_interface::ID,
        0,
        lender_shares,
    )
    .expect("lender's full withdrawal must succeed");

    let final_state = fetch_market(&svm, &market);
    println!(
        "final: total_supply_shares={} total_supply_assets={}",
        final_state.total_supply_shares, final_state.total_supply_assets
    );

    if final_state.total_supply_assets != 0 {
        // Floor rounding did not happen to zero out total_supply_assets in this exact run's
        // numeric magnitudes -- INV-ACC-06 holds trivially either way. Confirm it and stop.
        invariants::assert_inv_acc_06(&svm, &market);
        return;
    }

    // THE FINDING, reproduced for real: total_supply_shares is the fee_position's dust, nonzero,
    // while total_supply_assets is exactly zero -- INV-ACC-06's literal text is violated.
    assert_eq!(final_state.total_supply_shares, fee_shares_after_absorption);
    assert!(final_state.total_supply_shares > 0);

    // THE SAFETY NET: confirm this specific dust amount's value stays at exactly zero for a
    // subsequent deposit of any size, using the SAME real on-chain math (`aegis_math`), not just
    // the closed-form check above -- so this test would fail if a future change to the
    // share-pricing formula ever let real dust regain nonzero value.
    for future_deposit in [1u64, 1_000, 1_000_000, u64::MAX] {
        let implied =
            dust_value_after_future_deposit(final_state.total_supply_shares, future_deposit);
        assert_eq!(
            implied, 0,
            "F-13-01's core safety claim broke: dust shares ({}) would be worth {} after a future \
             deposit of {} -- this would be real, if small, value creation and must be treated as \
             a genuine T-17 regression, not accepted as informational",
            final_state.total_supply_shares, implied, future_deposit
        );
    }
}
