//! Phase 6 — `withdraw_collateral_fees`, the non-custodial admin proof (`A-ADM-02`), and
//! cross-market isolation (`I-ISO-01`, `A-PAR-02`) (`docs/phases/phase-06-liquidation.md`,
//! `docs/instruction-catalogue.md` §19, `docs/account-model.md` §8, `docs/invariants.md`
//! INV-ADM-01/08, INV-RES-03, INV-SOLV-05).

#![allow(clippy::result_large_err)]

use aegis::error::AegisError;
use aegis_test_kit::{
    absorb_bad_debt, assert_aegis_error, borrow, create_market, create_spl_mint,
    create_token_account, deploy, deposit_collateral, fetch_market, fetch_position, init_position,
    initialize_protocol, invariants, liquidate, mint_to, reference_market_args, set_price,
    spl_token_interface, supply, token_accounts::fetch_token_account_base,
    withdraw_collateral_fees, PriceFixture,
};
use anchor_lang::ToAccountMetas;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use std::collections::HashSet;

fn program_bytes() -> &'static [u8] {
    include_bytes!(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/aegis.so"))
}

const COLLATERAL_FEED_ID: [u8; 32] = [0xAAu8; 32];
const LOAN_FEED_ID: [u8; 32] = [0xBBu8; 32];

struct SeedGen(u8);
impl SeedGen {
    fn new() -> Self {
        Self(20)
    }
    fn next(&mut self) -> u8 {
        let s = self.0;
        self.0 = self.0.checked_add(1).expect("used more than 255 seeds");
        s
    }
}

fn fixed_pubkey(seed: u8) -> Pubkey {
    Keypair::new_from_array([seed; 32]).pubkey()
}

fn now(svm: &litesvm::LiteSVM) -> i64 {
    svm.get_sysvar::<solana_clock::Clock>().unix_timestamp
}

struct Fixture {
    market: Pubkey,
    fee_position: Pubkey,
    collateral_vault: Pubkey,
    loan_vault: Pubkey,
    collateral_mint: Pubkey,
    loan_mint: Pubkey,
}

/// Creates a SOL(9dp)/USDC(6dp) market at the given `config_id`, so two calls against the SAME
/// admin/protocol/mints still produce two markets with disjoint PDAs (account-model.md §4: the
/// market seed includes `config_id` precisely so several configurations can coexist).
#[allow(clippy::too_many_arguments)]
fn setup_market_n(
    svm: &mut litesvm::LiteSVM,
    admin: &Keypair,
    fee_recipient: Pubkey,
    collateral_mint: Pubkey,
    loan_mint: Pubkey,
    config_id: u16,
    collateral_feed: [u8; 32],
    loan_feed: [u8; 32],
) -> Fixture {
    let args = reference_market_args(config_id, collateral_feed, loan_feed, false);
    let (result, market, collateral_vault, loan_vault, fee_position) = create_market(
        svm,
        admin,
        collateral_mint,
        loan_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        fee_recipient,
        args,
    );
    result.expect("create_market must succeed");
    Fixture {
        market,
        fee_position,
        collateral_vault,
        loan_vault,
        collateral_mint,
        loan_mint,
    }
}

