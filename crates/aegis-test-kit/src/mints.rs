//! SPL Token and Token-2022 mint fixtures (`docs/zero-cost-demo.md` §6).
//!
//! Every mint is created via a real transaction against the actual embedded SPL Token /
//! Token-2022 program bytecode LiteSVM ships (`LiteSVM::new()` calls `with_default_programs()`
//! internally, so `svm::deploy` already has both loaded) — never by hand-crafting account bytes —
//! with one deliberate exception: [`create_token_2022_mint_with_unrecognized_extension`], which
//! exists specifically to simulate a Token-2022 extension type this repository's dependency does
//! not know about, and which the real program therefore has no instruction to create.

use litesvm::LiteSVM;
use solana_account::Account;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_message::{Message, VersionedMessage};
use solana_program_pack::Pack;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;
use spl_token_2022_interface::extension::default_account_state::instruction::initialize_default_account_state;
use spl_token_2022_interface::extension::transfer_fee::instruction::initialize_transfer_fee_config;
use spl_token_2022_interface::extension::transfer_hook::instruction::initialize as initialize_transfer_hook;
use spl_token_2022_interface::extension::{
    BaseStateWithExtensions, BaseStateWithExtensionsMut, ExtensionType, StateWithExtensions,
    StateWithExtensionsMut,
};
use spl_token_2022_interface::instruction::{
    initialize_mint2 as initialize_mint2_2022, initialize_mint_close_authority,
    initialize_permanent_delegate,
};
use spl_token_2022_interface::state::{AccountState, Mint as SplMint};
use spl_token_interface::instruction::initialize_mint2 as initialize_mint2_legacy;
use spl_token_interface::state::Mint as SplLegacyMint;

/// One Token-2022 mint-level extension to initialize before `InitializeMint2`, as Token-2022
/// requires (`token-compatibility.md` §5.4, §9 of the phase spec). Each variant here corresponds
/// to a Tier B/C row in `token-compatibility.md` §2 that a required adversarial test exercises.
pub enum Token2022Extension {
    /// Tier B — accepted as collateral, rejected as the loan asset (`A-TOK-06` in a later phase;
    /// Phase 2 exercises collateral-acceptance and loan-rejection directly).
    TransferFeeConfig { basis_points: u16, maximum_fee: u64 },
    /// Tier C — `A-TOK-02`.
    PermanentDelegate(Pubkey),
    /// Tier C — `A-TOK-03`.
    MintCloseAuthority(Pubkey),
    /// Tier C — `A-TOK-04`.
    DefaultAccountStateFrozen,
    /// Tier C — `A-TOK-01`.
    TransferHook(Pubkey),
}

fn extension_type_of(extension: &Token2022Extension) -> ExtensionType {
    match extension {
        Token2022Extension::TransferFeeConfig { .. } => ExtensionType::TransferFeeConfig,
        Token2022Extension::PermanentDelegate(_) => ExtensionType::PermanentDelegate,
        Token2022Extension::MintCloseAuthority(_) => ExtensionType::MintCloseAuthority,
        Token2022Extension::DefaultAccountStateFrozen => ExtensionType::DefaultAccountState,
        Token2022Extension::TransferHook(_) => ExtensionType::TransferHook,
    }
}

fn init_instruction(
    extension: &Token2022Extension,
    mint: &Pubkey,
    mint_authority: &Pubkey,
) -> Instruction {
    let program_id = spl_token_2022_interface::ID;
    match extension {
        Token2022Extension::TransferFeeConfig {
            basis_points,
            maximum_fee,
        } => initialize_transfer_fee_config(
            &program_id,
            mint,
            Some(mint_authority),
            Some(mint_authority),
            *basis_points,
            *maximum_fee,
        ),
        Token2022Extension::PermanentDelegate(delegate) => {
            initialize_permanent_delegate(&program_id, mint, delegate)
        }
        Token2022Extension::MintCloseAuthority(authority) => {
            initialize_mint_close_authority(&program_id, mint, Some(authority))
        }
        Token2022Extension::DefaultAccountStateFrozen => {
            initialize_default_account_state(&program_id, mint, &AccountState::Frozen)
        }
        Token2022Extension::TransferHook(hook_program) => initialize_transfer_hook(
            &program_id,
            mint,
            Some(*mint_authority),
            Some(*hook_program),
        ),
    }
    .expect("valid extension-init instruction")
}

fn send(
    svm: &mut LiteSVM,
    payer: &Keypair,
    extra_signers: &[&Keypair],
    instructions: Vec<Instruction>,
) {
    let blockhash = svm.latest_blockhash();
    let message = Message::new_with_blockhash(&instructions, Some(&payer.pubkey()), &blockhash);
    let mut signers: Vec<&Keypair> = vec![payer];
    signers.extend_from_slice(extra_signers);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(message), &signers)
        .expect("failed to sign fixture transaction");
    svm.send_transaction(tx)
        .expect("fixture transaction should succeed");
}

