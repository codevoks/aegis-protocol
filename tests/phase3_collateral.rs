//! Phase 3 — happy-path collateral flows: measured-delta deposit accounting on both token
//! programs (`U-TOK-01`, `U-TOK-02`), zero-debt withdrawal (`U-WDC-01`), exact-zero close
//! preconditions (`U-LIFE-01`), and the custody invariant across multiple positions (`I-CUS-02`).

#![allow(clippy::result_large_err)]

use aegis::state::{Market, Position};
use aegis_test_kit::{
    close_position, create_market, create_spl_mint, create_token_2022_mint, create_token_account,
    deploy, deposit_collateral, fetch_market, fetch_position, fetch_token_account_base,
    init_position, initialize_protocol, invariants, mint_to, reference_market_args,
    spl_token_2022_interface, spl_token_interface, withdraw_collateral, Token2022Extension,
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

fn setup_protocol(svm: &mut litesvm::LiteSVM, admin: &Keypair) -> Pubkey {
    let guardian = fixed_pubkey(2);
    let fee_recipient = fixed_pubkey(3);
    initialize_protocol(svm, admin, guardian, fee_recipient).expect("initialize_protocol");
    fee_recipient
}

// U-TOK-01 / I-CUS-02: a classic SPL Token deposit credits exactly the requested amount, and the
// custody invariant holds afterward.
#[test]
fn spl_deposit_credits_exact_amount() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fee_recipient = setup_protocol(&mut svm, &admin);

    let sol_mint = create_spl_mint(&mut svm, &admin, 10, 9, admin.pubkey(), None);
    let usdc_mint = create_spl_mint(&mut svm, &admin, 11, 6, admin.pubkey(), None);
    let args = reference_market_args(0, [1u8; 32], [2u8; 32], false);
    let (result, market, collateral_vault, ..) = create_market(
        &mut svm,
        &admin,
        sol_mint,
        usdc_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        fee_recipient,
        args,
    );
    result.expect("create_market must succeed");

    let owner = fixed_pubkey(100);
    let (r, position) = init_position(&mut svm, &admin, market, owner);
    r.expect("init_position must succeed");

    // The depositor need not be the position owner (owner here has no keypair at all).
    let depositor_ata = create_token_account(
        &mut svm,
        &admin,
        101,
        sol_mint,
        admin.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    mint_to(
        &mut svm,
        &admin,
        sol_mint,
        depositor_ata,
        &admin,
        5_000_000_000,
        spl_token_interface::ID,
    );

    let amount = 5_000_000_000u64;
    let result = deposit_collateral(
        &mut svm,
        &admin,
        market,
        position,
        collateral_vault,
        depositor_ata,
        sol_mint,
        spl_token_interface::ID,
        amount,
    );
    result.expect("deposit_collateral must succeed");

    let position_state = fetch_position(&svm, &position);
    assert_eq!(
        position_state.collateral_amount, amount,
        "U-TOK-01: credited must equal the requested amount on a fee-free mint"
    );
    let vault_state = fetch_token_account_base(&svm, &collateral_vault);
    assert_eq!(vault_state.amount, amount);

    invariants::assert_inv_cus_02(&svm, &market, &[position]);
}

// U-TOK-02: a Token-2022 transfer-fee collateral deposit credits strictly less than the requested
// amount — `credited == amount - fee` — and INV-CUS-02 holds exactly against the *credited*
// figure, not the requested one.
#[test]
fn token2022_transfer_fee_deposit_credits_net_of_fee() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fee_recipient = setup_protocol(&mut svm, &admin);

    let basis_points = 500u16; // 5%
    let maximum_fee = 10_000_000_000u64; // effectively uncapped for this test's amount
    let fee_mint = create_token_2022_mint(
        &mut svm,
        &admin,
        20,
        9,
        admin.pubkey(),
        None,
        &[Token2022Extension::TransferFeeConfig {
            basis_points,
            maximum_fee,
        }],
    );
    let usdc_mint = create_spl_mint(&mut svm, &admin, 21, 6, admin.pubkey(), None);
    let args = reference_market_args(0, [1u8; 32], [2u8; 32], false);
    let (result, market, collateral_vault, ..) = create_market(
        &mut svm,
        &admin,
        fee_mint,
        usdc_mint,
        spl_token_2022_interface::ID,
        spl_token_interface::ID,
        fee_recipient,
        args,
    );
    result.expect("create_market must succeed");

    let owner = fixed_pubkey(110);
    let (r, position) = init_position(&mut svm, &admin, market, owner);
    r.expect("init_position must succeed");

    let extensions = &[spl_token_2022_interface::extension::ExtensionType::TransferFeeConfig];
    let depositor_ata = create_token_account(
        &mut svm,
        &admin,
        111,
        fee_mint,
        admin.pubkey(),
        spl_token_2022_interface::ID,
        extensions,
    );
    let requested = 1_000_000_000u64; // 1.0 token @ 9dp
    mint_to(
        &mut svm,
        &admin,
        fee_mint,
        depositor_ata,
        &admin,
        requested,
        spl_token_2022_interface::ID,
    );

    let expected_fee = (requested as u128 * basis_points as u128 / 10_000) as u64;
    let expected_credited = requested - expected_fee;
    assert!(
        expected_fee > 0,
        "test fixture must actually exercise a nonzero fee"
    );

    let result = deposit_collateral(
        &mut svm,
        &admin,
        market,
        position,
        collateral_vault,
        depositor_ata,
        fee_mint,
        spl_token_2022_interface::ID,
        requested,
    );
    result.expect("deposit_collateral must succeed");

    let position_state = fetch_position(&svm, &position);
    assert_eq!(
        position_state.collateral_amount, expected_credited,
        "U-TOK-02: credited must equal amount - fee, never the requested amount"
    );
    assert_ne!(
        position_state.collateral_amount, requested,
        "requested and credited must differ on a transfer-fee mint"
    );

    let vault_state = fetch_token_account_base(&svm, &collateral_vault);
    assert_eq!(
        vault_state.amount, expected_credited,
        "the vault only ever received the net (post-fee) amount"
    );

    invariants::assert_inv_cus_02(&svm, &market, &[position]);
}

