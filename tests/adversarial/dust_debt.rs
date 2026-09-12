//! **Exploit regression — discovered by the Phase 10 stateful fuzzer**
//! (`docs/phases/phase-10-security.md`, `docs/security/findings.md` F-10-02,
//! `docs/security/mutation-report.md`'s companion extended-campaign finding).
//!
//! `tests/fuzz/`'s 100,000+-operation extended campaign (seed 21, step 3628) found a real,
//! reproducible violation of **INV-ACC-06** (`total_borrow_shares == 0 <=> total_borrow_assets ==
//! 0`) against the unmutated, correct program — not a mutation artifact. Minimized by hand from
//! the fuzz trace to the exact scenario below.
//!
//! **Root cause:** `repay`'s `assets`-denominated path computes `requested_shares =
//! to_shares_down(assets, total_borrow_assets, total_borrow_shares)` (floored), clamps it to the
//! position's real share balance, then recomputes `assets_to_pull =
//! to_assets_up(clamped_shares, total_borrow_assets, total_borrow_shares)` (ceiled) from the
//! *clamped* share count. When `total_borrow_assets` is already very small (a market that has
//! been almost fully repaid) and a position holds effectively all of `total_borrow_shares`, the
//! `VIRTUAL_SHARES`/`VIRTUAL_ASSETS` offsets (`crates/aegis-math/src/shares.rs`, `1_000_000`/`1`)
//! dominate the ratio at these tiny magnitudes: the floored `requested_shares` can land strictly
//! below the position's true remaining share balance, while the ceiled `assets_to_pull`
//! recomputed from that smaller share count still equals the *entire* remaining
//! `total_borrow_assets`. The result: `total_borrow_assets` reaches exactly 0 while a small,
//! nonzero "dust" share remainder is left on the position and in `total_borrow_shares` —
//! INV-ACC-06 violated, permanently: once `total_borrow_assets == 0`, `utilization()` reads 0
//! regardless of the dust shares, so `borrow_rate` is 0 and no future `accrue_interest` ever adds
//! assets back — the position's real economic obligation, however small, becomes permanently
//! uncollectible.
//!
//! **Severity:** Low. The stranded amount is bounded by construction to a sub-`VIRTUAL_SHARES`
//! fraction of one loan-asset base unit (in the reproduced case, worth `to_assets_up(3350, 0,
//! 3350) == 1` base unit) — this is not a large-scale drain. It is nonetheless a genuine,
//! permanent invariant violation with no self-healing path, which is why it is fixed here rather
//! than left as an accepted residual.
//!
//! **Fix:** `repay.rs` now detects the narrow, safe special case where the repaying position
//! holds *all* of `market.total_borrow_shares` (so no other position's claim is affected) and the
//! computed `assets_to_pull` would already consume all of `market.total_borrow_assets`; in that
//! case it clamps to the position's *full* share balance instead of the floored
//! `requested_shares`, so both totals reach exactly zero together. This does not change the
//! amount of tokens pulled from the payer (`assets_to_pull` is unchanged) — it only recognizes
//! that any residual share dust in this specific scenario is already worth exactly zero by the
//! market's own valuation, so leaving it on the books serves no one and only creates a permanent
//! accounting inconsistency.

#![allow(clippy::result_large_err)]

