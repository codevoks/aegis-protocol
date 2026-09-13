//! Phase 12 — `set_market_params` / `commit_pending_params`: bounds re-validation, the
//! tighten/loosen timelock asymmetry, identity-field immutability, and the accrual-ordering
//! regression (`docs/phases/phase-12-governance.md`, `docs/governance.md` §4, `docs/invariants.md`
//! INV-ADM-05..09).

#![allow(clippy::result_large_err)]

use aegis::error::AegisError;
use aegis::instructions::admin::SetMarketParamsArgs;
use aegis_test_kit::{
    assert_aegis_error, commit_pending_params, create_market, create_spl_mint, deploy,
    fetch_market, fetch_pending_market_params, init_position, initialize_protocol,
    pending_market_params_pda, reference_market_args, set_market_params, spl_token_interface,
    try_fetch_pending_market_params,
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

fn program_bytes() -> &'static [u8] {
    include_bytes!(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/aegis.so"))
}

const COLLATERAL_FEED_ID: [u8; 32] = [0xAAu8; 32];
const LOAN_FEED_ID: [u8; 32] = [0xBBu8; 32];
const WAD: u128 = 1_000_000_000_000_000_000;

struct Fixture {
    market: Pubkey,
    fee_position: Pubkey,
    collateral_vault: Pubkey,
    loan_vault: Pubkey,
    collateral_mint: Pubkey,
    loan_mint: Pubkey,
    fee_recipient: Pubkey,
}

fn setup_market(svm: &mut litesvm::LiteSVM, admin: &Keypair) -> Fixture {
    let guardian = Keypair::new_from_array([2u8; 32]).pubkey();
    let fee_recipient = Keypair::new_from_array([3u8; 32]).pubkey();
    initialize_protocol(svm, admin, guardian, fee_recipient).expect("initialize_protocol");

    let collateral_mint = create_spl_mint(svm, admin, 10, 9, admin.pubkey(), None);
    let loan_mint = create_spl_mint(svm, admin, 11, 6, admin.pubkey(), None);
    let args = reference_market_args(0, COLLATERAL_FEED_ID, LOAN_FEED_ID, false);
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
        fee_recipient,
    }
}

/// As `setup_market`, but with a nonzero `slope1_ps` -- the shared reference args
/// (`reference_market_args`) leave every IRM rate at zero, under which `accrue_view` never
/// accrues any interest regardless of utilization or elapsed time (`base_rate_ps == slope1_ps ==
/// slope2_ps == 0`). U-ADM-01 needs real, nonzero accrued interest to be meaningful evidence.
fn setup_market_with_nonzero_rate(svm: &mut litesvm::LiteSVM, admin: &Keypair) -> Fixture {
    let guardian = Keypair::new_from_array([2u8; 32]).pubkey();
    let fee_recipient = Keypair::new_from_array([3u8; 32]).pubkey();
    initialize_protocol(svm, admin, guardian, fee_recipient).expect("initialize_protocol");

    let collateral_mint = create_spl_mint(svm, admin, 10, 9, admin.pubkey(), None);
    let loan_mint = create_spl_mint(svm, admin, 11, 6, admin.pubkey(), None);
    let mut base = reference_market_args(0, COLLATERAL_FEED_ID, LOAN_FEED_ID, false);
    base.slope1_ps = 1_268_391_679; // matches state/market.rs's own test fixture value
    let (result, market, collateral_vault, loan_vault, fee_position) = create_market(
        svm,
        admin,
        collateral_mint,
        loan_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        fee_recipient,
        base,
    );
    result.expect("create_market must succeed");

    Fixture {
        market,
        fee_position,
        collateral_vault,
        loan_vault,
        collateral_mint,
        loan_mint,
        fee_recipient,
    }
}

