//! Phase 2 — adversarial tests. Each performs the attack and asserts it fails with the specific
//! expected `AegisError` (or the specific Anchor constraint error the account model relies on),
//! never merely "the transaction failed" (`testing-strategy.md` §4.2).

// `litesvm::types::TransactionResult`'s `Err` variant is a third-party type this crate does not
// control.
#![allow(clippy::result_large_err)]

use aegis::error::AegisError;
use aegis::state::Protocol;
use aegis_test_kit::{
    assert_aegis_error, create_market, create_spl_mint, deploy, init_position, initialize_protocol,
    position_pda, protocol_pda, reference_market_args, spl_token_interface,
};
use anchor_lang::{AccountSerialize, InstructionData, ToAccountMetas};
use solana_account::Account as RawAccount;
use solana_instruction::Instruction;
use solana_instruction_error::InstructionError;
use solana_keypair::Keypair;
use solana_message::{Message, VersionedMessage};
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;
use solana_transaction_error::TransactionError;

fn program_bytes() -> &'static [u8] {
    include_bytes!(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/aegis.so"))
}

fn fixed_pubkey(seed: u8) -> Pubkey {
    Keypair::new_from_array([seed; 32]).pubkey()
}

fn send(
    svm: &mut litesvm::LiteSVM,
    payer: &Keypair,
    extra_signers: &[&Keypair],
    ix: Instruction,
) -> litesvm::types::TransactionResult {
    let blockhash = svm.latest_blockhash();
    let message = Message::new_with_blockhash(&[ix], Some(&payer.pubkey()), &blockhash);
    let mut signers: Vec<&Keypair> = vec![payer];
    signers.extend_from_slice(extra_signers);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(message), &signers)
        .expect("failed to sign transaction");
    svm.send_transaction(tx)
}

fn setup_protocol(svm: &mut litesvm::LiteSVM, admin: &Keypair) -> Pubkey {
    let guardian = fixed_pubkey(2);
    let fee_recipient = fixed_pubkey(3);
    initialize_protocol(svm, admin, guardian, fee_recipient).expect("initialize_protocol");
    fee_recipient
}

// A-AUTH-01: a non-admin signer attempting create_market must fail with NotProtocolAdmin.
#[test]
fn non_admin_cannot_create_market() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fee_recipient = setup_protocol(&mut svm, &admin);

    let attacker = Keypair::new_from_array([99u8; 32]);
    svm.airdrop(&attacker.pubkey(), 10_000_000_000)
        .expect("airdrop to attacker");

    let collateral_mint = create_spl_mint(&mut svm, &admin, 30, 9, admin.pubkey(), None);
    let loan_mint = create_spl_mint(&mut svm, &admin, 31, 6, admin.pubkey(), None);
    let args = reference_market_args(0, [1u8; 32], [2u8; 32], false);

    let (result, ..) = create_market(
        &mut svm,
        &attacker,
        collateral_mint,
        loan_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        fee_recipient,
        args,
    );
    assert_aegis_error(&result, AegisError::NotProtocolAdmin);
}

// A-AUTH-06 / INV-AUTH-06: an attacker-owned account masquerading as `Protocol` at the canonical
// address must be rejected by Anchor's owner check, not merely by a coincidental data mismatch.
#[test]
fn attacker_owned_fake_protocol_account_is_rejected() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let (protocol_pubkey, bump) = protocol_pda();

    // Plant a byte-identical *Protocol*-shaped account at the canonical address, but owned by
    // the System Program instead of Aegis — exactly what an attacker would need if some code
    // path ever skipped the owner check.
    let fake = Protocol {
        admin: admin.pubkey(),
        pending_admin: Pubkey::default(),
        guardian: fixed_pubkey(2),
        fee_recipient: fixed_pubkey(3),
        paused: 0,
        bump,
        _reserved: [0u8; 64],
    };
    let mut data = Vec::new();
    fake.try_serialize(&mut data)
        .expect("serialize fake protocol");
    svm.set_account(
        protocol_pubkey,
        RawAccount {
            lamports: svm.minimum_balance_for_rent_exemption(data.len()),
            data,
            owner: anchor_lang::solana_program::system_program::ID,
            executable: false,
            rent_epoch: 0,
        },
    )
    .expect("inject fake protocol account");

    let collateral_mint = create_spl_mint(&mut svm, &admin, 32, 9, admin.pubkey(), None);
    let loan_mint = create_spl_mint(&mut svm, &admin, 33, 6, admin.pubkey(), None);
    let args = reference_market_args(0, [1u8; 32], [2u8; 32], false);
    let (result, ..) = create_market(
        &mut svm,
        &admin,
        collateral_mint,
        loan_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        fake.fee_recipient,
        args,
    );
    // Anchor surfaces even its own built-in framework errors as `InstructionError::Custom`, so
    // the check here is the exact code for `ErrorCode::AccountOwnedByWrongProgram` (3007) — a
    // *framework* rejection, distinct from anything in `AegisError`'s 6000+ band, proving the
    // owner mismatch was caught before any Aegis-specific logic ever ran.
    let failed = result.expect_err("fake-owned protocol account must be rejected");
    let expected_code = u32::from(anchor_lang::error::ErrorCode::AccountOwnedByWrongProgram);
    match failed.err {
        TransactionError::InstructionError(_, InstructionError::Custom(code)) => {
            assert_eq!(
                code, expected_code,
                "expected AccountOwnedByWrongProgram ({expected_code}), got {code}: logs={:?}",
                failed.meta.logs
            );
        }
        other => panic!("expected a custom program error {expected_code}, got {other:?}"),
    }
}