// U-WDC-01: a debt-free position can withdraw its full collateral balance.
#[test]
fn withdraw_all_with_zero_debt() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fee_recipient = setup_protocol(&mut svm, &admin);

    let sol_mint = create_spl_mint(&mut svm, &admin, 30, 9, admin.pubkey(), None);
    let usdc_mint = create_spl_mint(&mut svm, &admin, 31, 6, admin.pubkey(), None);
    let args = reference_market_args(0, [1u8; 32], [2u8; 32], false);
    let (result, market, collateral_vault, ..) = create_market(
        &mut svm,
        &admin,
        sol_mint,
        usdc_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        fee_recipient,
        args,
    );
    result.expect("create_market must succeed");

    let owner = Keypair::new_from_array([120u8; 32]);
    svm.airdrop(&owner.pubkey(), 10_000_000_000)
        .expect("airdrop to owner");
    let (r, position) = init_position(&mut svm, &admin, market, owner.pubkey());
    r.expect("init_position must succeed");

    let depositor_ata = create_token_account(
        &mut svm,
        &admin,
        121,
        sol_mint,
        admin.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    let amount = 7_000_000_000u64;
    mint_to(
        &mut svm,
        &admin,
        sol_mint,
        depositor_ata,
        &admin,
        amount,
        spl_token_interface::ID,
    );
    deposit_collateral(
        &mut svm,
        &admin,
        market,
        position,
        collateral_vault,
        depositor_ata,
        sol_mint,
        spl_token_interface::ID,
        amount,
    )
    .expect("deposit_collateral must succeed");
    invariants::assert_inv_cus_02(&svm, &market, &[position]);

    let owner_ata = create_token_account(
        &mut svm,
        &admin,
        122,
        sol_mint,
        owner.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    let result = withdraw_collateral(
        &mut svm,
        &owner,
        market,
        position,
        collateral_vault,
        owner_ata,
        sol_mint,
        spl_token_interface::ID,
        // Debt-free withdrawal reads no oracle at all (E-08) -- placeholder pubkeys are never
        // touched.
        Pubkey::default(),
        Pubkey::default(),
        amount,
    );
    result.expect("withdraw_collateral must succeed for a debt-free position");

    let position_state = fetch_position(&svm, &position);
    assert_eq!(position_state.collateral_amount, 0);
    let vault_state = fetch_token_account_base(&svm, &collateral_vault);
    assert_eq!(vault_state.amount, 0);
    let owner_ata_state = fetch_token_account_base(&svm, &owner_ata);
    assert_eq!(owner_ata_state.amount, amount);

    invariants::assert_inv_cus_02(&svm, &market, &[position]);
}

// U-LIFE-01: close_position requires supply_shares, borrow_shares and collateral_amount to be
// exactly zero — never a dust tolerance — and succeeds once they are.
#[test]
fn close_position_requires_exact_zero_balances() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fee_recipient = setup_protocol(&mut svm, &admin);

    let sol_mint = create_spl_mint(&mut svm, &admin, 40, 9, admin.pubkey(), None);
    let usdc_mint = create_spl_mint(&mut svm, &admin, 41, 6, admin.pubkey(), None);
    let args = reference_market_args(0, [1u8; 32], [2u8; 32], false);
    let (result, market, collateral_vault, ..) = create_market(
        &mut svm,
        &admin,
        sol_mint,
        usdc_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        fee_recipient,
        args,
    );
    result.expect("create_market must succeed");

    let owner = Keypair::new_from_array([130u8; 32]);
    svm.airdrop(&owner.pubkey(), 10_000_000_000)
        .expect("airdrop to owner");
    let (r, position) = init_position(&mut svm, &admin, market, owner.pubkey());
    r.expect("init_position must succeed");

    let depositor_ata = create_token_account(
        &mut svm,
        &admin,
        131,
        sol_mint,
        admin.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    let amount = 1_000_000_000u64;
    mint_to(
        &mut svm,
        &admin,
        sol_mint,
        depositor_ata,
        &admin,
        amount,
        spl_token_interface::ID,
    );
    deposit_collateral(
        &mut svm,
        &admin,
        market,
        position,
        collateral_vault,
        depositor_ata,
        sol_mint,
        spl_token_interface::ID,
        amount,
    )
    .expect("deposit_collateral must succeed");

    // Non-empty: close must be rejected.
    let premature = close_position(&mut svm, &owner, market, position);
    aegis_test_kit::assert_aegis_error(&premature, aegis::error::AegisError::PositionNotEmpty);

    // Withdraw everything, then close must succeed.
    let owner_ata = create_token_account(
        &mut svm,
        &admin,
        132,
        sol_mint,
        owner.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    withdraw_collateral(
        &mut svm,
        &owner,
        market,
        position,
        collateral_vault,
        owner_ata,
        sol_mint,
        spl_token_interface::ID,
        Pubkey::default(),
        Pubkey::default(),
        amount,
    )
    .expect("withdraw_collateral must succeed");

    let owner_lamports_before = svm.get_account(&owner.pubkey()).unwrap().lamports;
    let position_lamports = svm.get_account(&position).unwrap().lamports;

    // A fresh blockhash so this call isn't rejected as an `AlreadyProcessed` replay of the
    // (failed, but signature-consuming) premature close attempt above — same instruction, same
    // accounts, same signer.
    svm.expire_blockhash();
    let closed = close_position(&mut svm, &owner, market, position);
    closed.expect("close_position must succeed once every balance is exactly zero");

    // Rent reclaimed to owner (net of the small tx fee already deducted from owner's balance).
    let owner_lamports_after = svm.get_account(&owner.pubkey()).unwrap().lamports;
    assert!(
        owner_lamports_after + 10_000 >= owner_lamports_before + position_lamports,
        "position's rent lamports must be returned to owner"
    );
}

// I-CUS-02: the custody invariant is an exact sum across every position in the market, not a
// single-position coincidence — proven with two positions and a partial withdrawal in between.
#[test]
fn custody_invariant_holds_across_multiple_positions() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fee_recipient = setup_protocol(&mut svm, &admin);

    let sol_mint = create_spl_mint(&mut svm, &admin, 50, 9, admin.pubkey(), None);
    let usdc_mint = create_spl_mint(&mut svm, &admin, 51, 6, admin.pubkey(), None);
    let args = reference_market_args(0, [1u8; 32], [2u8; 32], false);
    let (result, market, collateral_vault, ..) = create_market(
        &mut svm,
        &admin,
        sol_mint,
        usdc_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        fee_recipient,
        args,
    );
    result.expect("create_market must succeed");

    let owner_a = Keypair::new_from_array([140u8; 32]);
    svm.airdrop(&owner_a.pubkey(), 10_000_000_000)
        .expect("airdrop");
    let owner_b = fixed_pubkey(141);

    let (ra, position_a) = init_position(&mut svm, &admin, market, owner_a.pubkey());
    ra.expect("init_position a");
    let (rb, position_b) = init_position(&mut svm, &admin, market, owner_b);
    rb.expect("init_position b");

    let depositor_ata = create_token_account(
        &mut svm,
        &admin,
        142,
        sol_mint,
        admin.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    mint_to(
        &mut svm,
        &admin,
        sol_mint,
        depositor_ata,
        &admin,
        10_000_000_000,
        spl_token_interface::ID,
    );

    deposit_collateral(
        &mut svm,
        &admin,
        market,
        position_a,
        collateral_vault,
        depositor_ata,
        sol_mint,
        spl_token_interface::ID,
        3_000_000_000,
    )
    .expect("deposit into position_a");
    invariants::assert_inv_cus_02(&svm, &market, &[position_a, position_b]);

    deposit_collateral(
        &mut svm,
        &admin,
        market,
        position_b,
        collateral_vault,
        depositor_ata,
        sol_mint,
        spl_token_interface::ID,
        2_000_000_000,
    )
    .expect("deposit into position_b");
    invariants::assert_inv_cus_02(&svm, &market, &[position_a, position_b]);

    let owner_a_ata = create_token_account(
        &mut svm,
        &admin,
        143,
        sol_mint,
        owner_a.pubkey(),
        spl_token_interface::ID,
        &[],
    );
    withdraw_collateral(
        &mut svm,
        &owner_a,
        market,
        position_a,
        collateral_vault,
        owner_a_ata,
        sol_mint,
        spl_token_interface::ID,
        Pubkey::default(),
        Pubkey::default(),
        1_000_000_000,
    )
    .expect("partial withdrawal from position_a");
    invariants::assert_inv_cus_02(&svm, &market, &[position_a, position_b]);

    let market_state: Market = fetch_market(&svm, &market);
    let position_a_state: Position = fetch_position(&svm, &position_a);
    let position_b_state: Position = fetch_position(&svm, &position_b);
    assert_eq!(position_a_state.collateral_amount, 2_000_000_000);
    assert_eq!(position_b_state.collateral_amount, 2_000_000_000);
    let vault_state = fetch_token_account_base(&svm, &collateral_vault);
    assert_eq!(
        vault_state.amount,
        position_a_state.collateral_amount
            + position_b_state.collateral_amount
            + market_state.collateral_fee_accrued
    );
}