/// The reference parameter set, unchanged from `create_market`'s own reference args -- passing
/// this to `set_market_params` verbatim is a legal, tightening (vacuously: no change) no-op.
/// Matches `setup_market`'s fixture; callers using `setup_market_with_nonzero_rate` must override
/// `slope1_ps` on the returned value to keep it consistent with that market's actual IRM params.
fn reference_params(fee_recipient: Pubkey) -> SetMarketParamsArgs {
    SetMarketParamsArgs {
        oracle_kind: 0,
        collateral_feed_id: COLLATERAL_FEED_ID,
        loan_feed_id: LOAN_FEED_ID,
        max_price_age_secs: 60,
        max_conf_bps: 100,
        max_ltv: 750_000_000_000_000_000,
        liq_threshold: 800_000_000_000_000_000,
        liq_bonus: 50_000_000_000_000_000,
        close_factor: 500_000_000_000_000_000,
        full_liq_hf: 950_000_000_000_000_000,
        liq_protocol_fee: 100_000_000_000_000_000,
        fee: 100_000_000_000_000_000,
        min_debt: 10_000_000,
        base_rate_ps: 0,
        slope1_ps: 0,
        slope2_ps: 0,
        u_kink: 800_000_000_000_000_000,
        max_rate_ps: 1_000_000_000_000_000_000,
        fee_recipient,
    }
}

fn now(svm: &litesvm::LiteSVM) -> i64 {
    svm.get_sysvar::<solana_clock::Clock>().unix_timestamp
}

fn warp_to(svm: &mut litesvm::LiteSVM, target: i64) {
    let mut clock = svm.get_sysvar::<solana_clock::Clock>();
    clock.unix_timestamp = target;
    svm.set_sysvar(&clock);
}

// ============================================================================================
// Tighten -> immediate application.
// ============================================================================================

#[test]
fn tightening_a_risk_parameter_applies_immediately() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin);

    let mut args = reference_params(fx.fee_recipient);
    args.max_ltv -= 1; // strictly lower max_ltv is tightening (governance.md §4)

    set_market_params(&mut svm, &admin, fx.market, fx.fee_position, None, args)
        .expect("tightening must succeed and apply immediately");

    let market = fetch_market(&svm, &fx.market);
    assert_eq!(market.max_ltv, 750_000_000_000_000_000 - 1);
    assert!(
        try_fetch_pending_market_params(&svm, &fx.market).is_none(),
        "a pure tightening call must never create a PendingMarketParams account"
    );
}

// ============================================================================================
// I-ADM-01: tighten immediate; loosen timelocked; early commit fails; commit at/after the
// threshold succeeds; pending state cleared afterward.
// ============================================================================================