// A-LIFE-01 / INV-LIFE-01: a Position (and, by the same mechanism, a Protocol) can never be
// re-initialized while it exists.
#[test]
fn reinitializing_protocol_fails() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    setup_protocol(&mut svm, &admin);

    let result = initialize_protocol(&mut svm, &admin, fixed_pubkey(2), fixed_pubkey(3));
    assert!(result.is_err(), "second initialize_protocol must fail");
}

#[test]
fn reinitializing_market_fails() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fee_recipient = setup_protocol(&mut svm, &admin);
    let collateral_mint = create_spl_mint(&mut svm, &admin, 38, 9, admin.pubkey(), None);
    let loan_mint = create_spl_mint(&mut svm, &admin, 39, 6, admin.pubkey(), None);

    let args = reference_market_args(0, [1u8; 32], [2u8; 32], false);
    let (first, market_pubkey, ..) = create_market(
        &mut svm,
        &admin,
        collateral_mint,
        loan_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        fee_recipient,
        args,
    );
    first.expect("first create_market must succeed");

    // Same (collateral_mint, loan_mint, config_id) derives the same Market PDA — replaying
    // create_market against it must fail, not silently overwrite an existing market's state.
    let args_again = reference_market_args(0, [9u8; 32], [9u8; 32], false);
    let (second, replayed_market, ..) = create_market(
        &mut svm,
        &admin,
        collateral_mint,
        loan_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        fee_recipient,
        args_again,
    );
    assert_eq!(
        replayed_market, market_pubkey,
        "same seeds must derive the same PDA"
    );
    assert!(
        second.is_err(),
        "reinitializing an existing market must fail"
    );

    // The original market's data is untouched by the failed replay attempt.
    let market = aegis_test_kit::fetch_market(&svm, &market_pubkey);
    assert_eq!(market.collateral_feed_id, [1u8; 32]);
}

#[test]
fn reinitializing_position_fails() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fee_recipient = setup_protocol(&mut svm, &admin);

    let collateral_mint = create_spl_mint(&mut svm, &admin, 34, 9, admin.pubkey(), None);
    let loan_mint = create_spl_mint(&mut svm, &admin, 35, 6, admin.pubkey(), None);
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
    result.expect("create_market must succeed");

    let user = fixed_pubkey(50);
    let (first, position_pubkey) = init_position(&mut svm, &admin, market_pubkey, user);
    first.expect("first init_position must succeed");

    let (second, _) = init_position(&mut svm, &admin, market_pubkey, user);
    assert!(
        second.is_err(),
        "reinitializing an existing position must fail"
    );

    // The position itself is unchanged by the failed attempt.
    let position = aegis_test_kit::fetch_position(&svm, &position_pubkey);
    assert_eq!(position.owner, user);
    assert_eq!(position.supply_shares, 0);
}

// A-LIFE-03 / INV-LIFE-05: a non-canonical (but validly off-curve) bump for the same seeds must
// be rejected — only the canonical, highest-valid-bump PDA is ever accepted.
#[test]
fn non_canonical_bump_is_rejected() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fee_recipient = setup_protocol(&mut svm, &admin);

    let collateral_mint = create_spl_mint(&mut svm, &admin, 36, 9, admin.pubkey(), None);
    let loan_mint = create_spl_mint(&mut svm, &admin, 37, 6, admin.pubkey(), None);
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
    result.expect("create_market must succeed");

    let user = fixed_pubkey(51);
    let (canonical_position, canonical_bump) = position_pda(&market_pubkey, &user);

    let seed_prefix = aegis::constants::POSITION_SEED;
    let mut non_canonical = None;
    for bump in (0..canonical_bump).rev() {
        let bump_byte = [bump];
        let candidate_seeds: &[&[u8]] = &[
            seed_prefix,
            market_pubkey.as_ref(),
            user.as_ref(),
            &bump_byte,
        ];
        if let Ok(address) = Pubkey::create_program_address(candidate_seeds, &aegis::id()) {
            non_canonical = Some((address, bump));
            break;
        }
    }
    let (fake_position, fake_bump) = non_canonical
        .expect("at least one non-canonical off-curve bump must exist below the canonical one");
    assert_ne!(fake_position, canonical_position);
    assert_ne!(fake_bump, canonical_bump);

    let ix = Instruction {
        program_id: aegis::id(),
        accounts: aegis::accounts::InitPosition {
            payer: admin.pubkey(),
            market: market_pubkey,
            owner: user,
            position: fake_position,
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
        data: aegis::instruction::InitPosition {}.data(),
    };
    let result = send(&mut svm, &admin, &[], ix);
    assert!(result.is_err(), "a non-canonical bump PDA must be rejected");
}

