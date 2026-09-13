//! Phase 11 — CU comparison of the Aegis custody primitive (initialize / deposit / withdraw a
//! vault PDA) across `labs/vault-anchor`, `labs/vault-native`, and `labs/vault-pinocchio`
//! (`docs/adr/0003-native-pinocchio-as-labs.md`, `docs/phases/phase-11-performance.md` #28).
//!
//! Methodology: identical to `tests/bench/harness.rs` -- real pre-state built via `LiteSVM` (mint
//! creation, funding), then each instruction measured through Mollusk against the real compiled
//! `.so` for that lab. The three programs are NOT identical bytes (different frameworks, by
//! design), but implement the byte-identical custody primitive: the same seeds
//! (`[b"vault_authority", mint]` / `[b"vault_token", mint]`), the same account order, and the
//! same security checks (signer, mint, PDA, authority, owner -- see each lab's own doc comment).

use anchor_lang::{InstructionData, ToAccountMetas};
use litesvm::LiteSVM;
use mollusk_svm::Mollusk;
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::{Message, VersionedMessage};
use solana_program_pack::Pack;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;
use spl_token_interface::state::{Account as SplTokenAccount, Mint as SplMint};

const VAULT_AUTHORITY_SEED: &[u8] = b"vault_authority";
const VAULT_TOKEN_SEED: &[u8] = b"vault_token";

fn pdas(program_id: &Pubkey, mint: &Pubkey) -> (Pubkey, u8, Pubkey) {
    let (vault_authority, bump) =
        Pubkey::find_program_address(&[VAULT_AUTHORITY_SEED, mint.as_ref()], program_id);
    let (vault_token_account, _) =
        Pubkey::find_program_address(&[VAULT_TOKEN_SEED, mint.as_ref()], program_id);
    (vault_authority, bump, vault_token_account)
}

fn new_mollusk(program_id: &Pubkey, so_bytes: &[u8]) -> Mollusk {
    let mut mollusk = Mollusk::default();
    mollusk.add_program_with_loader_and_elf(
        program_id,
        &mollusk_svm::program::loader_keys::LOADER_V3,
        so_bytes,
    );
    mollusk_svm_programs_token::token::add_program(&mut mollusk);
    mollusk
}

fn program_stub(program_id: &Pubkey) -> Option<Account> {
    if *program_id == anchor_lang::solana_program::system_program::ID {
        return Some(mollusk_svm::program::keyed_account_for_system_program().1);
    }
    if *program_id == spl_token_interface::ID {
        return Some(mollusk_svm_programs_token::token::account());
    }
    None
}

fn snapshot_for(svm: &LiteSVM, ix: &Instruction) -> Vec<(Pubkey, Account)> {
    ix.accounts
        .iter()
        .map(|meta| {
            let pubkey = meta.pubkey;
            let account = program_stub(&pubkey)
                .or_else(|| svm.get_account(&pubkey))
                .unwrap_or_default();
            (pubkey, account)
        })
        .collect()
}

fn measure(mollusk: &mut Mollusk, ix: &Instruction, accounts: &[(Pubkey, Account)]) -> u64 {
    let result = mollusk.process_instruction(ix, accounts);
    match result.program_result {
        mollusk_svm::result::ProgramResult::Success => result.compute_units_consumed,
        other => panic!(
            "cu-bench: instruction failed (program_result={other:?}, raw_result={:?})",
            result.raw_result
        ),
    }
}

fn send(svm: &mut LiteSVM, payer: &Keypair, extra: &[&Keypair], ix: Instruction) {
    let blockhash = svm.latest_blockhash();
    let message = Message::new_with_blockhash(&[ix], Some(&payer.pubkey()), &blockhash);
    let mut signers: Vec<&Keypair> = vec![payer];
    signers.extend_from_slice(extra);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(message), &signers).unwrap();
    svm.send_transaction(tx)
        .expect("setup transaction must succeed");
}

