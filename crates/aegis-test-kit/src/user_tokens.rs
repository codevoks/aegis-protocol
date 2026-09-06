//! User-owned token account fixtures: plain (non-PDA) SPL Token / Token-2022 accounts that hold a
//! collateral mint on behalf of a depositor or withdrawal recipient, and minting into them. These
//! are the "wallet" side of a collateral flow; `token/vault.rs` in the program itself owns the
//! PDA-vault side (`docs/phases/phase-03-collateral.md`).

use crate::mints::send;
use litesvm::LiteSVM;
use solana_keypair::Keypair;
use solana_program_pack::Pack;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use spl_token_2022_interface::extension::ExtensionType;
use spl_token_2022_interface::state::Account as SplTokenAccount;

/// Creates and initializes a plain (non-associated, non-PDA) token account of `token_program_id`
/// for `mint`, owned by `owner`. `seed` derives a fixed keypair so a failing test is reproducible
/// (`docs/zero-cost-demo.md` §6) — no `Keypair::new()`.
///
/// `mint_extension_types` is the mint's own Token-2022 extension inventory (empty for a classic
/// SPL Token mint): a mint carrying e.g. `TransferFeeConfig` requires every holder account to
/// carry the matching `TransferFeeAmount` extension, so the account must be sized for it up front
/// — the same `ExtensionType::try_calculate_account_len` sizing `token/vault.rs` uses, minus the
/// `ImmutableOwner` extension that is specific to Aegis's own PDA vaults (`token-compatibility.md`
/// §5.4).
pub fn create_token_account(
    svm: &mut LiteSVM,
    payer: &Keypair,
    seed: u8,
    mint: Pubkey,
    owner: Pubkey,
    token_program_id: Pubkey,
    mint_extension_types: &[ExtensionType],
) -> Pubkey {
    let account = Keypair::new_from_array([seed; 32]);
    let account_pubkey = account.pubkey();

    let space = if token_program_id == spl_token_2022_interface::ID {
        let required_extensions =
            ExtensionType::get_required_init_account_extensions(mint_extension_types);
        ExtensionType::try_calculate_account_len::<SplTokenAccount>(&required_extensions)
            .expect("extension combination must be representable")
    } else {
        SplTokenAccount::LEN
    };
    let lamports = svm.minimum_balance_for_rent_exemption(space);

    let create_ix = solana_system_interface::instruction::create_account(
        &payer.pubkey(),
        &account_pubkey,
        lamports,
        space as u64,
        &token_program_id,
    );
    let init_ix = if token_program_id == spl_token_2022_interface::ID {
        spl_token_2022_interface::instruction::initialize_account3(
            &token_program_id,
            &account_pubkey,
            &mint,
            &owner,
        )
    } else {
        spl_token_interface::instruction::initialize_account3(
            &token_program_id,
            &account_pubkey,
            &mint,
            &owner,
        )
    }
    .expect("valid initialize_account3 instruction");

    send(svm, payer, &[&account], vec![create_ix, init_ix]);
    account_pubkey
}

/// Mints `amount` of `mint` into `destination`, signed by `mint_authority`.
pub fn mint_to(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint: Pubkey,
    destination: Pubkey,
    mint_authority: &Keypair,
    amount: u64,
    token_program_id: Pubkey,
) {
    let ix = if token_program_id == spl_token_2022_interface::ID {
        spl_token_2022_interface::instruction::mint_to(
            &token_program_id,
            &mint,
            &destination,
            &mint_authority.pubkey(),
            &[],
            amount,
        )
    } else {
        spl_token_interface::instruction::mint_to(
            &token_program_id,
            &mint,
            &destination,
            &mint_authority.pubkey(),
            &[],
            amount,
        )
    }
    .expect("valid mint_to instruction");

    // Avoid presenting the same underlying signer twice when `mint_authority` and `payer` happen
    // to be the same keypair (a common fixture shortcut) — `send`'s signer list is only for
    // signers *other than* `payer`.
    let extra_signers: &[&Keypair] = if mint_authority.pubkey() == payer.pubkey() {
        &[]
    } else {
        &[mint_authority]
    };
    send(svm, payer, extra_signers, vec![ix]);
}