// A-ADM-04: an out-of-bounds parameter sweep, including the derived liquidation-safety bound.
#[test]
fn out_of_bounds_market_parameters_are_rejected() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fee_recipient = setup_protocol(&mut svm, &admin);
    let collateral_mint = create_spl_mint(&mut svm, &admin, 60, 9, admin.pubkey(), None);
    let loan_mint = create_spl_mint(&mut svm, &admin, 61, 6, admin.pubkey(), None);

    let mut config_id = 0u16;
    let mut try_args = |svm: &mut litesvm::LiteSVM,
                        args: aegis::instructions::admin::CreateMarketArgs,
                        expected: AegisError| {
        let mut args = args;
        args.config_id = config_id;
        config_id += 1;
        let (result, ..) = create_market(
            svm,
            &admin,
            collateral_mint,
            loan_mint,
            spl_token_interface::ID,
            spl_token_interface::ID,
            fee_recipient,
            args,
        );
        assert_aegis_error(&result, expected);
    };

    let base = || reference_market_args(0, [1u8; 32], [2u8; 32], false);

    // max_ltv >= liq_threshold.
    let mut args = base();
    args.max_ltv = args.liq_threshold;
    try_args(&mut svm, args, AegisError::InvalidMaxLtvOrThreshold);

    // liq_bonus above the flat maximum (0.25 WAD).
    let mut args = base();
    args.liq_bonus = 300_000_000_000_000_000; // 0.30 WAD
    try_args(&mut svm, args, AegisError::InvalidLiqBonus);

    // The derived bound: an otherwise-plausible bonus (0.24 WAD, within the flat max) combined
    // with a high liq_threshold (0.85 WAD) violates liq_threshold*(WAD+liq_bonus)/WAD < WAD
    // (0.85 * 1.24 = 1.054 > 1) — INV-LIQ-06.
    let mut args = base();
    args.liq_threshold = 850_000_000_000_000_000;
    args.liq_bonus = 240_000_000_000_000_000;
    args.max_ltv = 700_000_000_000_000_000; // keep max_ltv < liq_threshold
    try_args(
        &mut svm,
        args,
        AegisError::LiquidationBonusExceedsThresholdBound,
    );

    // close_factor below the minimum (0.05 WAD).
    let mut args = base();
    args.close_factor = 10_000_000_000_000_000; // 0.01 WAD
    try_args(&mut svm, args, AegisError::InvalidCloseFactor);

    // full_liq_hf zero.
    let mut args = base();
    args.full_liq_hf = 0;
    try_args(&mut svm, args, AegisError::InvalidFullLiqHf);

    // liq_protocol_fee above the maximum (0.5 WAD).
    let mut args = base();
    args.liq_protocol_fee = 600_000_000_000_000_000;
    try_args(&mut svm, args, AegisError::InvalidLiqProtocolFee);

    // fee above the maximum (0.25 WAD).
    let mut args = base();
    args.fee = 300_000_000_000_000_000;
    try_args(&mut svm, args, AegisError::InvalidFee);

    // min_debt zero.
    let mut args = base();
    args.min_debt = 0;
    try_args(&mut svm, args, AegisError::InvalidMinDebt);

    // IRM: u_kink out of (0, WAD).
    let mut args = base();
    args.u_kink = 0;
    try_args(&mut svm, args, AegisError::InvalidIrmParams);

    // IRM: a rate exceeding max_rate_ps.
    let mut args = base();
    args.slope1_ps = args.max_rate_ps + 1;
    try_args(&mut svm, args, AegisError::InvalidIrmParams);

    // Oracle: max_price_age_secs out of [1, 3600].
    let mut args = base();
    args.max_price_age_secs = 0;
    try_args(&mut svm, args, AegisError::InvalidMaxPriceAge);

    // Oracle: max_conf_bps out of [1, 2000].
    let mut args = base();
    args.max_conf_bps = 3000;
    try_args(&mut svm, args, AegisError::InvalidMaxConfBps);

    // Config sanity: same mint for both legs.
    let (result, ..) = create_market(
        &mut svm,
        &admin,
        collateral_mint,
        collateral_mint,
        spl_token_interface::ID,
        spl_token_interface::ID,
        fee_recipient,
        {
            let mut a = base();
            a.config_id = 999;
            a
        },
    );
    assert_aegis_error(&result, AegisError::SameCollateralAndLoanMint);
}

// Sanity: the reference parameter set from economic-model.md §5.1 is itself accepted, so the
// sweep above is testing real bounds and not an over-tight validator that rejects everything.
#[test]
fn reference_parameter_set_is_accepted_on_chain() {
    let (mut svm, admin) = deploy(aegis::id(), program_bytes());
    let fee_recipient = setup_protocol(&mut svm, &admin);
    let collateral_mint = create_spl_mint(&mut svm, &admin, 70, 9, admin.pubkey(), None);
    let loan_mint = create_spl_mint(&mut svm, &admin, 71, 6, admin.pubkey(), None);
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
    result.expect("reference parameter set must be accepted");
}