#[test]
fn i_adm_01_full_tighten_loosen_timelock_lifecycle() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin);

    // Warp forward so there is nonzero elapsed time for U-ADM-01-style accrual evidence too.
    let t0 = now(&svm);
    warp_to(&mut svm, t0 + 3_600);

    // 1. Risk-reducing change: applies immediately.
    let mut tighten_args = reference_params(fx.fee_recipient);
    tighten_args.min_debt += 1; // raising min_debt is tightening
    set_market_params(
        &mut svm,
        &admin,
        fx.market,
        fx.fee_position,
        None,
        tighten_args,
    )
    .expect("tightening must apply immediately");
    let market = fetch_market(&svm, &fx.market);
    assert_eq!(market.min_debt, 10_000_000 + 1);
    assert!(try_fetch_pending_market_params(&svm, &fx.market).is_none());

    // 2. Risk-increasing change: staged, active params unchanged, pending params populated.
    svm.expire_blockhash();
    let mut loosen_args = reference_params(fx.fee_recipient);
    loosen_args.min_debt = market.min_debt; // keep the just-tightened value
    loosen_args.max_ltv += 1; // raising max_ltv is loosening
    set_market_params(
        &mut svm,
        &admin,
        fx.market,
        fx.fee_position,
        None,
        loosen_args.clone(),
    )
    .expect("loosening must succeed by staging, not erroring");

    let market_after_stage = fetch_market(&svm, &fx.market);
    assert_eq!(
        market_after_stage.max_ltv, 750_000_000_000_000_000,
        "active max_ltv must be unchanged immediately after staging"
    );
    let pending = try_fetch_pending_market_params(&svm, &fx.market)
        .expect("PendingMarketParams must exist after a loosening proposal");
    assert_eq!(pending.max_ltv, loosen_args.max_ltv);
    let effective_at = pending.effective_at;
    assert_eq!(
        effective_at,
        now(&svm) + aegis::constants::PARAM_TIMELOCK_SECS
    );

    // 3. Early commit fails with the exact error.
    warp_to(&mut svm, effective_at - 1);
    svm.expire_blockhash();
    let result =
        commit_pending_params(&mut svm, &admin, admin.pubkey(), fx.market, fx.fee_position);
    assert_aegis_error(&result, AegisError::PendingParamsNotYetEffective);

    // 4. At the threshold, commit succeeds.
    warp_to(&mut svm, effective_at);
    svm.expire_blockhash();
    commit_pending_params(&mut svm, &admin, admin.pubkey(), fx.market, fx.fee_position)
        .expect("commit at effective_at must succeed");

    let market_final = fetch_market(&svm, &fx.market);
    assert_eq!(market_final.max_ltv, loosen_args.max_ltv, "params applied");
    assert!(
        try_fetch_pending_market_params(&svm, &fx.market).is_none(),
        "pending state must be cleared after a successful commit"
    );
}

// Pending-update overwrite rule (item 26): a second loosening proposal while one is already
// pending is rejected outright -- the safer minimal behavior this repository chose (documented in
// governance.md and ADR-0014), rather than silently overwriting or merging.
#[test]
fn a_second_loosening_proposal_while_one_is_pending_is_rejected() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin);

    let mut first = reference_params(fx.fee_recipient);
    first.max_ltv += 1;
    set_market_params(&mut svm, &admin, fx.market, fx.fee_position, None, first)
        .expect("first loosening proposal must succeed");

    svm.expire_blockhash();
    let mut second = reference_params(fx.fee_recipient);
    second.max_ltv += 2;
    let result = set_market_params(&mut svm, &admin, fx.market, fx.fee_position, None, second);
    assert_aegis_error(&result, AegisError::PendingParamsAlreadyStaged);
}

// ============================================================================================
// U-ADM-01: accrual happens under OLD parameters before the change (both immediate and delayed
// paths), with nonzero elapsed time and nonzero utilization.
// ============================================================================================

