//! Phase 11 lab correctness check for `vault-native` -- offline, in-process `LiteSVM`, mirroring
//! `labs/vault-anchor/tests/basic.rs` exactly in structure and assertions so the two labs are
//! genuinely comparable. No Anchor codegen here (no `InstructionData`/`ToAccountMetas`), so every
//! `Instruction` is built by hand: `data: vec![tag_byte, ...amount_le_bytes]`.

use litesvm::LiteSVM;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::{Message, VersionedMessage};
use solana_program_pack::Pack;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;
use spl_token_interface::state::{Account as SplTokenAccount, Mint as SplMint};
use vault_native::{VaultAuthority, ID as PROGRAM_ID, VAULT_AUTHORITY_SEED, VAULT_TOKEN_SEED};

const TAG_INITIALIZE: u8 = 0;
const TAG_DEPOSIT: u8 = 1;
const TAG_WITHDRAW: u8 = 2;

fn program_bytes() -> &'static [u8] {
    include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/deploy/vault_native.so"
    ))
}

fn deploy() -> (LiteSVM, Keypair) {
    let mut svm = LiteSVM::new();
    svm.add_program(PROGRAM_ID, program_bytes())
        .expect("load vault_native.so");
    let payer = Keypair::new_from_array([1u8; 32]);
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    (svm, payer)
}

fn create_mint(svm: &mut LiteSVM, payer: &Keypair, seed: u8, decimals: u8) -> Pubkey {
    let mint = Keypair::new_from_array([seed; 32]);
    let space = SplMint::LEN;
    let lamports = svm.minimum_balance_for_rent_exemption(space);
    let create_ix = solana_system_interface::instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        lamports,
        space as u64,
        &spl_token_interface::ID,
    );
    let init_ix = spl_token_interface::instruction::initialize_mint2(
        &spl_token_interface::ID,
        &mint.pubkey(),
        &payer.pubkey(),
        None,
        decimals,
    )
    .unwrap();
    send(svm, payer, &[&mint], vec![create_ix, init_ix]);
    mint.pubkey()
}

fn create_token_account(
    svm: &mut LiteSVM,
    payer: &Keypair,
    seed: u8,
    mint: Pubkey,
    owner: Pubkey,
) -> Pubkey {
    let account = Keypair::new_from_array([seed; 32]);
    let space = SplTokenAccount::LEN;
    let lamports = svm.minimum_balance_for_rent_exemption(space);
    let create_ix = solana_system_interface::instruction::create_account(
        &payer.pubkey(),
        &account.pubkey(),
        lamports,
        space as u64,
        &spl_token_interface::ID,
    );
    let init_ix = spl_token_interface::instruction::initialize_account3(
        &spl_token_interface::ID,
        &account.pubkey(),
        &mint,
        &owner,
    )
    .unwrap();
    send(svm, payer, &[&account], vec![create_ix, init_ix]);
    account.pubkey()
}

fn mint_to(svm: &mut LiteSVM, payer: &Keypair, mint: Pubkey, destination: Pubkey, amount: u64) {
    let ix = spl_token_interface::instruction::mint_to(
        &spl_token_interface::ID,
        &mint,
        &destination,
        &payer.pubkey(),
        &[],
        amount,
    )
    .unwrap();
    send(svm, payer, &[], vec![ix]);
}

fn token_balance(svm: &LiteSVM, account: &Pubkey) -> u64 {
    let data = svm.get_account(account).unwrap().data;
    SplTokenAccount::unpack(&data[..SplTokenAccount::LEN])
        .unwrap()
        .amount
}

fn send(svm: &mut LiteSVM, payer: &Keypair, extra_signers: &[&Keypair], ixs: Vec<Instruction>) {
    let blockhash = svm.latest_blockhash();
    let message = Message::new_with_blockhash(&ixs, Some(&payer.pubkey()), &blockhash);
    let mut signers: Vec<&Keypair> = vec![payer];
    signers.extend_from_slice(extra_signers);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(message), &signers).unwrap();
    svm.send_transaction(tx).expect("transaction must succeed");
}

fn pdas(mint: &Pubkey) -> (Pubkey, Pubkey) {
    let (vault_authority, _bump) =
        Pubkey::find_program_address(&[VAULT_AUTHORITY_SEED, mint.as_ref()], &PROGRAM_ID);
    let (vault_token_account, _) =
        Pubkey::find_program_address(&[VAULT_TOKEN_SEED, mint.as_ref()], &PROGRAM_ID);
    (vault_authority, vault_token_account)
}

fn initialize_ix(
    payer: Pubkey,
    owner: Pubkey,
    mint: Pubkey,
    vault_authority: Pubkey,
    vault_token_account: Pubkey,
) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(owner, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(vault_authority, false),
            AccountMeta::new(vault_token_account, false),
            AccountMeta::new_readonly(spl_token_interface::ID, false),
            AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        ],
        data: vec![TAG_INITIALIZE],
    }
}

