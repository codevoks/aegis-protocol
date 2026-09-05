//! LiteSVM bootstrap: load a built program and fund a deterministic payer.
//!
//! Keypairs here are derived from a fixed seed rather than `Keypair::new()`, so that a
//! failing test is reproducible from its seed alone (docs/zero-cost-demo.md §6).

use litesvm::LiteSVM;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

const PAYER_SEED: [u8; 32] = [7u8; 32];

/// The deterministic payer used across test fixtures.
pub fn deterministic_payer() -> Keypair {
    Keypair::new_from_array(PAYER_SEED)
}

/// Boots a fresh, offline LiteSVM instance with `program_id` loaded from `program_bytes`
/// and a funded deterministic payer.
pub fn deploy(program_id: Pubkey, program_bytes: &[u8]) -> (LiteSVM, Keypair) {
    let mut svm = LiteSVM::new();
    svm.add_program(program_id, program_bytes)
        .expect("failed to load program into LiteSVM");

    let payer = deterministic_payer();
    svm.airdrop(&payer.pubkey(), 10_000_000_000)
        .expect("failed to airdrop to the deterministic payer");

    (svm, payer)
}