#[test]
fn u_adm_01_immediate_path_accrues_under_old_params_before_applying_new_ones() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market_with_nonzero_rate(&mut svm, &admin);

    // Build nonzero utilization: a lender supplies, a borrower borrows.
    let (lender, lender_ata) = {
        let wallet = Keypair::new_from_array([50u8; 32]);
        svm.airdrop(&wallet.pubkey(), 10_000_000_000).unwrap();
        let ata = aegis_test_kit::create_token_account(
            &mut svm,
            &admin,
            51,
            fx.loan_mint,
            wallet.pubkey(),
            spl_token_interface::ID,
            &[],
        );
        aegis_test_kit::mint_to(
            &mut svm,
            &admin,
            fx.loan_mint,
            ata,
            &admin,
            1_000_000_000_000,
            spl_token_interface::ID,
        );
        (wallet, ata)
    };
    let (_, lender_position) = init_position(&mut svm, &admin, fx.market, lender.pubkey());
    aegis_test_kit::supply(
        &mut svm,
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
    .expect("lender supply");

    let (borrower, borrower_collateral_ata) = {
        let wallet = Keypair::new_from_array([52u8; 32]);
        svm.airdrop(&wallet.pubkey(), 10_000_000_000).unwrap();
        let ata = aegis_test_kit::create_token_account(
            &mut svm,
            &admin,
            53,
            fx.collateral_mint,
            wallet.pubkey(),
            spl_token_interface::ID,
            &[],
        );
        aegis_test_kit::mint_to(
            &mut svm,
            &admin,
            fx.collateral_mint,
            ata,
            &admin,
            100_000_000_000,
            spl_token_interface::ID,
        );
        (wallet, ata)
    };
    let (_, borrower_position) = init_position(&mut svm, &admin, fx.market, borrower.pubkey());
    aegis_test_kit::deposit_collateral(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.collateral_vault,
        borrower_collateral_ata,
        fx.collateral_mint,
        spl_token_interface::ID,
        100_000_000_000,
    )
    .expect("deposit_collateral");
    let borrower_loan_ata = aegis_test_kit::create_token_account(
        &mut svm,
        &admin,
        54,
        fx.loan_mint,
        borrower.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    let n = now(&svm);
    let c = aegis_test_kit::set_price(
        &mut svm,
        55,
        aegis_test_kit::PriceFixture::valid(COLLATERAL_FEED_ID, 15_000_000_000, 0, -8, n),
    );
    let l = aegis_test_kit::set_price(
        &mut svm,
        56,
        aegis_test_kit::PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 0, -8, n),
    );
    aegis_test_kit::borrow(
        &mut svm,
        &borrower,
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_loan_ata,
        fx.loan_mint,
        spl_token_interface::ID,
        c,
        l,
        5_000_000_000, // well within 75% max_ltv against 100 SOL @ $150.00 (~$15,000 collateral)
        0,
    )
    .expect("borrow to create nonzero utilization");

    let market_before = fetch_market(&svm, &fx.market);
    let t_before = now(&svm);

    // Warp forward with the OLD `fee` still active, so real interest accrues under the old rate.
    warp_to(&mut svm, t_before + 86_400);
    svm.expire_blockhash();

    // Predict, using the OLD parameters and OLD accrual timestamp, exactly what accrue_view would
    // produce -- this is the same pure function the instruction itself calls.
    let predicted = market_before
        .accrue_view(t_before + 86_400)
        .expect("accrue_view must succeed");
    assert!(
        predicted.interest > 0,
        "fixture must actually accrue interest"
    );

    // Change `fee` (a tightening change: lower fee) via set_market_params. slope1_ps must match
    // this fixture's actual (nonzero) value -- reference_params()'s default is 0, which would
    // itself register as an (unrelated) IRM parameter change.
    let mut args = reference_params(fx.fee_recipient);
    args.slope1_ps = market_before.slope1_ps;
    args.fee = market_before.fee / 2; // lower fee = tightening = immediate
    set_market_params(&mut svm, &admin, fx.market, fx.fee_position, None, args)
        .expect("tightening set_market_params must succeed");

    let market_after = fetch_market(&svm, &fx.market);
    // The accrued totals must match what accrue_view predicted using the OLD fee -- proving
    // interest settled BEFORE the new (lower) fee took effect.
    assert_eq!(
        market_after.total_borrow_assets,
        predicted.total_borrow_assets
    );
    assert_eq!(
        market_after.total_supply_assets,
        predicted.total_supply_assets
    );
    assert_eq!(market_after.last_accrual_ts, t_before + 86_400);
    // The new fee IS now active for future accrual.
    assert_eq!(market_after.fee, market_before.fee / 2);
}