fn deposit_ix(
    depositor: Pubkey,
    vault_authority: Pubkey,
    vault_token_account: Pubkey,
    depositor_token_account: Pubkey,
    mint: Pubkey,
    amount: u64,
) -> Instruction {
    let mut data = vec![TAG_DEPOSIT];
    data.extend_from_slice(&amount.to_le_bytes());
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(depositor, true),
            AccountMeta::new_readonly(vault_authority, false),
            AccountMeta::new(vault_token_account, false),
            AccountMeta::new(depositor_token_account, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(spl_token_interface::ID, false),
        ],
        data,
    }
}

fn withdraw_ix(
    owner: Pubkey,
    vault_authority: Pubkey,
    vault_token_account: Pubkey,
    recipient_token_account: Pubkey,
    mint: Pubkey,
    amount: u64,
) -> Instruction {
    let mut data = vec![TAG_WITHDRAW];
    data.extend_from_slice(&amount.to_le_bytes());
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(owner, true),
            AccountMeta::new_readonly(vault_authority, false),
            AccountMeta::new(vault_token_account, false),
            AccountMeta::new(recipient_token_account, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(spl_token_interface::ID, false),
        ],
        data,
    }
}

#[test]
fn initialize_deposit_withdraw_round_trip() {
    let (mut svm, payer) = deploy();
    let mint = create_mint(&mut svm, &payer, 10, 6);
    let (vault_authority, vault_token_account) = pdas(&mint);
    let owner = payer.pubkey();

    send(
        &mut svm,
        &payer,
        &[],
        vec![initialize_ix(
            payer.pubkey(),
            owner,
            mint,
            vault_authority,
            vault_token_account,
        )],
    );

    let depositor_ata = create_token_account(&mut svm, &payer, 11, mint, payer.pubkey());
    mint_to(&mut svm, &payer, mint, depositor_ata, 1_000_000);

    send(
        &mut svm,
        &payer,
        &[],
        vec![deposit_ix(
            payer.pubkey(),
            vault_authority,
            vault_token_account,
            depositor_ata,
            mint,
            400_000,
        )],
    );

    assert_eq!(token_balance(&svm, &vault_token_account), 400_000);
    assert_eq!(token_balance(&svm, &depositor_ata), 600_000);

    let recipient_ata = create_token_account(&mut svm, &payer, 12, mint, payer.pubkey());
    send(
        &mut svm,
        &payer,
        &[],
        vec![withdraw_ix(
            owner,
            vault_authority,
            vault_token_account,
            recipient_ata,
            mint,
            150_000,
        )],
    );

    assert_eq!(token_balance(&svm, &vault_token_account), 250_000);
    assert_eq!(token_balance(&svm, &recipient_ata), 150_000);

    let account_data = svm.get_account(&vault_authority).unwrap().data;
    let state = VaultAuthority::unpack(&account_data).unwrap();
    assert_eq!(state.owner, owner);
    assert_eq!(state.mint, mint);
    assert_eq!(state.vault_token_account, vault_token_account);
}

#[test]
fn non_owner_cannot_withdraw() {
    let (mut svm, payer) = deploy();
    let mint = create_mint(&mut svm, &payer, 20, 6);
    let (vault_authority, vault_token_account) = pdas(&mint);
    let owner = payer.pubkey();

    send(
        &mut svm,
        &payer,
        &[],
        vec![initialize_ix(
            payer.pubkey(),
            owner,
            mint,
            vault_authority,
            vault_token_account,
        )],
    );

    let depositor_ata = create_token_account(&mut svm, &payer, 21, mint, payer.pubkey());
    mint_to(&mut svm, &payer, mint, depositor_ata, 1_000_000);
    send(
        &mut svm,
        &payer,
        &[],
        vec![deposit_ix(
            payer.pubkey(),
            vault_authority,
            vault_token_account,
            depositor_ata,
            mint,
            500_000,
        )],
    );

    let attacker = Keypair::new_from_array([99u8; 32]);
    svm.airdrop(&attacker.pubkey(), 10_000_000_000).unwrap();
    let attacker_ata = create_token_account(&mut svm, &payer, 22, mint, attacker.pubkey());
    let ix = withdraw_ix(
        attacker.pubkey(),
        vault_authority,
        vault_token_account,
        attacker_ata,
        mint,
        1,
    );
    let blockhash = svm.latest_blockhash();
    let message = Message::new_with_blockhash(&[ix], Some(&attacker.pubkey()), &blockhash);
    let tx =
        VersionedTransaction::try_new(VersionedMessage::Legacy(message), &[&attacker]).unwrap();
    let result = svm.send_transaction(tx);
    assert!(result.is_err(), "a non-owner must not be able to withdraw");
}
