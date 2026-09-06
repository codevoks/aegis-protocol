//! Phase 2 — happy-path account model, lifecycle, and layout evidence.
//!
//! `U-ACCT-01` (`_reserved` zero), `U-ACCT-02` (no realloc needed — accounts are created at their
//! exact final size), `U-LIFE-02` (seed prefixes distinct), `A-CUS-03` (vault authority is the
//! `Market` PDA), `I-DEPLOY-01` (post-deploy admin assertion), INV-ACCT-07 (`create_market` never
//! writes `Protocol`), and the "two markets, same asset pair, different `config_id`" acceptance
//! criterion all live here.

use aegis::state::{Market, Position, Protocol};
use aegis_test_kit::{
    collateral_vault_pda, create_market, create_spl_mint, deploy, fetch_market, fetch_position,
    fetch_protocol, fetch_token_account_base, init_position, initialize_protocol, loan_vault_pda,
    market_pda, position_pda, protocol_pda, reference_market_args, spl_token_interface,
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

#[test]
fn protocol_initializes_with_expected_admin_and_layout() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let guardian = fixed_pubkey(2);
    let fee_recipient = fixed_pubkey(3);

    let result = initialize_protocol(&mut svm, &admin, guardian, fee_recipient);
    assert!(
        result.is_ok(),
        "initialize_protocol should succeed: {:?}",
        result.err()
    );

    let (protocol_pubkey, bump) = protocol_pda();
    let protocol = fetch_protocol(&svm, &protocol_pubkey);

    // I-DEPLOY-01: post-deploy admin assertion — the deployment checklist this test encodes.
    assert_eq!(protocol.admin, admin.pubkey());
    assert_eq!(protocol.guardian, guardian);
    assert_eq!(protocol.fee_recipient, fee_recipient);
    assert_eq!(protocol.pending_admin, Pubkey::default());
    assert_eq!(protocol.paused, 0);
    assert_eq!(protocol.bump, bump);

    // U-ACCT-01: _reserved is all-zero.
    assert_eq!(protocol._reserved, [0u8; 64]);

    // U-ACCT-02: the account is exactly Protocol::LEN — created at final size, no realloc.
    let account = svm
        .get_account(&protocol_pubkey)
        .expect("protocol account exists");
    assert_eq!(account.data.len(), Protocol::LEN);
    assert_eq!(account.owner, aegis::id());
}

// U-LIFE-02: every account type's seed prefix is distinct, so no two account types can ever
// collide at the same address (INV-LIFE-06).
#[test]
fn seed_prefixes_are_pairwise_distinct() {
    let seeds: &[&[u8]] = &[
        aegis::constants::PROTOCOL_SEED,
        aegis::constants::MARKET_SEED,
        aegis::constants::POSITION_SEED,
        aegis::constants::COLLATERAL_VAULT_SEED,
        aegis::constants::LOAN_VAULT_SEED,
    ];
    for (i, a) in seeds.iter().enumerate() {
        for (j, b) in seeds.iter().enumerate() {
            if i != j {
                assert_ne!(a, b, "seed prefixes at indices {i} and {j} collide");
            }
        }
    }
}

fn setup_protocol(svm: &mut litesvm::LiteSVM, admin: &Keypair) -> Pubkey {
    let guardian = fixed_pubkey(2);
    let fee_recipient = fixed_pubkey(3);
    initialize_protocol(svm, admin, guardian, fee_recipient).expect("initialize_protocol");
    fee_recipient
}