fn wallet_with_ata(
    svm: &mut litesvm::LiteSVM,
    admin: &Keypair,
    mint: Pubkey,
    seeds: &mut SeedGen,
    balance: u64,
) -> (Keypair, Pubkey) {
    let wallet = Keypair::new_from_array([seeds.next(); 32]);
    svm.airdrop(&wallet.pubkey(), 10_000_000_000)
        .expect("airdrop");
    let ata = create_token_account(
        svm,
        admin,
        seeds.next(),
        mint,
        wallet.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    if balance > 0 {
        mint_to(
            svm,
            admin,
            mint,
            ata,
            admin,
            balance,
            spl_token_interface::ID,
        );
    }
    (wallet, ata)
}

/// Produces real `collateral_fee_accrued` in `fx` via a real, worked-example-style liquidation
/// (the ONLY code path that is ever allowed to increase it, INV-CUS-09) -- not a fixture. Returns
/// `(protocol_cut, borrower_position)`; the borrower's position keeps its post-liquidation
/// remainder collateral (`economic-model.md` §7.5) and must be included in any INV-CUS-02 sum a
/// caller computes afterward.
fn accrue_real_protocol_cut(
    svm: &mut litesvm::LiteSVM,
    admin: &Keypair,
    fx: &Fixture,
    seeds: &mut SeedGen,
) -> (u64, Pubkey) {
    let (lender, lender_ata) = wallet_with_ata(svm, admin, fx.loan_mint, seeds, 1_000_000_000_000);
    let (_, lender_position) = init_position(svm, admin, fx.market, lender.pubkey());
    supply(
        svm,
        &lender,
        fx.market,
        lender_position,
        fx.fee_position,
        fx.loan_vault,
        lender_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        1_000_000_000_000,
        0,
    )
    .expect("supply must succeed");

    let (borrower, borrower_collateral_ata) =
        wallet_with_ata(svm, admin, fx.collateral_mint, seeds, 10_000_000_000);
    let (_, borrower_position) = init_position(svm, admin, fx.market, borrower.pubkey());
    deposit_collateral(
        svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.collateral_vault,
        borrower_collateral_ata,
        fx.collateral_mint,
        spl_token_interface::ID,
        10_000_000_000,
    )
    .expect("deposit_collateral must succeed");
    let borrower_loan_ata = create_token_account(
        svm,
        admin,
        seeds.next(),
        fx.loan_mint,
        borrower.pubkey(),
        spl_token_interface::ID,
        &[],
    );

    let n = now(svm);
    let c0 = set_price(
        svm,
        seeds.next(),
        PriceFixture::valid(COLLATERAL_FEED_ID, 15_000_000_000, 0, -8, n),
    );
    let l0 = set_price(
        svm,
        seeds.next(),
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 0, -8, n),
    );
    borrow(
        svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        c0,
        l0,
        900_000_000,
        0,
    )
    .expect("borrow must succeed");

    let (liquidator, liquidator_loan_ata) =
        wallet_with_ata(svm, admin, fx.loan_mint, seeds, 1_000_000_000);
    let liquidator_collateral_ata = create_token_account(
        svm,
        admin,
        seeds.next(),
        fx.collateral_mint,
        liquidator.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    let c1 = set_price(
        svm,
        seeds.next(),
        PriceFixture::valid(COLLATERAL_FEED_ID, 9_500_000_000, 20_000_000, -8, n),
    );
    let l1 = set_price(
        svm,
        seeds.next(),
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 20_000, -8, n),
    );
    liquidate(
        svm,
        &liquidator,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        fx.collateral_vault,
        liquidator_loan_ata,
        liquidator_collateral_ata,
        fx.loan_mint,
        fx.collateral_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        c1,
        l1,
        900_000_000,
        0,
    )
    .expect("liquidation must succeed and accrue a real protocol cut");

    (
        fetch_market(svm, &fx.market).collateral_fee_accrued,
        borrower_position,
    )
}

// ============================================================================================
// withdraw_collateral_fees: happy path.
// ============================================================================================

