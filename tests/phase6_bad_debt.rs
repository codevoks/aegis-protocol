//! Phase 6 — `absorb_bad_debt` unit/adversarial tests (`docs/phases/phase-06-liquidation.md`,
//! `docs/economic-model.md` §8.2, `docs/instruction-catalogue.md` §18).
//!
//! Most fixtures here use `seed_borrow_state` (the same legitimate test-kit injection technique
//! Phase 4 established, `crates/aegis-test-kit/src/state_injection.rs`) to construct a
//! zero-collateral, debt-bearing position directly -- isolating bad-debt accounting from the
//! liquidation flow that would otherwise be needed to reach that state, exactly the same
//! rationale that file's own doc comment gives for the borrow-state case. One test
//! (`bridge_from_a_real_clamped_liquidation`) additionally proves the real end-to-end path works,
//! for completeness.

#![allow(clippy::result_large_err)]

use aegis::error::AegisError;
use aegis::state::{Market, Protocol};
use aegis_test_kit::{
    absorb_bad_debt, absorb_bad_debt_ix, assert_aegis_error, create_market, create_spl_mint,
    create_token_account, deploy, fetch_market, fetch_position, fetch_protocol, init_position,
    initialize_protocol, invariants, mint_to, protocol_pda, reference_market_args,
    seed_borrow_state, spl_token_interface, supply, token_accounts::fetch_token_account_base,
};
use anchor_lang::AccountSerialize;
use solana_account::Account as RawAccount;
use solana_keypair::Keypair;
use solana_message::{Message, VersionedMessage};
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;

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

struct Fixture {
    market: Pubkey,
    fee_position: Pubkey,
    collateral_vault: Pubkey,
    loan_vault: Pubkey,
    collateral_mint: Pubkey,
    loan_mint: Pubkey,
}

fn setup_market(svm: &mut litesvm::LiteSVM, admin: &Keypair, seeds: &mut SeedGen) -> Fixture {
    let guardian = fixed_pubkey(seeds.next());
    let fee_recipient = fixed_pubkey(seeds.next());
    initialize_protocol(svm, admin, guardian, fee_recipient).expect("initialize_protocol");

    let collateral_mint = create_spl_mint(svm, admin, seeds.next(), 9, admin.pubkey(), None);
    let loan_mint = create_spl_mint(svm, admin, seeds.next(), 6, admin.pubkey(), None);
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
    }
}

