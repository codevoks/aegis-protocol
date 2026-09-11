//! Shared fixture code for the Phase 8 test files (`tests/phase8_composability.rs`,
//! `tests/phase8_hostile_callback.rs`). Named `mod.rs` under a subdirectory specifically so Cargo
//! does not treat it as its own integration-test binary (only `tests/*.rs` direct children are
//! auto-discovered as test targets); each Phase 8 test file pulls it in with `mod phase8_common;`.
//!
//! Mirrors `tests/phase6_liquidation.rs`'s `Fixture`/`setup_market`/`wallet_with_ata`/
//! `setup_borrowed_position`/`liquidator_wallets` shapes exactly, plus the Phase 8 additions:
//! deploying the two `labs/` programs alongside `aegis`, and driving the `example-liquidator` /
//! `hostile-callback` account contracts.

#![allow(dead_code, clippy::too_many_arguments, clippy::result_large_err)]

use aegis_test_kit::{
    create_market, create_spl_mint, create_token_account, deploy, deposit_collateral,
    init_position, initialize_protocol, mint_to, reference_market_args, set_price,
    spl_token_interface, supply, PriceFixture,
};
use anchor_lang::InstructionData;
use litesvm::LiteSVM;
use solana_instruction::AccountMeta;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

pub fn aegis_program_bytes() -> &'static [u8] {
    include_bytes!(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/aegis.so"))
}

pub fn example_liquidator_program_bytes() -> &'static [u8] {
    include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/example_liquidator.so"
    ))
}

pub fn hostile_callback_program_bytes() -> &'static [u8] {
    include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/hostile_callback.so"
    ))
}

/// As `aegis_test_kit::deploy`, but with the two Phase 8 lab programs also loaded into the same
/// LiteSVM instance -- one process, three real, separately-compiled `.so` artifacts, exactly the
/// zero-cost/offline model `docs/zero-cost-demo.md` §6 already establishes for `aegis` alone.
pub fn deploy_with_labs() -> (LiteSVM, Keypair) {
    let (mut svm, admin) = deploy(aegis::ID, aegis_program_bytes());
    svm.add_program(example_liquidator::ID, example_liquidator_program_bytes())
        .expect("failed to load example-liquidator into LiteSVM");
    svm.add_program(hostile_callback::ID, hostile_callback_program_bytes())
        .expect("failed to load hostile-callback into LiteSVM");
    (svm, admin)
}

pub const COLLATERAL_FEED_ID: [u8; 32] = [0xAAu8; 32];
pub const LOAN_FEED_ID: [u8; 32] = [0xBBu8; 32];

pub struct SeedGen(u8);
impl SeedGen {
    pub fn new() -> Self {
        Self(20)
    }
    pub fn next(&mut self) -> u8 {
        let s = self.0;
        self.0 = self.0.checked_add(1).expect("used more than 255 seeds");
        s
    }
}

pub fn fixed_pubkey(seed: u8) -> Pubkey {
    Keypair::new_from_array([seed; 32]).pubkey()
}

pub fn now(svm: &LiteSVM) -> i64 {
    svm.get_sysvar::<solana_clock::Clock>().unix_timestamp
}

pub struct Fixture {
    pub market: Pubkey,
    pub fee_position: Pubkey,
    pub collateral_vault: Pubkey,
    pub loan_vault: Pubkey,
    pub collateral_mint: Pubkey,
    pub loan_mint: Pubkey,
}

/// SOL(9dp)/USDC(6dp) reference market -- identical parameters to Phase 5/6's fixture
/// (`max_ltv=0.75, LT=0.80, bonus=0.05, close_factor=0.50, full_liq_hf=0.95,
/// liq_protocol_fee=0.10, min_debt=10 USDC`).
/// Returns the new market's `Fixture` plus the protocol-wide `fee_recipient` used to initialize
/// `Protocol` -- every market's `fee_position` is seeded from `protocol.fee_recipient`
/// specifically (`programs/aegis/src/instructions/admin/create_market.rs`), a single protocol-wide
/// value snapshotted per-market at creation, not a genuinely per-market choice -- so a second
/// market in the same `Protocol` (`setup_second_market`) must reuse this exact value.
pub fn setup_market(svm: &mut LiteSVM, admin: &Keypair, seeds: &mut SeedGen) -> (Fixture, Pubkey) {
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

    (
        Fixture {
            market,
            fee_position,
            collateral_vault,
            loan_vault,
            collateral_mint,
            loan_mint,
        },
        fee_recipient,
    )
}