#[test]
fn withdraw_collateral_fees_happy_path() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let guardian = fixed_pubkey(seeds.next());
    let fee_recipient = fixed_pubkey(seeds.next());
    initialize_protocol(&mut svm, &admin, guardian, fee_recipient).unwrap();
    let collateral_mint = create_spl_mint(&mut svm, &admin, seeds.next(), 9, admin.pubkey(), None);
    let loan_mint = create_spl_mint(&mut svm, &admin, seeds.next(), 6, admin.pubkey(), None);
    let fx = setup_market_n(
        &mut svm,
        &admin,
        fee_recipient,
        collateral_mint,
        loan_mint,
        0,
        COLLATERAL_FEED_ID,
        LOAN_FEED_ID,
    );

    let (protocol_cut, borrower_position) =
        accrue_real_protocol_cut(&mut svm, &admin, &fx, &mut seeds);
    assert!(protocol_cut > 0, "fixture sanity: a real cut accrued");

    let admin_collateral_ata = create_token_account(
        &mut svm,
        &admin,
        seeds.next(),
        fx.collateral_mint,
        admin.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    let vault_before = fetch_token_account_base(&svm, &fx.collateral_vault).amount;

    withdraw_collateral_fees(
        &mut svm,
        &admin,
        fx.market,
        fx.collateral_vault,
        admin_collateral_ata,
        fx.collateral_mint,
        spl_token_interface::ID,
        protocol_cut,
    )
    .expect("admin withdrawal of the protocol's own accrued fee must succeed");

    let market_after = fetch_market(&svm, &fx.market);
    assert_eq!(market_after.collateral_fee_accrued, 0);
    let vault_after = fetch_token_account_base(&svm, &fx.collateral_vault).amount;
    assert_eq!(vault_before - vault_after, protocol_cut);
    let admin_ata_after = fetch_token_account_base(&svm, &admin_collateral_ata).amount;
    assert_eq!(admin_ata_after, protocol_cut);

    invariants::assert_inv_cus_02(&svm, &fx.market, &[borrower_position]);
}

// ============================================================================================
// A-ADM-02 / INV-ADM-01 / INV-ADM-08: the flagship non-custodial-admin proof.
// ============================================================================================

#[test]
fn a_adm_02_admin_cannot_withdraw_user_collateral() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let guardian = fixed_pubkey(seeds.next());
    let fee_recipient = fixed_pubkey(seeds.next());
    initialize_protocol(&mut svm, &admin, guardian, fee_recipient).unwrap();
    let collateral_mint = create_spl_mint(&mut svm, &admin, seeds.next(), 9, admin.pubkey(), None);
    let loan_mint = create_spl_mint(&mut svm, &admin, seeds.next(), 6, admin.pubkey(), None);
    let fx = setup_market_n(
        &mut svm,
        &admin,
        fee_recipient,
        collateral_mint,
        loan_mint,
        0,
        COLLATERAL_FEED_ID,
        LOAN_FEED_ID,
    );

    // A real user deposits real collateral -- this is what the admin will attempt to reach.
    let (depositor, depositor_ata) = wallet_with_ata(
        &mut svm,
        &admin,
        fx.collateral_mint,
        &mut seeds,
        50_000_000_000,
    );
    let (_, position) = init_position(&mut svm, &admin, fx.market, depositor.pubkey());
    deposit_collateral(
        &mut svm,
        &depositor,
        fx.market,
        position,
        fx.collateral_vault,
        depositor_ata,
        fx.collateral_mint,
        spl_token_interface::ID,
        50_000_000_000,
    )
    .expect("deposit_collateral must succeed");

    let (protocol_cut, borrower_position) =
        accrue_real_protocol_cut(&mut svm, &admin, &fx, &mut seeds);
    assert!(protocol_cut > 0);

    let position_before = fetch_position(&svm, &position).collateral_amount;
    let vault_before = fetch_token_account_base(&svm, &fx.collateral_vault).amount;
    assert!(
        vault_before > protocol_cut,
        "the vault must hold MORE than the protocol's own cut -- the excess is user collateral"
    );

    let admin_collateral_ata = create_token_account(
        &mut svm,
        &admin,
        seeds.next(),
        fx.collateral_mint,
        admin.pubkey(),
        spl_token_interface::ID,
        &[],
    );

    // Attempt 1: withdraw exactly one unit MORE than collateral_fee_accrued.
    let result = withdraw_collateral_fees(
        &mut svm,
        &admin,
        fx.market,
        fx.collateral_vault,
        admin_collateral_ata,
        fx.collateral_mint,
        spl_token_interface::ID,
        protocol_cut + 1,
    );
    assert_aegis_error(&result, AegisError::InsufficientCollateralFees);

    // Attempt 2: withdraw the ENTIRE vault balance (user collateral + protocol cut) -- the
    // maximally aggressive version of the attack.
    let result2 = withdraw_collateral_fees(
        &mut svm,
        &admin,
        fx.market,
        fx.collateral_vault,
        admin_collateral_ata,
        fx.collateral_mint,
        spl_token_interface::ID,
        vault_before,
    );
    assert_aegis_error(&result2, AegisError::InsufficientCollateralFees);

    // Concrete evidence: user collateral, Position balance, and the custody invariant are all
    // completely unchanged by either attempt.
    let position_after = fetch_position(&svm, &position).collateral_amount;
    assert_eq!(
        position_before, position_after,
        "user collateral must remain intact"
    );
    let vault_after = fetch_token_account_base(&svm, &fx.collateral_vault).amount;
    assert_eq!(
        vault_before, vault_after,
        "vault balance must be completely unchanged"
    );
    invariants::assert_inv_cus_02(&svm, &fx.market, &[position, borrower_position]);

    // Sanity: the LEGITIMATE amount (exactly the protocol's own cut) still works, proving the
    // rejection above was about the AMOUNT, not a broken instruction.
    withdraw_collateral_fees(
        &mut svm,
        &admin,
        fx.market,
        fx.collateral_vault,
        admin_collateral_ata,
        fx.collateral_mint,
        spl_token_interface::ID,
        protocol_cut,
    )
    .expect("withdrawing exactly the protocol's own cut must still succeed");
    let position_final = fetch_position(&svm, &position).collateral_amount;
    assert_eq!(
        position_before, position_final,
        "still untouched after the legitimate withdrawal"
    );
}