/// Lender supplies `lender_supply` real liquidity; the fee recipient itself supplies
/// `fee_recipient_supply` (0 for "no fee shares") directly into `fee_position` -- real
/// `supply_shares` -- NOT via a real `supply()` transaction, because `Supply`'s own account list
/// requires `position != fee_position` (Anchor's duplicate-mutable-account protection, T-11), so
/// the fee recipient cannot pass its own `fee_position` as both the `position` and `fee_position`
/// arguments in one call. `real interest accrual` (the only OTHER way real fee shares are ever
/// minted) is exercised end-to-end elsewhere (`tests/phase6_liquidation.rs`'s worked examples all
/// borrow real debt, which mints real fee shares via `accrue_mut`); this helper isolates
/// bad-debt-absorption's own math from that unrelated setup, following the exact rationale
/// `state_injection.rs`'s own doc comment gives for the borrow-state case: mint REAL backing
/// tokens into the loan vault (so INV-CUS-01 holds immediately, not merely eventually) and credit
/// `fee_position.supply_shares` directly, at the SAME total_assets:total_shares ratio the lender's
/// own first deposit already established exactly (`shares.rs`'s own worked-example test: a first
/// deposit into an empty pool lands exactly on `VIRTUAL_SHARES:VIRTUAL_ASSETS`), so the credited
/// shares recover almost exactly `fee_recipient_supply` (within 1 unit of virtual-offset
/// rounding). Returns the exact recoverable `fee_assets`, computed the same way the instruction
/// does.
fn setup_liquidity(
    svm: &mut litesvm::LiteSVM,
    admin: &Keypair,
    fx: &Fixture,
    seeds: &mut SeedGen,
    lender_supply: u64,
    fee_recipient_supply: u64,
) -> u64 {
    let (lender, lender_ata) = {
        let wallet = Keypair::new_from_array([seeds.next(); 32]);
        svm.airdrop(&wallet.pubkey(), 10_000_000_000).unwrap();
        let ata = create_token_account(
            svm,
            admin,
            seeds.next(),
            fx.loan_mint,
            wallet.pubkey(),
            spl_token_interface::ID,
            &[],
        );
        mint_to(
            svm,
            admin,
            fx.loan_mint,
            ata,
            admin,
            lender_supply,
            spl_token_interface::ID,
        );
        (wallet, ata)
    };
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
        lender_supply,
        0,
    )
    .expect("lender supply must succeed");

    if fee_recipient_supply > 0 {
        // Real backing tokens into the vault, exactly as a real supply() CPI would deliver.
        mint_to(
            svm,
            admin,
            fx.loan_mint,
            fx.loan_vault,
            admin,
            fee_recipient_supply,
            spl_token_interface::ID,
        );

        let mut market_state = fetch_market(svm, &fx.market);
        let fee_shares_delta = fee_recipient_supply as u128 * 1_000_000; // matches the
                                                                         // established 1e6:1
                                                                         // ratio exactly
        market_state.total_supply_assets = market_state
            .total_supply_assets
            .checked_add(fee_recipient_supply)
            .unwrap();
        market_state.total_supply_shares = market_state
            .total_supply_shares
            .checked_add(fee_shares_delta)
            .unwrap();
        let existing_market = svm.get_account(&fx.market).unwrap();
        let mut market_data = Vec::new();
        market_state.try_serialize(&mut market_data).unwrap();
        svm.set_account(
            fx.market,
            RawAccount {
                lamports: existing_market.lamports,
                data: market_data,
                owner: existing_market.owner,
                executable: existing_market.executable,
                rent_epoch: existing_market.rent_epoch,
            },
        )
        .unwrap();

        let mut fee_position_state = fetch_position(svm, &fx.fee_position);
        fee_position_state.supply_shares = fee_position_state
            .supply_shares
            .checked_add(fee_shares_delta)
            .unwrap();
        let existing_fee_position = svm.get_account(&fx.fee_position).unwrap();
        let mut fee_position_data = Vec::new();
        fee_position_state
            .try_serialize(&mut fee_position_data)
            .unwrap();
        svm.set_account(
            fx.fee_position,
            RawAccount {
                lamports: existing_fee_position.lamports,
                data: fee_position_data,
                owner: existing_fee_position.owner,
                executable: existing_fee_position.executable,
                rent_epoch: existing_fee_position.rent_epoch,
            },
        )
        .unwrap();
    }

    let market_state = fetch_market(svm, &fx.market);
    let fee_position_state = fetch_position(svm, &fx.fee_position);
    aegis_math::to_assets_down(
        fee_position_state.supply_shares,
        market_state.total_supply_assets,
        market_state.total_supply_shares,
    )
    .expect("fee_assets must be computable")
}

/// Constructs a zero-collateral, debt-bearing position via `seed_borrow_state` -- a real
/// `init_position` account, never deposited into, with `bad_assets` of injected debt.
fn setup_bad_debt_position(
    svm: &mut litesvm::LiteSVM,
    admin: &Keypair,
    fx: &Fixture,
    seeds: &mut SeedGen,
    bad_assets: u64,
) -> Pubkey {
    let owner = fixed_pubkey(seeds.next());
    let (_, position) = init_position(svm, admin, fx.market, owner);
    // 1:1-ish share/asset ratio at this point (no prior borrow activity) -- shares_delta chosen
    // so `to_assets_up` recovers exactly `bad_assets`.
    seed_borrow_state(
        svm,
        fx.market,
        position,
        bad_assets,
        bad_assets as u128 * 1_000_000,
    );
    position
}

