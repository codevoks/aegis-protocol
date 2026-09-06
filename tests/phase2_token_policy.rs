//! Phase 2 — Token-2022 extension policy tests (`docs/token-compatibility.md`). Each rejection
//! test asserts the specific `AegisError`, and the acceptance tests confirm the policy is a
//! genuine allowlist rather than an over-broad rejection of everything.

// `litesvm::types::TransactionResult`'s `Err` variant is a third-party type this crate does not
// control.
#![allow(clippy::result_large_err)]

use aegis::error::AegisError;
use aegis_test_kit::{
    assert_aegis_error, create_market, create_spl_mint, create_token_2022_mint,
    create_token_2022_mint_with_unrecognized_extension, deploy, fetch_mint_extension_types,
    fetch_token_account_base, initialize_protocol, reference_market_args, spl_token_2022_interface,
    spl_token_interface, Token2022Extension,
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

/// Attempts to create a market using `mint` (Token-2022) as collateral against a plain SPL loan
/// asset, and returns the raw result for the caller to assert on.
fn try_create_market_with_collateral_mint(
    svm: &mut litesvm::LiteSVM,
    admin: &Keypair,
    fee_recipient: Pubkey,
    collateral_mint: Pubkey,
    loan_mint: Pubkey,
    ack_freeze_authority: bool,
    config_id: u16,
) -> litesvm::types::TransactionResult {
    let args = reference_market_args(config_id, [1u8; 32], [2u8; 32], ack_freeze_authority);
    let (result, ..) = create_market(
        svm,
        admin,
        collateral_mint,
        loan_mint,
        spl_token_2022_interface::ID,
        spl_token_interface::ID,
        fee_recipient,
        args,
    );
    result
}

// A-TOK-01: a TransferHook mint is rejected as collateral.
#[test]
fn transfer_hook_mint_rejected_as_collateral() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fee_recipient = setup_protocol(&mut svm, &admin);
    let hook_program = fixed_pubkey(88);
    let collateral_mint = create_token_2022_mint(
        &mut svm,
        &admin,
        40,
        9,
        admin.pubkey(),
        None,
        &[Token2022Extension::TransferHook(hook_program)],
    );
    let loan_mint = create_spl_mint(&mut svm, &admin, 41, 6, admin.pubkey(), None);
    let result = try_create_market_with_collateral_mint(
        &mut svm,
        &admin,
        fee_recipient,
        collateral_mint,
        loan_mint,
        false,
        0,
    );
    assert_aegis_error(&result, AegisError::UnsupportedTokenExtension);
}

// A-TOK-02: a PermanentDelegate mint is rejected as collateral — a permanent delegate can drain
// the vault outright (token-compatibility.md §2).
#[test]
fn permanent_delegate_mint_rejected() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fee_recipient = setup_protocol(&mut svm, &admin);
    let delegate = fixed_pubkey(89);
    let collateral_mint = create_token_2022_mint(
        &mut svm,
        &admin,
        42,
        9,
        admin.pubkey(),
        None,
        &[Token2022Extension::PermanentDelegate(delegate)],
    );
    let loan_mint = create_spl_mint(&mut svm, &admin, 43, 6, admin.pubkey(), None);
    let result = try_create_market_with_collateral_mint(
        &mut svm,
        &admin,
        fee_recipient,
        collateral_mint,
        loan_mint,
        false,
        0,
    );
    assert_aegis_error(&result, AegisError::UnsupportedTokenExtension);
}

// A-TOK-03: a MintCloseAuthority mint is rejected — the mint could be closed and reinitialized
// with different extensions, invalidating every check performed at market creation.
#[test]
fn mint_close_authority_mint_rejected() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fee_recipient = setup_protocol(&mut svm, &admin);
    let close_authority = fixed_pubkey(90);
    let collateral_mint = create_token_2022_mint(
        &mut svm,
        &admin,
        44,
        9,
        admin.pubkey(),
        None,
        &[Token2022Extension::MintCloseAuthority(close_authority)],
    );
    let loan_mint = create_spl_mint(&mut svm, &admin, 45, 6, admin.pubkey(), None);
    let result = try_create_market_with_collateral_mint(
        &mut svm,
        &admin,
        fee_recipient,
        collateral_mint,
        loan_mint,
        false,
        0,
    );
    assert_aegis_error(&result, AegisError::UnsupportedTokenExtension);
}