#[test]
fn withdraw_collateral_fees_requires_the_real_admin() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let guardian = fixed_pubkey(seeds.next());
    let fee_recipient = fixed_pubkey(seeds.next());
    initialize_protocol(&mut svm, &admin, guardian, fee_recipient).unwrap();
    let collateral_mint = create_spl_mint(&mut svm, &admin, seeds.next(), 9, admin.pubkey(), None);
    let loan_mint = create_spl_mint(&mut svm, &admin, seeds.next(), 6, admin.pubkey(), None);
    let fx = setup_market_n(
        &mut svm,
        &admin,
        fee_recipient,
        collateral_mint,
        loan_mint,
        0,
        COLLATERAL_FEED_ID,
        LOAN_FEED_ID,
    );
    let (protocol_cut, _borrower_position) =
        accrue_real_protocol_cut(&mut svm, &admin, &fx, &mut seeds);

    let impostor = Keypair::new_from_array([seeds.next(); 32]);
    svm.airdrop(&impostor.pubkey(), 10_000_000_000).unwrap();
    let impostor_ata = create_token_account(
        &mut svm,
        &admin,
        seeds.next(),
        fx.collateral_mint,
        impostor.pubkey(),
        spl_token_interface::ID,
        &[],
    );

    let result = withdraw_collateral_fees(
        &mut svm,
        &impostor,
        fx.market,
        fx.collateral_vault,
        impostor_ata,
        fx.collateral_mint,
        spl_token_interface::ID,
        protocol_cut,
    );
    assert_aegis_error(&result, AegisError::NotProtocolAdmin);
}

// ============================================================================================
// I-ISO-01 / INV-SOLV-05: bad debt (and any other state) in Market A never touches Market B.
// ============================================================================================

