//! Phase 11 lab correctness check for `vault-anchor` — offline, in-process `LiteSVM`, no
//! dependency on `aegis-test-kit` (labs stay isolated from the production test infra). Proves the
//! happy path (initialize -> deposit -> withdraw) and the one security property specific to this
//! primitive that is cheap to test directly: a non-owner cannot withdraw.

use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use litesvm::LiteSVM;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_message::{Message, VersionedMessage};
use solana_program_pack::Pack;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;
use spl_token_interface::state::{Account as SplTokenAccount, Mint as SplMint};
use vault_anchor::{VaultAuthority, ID as PROGRAM_ID, VAULT_AUTHORITY_SEED, VAULT_TOKEN_SEED};

fn program_bytes() -> &'static [u8] {
    include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/deploy/vault_anchor.so"
    ))
}

fn deploy() -> (LiteSVM, Keypair) {
    let mut svm = LiteSVM::new();
    svm.add_program(PROGRAM_ID, program_bytes())
        .expect("load vault_anchor.so");
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

fn pdas(mint: &Pubkey) -> (Pubkey, u8, Pubkey) {
    let (vault_authority, bump) =
        Pubkey::find_program_address(&[VAULT_AUTHORITY_SEED, mint.as_ref()], &PROGRAM_ID);
    let (vault_token_account, _) =
        Pubkey::find_program_address(&[VAULT_TOKEN_SEED, mint.as_ref()], &PROGRAM_ID);
    (vault_authority, bump, vault_token_account)
}

#[test]
fn initialize_deposit_withdraw_round_trip() {
    let (mut svm, payer) = deploy();
    let mint = create_mint(&mut svm, &payer, 10, 6);
    let (vault_authority, _bump, vault_token_account) = pdas(&mint);
    let owner = payer.pubkey();

    let init_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vault_anchor::accounts::InitializeVault {
            payer: payer.pubkey(),
            owner,
            mint,
            vault_authority,
            vault_token_account,
            token_program: spl_token_interface::ID,
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
        data: vault_anchor::instruction::InitializeVault {}.data(),
    };
    send(&mut svm, &payer, &[], vec![init_ix]);

    let depositor_ata = create_token_account(&mut svm, &payer, 11, mint, payer.pubkey());
    mint_to(&mut svm, &payer, mint, depositor_ata, 1_000_000);

    let deposit_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vault_anchor::accounts::Deposit {
            depositor: payer.pubkey(),
            vault_authority,
            vault_token_account,
            depositor_token_account: depositor_ata,
            mint,
            token_program: spl_token_interface::ID,
        }
        .to_account_metas(None),
        data: vault_anchor::instruction::Deposit { amount: 400_000 }.data(),
    };
    send(&mut svm, &payer, &[], vec![deposit_ix]);

    assert_eq!(token_balance(&svm, &vault_token_account), 400_000);
    assert_eq!(token_balance(&svm, &depositor_ata), 600_000);

    let recipient_ata = create_token_account(&mut svm, &payer, 12, mint, payer.pubkey());
    let withdraw_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vault_anchor::accounts::Withdraw {
            owner,
            vault_authority,
            vault_token_account,
            recipient_token_account: recipient_ata,
            mint,
            token_program: spl_token_interface::ID,
        }
        .to_account_metas(None),
        data: vault_anchor::instruction::Withdraw { amount: 150_000 }.data(),
    };
    send(&mut svm, &payer, &[], vec![withdraw_ix]);

    assert_eq!(token_balance(&svm, &vault_token_account), 250_000);
    assert_eq!(token_balance(&svm, &recipient_ata), 150_000);

    let account_data = svm.get_account(&vault_authority).unwrap().data;
    let state = VaultAuthority::try_deserialize(&mut account_data.as_slice()).unwrap();
    assert_eq!(state.owner, owner);
    assert_eq!(state.mint, mint);
}

#[test]
fn non_owner_cannot_withdraw() {
    let (mut svm, payer) = deploy();
    let mint = create_mint(&mut svm, &payer, 20, 6);
    let (vault_authority, _bump, vault_token_account) = pdas(&mint);
    let owner = payer.pubkey();

    let init_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vault_anchor::accounts::InitializeVault {
            payer: payer.pubkey(),
            owner,
            mint,
            vault_authority,
            vault_token_account,
            token_program: spl_token_interface::ID,
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
        data: vault_anchor::instruction::InitializeVault {}.data(),
    };
    send(&mut svm, &payer, &[], vec![init_ix]);

    let depositor_ata = create_token_account(&mut svm, &payer, 21, mint, payer.pubkey());
    mint_to(&mut svm, &payer, mint, depositor_ata, 1_000_000);
    let deposit_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vault_anchor::accounts::Deposit {
            depositor: payer.pubkey(),
            vault_authority,
            vault_token_account,
            depositor_token_account: depositor_ata,
            mint,
            token_program: spl_token_interface::ID,
        }
        .to_account_metas(None),
        data: vault_anchor::instruction::Deposit { amount: 500_000 }.data(),
    };
    send(&mut svm, &payer, &[], vec![deposit_ix]);

    let attacker = Keypair::new_from_array([99u8; 32]);
    svm.airdrop(&attacker.pubkey(), 10_000_000_000).unwrap();
    let attacker_ata = create_token_account(&mut svm, &payer, 22, mint, attacker.pubkey());
    let withdraw_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vault_anchor::accounts::Withdraw {
            owner: attacker.pubkey(),
            vault_authority,
            vault_token_account,
            recipient_token_account: attacker_ata,
            mint,
            token_program: spl_token_interface::ID,
        }
        .to_account_metas(None),
        data: vault_anchor::instruction::Withdraw { amount: 1 }.data(),
    };
    let blockhash = svm.latest_blockhash();
    let message = Message::new_with_blockhash(&[withdraw_ix], Some(&attacker.pubkey()), &blockhash);
    let tx =
        VersionedTransaction::try_new(VersionedMessage::Legacy(message), &[&attacker]).unwrap();
    let result = svm.send_transaction(tx);
    assert!(result.is_err(), "a non-owner must not be able to withdraw");
}