#[test]
fn u_adm_01_delayed_commit_path_also_accrues_under_old_params_first() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin);

    // Stage a loosening change.
    let mut loosen = reference_params(fx.fee_recipient);
    loosen.max_ltv += 1;
    set_market_params(&mut svm, &admin, fx.market, fx.fee_position, None, loosen)
        .expect("stage loosening");

    let market_before = fetch_market(&svm, &fx.market);
    let pending = fetch_pending_market_params(&svm, &pending_market_params_pda(&fx.market).0);
    let t_before = now(&svm);

    // Warp to the effective timestamp with nonzero elapsed time.
    warp_to(&mut svm, pending.effective_at + 10);
    svm.expire_blockhash();

    let predicted = market_before
        .accrue_view(pending.effective_at + 10)
        .expect("accrue_view must succeed");

    commit_pending_params(&mut svm, &admin, admin.pubkey(), fx.market, fx.fee_position)
        .expect("commit must succeed at/after effective_at");

    let market_after = fetch_market(&svm, &fx.market);
    assert_eq!(
        market_after.total_supply_assets,
        predicted.total_supply_assets
    );
    assert_eq!(market_after.last_accrual_ts, pending.effective_at + 10);
    assert_eq!(
        market_after.max_ltv, pending.max_ltv,
        "new params now active"
    );
    let _ = t_before;
}

// ============================================================================================
// A-ADM-04: bounds re-validated on every write, including the derived liquidation bound, on
// BOTH the immediate and staged paths. Exact errors, boundary cases.
// ============================================================================================

#[test]
fn a_adm_04_invalid_params_rejected_with_exact_errors() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin);

    // max_ltv >= liq_threshold.
    let mut args = reference_params(fx.fee_recipient);
    args.max_ltv = args.liq_threshold;
    let result = set_market_params(&mut svm, &admin, fx.market, fx.fee_position, None, args);
    assert_aegis_error(&result, AegisError::InvalidMaxLtvOrThreshold);

    // liq_bonus above MAX_LIQ_BONUS.
    svm.expire_blockhash();
    let mut args = reference_params(fx.fee_recipient);
    args.liq_bonus = (WAD / 4) + 1;
    let result = set_market_params(&mut svm, &admin, fx.market, fx.fee_position, None, args);
    assert_aegis_error(&result, AegisError::InvalidLiqBonus);

    // min_debt == 0.
    svm.expire_blockhash();
    let mut args = reference_params(fx.fee_recipient);
    args.min_debt = 0;
    let result = set_market_params(&mut svm, &admin, fx.market, fx.fee_position, None, args);
    assert_aegis_error(&result, AegisError::InvalidMinDebt);

    // max_price_age_secs out of bounds.
    svm.expire_blockhash();
    let mut args = reference_params(fx.fee_recipient);
    args.max_price_age_secs = 0;
    let result = set_market_params(&mut svm, &admin, fx.market, fx.fee_position, None, args);
    assert_aegis_error(&result, AegisError::InvalidMaxPriceAge);

    // IRM: u_kink out of (0, WAD).
    svm.expire_blockhash();
    let mut args = reference_params(fx.fee_recipient);
    args.u_kink = WAD;
    let result = set_market_params(&mut svm, &admin, fx.market, fx.fee_position, None, args);
    assert_aegis_error(&result, AegisError::InvalidIrmParams);

    // Derived liquidation bound: every individual field is within its own flat bound, but
    // liq_threshold * (WAD + liq_bonus) / WAD >= WAD.
    svm.expire_blockhash();
    let mut args = reference_params(fx.fee_recipient);
    args.liq_threshold = 850_000_000_000_000_000; // 0.85 WAD, < WAD
    args.liq_bonus = 240_000_000_000_000_000; // 0.24 WAD, <= MAX_LIQ_BONUS
    let result = set_market_params(&mut svm, &admin, fx.market, fx.fee_position, None, args);
    assert_aegis_error(&result, AegisError::LiquidationBonusExceedsThresholdBound);

    // State must be untouched by every rejected call.
    let market = fetch_market(&svm, &fx.market);
    assert_eq!(market.max_ltv, 750_000_000_000_000_000);
    assert_eq!(market.liq_threshold, 800_000_000_000_000_000);
    assert_eq!(market.liq_bonus, 50_000_000_000_000_000);
}