#[test]
fn i_iso_01_bad_debt_in_market_a_leaves_market_b_completely_untouched() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let guardian = fixed_pubkey(seeds.next());
    let fee_recipient = fixed_pubkey(seeds.next());
    initialize_protocol(&mut svm, &admin, guardian, fee_recipient).unwrap();

    let collateral_mint = create_spl_mint(&mut svm, &admin, seeds.next(), 9, admin.pubkey(), None);
    let loan_mint = create_spl_mint(&mut svm, &admin, seeds.next(), 6, admin.pubkey(), None);

    // Two independent markets on the SAME mint pair, distinguished only by config_id
    // (account-model.md §4) -- the hardest case for isolation, since a bug that accidentally
    // shared a vault or a totals field would only be caught here, not with distinct mints.
    let fx_a = setup_market_n(
        &mut svm,
        &admin,
        fee_recipient,
        collateral_mint,
        loan_mint,
        0,
        [0x11u8; 32],
        [0x22u8; 32],
    );
    let fx_b = setup_market_n(
        &mut svm,
        &admin,
        fee_recipient,
        collateral_mint,
        loan_mint,
        1,
        [0x33u8; 32],
        [0x44u8; 32],
    );
    assert_ne!(fx_a.market, fx_b.market);
    assert_ne!(fx_a.collateral_vault, fx_b.collateral_vault);
    assert_ne!(fx_a.loan_vault, fx_b.loan_vault);
    assert_ne!(fx_a.fee_position, fx_b.fee_position);

    // Market B gets its own real activity: a lender and a healthy borrower.
    let (lender_b, lender_b_ata) = wallet_with_ata(
        &mut svm,
        &admin,
        fx_b.loan_mint,
        &mut seeds,
        1_000_000_000_000,
    );
    let (_, lender_b_position) = init_position(&mut svm, &admin, fx_b.market, lender_b.pubkey());
    supply(
        &mut svm,
        &lender_b,
        fx_b.market,
        lender_b_position,
        fx_b.fee_position,
        fx_b.loan_vault,
        lender_b_ata,
        fx_b.loan_mint,
        spl_token_interface::ID,
        1_000_000_000_000,
        0,
    )
    .expect("market B lender supply must succeed");

    let (borrower_b, borrower_b_collateral_ata) = wallet_with_ata(
        &mut svm,
        &admin,
        fx_b.collateral_mint,
        &mut seeds,
        10_000_000_000,
    );
    let (_, borrower_b_position) =
        init_position(&mut svm, &admin, fx_b.market, borrower_b.pubkey());
    deposit_collateral(
        &mut svm,
        &borrower_b,
        fx_b.market,
        borrower_b_position,
        fx_b.collateral_vault,
        borrower_b_collateral_ata,
        fx_b.collateral_mint,
        spl_token_interface::ID,
        10_000_000_000,
    )
    .expect("market B deposit_collateral must succeed");

    // Full snapshot of EVERYTHING in Market B before Market A's catastrophe.
    let market_b_before = svm.get_account(&fx_b.market).unwrap();
    let fee_position_b_before = svm.get_account(&fx_b.fee_position).unwrap();
    let lender_b_position_before = svm.get_account(&lender_b_position).unwrap();
    let borrower_b_position_before = svm.get_account(&borrower_b_position).unwrap();
    let collateral_vault_b_before = svm.get_account(&fx_b.collateral_vault).unwrap();
    let loan_vault_b_before = svm.get_account(&fx_b.loan_vault).unwrap();

    // --- Catastrophe in Market A: real borrow, real crash, real clamped liquidation, real bad
    // debt, real absorb_bad_debt. ---
    let (lender_a, lender_a_ata) = wallet_with_ata(
        &mut svm,
        &admin,
        fx_a.loan_mint,
        &mut seeds,
        1_000_000_000_000,
    );
    let (_, lender_a_position) = init_position(&mut svm, &admin, fx_a.market, lender_a.pubkey());
    supply(
        &mut svm,
        &lender_a,
        fx_a.market,
        lender_a_position,
        fx_a.fee_position,
        fx_a.loan_vault,
        lender_a_ata,
        fx_a.loan_mint,
        spl_token_interface::ID,
        1_000_000_000_000,
        0,
    )
    .expect("market A lender supply must succeed");

    let (borrower_a, borrower_a_collateral_ata) = wallet_with_ata(
        &mut svm,
        &admin,
        fx_a.collateral_mint,
        &mut seeds,
        9_000_000_000,
    );
    let (_, borrower_a_position) =
        init_position(&mut svm, &admin, fx_a.market, borrower_a.pubkey());
    deposit_collateral(
        &mut svm,
        &borrower_a,
        fx_a.market,
        borrower_a_position,
        fx_a.collateral_vault,
        borrower_a_collateral_ata,
        fx_a.collateral_mint,
        spl_token_interface::ID,
        9_000_000_000,
    )
    .expect("market A deposit_collateral must succeed");
    let borrower_a_loan_ata = create_token_account(
        &mut svm,
        &admin,
        seeds.next(),
        fx_a.loan_mint,
        borrower_a.pubkey(),
        spl_token_interface::ID,
        &[],
    );

    let n = now(&svm);
    let c0 = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid([0x11u8; 32], 15_000_000_000, 0, -8, n),
    );
    let l0 = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid([0x22u8; 32], 100_000_000, 0, -8, n),
    );
    borrow(
        &mut svm,
        &borrower_a,
        fx_a.market,
        borrower_a_position,
        fx_a.fee_position,
        fx_a.loan_vault,
        borrower_a_loan_ata,
        fx_a.loan_mint,
        spl_token_interface::ID,
        c0,
        l0,
        900_000_000,
        0,
    )
    .expect("market A borrow must succeed");

    let (liquidator, liquidator_loan_ata) =
        wallet_with_ata(&mut svm, &admin, fx_a.loan_mint, &mut seeds, 1_000_000_000);
    let liquidator_collateral_ata = create_token_account(
        &mut svm,
        &admin,
        seeds.next(),
        fx_a.collateral_mint,
        liquidator.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    let c1 = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid([0x11u8; 32], 9_500_000_000, 20_000_000, -8, n),
    );
    let l1 = set_price(
        &mut svm,
        seeds.next(),
        PriceFixture::valid([0x22u8; 32], 100_000_000, 20_000, -8, n),
    );
    liquidate(
        &mut svm,
        &liquidator,
        fx_a.market,
        borrower_a_position,
        fx_a.fee_position,
        fx_a.loan_vault,
        fx_a.collateral_vault,
        liquidator_loan_ata,
        liquidator_collateral_ata,
        fx_a.loan_mint,
        fx_a.collateral_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        c1,
        l1,
        900_000_000,
        0,
    )
    .expect("market A clamped liquidation must succeed");
    assert_eq!(
        fetch_position(&svm, &borrower_a_position).collateral_amount,
        0,
        "fixture sanity: market A position fully seized"
    );

    absorb_bad_debt(
        &mut svm,
        &admin,
        fx_a.market,
        borrower_a_position,
        fx_a.fee_position,
    )
    .expect("market A absorb_bad_debt must succeed");

    // --- Market B: byte-exact, EVERYTHING unchanged. ---
    let market_b_after = svm.get_account(&fx_b.market).unwrap();
    let fee_position_b_after = svm.get_account(&fx_b.fee_position).unwrap();
    let lender_b_position_after = svm.get_account(&lender_b_position).unwrap();
    let borrower_b_position_after = svm.get_account(&borrower_b_position).unwrap();
    let collateral_vault_b_after = svm.get_account(&fx_b.collateral_vault).unwrap();
    let loan_vault_b_after = svm.get_account(&fx_b.loan_vault).unwrap();

    assert_eq!(
        market_b_before.data, market_b_after.data,
        "Market B market state changed"
    );
    assert_eq!(
        fee_position_b_before.data, fee_position_b_after.data,
        "Market B fee_position changed"
    );
    assert_eq!(
        lender_b_position_before.data, lender_b_position_after.data,
        "Market B lender position changed"
    );
    assert_eq!(
        borrower_b_position_before.data, borrower_b_position_after.data,
        "Market B borrower position changed"
    );
    assert_eq!(
        collateral_vault_b_before.data, collateral_vault_b_after.data,
        "Market B collateral vault changed"
    );
    assert_eq!(
        loan_vault_b_before.data, loan_vault_b_after.data,
        "Market B loan vault changed"
    );

    // Market B's own custody/accounting invariants still hold, undisturbed.
    invariants::assert_inv_cus_01(&svm, &fx_b.market);
    invariants::assert_inv_cus_02(
        &svm,
        &fx_b.market,
        &[lender_b_position, borrower_b_position],
    );
    let market_b_final = fetch_market(&svm, &fx_b.market);
    assert_eq!(market_b_final.total_supply_assets, 1_000_000_000_000);
    assert_eq!(market_b_final.total_borrow_assets, 0);
    assert_eq!(market_b_final.collateral_fee_accrued, 0);

    // And Market A genuinely IS in the post-bad-debt state (fixture sanity, not the claim under
    // test): the isolation claim would be vacuous if Market A hadn't actually changed.
    let market_a_final = fetch_market(&svm, &fx_a.market);
    assert_eq!(
        market_a_final.total_borrow_assets, 0,
        "market A debt fully absorbed"
    );
}