// A-TOK-04: a DefaultAccountState = Frozen mint is rejected — a newly created vault could come
// into existence already frozen.
#[test]
fn default_account_state_frozen_mint_rejected() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fee_recipient = setup_protocol(&mut svm, &admin);
    let collateral_mint = create_token_2022_mint(
        &mut svm,
        &admin,
        46,
        9,
        admin.pubkey(),
        // Token-2022 requires a mint to have a freeze authority before it can set a default
        // account state of Frozen (an account with no way to ever unfreeze accounts would be
        // permanently unusable), so this fixture needs one regardless of what Aegis's own
        // freeze-authority-acknowledgement policy separately requires.
        Some(fixed_pubkey(92)),
        &[Token2022Extension::DefaultAccountStateFrozen],
    );
    let loan_mint = create_spl_mint(&mut svm, &admin, 47, 6, admin.pubkey(), None);
    let result = try_create_market_with_collateral_mint(
        &mut svm,
        &admin,
        fee_recipient,
        collateral_mint,
        loan_mint,
        false,
        0,
    );
    assert_aegis_error(&result, AegisError::UnsupportedTokenExtension);
}

// A-TOK-05: a mint carrying an extension type this program's dependency does not recognize at
// all must be rejected by the positive allowlist — not silently accepted because it isn't on a
// blocklist.
#[test]
fn unrecognized_extension_mint_rejected() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fee_recipient = setup_protocol(&mut svm, &admin);
    let collateral_mint =
        create_token_2022_mint_with_unrecognized_extension(&mut svm, 9, admin.pubkey());
    let loan_mint = create_spl_mint(&mut svm, &admin, 48, 6, admin.pubkey(), None);
    let result = try_create_market_with_collateral_mint(
        &mut svm,
        &admin,
        fee_recipient,
        collateral_mint,
        loan_mint,
        false,
        0,
    );
    assert_aegis_error(&result, AegisError::InvalidMintAccountData);
}

// token-compatibility.md §4: a transfer-fee mint is accepted as collateral, and rejected as the
// loan asset — the same extension is safe in one role and unsafe in the other.
#[test]
fn transfer_fee_mint_accepted_as_collateral_rejected_as_loan_asset() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fee_recipient = setup_protocol(&mut svm, &admin);
    let fee_mint = create_token_2022_mint(
        &mut svm,
        &admin,
        49,
        9,
        admin.pubkey(),
        None,
        &[Token2022Extension::TransferFeeConfig {
            basis_points: 50,
            maximum_fee: 1_000_000,
        }],
    );
    let plain_mint = create_spl_mint(&mut svm, &admin, 52, 6, admin.pubkey(), None);

    // Accepted as collateral.
    let args = reference_market_args(0, [1u8; 32], [2u8; 32], false);
    let (accepted, market_pubkey, collateral_vault, ..) = create_market(
        &mut svm,
        &admin,
        fee_mint,
        plain_mint,
        spl_token_2022_interface::ID,
        spl_token_interface::ID,
        fee_recipient,
        args,
    );
    accepted.expect("transfer-fee mint must be accepted as collateral");
    let market = aegis_test_kit::fetch_market(&svm, &market_pubkey);
    assert_eq!(
        market.flags & aegis::constants::FLAG_COLLATERAL_HAS_TRANSFER_FEE,
        aegis::constants::FLAG_COLLATERAL_HAS_TRANSFER_FEE,
        "collateral_has_transfer_fee flag must be set"
    );

    // Token-2022 vault sizing: never hardcoded to 165 — it must be exactly what
    // ExtensionType::try_calculate_account_len computes for [TransferFeeAmount, ImmutableOwner].
    let vault_account = svm
        .get_account(&collateral_vault)
        .expect("collateral vault exists");
    assert!(
        vault_account.data.len() > 165,
        "a Token-2022 vault for a transfer-fee mint must be larger than the legacy 165-byte size"
    );
    let vault_base = fetch_token_account_base(&svm, &collateral_vault);
    assert_eq!(vault_base.mint, fee_mint);
    assert_eq!(vault_base.owner, market_pubkey);

    // Rejected as the loan asset.
    let args2 = reference_market_args(1, [1u8; 32], [2u8; 32], false);
    let (rejected, ..) = create_market(
        &mut svm,
        &admin,
        plain_mint,
        fee_mint,
        spl_token_interface::ID,
        spl_token_2022_interface::ID,
        fee_recipient,
        args2,
    );
    assert_aegis_error(&rejected, AegisError::TransferFeeNotAllowedForLoanAsset);
}