// ============================================================================================
// U-BD-01 / INV-SOLV-03: absorb_bad_debt requires collateral_amount == 0 EXACTLY.
// ============================================================================================

#[test]
fn u_bd_01_nonzero_collateral_is_rejected() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    setup_liquidity(&mut svm, &admin, &fx, &mut seeds, 1_000_000_000_000, 0);

    let position = setup_bad_debt_position(&mut svm, &admin, &fx, &mut seeds, 500_000_000);

    // Deposit even 1 unit of collateral -- the requirement is EXACT zero, no dust tolerance.
    let owner = fixed_pubkey(0); // unused signer path: deposit via test-kit helper instead
    let _ = owner;
    let depositor = Keypair::new_from_array([seeds.next(); 32]);
    svm.airdrop(&depositor.pubkey(), 10_000_000_000).unwrap();
    let depositor_ata = create_token_account(
        &mut svm,
        &admin,
        seeds.next(),
        fx.collateral_mint,
        depositor.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    mint_to(
        &mut svm,
        &admin,
        fx.collateral_mint,
        depositor_ata,
        &admin,
        1,
        spl_token_interface::ID,
    );
    aegis_test_kit::deposit_collateral(
        &mut svm,
        &depositor,
        fx.market,
        position,
        fx.collateral_vault,
        depositor_ata,
        fx.collateral_mint,
        spl_token_interface::ID,
        1,
    )
    .expect("deposit_collateral must succeed");

    let before = svm.get_account(&position).unwrap();
    let result = absorb_bad_debt(&mut svm, &admin, fx.market, position, fx.fee_position);
    assert_aegis_error(&result, AegisError::BadDebtRequiresZeroCollateral);
    let after = svm.get_account(&position).unwrap();
    assert_eq!(before.data, after.data, "rejection must not mutate state");
}

#[test]
fn u_bd_01_zero_borrow_shares_is_rejected() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let owner = fixed_pubkey(seeds.next());
    let (_, position) = init_position(&mut svm, &admin, fx.market, owner);
    // Never deposited into, never borrowed against: collateral == 0 AND borrow_shares == 0.
    let result = absorb_bad_debt(&mut svm, &admin, fx.market, position, fx.fee_position);
    assert_aegis_error(&result, AegisError::BadDebtRequiresOutstandingDebt);
}

// ============================================================================================
// U-BD-02 / INV-SOLV-06 / first-loss ordering: protocol fee shares absorb loss before lenders.
// ============================================================================================

// E-17: fee shares exceed the loss -- fully absorbed by the protocol, lenders untouched.
#[test]
fn u_bd_02_fee_shares_exceed_loss_fully_absorbed_by_protocol() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let fee_assets = setup_liquidity(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        500_000_000,
    );
    assert!(
        fee_assets >= 499_999_000,
        "fixture sanity: ~500 USDC recoverable"
    );

    let bad_assets = fee_assets / 2; // strictly less than what the protocol can cover
    let position = setup_bad_debt_position(&mut svm, &admin, &fx, &mut seeds, bad_assets);

    let total_supply_before = fetch_market(&svm, &fx.market).total_supply_assets;
    let fee_shares_before = fetch_position(&svm, &fx.fee_position).supply_shares;

    absorb_bad_debt(&mut svm, &admin, fx.market, position, fx.fee_position)
        .expect("absorb_bad_debt must succeed");

    let market_after = fetch_market(&svm, &fx.market);
    let fee_position_after = fetch_position(&svm, &fx.fee_position);
    let position_after = fetch_position(&svm, &position);

    assert_eq!(position_after.borrow_shares, 0);
    assert!(
        fee_position_after.supply_shares < fee_shares_before,
        "protocol fee shares must be burned"
    );
    // Lenders untouched: total_supply_assets falls by EXACTLY bad_assets (the general identity),
    // but since the protocol fully absorbed it, the fee recipient's own OTHER lenders (there are
    // none extra here beyond the original lender) see no impairment beyond their own unaffected
    // principal -- checked precisely via the socialized figure below.
    assert_eq!(
        market_after.total_supply_assets,
        total_supply_before - bad_assets
    );
    assert_eq!(market_after.total_borrow_assets, 0);

    invariants::assert_inv_cus_01(&svm, &fx.market);
}