/// As `setup_market`, but for a second (or later) market in an SVM that already has `Protocol`
/// initialized -- `Protocol` is one single, global PDA (`account-model.md` §3); calling
/// `initialize_protocol` twice in the same LiteSVM instance fails outright. `fee_recipient` must
/// be the exact value the first `setup_market` call used (see its doc comment).
pub fn setup_second_market(
    svm: &mut LiteSVM,
    admin: &Keypair,
    fee_recipient: Pubkey,
    seeds: &mut SeedGen,
) -> Fixture {
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

pub fn wallet_with_ata(
    svm: &mut LiteSVM,
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

pub fn valid_prices(svm: &mut LiteSVM, seeds: &mut SeedGen, at: i64) -> (Pubkey, Pubkey) {
    let c = set_price(
        svm,
        seeds.next(),
        PriceFixture::valid(COLLATERAL_FEED_ID, 15_000_000_000, 0, -8, at),
    );
    let l = set_price(
        svm,
        seeds.next(),
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 0, -8, at),
    );
    (c, l)
}

/// The exact SOL=$95.00±$0.20 / USDC=$1.0000±$0.0002 crash prices `tests/phase6_liquidation.rs`
/// uses, reused here so the liquidation math this file exercises is the same worked example.
pub fn crash_prices(svm: &mut LiteSVM, seeds: &mut SeedGen, at: i64) -> (Pubkey, Pubkey) {
    let c = set_price(
        svm,
        seeds.next(),
        PriceFixture::valid(COLLATERAL_FEED_ID, 9_500_000_000, 20_000_000, -8, at),
    );
    let l = set_price(
        svm,
        seeds.next(),
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 20_000, -8, at),
    );
    (c, l)
}

/// Lender supplies, borrower deposits and borrows against SOL at $150.00 -- returns everything a
/// liquidation test needs. Identical shape and identical worked-example inputs to
/// `tests/phase6_liquidation.rs::setup_borrowed_position`.
pub fn setup_borrowed_position(
    svm: &mut LiteSVM,
    admin: &Keypair,
    fx: &Fixture,
    seeds: &mut SeedGen,
    supply_amount: u64,
    collateral_amount: u64,
    borrow_amount: u64,
) -> (Keypair, Pubkey, Pubkey, Pubkey) {
    let (lender, lender_ata) = wallet_with_ata(svm, admin, fx.loan_mint, seeds, supply_amount);
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
        supply_amount,
        0,
    )
    .expect("supply must succeed");

    let (borrower, borrower_collateral_ata) =
        wallet_with_ata(svm, admin, fx.collateral_mint, seeds, collateral_amount);
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
        collateral_amount,
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
    let (c, l) = valid_prices(svm, seeds, n);
    aegis_test_kit::borrow(
        svm,
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
        borrow_amount,
        0,
    )
    .expect("borrow must succeed");

    (
        borrower,
        borrower_position,
        borrower_collateral_ata,
        borrower_loan_ata,
    )
}

pub fn liquidator_wallets(
    svm: &mut LiteSVM,
    admin: &Keypair,
    fx: &Fixture,
    seeds: &mut SeedGen,
    loan_balance: u64,
) -> (Keypair, Pubkey, Pubkey) {
    let (liquidator, liquidator_loan_ata) =
        wallet_with_ata(svm, admin, fx.loan_mint, seeds, loan_balance);
    let liquidator_collateral_ata = create_token_account(
        svm,
        admin,
        seeds.next(),
        fx.collateral_mint,
        liquidator.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    (liquidator, liquidator_loan_ata, liquidator_collateral_ata)
}

// --- example-liquidator (honest callback) helpers ---

pub fn example_liquidator_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"authority"], &example_liquidator::ID)
}

/// Creates and funds `example-liquidator`'s own two PDA-owned accounts: the collateral-receiving
/// account (starts empty -- Aegis funds it during the callback CPI) and the loan-asset reserve
/// (pre-funded here, exactly as a real deployment's operator would fund it ahead of time).
pub fn setup_example_liquidator_reserve(
    svm: &mut LiteSVM,
    admin: &Keypair,
    fx: &Fixture,
    seeds: &mut SeedGen,
    reserve_amount: u64,
) -> (Pubkey, Pubkey) {
    let (authority, _bump) = example_liquidator_authority_pda();

    let collateral_account = create_token_account(
        svm,
        admin,
        seeds.next(),
        fx.collateral_mint,
        authority,
        spl_token_interface::ID,
        &[],
    );
    let loan_reserve = create_token_account(
        svm,
        admin,
        seeds.next(),
        fx.loan_mint,
        authority,
        spl_token_interface::ID,
        &[],
    );
    if reserve_amount > 0 {
        mint_to(
            svm,
            admin,
            fx.loan_mint,
            loan_reserve,
            admin,
            reserve_amount,
            spl_token_interface::ID,
        );
    }

    (collateral_account, loan_reserve)
}

/// The `remaining_accounts` `example-liquidator::handle_liquidation` needs beyond Aegis's own
/// fixed 6 (ADR-0013 §1.4): its own `authority` PDA and `loan_reserve`.
pub fn example_liquidator_remaining_accounts(loan_reserve: Pubkey) -> Vec<AccountMeta> {
    let (authority, _bump) = example_liquidator_authority_pda();
    vec![
        AccountMeta::new_readonly(authority, false),
        AccountMeta::new(loan_reserve, false),
    ]
}

pub fn example_liquidator_callback_data(rate_wad: u128) -> Vec<u8> {
    example_liquidator::instruction::HandleLiquidation { rate_wad }.data()
}

// --- hostile-callback helpers ---

pub fn hostile_callback_data(mode: hostile_callback::AttackMode) -> Vec<u8> {
    hostile_callback::instruction::Attack { mode }.data()
}