#[test]
fn create_market_spl_and_position_lifecycle() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fee_recipient = setup_protocol(&mut svm, &admin);

    let collateral_mint = create_spl_mint(&mut svm, &admin, 10, 9, admin.pubkey(), None);
    let loan_mint = create_spl_mint(&mut svm, &admin, 11, 6, admin.pubkey(), None);

    let args = reference_market_args(0, [1u8; 32], [2u8; 32], false);
    let (result, market_pubkey, collateral_vault, loan_vault, fee_position) = create_market(
        &mut svm,
        &admin,
        collateral_mint,
        loan_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        fee_recipient,
        args,
    );
    assert!(
        result.is_ok(),
        "create_market should succeed: {:?}",
        result.err()
    );

    // INV-ACCT-02: canonical PDA.
    let (expected_market, expected_bump) = market_pda(&collateral_mint, &loan_mint, 0);
    assert_eq!(market_pubkey, expected_market);

    let market = fetch_market(&svm, &market_pubkey);
    assert_eq!(market.bump, expected_bump);
    assert_eq!(market.collateral_mint, collateral_mint);
    assert_eq!(market.loan_mint, loan_mint);
    assert_eq!(market.collateral_token_program, spl_token_interface::ID);
    assert_eq!(market.loan_token_program, spl_token_interface::ID);
    assert_eq!(market.collateral_vault, collateral_vault);
    assert_eq!(market.loan_vault, loan_vault);
    assert_eq!(market.fee_recipient, fee_recipient);
    assert_eq!(market.config_id, 0);
    assert_eq!(market.collateral_decimals, 9);
    assert_eq!(market.loan_decimals, 6);

    // Accounting scalars start and stay at zero (Phase 2 non-scope: no share math yet).
    assert_eq!(market.total_supply_assets, 0);
    assert_eq!(market.total_supply_shares, 0);
    assert_eq!(market.total_borrow_assets, 0);
    assert_eq!(market.total_borrow_shares, 0);
    assert_eq!(market.collateral_fee_accrued, 0);
    assert_eq!(market.flags, 0); // no freeze authority, no transfer-fee collateral

    // U-ACCT-01 / U-ACCT-02.
    assert_eq!(market._reserved, [0u8; 64]);
    let market_account = svm
        .get_account(&market_pubkey)
        .expect("market account exists");
    assert_eq!(market_account.data.len(), Market::LEN);

    // INV-ACCT-04 / A-CUS-03 / INV-ACCT-05: vaults at canonical PDAs, authority is the Market PDA,
    // mint pinned, owned by the pinned token program.
    let (expected_cvault, _) = collateral_vault_pda(&market_pubkey);
    let (expected_lvault, _) = loan_vault_pda(&market_pubkey);
    assert_eq!(collateral_vault, expected_cvault);
    assert_eq!(loan_vault, expected_lvault);

    let cvault_account = svm
        .get_account(&collateral_vault)
        .expect("collateral vault exists");
    assert_eq!(cvault_account.owner, spl_token_interface::ID);
    assert_eq!(
        cvault_account.data.len(),
        165,
        "legacy SPL vault must be exactly 165 bytes"
    );
    let cvault_state = fetch_token_account_base(&svm, &collateral_vault);
    assert_eq!(cvault_state.mint, collateral_mint);
    assert_eq!(
        cvault_state.owner, market_pubkey,
        "vault authority must be the Market PDA"
    );

    let lvault_state = fetch_token_account_base(&svm, &loan_vault);
    assert_eq!(lvault_state.mint, loan_mint);
    assert_eq!(lvault_state.owner, market_pubkey);

    // The mandatory protocol fee Position (account-model.md §9).
    let fee_pos = fetch_position(&svm, &fee_position);
    assert_eq!(fee_pos.market, market_pubkey);
    assert_eq!(fee_pos.owner, fee_recipient);
    assert_eq!(fee_pos.supply_shares, 0);
    assert_eq!(fee_pos.borrow_shares, 0);
    assert_eq!(fee_pos.collateral_amount, 0);
    assert_eq!(fee_pos._reserved, [0u8; 32]);
    let fee_pos_account = svm.get_account(&fee_position).expect("fee position exists");
    assert_eq!(fee_pos_account.data.len(), Position::LEN);

    // init_position for an ordinary user.
    let user = fixed_pubkey(42);
    let (pos_result, position_pubkey) = init_position(&mut svm, &admin, market_pubkey, user);
    assert!(
        pos_result.is_ok(),
        "init_position should succeed: {:?}",
        pos_result.err()
    );

    let (expected_position, expected_pos_bump) = position_pda(&market_pubkey, &user);
    assert_eq!(position_pubkey, expected_position);

    let position = fetch_position(&svm, &position_pubkey);
    assert_eq!(position.market, market_pubkey);
    assert_eq!(position.owner, user);
    assert_eq!(position.bump, expected_pos_bump);
    assert_eq!(position.supply_shares, 0);
    assert_eq!(position.borrow_shares, 0);
    assert_eq!(position.collateral_amount, 0);
    assert_eq!(position._reserved, [0u8; 32]);
    let position_account = svm
        .get_account(&position_pubkey)
        .expect("position account exists");
    assert_eq!(position_account.data.len(), Position::LEN);
    assert_eq!(position_account.owner, aegis::id());
}