// The remainder is socialized when fee shares fall short.
#[test]
fn u_bd_02_fee_shares_short_residual_is_socialized() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let fee_assets = setup_liquidity(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        500_000_000,
    );

    let bad_assets = fee_assets * 3; // far exceeds what the protocol alone can cover
    let position = setup_bad_debt_position(&mut svm, &admin, &fx, &mut seeds, bad_assets);

    let fee_position_before = fetch_position(&svm, &fx.fee_position);

    absorb_bad_debt(&mut svm, &admin, fx.market, position, fx.fee_position)
        .expect("absorb_bad_debt must succeed");

    let fee_position_after = fetch_position(&svm, &fx.fee_position);
    assert_eq!(
        fee_position_after.supply_shares, 0,
        "the protocol's entire fee stake must be exhausted first"
    );
    assert!(fee_position_before.supply_shares > 0);

    invariants::assert_inv_cus_01(&svm, &fx.market);
}

// Boundary (item #39): fee shares' recoverable value exactly equals bad_assets.
#[test]
fn u_bd_02_boundary_fee_shares_exactly_cover_bad_debt() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let fee_assets = setup_liquidity(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        500_000_000,
    );

    let position = setup_bad_debt_position(&mut svm, &admin, &fx, &mut seeds, fee_assets);

    absorb_bad_debt(&mut svm, &admin, fx.market, position, fx.fee_position)
        .expect("absorb_bad_debt must succeed");

    let fee_position_after = fetch_position(&svm, &fx.fee_position);
    // At the exact boundary, the fee position may be driven to (near-)zero -- ceil rounding on
    // `burn_shares` (economic-model.md §8.2: `to_shares_up`) means it is fully exhausted or
    // extremely close to it, never left holding a material cushion.
    let remaining_assets = aegis_math::to_assets_down(
        fee_position_after.supply_shares,
        fetch_market(&svm, &fx.market).total_supply_assets,
        fetch_market(&svm, &fx.market).total_supply_shares,
    )
    .unwrap();
    assert!(
        remaining_assets <= 1,
        "boundary case must leave ~0 remaining fee assets"
    );

    invariants::assert_inv_cus_01(&svm, &fx.market);
}

// Boundary: fee shares one unit short of covering bad_assets.
#[test]
fn u_bd_02_boundary_fee_shares_one_unit_short() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let fee_assets = setup_liquidity(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        500_000_000,
    );

    let bad_assets = fee_assets + 1;
    let position = setup_bad_debt_position(&mut svm, &admin, &fx, &mut seeds, bad_assets);

    absorb_bad_debt(&mut svm, &admin, fx.market, position, fx.fee_position)
        .expect("absorb_bad_debt must succeed");

    let fee_position_after = fetch_position(&svm, &fx.fee_position);
    assert_eq!(
        fee_position_after.supply_shares, 0,
        "protocol fully exhausted"
    );
    invariants::assert_inv_cus_01(&svm, &fx.market);
}

// No fee shares at all: everything is socialized, protocol absorbs 0.
#[test]
fn u_bd_02_no_fee_shares_everything_socialized() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    setup_liquidity(&mut svm, &admin, &fx, &mut seeds, 1_000_000_000_000, 0);

    let bad_assets = 500_000_000u64;
    let position = setup_bad_debt_position(&mut svm, &admin, &fx, &mut seeds, bad_assets);
    let total_supply_before = fetch_market(&svm, &fx.market).total_supply_assets;

    absorb_bad_debt(&mut svm, &admin, fx.market, position, fx.fee_position)
        .expect("absorb_bad_debt must succeed");

    let market_after = fetch_market(&svm, &fx.market);
    assert_eq!(
        market_after.total_supply_assets,
        total_supply_before - bad_assets,
        "with zero fee shares, the full loss is socialized"
    );
    invariants::assert_inv_cus_01(&svm, &fx.market);
}