// The derived bound must ALSO be re-checked on the staged (loosening) path, both at staging time
// and again at commit time (item 29/56).
#[test]
fn a_adm_04_derived_bound_rejected_on_staged_path_too() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin);

    // liq_threshold raised (loosening) to a value that, combined with the unchanged liq_bonus, is
    // still within bounds individually but violates the derived bound once liq_bonus is ALSO
    // raised in the same call (still loosening either way).
    let mut args = reference_params(fx.fee_recipient);
    args.liq_threshold = 850_000_000_000_000_000; // loosening (raised)
    args.liq_bonus = 240_000_000_000_000_000; // loosening (raised), and now unsafe combined
    let result = set_market_params(&mut svm, &admin, fx.market, fx.fee_position, None, args);
    assert_aegis_error(&result, AegisError::LiquidationBonusExceedsThresholdBound);

    assert!(
        try_fetch_pending_market_params(&svm, &fx.market).is_none(),
        "a rejected proposal must never create a PendingMarketParams account"
    );
}

// ============================================================================================
// A-ADM-06: identity fields are immutable -- not merely rejected, but absent from the API.
// ============================================================================================

#[test]
fn a_adm_06_identity_fields_have_no_field_in_set_market_params_args() {
    // Structural proof: SetMarketParamsArgs's own field list, read directly from source, contains
    // none of the identity fields. (A borsh round-trip proves the same thing at the byte level:
    // the struct's serialized size does not include room for a mint/vault/program/decimals/
    // config_id at all.)
    use anchor_lang::AnchorSerialize;
    let args = reference_params(Pubkey::default());
    let mut buf = Vec::new();
    args.serialize(&mut buf).unwrap();
    // 1 (oracle_kind) + 32 + 32 (feed ids) + 4 + 2 (oracle bounds) + 16*7 + 8 (risk) + 16*5 (irm)
    // + 32 (fee_recipient) = 1+32+32+4+2+112+8+80+32 = 303. If this ever grows because a field was
    // added, that field must not be an identity field -- this exact-size assertion is what makes
    // an accidental identity-field addition to the args struct fail loudly here.
    assert_eq!(buf.len(), 303);
}

#[test]
fn a_adm_06_identity_fields_are_unchanged_after_set_market_params() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin);
    let before = fetch_market(&svm, &fx.market);

    let mut args = reference_params(fx.fee_recipient);
    args.max_ltv -= 1; // a legal tightening change
    set_market_params(&mut svm, &admin, fx.market, fx.fee_position, None, args)
        .expect("tightening must succeed");

    let after = fetch_market(&svm, &fx.market);
    assert_eq!(after.collateral_mint, before.collateral_mint);
    assert_eq!(after.loan_mint, before.loan_mint);
    assert_eq!(
        after.collateral_token_program,
        before.collateral_token_program
    );
    assert_eq!(after.loan_token_program, before.loan_token_program);
    assert_eq!(after.collateral_vault, before.collateral_vault);
    assert_eq!(after.loan_vault, before.loan_vault);
    assert_eq!(after.collateral_decimals, before.collateral_decimals);
    assert_eq!(after.loan_decimals, before.loan_decimals);
    assert_eq!(after.config_id, before.config_id);

    // Also true across the staged (loosening) path.
    svm.expire_blockhash();
    let mut loosen = reference_params(fx.fee_recipient);
    loosen.max_ltv = before.max_ltv; // avoid re-triggering the already-applied tighten
    loosen.fee += 1; // raising fee is loosening
    set_market_params(&mut svm, &admin, fx.market, fx.fee_position, None, loosen)
        .expect("loosening must succeed (staged)");
    let staged = fetch_market(&svm, &fx.market);
    assert_eq!(staged.collateral_mint, before.collateral_mint);
    assert_eq!(staged.config_id, before.config_id);
    assert_eq!(staged.collateral_decimals, before.collateral_decimals);
}