// INV-ACCT-07: no instruction other than an admin `set_*` writes Protocol. Phase 2 has no `set_*`
// instruction at all, so `create_market` (which only *reads* `protocol`) must leave it
// byte-for-byte unchanged, even though `protocol.fee_recipient` is used to derive the market's
// fee position.
#[test]
fn create_market_does_not_write_protocol() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fee_recipient = setup_protocol(&mut svm, &admin);
    let (protocol_pubkey, _) = protocol_pda();
    let before = svm
        .get_account(&protocol_pubkey)
        .expect("protocol account exists")
        .data;

    let collateral_mint = create_spl_mint(&mut svm, &admin, 12, 9, admin.pubkey(), None);
    let loan_mint = create_spl_mint(&mut svm, &admin, 13, 6, admin.pubkey(), None);
    let args = reference_market_args(0, [1u8; 32], [2u8; 32], false);
    let (result, ..) = create_market(
        &mut svm,
        &admin,
        collateral_mint,
        loan_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        fee_recipient,
        args,
    );
    result.expect("create_market must succeed");

    let after = svm
        .get_account(&protocol_pubkey)
        .expect("protocol account still exists")
        .data;
    assert_eq!(before, after, "create_market must not write Protocol");
}

// Acceptance criterion: two markets for the same asset pair coexist with different `config_id`,
// with distinct PDAs, distinct vaults, and no unintended shared writable state.
#[test]
fn two_markets_same_asset_pair_different_config_id_coexist() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fee_recipient = setup_protocol(&mut svm, &admin);

    let collateral_mint = create_spl_mint(&mut svm, &admin, 20, 9, admin.pubkey(), None);
    let loan_mint = create_spl_mint(&mut svm, &admin, 21, 6, admin.pubkey(), None);

    let args_a = reference_market_args(0, [1u8; 32], [2u8; 32], false);
    let (result_a, market_a, cvault_a, lvault_a, fee_pos_a) = create_market(
        &mut svm,
        &admin,
        collateral_mint,
        loan_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        fee_recipient,
        args_a,
    );
    assert!(
        result_a.is_ok(),
        "market A should succeed: {:?}",
        result_a.err()
    );

    let args_b = reference_market_args(1, [1u8; 32], [2u8; 32], false);
    let (result_b, market_b, cvault_b, lvault_b, fee_pos_b) = create_market(
        &mut svm,
        &admin,
        collateral_mint,
        loan_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        fee_recipient,
        args_b,
    );
    assert!(
        result_b.is_ok(),
        "market B should succeed: {:?}",
        result_b.err()
    );

    // Distinct PDAs and distinct vaults/state — no unintended shared writable market state.
    assert_ne!(market_a, market_b);
    assert_ne!(cvault_a, cvault_b);
    assert_ne!(lvault_a, lvault_b);
    assert_ne!(
        fee_pos_a, fee_pos_b,
        "distinct config_id markets get distinct fee positions too"
    );

    let market_a_state = fetch_market(&svm, &market_a);
    let market_b_state = fetch_market(&svm, &market_b);
    assert_eq!(market_a_state.config_id, 0);
    assert_eq!(market_b_state.config_id, 1);
    // Both still isolated: same identity mints, independent accounting totals.
    assert_eq!(
        market_a_state.collateral_mint,
        market_b_state.collateral_mint
    );
    assert_eq!(market_a_state.total_supply_assets, 0);
    assert_eq!(market_b_state.total_supply_assets, 0);
}