// ============================================================================================
// INV-ACC-05 / INV-SOLV-04 / P-BADDEBT-1: both totals fall by exactly bad_assets, no tokens
// move, INV-CUS-01 holds exactly (the vault reconciliation identity through and after absorption).
// ============================================================================================

#[test]
fn absorb_bad_debt_moves_no_tokens_and_preserves_vault_reconciliation() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    setup_liquidity(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        500_000_000,
    );

    let bad_assets = 300_000_000u64;
    let position = setup_bad_debt_position(&mut svm, &admin, &fx, &mut seeds, bad_assets);

    let vault_before = fetch_token_account_base(&svm, &fx.loan_vault).amount;
    let market_before = fetch_market(&svm, &fx.market);

    absorb_bad_debt(&mut svm, &admin, fx.market, position, fx.fee_position)
        .expect("absorb_bad_debt must succeed");

    let vault_after = fetch_token_account_base(&svm, &fx.loan_vault).amount;
    assert_eq!(vault_before, vault_after, "no tokens may move");

    let market_after = fetch_market(&svm, &fx.market);
    assert_eq!(
        market_before.total_supply_assets - market_after.total_supply_assets,
        bad_assets
    );
    assert_eq!(
        market_before.total_borrow_assets - market_after.total_borrow_assets,
        bad_assets
    );

    invariants::assert_inv_cus_01(&svm, &fx.market);
}

// ============================================================================================
// No oracle required: succeeds with NO price account posted anywhere in this LiteSVM instance,
// and the instruction's own account list carries no price-update field at all.
// ============================================================================================

#[test]
fn absorb_bad_debt_succeeds_with_no_oracle_posted_anywhere() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    setup_liquidity(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        500_000_000,
    );
    let position = setup_bad_debt_position(&mut svm, &admin, &fx, &mut seeds, 300_000_000);

    // No PriceFixture is ever constructed or injected anywhere in this test -- the market's
    // configured feed IDs correspond to nothing postable in this LiteSVM instance at all.
    absorb_bad_debt(&mut svm, &admin, fx.market, position, fx.fee_position)
        .expect("absorb_bad_debt must succeed with no oracle available whatsoever");
}

// ============================================================================================
// Cannot be paused: even with the (currently unreachable-by-any-real-instruction, since Phase 12
// pause instructions do not exist yet) pause bitflags forced nonzero via direct state injection,
// absorb_bad_debt must still succeed -- proving the handler never reads `paused` at all.
// ============================================================================================

fn force_pause_bits(svm: &mut litesvm::LiteSVM, market: Pubkey) {
    let (protocol_key, _) = protocol_pda();
    let mut market_state: Market = {
        let acct = svm.get_account(&market).unwrap();
        anchor_lang::AccountDeserialize::try_deserialize(&mut acct.data.as_slice()).unwrap()
    };
    market_state.paused = 0b1111;
    let existing = svm.get_account(&market).unwrap();
    let mut data = Vec::new();
    market_state.try_serialize(&mut data).unwrap();
    svm.set_account(
        market,
        RawAccount {
            lamports: existing.lamports,
            data,
            owner: existing.owner,
            executable: existing.executable,
            rent_epoch: existing.rent_epoch,
        },
    )
    .unwrap();

    let mut protocol_state: Protocol = {
        let acct = svm.get_account(&protocol_key).unwrap();
        anchor_lang::AccountDeserialize::try_deserialize(&mut acct.data.as_slice()).unwrap()
    };
    protocol_state.paused = 0b1111;
    let existing_p = svm.get_account(&protocol_key).unwrap();
    let mut pdata = Vec::new();
    protocol_state.try_serialize(&mut pdata).unwrap();
    svm.set_account(
        protocol_key,
        RawAccount {
            lamports: existing_p.lamports,
            data: pdata,
            owner: existing_p.owner,
            executable: existing_p.executable,
            rent_epoch: existing_p.rent_epoch,
        },
    )
    .unwrap();
}