// ============================================================================================
// A-PAR-02 / INV-RES-03: no writable account is shared between two distinct markets, for every
// Phase 6 instruction.
// ============================================================================================

#[test]
fn a_par_02_no_writable_account_shared_between_two_markets() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let guardian = fixed_pubkey(seeds.next());
    let fee_recipient = fixed_pubkey(seeds.next());
    initialize_protocol(&mut svm, &admin, guardian, fee_recipient).unwrap();
    let collateral_mint = create_spl_mint(&mut svm, &admin, seeds.next(), 9, admin.pubkey(), None);
    let loan_mint = create_spl_mint(&mut svm, &admin, seeds.next(), 6, admin.pubkey(), None);
    let fx_a = setup_market_n(
        &mut svm,
        &admin,
        fee_recipient,
        collateral_mint,
        loan_mint,
        0,
        [0x11u8; 32],
        [0x22u8; 32],
    );
    let fx_b = setup_market_n(
        &mut svm,
        &admin,
        fee_recipient,
        collateral_mint,
        loan_mint,
        1,
        [0x33u8; 32],
        [0x44u8; 32],
    );

    // Build representative, fully-populated Phase 6 instructions for each market with distinct
    // per-market positions/ATAs, and collect every WRITABLE account each one declares.
    let owner_a = fixed_pubkey(seeds.next());
    let (_, position_a) = init_position(&mut svm, &admin, fx_a.market, owner_a);
    let owner_b = fixed_pubkey(seeds.next());
    let (_, position_b) = init_position(&mut svm, &admin, fx_b.market, owner_b);

    let liquidator_a = fixed_pubkey(seeds.next());
    let liquidator_a_loan_ata = create_token_account(
        &mut svm,
        &admin,
        seeds.next(),
        fx_a.loan_mint,
        liquidator_a,
        spl_token_interface::ID,
        &[],
    );
    let liquidator_a_collateral_ata = create_token_account(
        &mut svm,
        &admin,
        seeds.next(),
        fx_a.collateral_mint,
        liquidator_a,
        spl_token_interface::ID,
        &[],
    );
    let price_a1 = fixed_pubkey(seeds.next());
    let price_a2 = fixed_pubkey(seeds.next());

    let liquidator_b = fixed_pubkey(seeds.next());
    let liquidator_b_loan_ata = create_token_account(
        &mut svm,
        &admin,
        seeds.next(),
        fx_b.loan_mint,
        liquidator_b,
        spl_token_interface::ID,
        &[],
    );
    let liquidator_b_collateral_ata = create_token_account(
        &mut svm,
        &admin,
        seeds.next(),
        fx_b.collateral_mint,
        liquidator_b,
        spl_token_interface::ID,
        &[],
    );
    let price_b1 = fixed_pubkey(seeds.next());
    let price_b2 = fixed_pubkey(seeds.next());

    let liquidate_metas_a = aegis::accounts::Liquidate {
        liquidator: liquidator_a,
        market: fx_a.market,
        position: position_a,
        fee_position: fx_a.fee_position,
        loan_vault: fx_a.loan_vault,
        collateral_vault: fx_a.collateral_vault,
        liquidator_loan_ata: liquidator_a_loan_ata,
        liquidator_collateral_ata: liquidator_a_collateral_ata,
        loan_mint: fx_a.loan_mint,
        collateral_mint: fx_a.collateral_mint,
        loan_token_program: spl_token_interface::ID,
        collateral_token_program: spl_token_interface::ID,
        collateral_price_update: price_a1,
        loan_price_update: price_a2,
        callback_program: None,
        callback_collateral_account: None,
    }
    .to_account_metas(None);
    let liquidate_metas_b = aegis::accounts::Liquidate {
        liquidator: liquidator_b,
        market: fx_b.market,
        position: position_b,
        fee_position: fx_b.fee_position,
        loan_vault: fx_b.loan_vault,
        collateral_vault: fx_b.collateral_vault,
        liquidator_loan_ata: liquidator_b_loan_ata,
        liquidator_collateral_ata: liquidator_b_collateral_ata,
        loan_mint: fx_b.loan_mint,
        collateral_mint: fx_b.collateral_mint,
        loan_token_program: spl_token_interface::ID,
        collateral_token_program: spl_token_interface::ID,
        collateral_price_update: price_b1,
        loan_price_update: price_b2,
        callback_program: None,
        callback_collateral_account: None,
    }
    .to_account_metas(None);

    let absorb_metas_a = aegis::accounts::AbsorbBadDebt {
        market: fx_a.market,
        position: position_a,
        fee_position: fx_a.fee_position,
    }
    .to_account_metas(None);
    let absorb_metas_b = aegis::accounts::AbsorbBadDebt {
        market: fx_b.market,
        position: position_b,
        fee_position: fx_b.fee_position,
    }
    .to_account_metas(None);

    let (protocol_key, _) = aegis_test_kit::protocol_pda();
    let admin_ata_a = create_token_account(
        &mut svm,
        &admin,
        seeds.next(),
        fx_a.collateral_mint,
        admin.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    let admin_ata_b = create_token_account(
        &mut svm,
        &admin,
        seeds.next(),
        fx_b.collateral_mint,
        admin.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    let fees_metas_a = aegis::accounts::WithdrawCollateralFees {
        admin: admin.pubkey(),
        protocol: protocol_key,
        market: fx_a.market,
        collateral_vault: fx_a.collateral_vault,
        admin_collateral_ata: admin_ata_a,
        collateral_mint: fx_a.collateral_mint,
        collateral_token_program: spl_token_interface::ID,
    }
    .to_account_metas(None);
    let fees_metas_b = aegis::accounts::WithdrawCollateralFees {
        admin: admin.pubkey(),
        protocol: protocol_key,
        market: fx_b.market,
        collateral_vault: fx_b.collateral_vault,
        admin_collateral_ata: admin_ata_b,
        collateral_mint: fx_b.collateral_mint,
        collateral_token_program: spl_token_interface::ID,
    }
    .to_account_metas(None);

    let writable_of = |metas: &[anchor_lang::prelude::AccountMeta]| -> HashSet<Pubkey> {
        metas
            .iter()
            .filter(|m| m.is_writable)
            .map(|m| m.pubkey)
            .collect()
    };

    let mut writable_a: HashSet<Pubkey> = HashSet::new();
    writable_a.extend(writable_of(&liquidate_metas_a));
    writable_a.extend(writable_of(&absorb_metas_a));
    writable_a.extend(writable_of(&fees_metas_a));

    let mut writable_b: HashSet<Pubkey> = HashSet::new();
    writable_b.extend(writable_of(&liquidate_metas_b));
    writable_b.extend(writable_of(&absorb_metas_b));
    writable_b.extend(writable_of(&fees_metas_b));

    let overlap: Vec<&Pubkey> = writable_a.intersection(&writable_b).collect();
    assert!(
        overlap.is_empty(),
        "INV-RES-03 violated: writable account(s) shared between Market A and Market B: {overlap:?}"
    );

    // `protocol` itself is shared -- but only ever READ-ONLY in these instructions (admin-only
    // `set_*` instructions, Phase 12, are the sole writers) -- confirm it is not writable here.
    let protocol_meta = fees_metas_a
        .iter()
        .find(|m| m.pubkey == protocol_key)
        .expect("protocol account must be present");
    assert!(
        !protocol_meta.is_writable,
        "protocol must be read-only in withdraw_collateral_fees"
    );
}