/// Creates a classic SPL Token mint with a fixed-seed keypair — no `Keypair::new()` in fixtures,
/// so a failing test is reproducible from its seed alone (`docs/zero-cost-demo.md` §6).
pub fn create_spl_mint(
    svm: &mut LiteSVM,
    payer: &Keypair,
    seed: u8,
    decimals: u8,
    mint_authority: Pubkey,
    freeze_authority: Option<Pubkey>,
) -> Pubkey {
    let mint = Keypair::new_from_array([seed; 32]);
    let mint_pubkey = mint.pubkey();
    let space = SplLegacyMint::LEN;
    let lamports = svm.minimum_balance_for_rent_exemption(space);

    let create_ix = solana_system_interface::instruction::create_account(
        &payer.pubkey(),
        &mint_pubkey,
        lamports,
        space as u64,
        &spl_token_interface::ID,
    );
    let init_ix = initialize_mint2_legacy(
        &spl_token_interface::ID,
        &mint_pubkey,
        &mint_authority,
        freeze_authority.as_ref(),
        decimals,
    )
    .expect("valid initialize_mint2 instruction");

    send(svm, payer, &[&mint], vec![create_ix, init_ix]);
    mint_pubkey
}

/// Creates a Token-2022 mint carrying `extensions`, initialized in the order Token-2022 requires
/// (every extension before `InitializeMint2`).
pub fn create_token_2022_mint(
    svm: &mut LiteSVM,
    payer: &Keypair,
    seed: u8,
    decimals: u8,
    mint_authority: Pubkey,
    freeze_authority: Option<Pubkey>,
    extensions: &[Token2022Extension],
) -> Pubkey {
    let mint = Keypair::new_from_array([seed; 32]);
    let mint_pubkey = mint.pubkey();

    let extension_types: Vec<ExtensionType> = extensions.iter().map(extension_type_of).collect();
    let space = ExtensionType::try_calculate_account_len::<SplMint>(&extension_types)
        .expect("extension combination must be representable");
    let lamports = svm.minimum_balance_for_rent_exemption(space);

    let mut instructions = vec![solana_system_interface::instruction::create_account(
        &payer.pubkey(),
        &mint_pubkey,
        lamports,
        space as u64,
        &spl_token_2022_interface::ID,
    )];
    instructions.extend(
        extensions
            .iter()
            .map(|e| init_instruction(e, &mint_pubkey, &mint_authority)),
    );
    instructions.push(
        initialize_mint2_2022(
            &spl_token_2022_interface::ID,
            &mint_pubkey,
            &mint_authority,
            freeze_authority.as_ref(),
            decimals,
        )
        .expect("valid initialize_mint2 instruction"),
    );

    send(svm, payer, &[&mint], instructions);
    mint_pubkey
}

/// A Token-2022 mint carrying one TLV entry whose type code is not defined by this repository's
/// `spl-token-2022-interface` dependency — simulating a genuinely new extension a future
/// Token-2022 release might ship. This is the fixture for `A-TOK-05`, the positive-allowlist
/// test: `create_market` must reject it even though nothing in this program's own code
/// recognizes the specific type code as dangerous — it isn't recognized *at all*.
///
/// Built by direct byte construction (via `StateWithExtensionsMut`'s buffer accessors, not raw
/// offset arithmetic) rather than a transaction, since the real Token-2022 program has no
/// instruction that writes a type code it does not itself define.
pub fn create_token_2022_mint_with_unrecognized_extension(
    svm: &mut LiteSVM,
    decimals: u8,
    mint_authority: Pubkey,
) -> Pubkey {
    const UNRECOGNIZED_EXTENSION_TYPE: u16 = 0xBEEF;
    const ACCOUNT_TYPE_LEN: usize = 1;
    const FAKE_TLV_HEADER_LEN: usize = 4; // 2-byte type + 2-byte length, zero value bytes

    let mint_pubkey = Keypair::new_from_array([0xEEu8; 32]).pubkey();
    // The TLV region always starts after the *account*-sized base-and-padding zone
    // (`SplTokenAccount::LEN` = 165), even for a mint — this uniformity is what lets a Token-2022
    // parser find the `AccountType` marker byte at the same offset regardless of which base state
    // the account holds.
    let account_len =
        spl_token_2022_interface::state::Account::LEN + ACCOUNT_TYPE_LEN + FAKE_TLV_HEADER_LEN;
    let mut data = vec![0u8; account_len];

    {
        let mut state = StateWithExtensionsMut::<SplMint>::unpack_uninitialized(&mut data)
            .expect("uninitialized buffer of the right size must unpack");
        state.base = SplMint {
            mint_authority: solana_program_option::COption::Some(mint_authority),
            supply: 0,
            decimals,
            is_initialized: true,
            freeze_authority: solana_program_option::COption::None,
        };
        state.pack_base();
        state.get_account_type_mut()[0] =
            spl_token_2022_interface::extension::AccountType::Mint.into();
        let tlv = state.get_tlv_data_mut();
        tlv[0..2].copy_from_slice(&UNRECOGNIZED_EXTENSION_TYPE.to_le_bytes());
        tlv[2..4].copy_from_slice(&0u16.to_le_bytes());
    }

    // Sanity check performed at fixture-construction time (not by the code under test): the
    // crate's own parser must fail closed on this buffer, exactly like a genuinely new,
    // unrecognized Token-2022 extension would.
    debug_assert!(
        StateWithExtensions::<SplMint>::unpack(&data)
            .and_then(|s| s.get_extension_types())
            .is_err(),
        "fixture did not actually produce an unparseable extension list"
    );

    let lamports = svm.minimum_balance_for_rent_exemption(account_len);
    svm.set_account(
        mint_pubkey,
        Account {
            lamports,
            data,
            owner: spl_token_2022_interface::ID,
            executable: false,
            rent_epoch: 0,
        },
    )
    .expect("failed to inject synthetic mint fixture");

    mint_pubkey
}