fn create_mint(svm: &mut LiteSVM, payer: &Keypair, seed: u8) -> Pubkey {
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
        6,
    )
    .unwrap();
    send(svm, payer, &[&mint], create_ix);
    send(svm, payer, &[], init_ix);
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
    send(svm, payer, &[&account], create_ix);
    send(svm, payer, &[], init_ix);
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
    send(svm, payer, &[], ix);
}

/// One row of the CU comparison table.
#[derive(Debug)]
struct Row {
    label: &'static str,
    initialize: u64,
    deposit: u64,
    withdraw: u64,
}

// --- vault-anchor ---

fn bench_vault_anchor() -> Row {
    let program_id = vault_anchor::ID;
    let so_bytes = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../vault-anchor/../../target/deploy/vault_anchor.so"
    ));
    let mut svm = LiteSVM::new();
    svm.add_program(program_id, so_bytes).unwrap();
    let payer = Keypair::new_from_array([1u8; 32]);
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    let mint = create_mint(&mut svm, &payer, 10);
    let owner = payer.pubkey();
    let (vault_authority, _bump, vault_token_account) = pdas(&program_id, &mint);

    let init_ix = Instruction {
        program_id,
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
    let mut mollusk = new_mollusk(&program_id, so_bytes);
    let init_cu = measure(&mut mollusk, &init_ix, &snapshot_for(&svm, &init_ix));
    send(&mut svm, &payer, &[], init_ix);

    let depositor_ata = create_token_account(&mut svm, &payer, 11, mint, payer.pubkey());
    mint_to(&mut svm, &payer, mint, depositor_ata, 1_000_000);
    let deposit_ix = Instruction {
        program_id,
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
    let deposit_cu = measure(&mut mollusk, &deposit_ix, &snapshot_for(&svm, &deposit_ix));
    send(&mut svm, &payer, &[], deposit_ix);

    let recipient_ata = create_token_account(&mut svm, &payer, 12, mint, payer.pubkey());
    let withdraw_ix = Instruction {
        program_id,
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
    let withdraw_cu = measure(
        &mut mollusk,
        &withdraw_ix,
        &snapshot_for(&svm, &withdraw_ix),
    );

    Row {
        label: "vault-anchor",
        initialize: init_cu,
        deposit: deposit_cu,
        withdraw: withdraw_cu,
    }
}

// --- shared tag-based instruction encoding for vault-native / vault-pinocchio ---
// tag 0 = Initialize, 1 = Deposit(amount: u64 LE), 2 = Withdraw(amount: u64 LE)

fn tagged_ix(
    program_id: Pubkey,
    accounts: Vec<AccountMeta>,
    tag: u8,
    amount: Option<u64>,
) -> Instruction {
    let mut data = vec![tag];
    if let Some(a) = amount {
        data.extend_from_slice(&a.to_le_bytes());
    }
    Instruction {
        program_id,
        accounts,
        data,
    }
}

fn bench_tagged(label: &'static str, program_id: Pubkey, so_bytes: &[u8]) -> Row {
    let mut svm = LiteSVM::new();
    svm.add_program(program_id, so_bytes).unwrap();
    let payer = Keypair::new_from_array([2u8; 32]);
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    let mint = create_mint(&mut svm, &payer, 20);
    let owner = payer.pubkey();
    let (vault_authority, _bump, vault_token_account) = pdas(&program_id, &mint);

    let init_ix = tagged_ix(
        program_id,
        vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(owner, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(vault_authority, false),
            AccountMeta::new(vault_token_account, false),
            AccountMeta::new_readonly(spl_token_interface::ID, false),
            AccountMeta::new_readonly(anchor_lang::solana_program::system_program::ID, false),
        ],
        0,
        None,
    );
    let mut mollusk = new_mollusk(&program_id, so_bytes);
    let init_cu = measure(&mut mollusk, &init_ix, &snapshot_for(&svm, &init_ix));
    send(&mut svm, &payer, &[], init_ix);

    let depositor_ata = create_token_account(&mut svm, &payer, 21, mint, payer.pubkey());
    mint_to(&mut svm, &payer, mint, depositor_ata, 1_000_000);
    let deposit_ix = tagged_ix(
        program_id,
        vec![
            AccountMeta::new_readonly(payer.pubkey(), true),
            AccountMeta::new_readonly(vault_authority, false),
            AccountMeta::new(vault_token_account, false),
            AccountMeta::new(depositor_ata, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(spl_token_interface::ID, false),
        ],
        1,
        Some(400_000),
    );
    let deposit_cu = measure(&mut mollusk, &deposit_ix, &snapshot_for(&svm, &deposit_ix));
    send(&mut svm, &payer, &[], deposit_ix);

    let recipient_ata = create_token_account(&mut svm, &payer, 22, mint, payer.pubkey());
    let withdraw_ix = tagged_ix(
        program_id,
        vec![
            AccountMeta::new_readonly(owner, true),
            AccountMeta::new_readonly(vault_authority, false),
            AccountMeta::new(vault_token_account, false),
            AccountMeta::new(recipient_ata, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(spl_token_interface::ID, false),
        ],
        2,
        Some(150_000),
    );
    let withdraw_cu = measure(
        &mut mollusk,
        &withdraw_ix,
        &snapshot_for(&svm, &withdraw_ix),
    );

    Row {
        label,
        initialize: init_cu,
        deposit: deposit_cu,
        withdraw: withdraw_cu,
    }
}

#[test]
fn cu_comparison() {
    let anchor_row = bench_vault_anchor();

    let native_so = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/deploy/vault_native.so"
    ));
    let native_id: Pubkey = vault_native::ID;
    let native_row = bench_tagged("vault-native", native_id, native_so);

    let pinocchio_so = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/deploy/vault_pinocchio.so"
    ));
    // `vault_pinocchio::ID` is a `pinocchio::address::Address`, structurally identical to but
    // nominally distinct from `solana_pubkey::Pubkey` (see labs/vault-pinocchio/Cargo.toml's own
    // doc comment on the pinocchio-pubkey version mismatch this crate works around).
    let pinocchio_id: Pubkey = Pubkey::new_from_array(*vault_pinocchio::ID.as_array());
    let pinocchio_row = bench_tagged("vault-pinocchio", pinocchio_id, pinocchio_so);

    eprintln!("\n=== labs/cu-bench: custody primitive CU comparison ===");
    eprintln!(
        "{:<18} {:>12} {:>12} {:>12}",
        "lab", "initialize", "deposit", "withdraw"
    );
    for row in [&anchor_row, &native_row, &pinocchio_row] {
        eprintln!(
            "{:<18} {:>12} {:>12} {:>12}",
            row.label, row.initialize, row.deposit, row.withdraw
        );
    }

    let native_delta_init = anchor_row.initialize as i64 - native_row.initialize as i64;
    let native_delta_dep = anchor_row.deposit as i64 - native_row.deposit as i64;
    let native_delta_wd = anchor_row.withdraw as i64 - native_row.withdraw as i64;
    let pino_delta_init = anchor_row.initialize as i64 - pinocchio_row.initialize as i64;
    let pino_delta_dep = anchor_row.deposit as i64 - pinocchio_row.deposit as i64;
    let pino_delta_wd = anchor_row.withdraw as i64 - pinocchio_row.withdraw as i64;

    eprintln!(
        "\nNative Δ vs Anchor: initialize={native_delta_init} deposit={native_delta_dep} withdraw={native_delta_wd}"
    );
    eprintln!(
        "Pinocchio Δ vs Anchor: initialize={pino_delta_init} deposit={pino_delta_dep} withdraw={pino_delta_wd}"
    );
}