use aegis_test_kit::{
    create_market, create_spl_mint, create_token_account, deploy, fetch_market, fetch_position,
    init_position, initialize_protocol, invariants, mint_to, reference_market_args, repay,
    seed_borrow_state, spl_token_interface, supply,
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

/// T-17 (exploitable rounding) / INV-ACC-06. Minimized from the Phase 10 extended fuzz campaign's
/// seed 21 finding (step 3628): a sole borrower whose remaining debt has been repaid down to a
/// tiny `total_borrow_assets` (here `1`, backed by `1_006_700` borrow shares — the exact
/// post-repay state the fuzz trace reached) repays a further small amount and is left with
/// nonzero `total_borrow_shares`/`position.borrow_shares` while `total_borrow_assets` reaches
/// exactly zero.
#[test]
fn a_dust_01_repay_of_the_last_borrower_never_strands_shares_without_assets() {
    let mut svm;
    let admin = Keypair::new_from_array([21u8; 32]);
    (svm, _) = deploy(aegis::id(), program_bytes());
    svm.airdrop(&admin.pubkey(), 10_000_000_000).unwrap();

    let guardian = fixed_pubkey(22);
    let fee_recipient = fixed_pubkey(23);
    initialize_protocol(&mut svm, &admin, guardian, fee_recipient).expect("initialize_protocol");

    let collateral_mint = create_spl_mint(&mut svm, &admin, 24, 9, admin.pubkey(), None);
    let loan_mint = create_spl_mint(&mut svm, &admin, 25, 6, admin.pubkey(), None);
    let args = reference_market_args(0, [0xAAu8; 32], [0xBBu8; 32], false);
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

    // Fund the pool so `seed_borrow_state` can debit `loan_vault` for the injected debt, and so
    // `total_supply_assets` is large enough that this scenario is about the BORROW side only.
    let lender = Keypair::new_from_array([26u8; 32]);
    svm.airdrop(&lender.pubkey(), 10_000_000_000).unwrap();
    let lender_ata = create_token_account(
        &mut svm,
        &admin,
        27,
        loan_mint,
        lender.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    mint_to(
        &mut svm,
        &admin,
        loan_mint,
        lender_ata,
        &admin,
        10_000_000_000,
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
        10_000_000_000,
        0,
    )
    .expect("supply");

    // Seed the exact pre-repay state the fuzz trace reached: one borrower holding ALL outstanding
    // borrow shares, with total_borrow_assets already down to 1 base unit.
    let borrower = Keypair::new_from_array([28u8; 32]);
    svm.airdrop(&borrower.pubkey(), 10_000_000_000).unwrap();
    let borrower_loan_ata = create_token_account(
        &mut svm,
        &admin,
        29,
        loan_mint,
        borrower.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    mint_to(
        &mut svm,
        &admin,
        loan_mint,
        borrower_loan_ata,
        &admin,
        1_000,
        spl_token_interface::ID,
    );
    let (_, borrower_position) = init_position(&mut svm, &admin, market, borrower.pubkey());

    const SEED_ASSETS: u64 = 1;
    const SEED_SHARES: u128 = 1_006_700;
    seed_borrow_state(
        &mut svm,
        market,
        borrower_position,
        SEED_ASSETS,
        SEED_SHARES,
    );

    let before = fetch_market(&svm, &market);
    assert_eq!(before.total_borrow_assets, SEED_ASSETS);
    assert_eq!(before.total_borrow_shares, SEED_SHARES);
    invariants::assert_inv_cus_01(&svm, &market);

    // The triggering action: repay a small, nonzero amount while the pool's remaining
    // total_borrow_assets is 1.
    repay(
        &mut svm,
        &borrower,
        market,
        borrower_position,
        fee_position,
        loan_vault,
        borrower_loan_ata,
        loan_mint,
        spl_token_interface::ID,
        1,
        0,
    )
    .expect("repay must succeed");

    let after = fetch_market(&svm, &market);
    let after_position = fetch_position(&svm, &borrower_position);

    // THE FIX: total_borrow_assets and total_borrow_shares must reach zero TOGETHER, never one
    // without the other -- this is INV-ACC-06 itself, asserted directly rather than only via the
    // generic checker, so a future regression here fails with an unambiguous message.
    assert_eq!(
        after.total_borrow_assets == 0,
        after.total_borrow_shares == 0,
        "INV-ACC-06 violated: total_borrow_assets={} total_borrow_shares={}",
        after.total_borrow_assets,
        after.total_borrow_shares,
    );
    assert_eq!(
        after_position.borrow_shares == 0,
        after.total_borrow_shares == 0,
        "the sole borrower's own position must be exactly fully cleared when the pool's last \
         asset is repaid: position.borrow_shares={} total_borrow_shares={}",
        after_position.borrow_shares,
        after.total_borrow_shares,
    );

    invariants::assert_inv_cus_01(&svm, &market);
    invariants::assert_inv_acc_02(&svm, &market, &[borrower_position]);
    invariants::assert_inv_acc_06(&svm, &market);
}
