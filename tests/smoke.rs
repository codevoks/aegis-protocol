//! Phase 1 zero-cost smoke test.
//!
//! Loads the built `aegis` program into an in-process LiteSVM instance and invokes its
//! only instruction, `ping`. No RPC, no validator, no network of any kind — the whole
//! test runs against bytes read from `target/deploy/aegis.so` and an in-memory SVM
//! (see `docs/zero-cost-demo.md` and `docs/phases/phase-01-foundation.md` §9).

use aegis::accounts::Ping as PingAccounts;
use aegis::instruction::Ping as PingInstruction;
use aegis_test_kit::deploy;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::{InstructionData, ToAccountMetas};
use solana_message::{Message, VersionedMessage};
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;

#[test]
fn ping_deploys_and_invokes_offline() {
    let program_id = aegis::id();
    let program_bytes = include_bytes!(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/aegis.so"));

    let (mut svm, payer) = deploy(program_id, program_bytes);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &PingInstruction {}.data(),
        PingAccounts {}.to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let message = Message::new_with_blockhash(&[instruction], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(message), &[&payer])
        .expect("failed to sign transaction");

    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "ping should succeed: {:?}", result.err());
}