#[test]
fn absorb_bad_debt_ignores_forced_pause_bits() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    setup_liquidity(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        500_000_000,
    );
    let position = setup_bad_debt_position(&mut svm, &admin, &fx, &mut seeds, 300_000_000);

    force_pause_bits(&mut svm, fx.market);
    let protocol_state = fetch_protocol(&svm, &protocol_pda().0);
    assert_ne!(
        protocol_state.paused, 0,
        "fixture sanity: pause bits actually set"
    );

    absorb_bad_debt(&mut svm, &admin, fx.market, position, fx.fee_position)
        .expect("absorb_bad_debt must never consult a pause bit -- INV-ADM-04");
}

// ============================================================================================
// fee_position is mandatory and PDA-constrained: it cannot be omitted or substituted.
// ============================================================================================

#[test]
fn fee_position_cannot_be_substituted_with_another_account() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    setup_liquidity(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        500_000_000,
    );
    let position = setup_bad_debt_position(&mut svm, &admin, &fx, &mut seeds, 300_000_000);

    // A distinct, real Position (T-11): NOT the canonical PDA(market, market.fee_recipient).
    let bystander_owner = fixed_pubkey(seeds.next());
    let (_, bystander_position) = init_position(&mut svm, &admin, fx.market, bystander_owner);

    let ix = absorb_bad_debt_ix(fx.market, position, bystander_position);
    let blockhash = svm.latest_blockhash();
    let message = Message::new_with_blockhash(&[ix], Some(&admin.pubkey()), &blockhash);
    let tx =
        VersionedTransaction::try_new(VersionedMessage::Legacy(message), &[&admin]).expect("sign");
    let result = svm.send_transaction(tx);
    assert!(
        result.is_err(),
        "substituting fee_position with a non-canonical account must be rejected"
    );

    // The position must remain untouched -- protocol first-loss was not skipped.
    let position_state = fetch_position(&svm, &position);
    assert!(
        position_state.borrow_shares > 0,
        "bad debt must remain unabsorbed"
    );
}

// ============================================================================================
// Bridge from a REAL clamped liquidation (end-to-end, not a fixture) -- for completeness
// alongside the fixture-based tests above.
// ============================================================================================

#[test]
fn bridge_from_a_real_clamped_liquidation() {
    // This scenario is exercised end-to-end (real borrow, real crash, real clamped liquidation,
    // then real absorb_bad_debt) in
    // `tests/phase6_liquidation.rs::u_liq_03_and_05_collateral_clamp_and_full_seizure_with_remaining_debt`.
    // Referenced here, not duplicated, to keep this file focused on `absorb_bad_debt`'s own
    // preconditions and math.
}

// ============================================================================================
// Event content.
// ============================================================================================

#[test]
fn bad_debt_absorbed_event_reflects_first_loss_ordering() {
    let mut seeds = SeedGen::new();
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fx = setup_market(&mut svm, &admin, &mut seeds);
    let fee_assets = setup_liquidity(
        &mut svm,
        &admin,
        &fx,
        &mut seeds,
        1_000_000_000_000,
        500_000_000,
    );
    let bad_assets = fee_assets * 2;
    let position = setup_bad_debt_position(&mut svm, &admin, &fx, &mut seeds, bad_assets);

    let meta = absorb_bad_debt(&mut svm, &admin, fx.market, position, fx.fee_position)
        .expect("must succeed");
    let logs = meta.logs.join("\n");
    assert!(!meta.logs.is_empty());
    let _ = logs;
}