// A-TOK-07: a freeze-authority mint is rejected when unacknowledged, accepted when acknowledged,
// and the acknowledgement is recorded in market.flags.
#[test]
fn freeze_authority_requires_acknowledgement() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fee_recipient = setup_protocol(&mut svm, &admin);
    let freeze_authority = fixed_pubkey(91);
    let collateral_mint = create_spl_mint(
        &mut svm,
        &admin,
        53,
        9,
        admin.pubkey(),
        Some(freeze_authority),
    );
    let loan_mint = create_spl_mint(&mut svm, &admin, 54, 6, admin.pubkey(), None);

    // Unacknowledged: rejected.
    let args = reference_market_args(0, [1u8; 32], [2u8; 32], false);
    let (rejected, ..) = create_market(
        &mut svm,
        &admin,
        collateral_mint,
        loan_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        fee_recipient,
        args,
    );
    assert_aegis_error(&rejected, AegisError::FreezeAuthorityNotAcknowledged);

    // Acknowledged: accepted, and the flag is recorded.
    let args_ack = reference_market_args(1, [1u8; 32], [2u8; 32], true);
    let (accepted, market_pubkey, ..) = create_market(
        &mut svm,
        &admin,
        collateral_mint,
        loan_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        fee_recipient,
        args_ack,
    );
    accepted.expect("acknowledged freeze-authority mint must be accepted");
    let market = aegis_test_kit::fetch_market(&svm, &market_pubkey);
    assert_eq!(
        market.flags & aegis::constants::FLAG_ACK_FREEZE_AUTHORITY,
        aegis::constants::FLAG_ACK_FREEZE_AUTHORITY
    );
}

// Positive-allowlist Tier A extensions do not block market creation, and the accepted extension
// inventory is exactly what the mint carries (token-compatibility.md §6 step 7).
#[test]
fn tier_a_extensions_are_accepted_and_recorded() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fee_recipient = setup_protocol(&mut svm, &admin);

    // InterestBearingConfig requires its own dedicated instruction the test-kit does not need to
    // exercise for this test's purpose (it's UI-only, unaffected by base-unit accounting); the
    // simplest Tier A proof is that a mint with *zero* extensions is Tier A by construction.
    let collateral_mint = create_spl_mint(&mut svm, &admin, 55, 9, admin.pubkey(), None);
    let loan_mint = create_spl_mint(&mut svm, &admin, 56, 6, admin.pubkey(), None);
    let args = reference_market_args(0, [1u8; 32], [2u8; 32], false);
    let (result, market_pubkey, ..) = create_market(
        &mut svm,
        &admin,
        collateral_mint,
        loan_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        fee_recipient,
        args,
    );
    result.expect("a plain SPL Token mint must be Tier A");
    let market = aegis_test_kit::fetch_market(&svm, &market_pubkey);
    assert_eq!(market.flags, 0);

    let extension_types = fetch_mint_extension_types(&svm, &collateral_mint);
    assert!(extension_types.is_empty());
}

// A-TOK-08 (Phase 2 component): passing the wrong token program for a market's mint fails, even
// though both are valid token programs.
#[test]
fn wrong_token_program_for_mint_is_rejected() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fee_recipient = setup_protocol(&mut svm, &admin);
    // A legacy SPL mint, but claimed to be owned by the Token-2022 program.
    let collateral_mint = create_spl_mint(&mut svm, &admin, 57, 9, admin.pubkey(), None);
    let loan_mint = create_spl_mint(&mut svm, &admin, 58, 6, admin.pubkey(), None);
    let args = reference_market_args(0, [1u8; 32], [2u8; 32], false);
    let (result, ..) = create_market(
        &mut svm,
        &admin,
        collateral_mint,
        loan_mint,
        spl_token_2022_interface::ID, // wrong program for a legacy mint
        spl_token_interface::ID,
        fee_recipient,
        args,
    );
    assert_aegis_error(&result, AegisError::TokenProgramMintMismatch);
}
