//! `Initialize` (tag 0): `[payer (signer, writable), owner (readonly), mint (readonly),
//! vault_authority (writable, PDA to create), vault_token_account (writable, PDA to create),
//! token_program (readonly), system_program (readonly)]`.
//!
//! `owner` deliberately does not need to sign -- it is only recorded as the vault's owner
//! pubkey, matching `labs/vault-anchor`'s `InitializeVault::owner: UncheckedAccount`.

use pinocchio::{
    account::AccountView,
    address::Address,
    cpi::{Seed, Signer},
    ProgramResult,
};
use pinocchio_system::{instructions::CreateAccount, ID as SYSTEM_PROGRAM_ID};
use pinocchio_token::{instructions::InitializeAccount3, ID as TOKEN_PROGRAM_ID};

use crate::{error::VaultError, state::VaultAuthority, VAULT_AUTHORITY_SEED, VAULT_TOKEN_SEED};

/// The real SPL Token `Account` on-chain layout: `mint`(32) + `owner`(32) + `amount`(8) +
/// `delegate: COption<Pubkey>`(36) + `state`(1) + `is_native: COption<u64>`(12) +
/// `delegate_amount`(8) + `close_authority: COption<Pubkey>`(36) = 165 bytes.
const SPL_TOKEN_ACCOUNT_LEN: u64 = 165;

pub fn process(program_id: &Address, accounts: &mut [AccountView]) -> ProgramResult {
    let [payer, owner, mint, vault_authority, vault_token_account, token_program, system_program] =
        accounts
    else {
        return Err(VaultError::NotEnoughAccountKeys.into());
    };

    if !payer.is_signer() {
        return Err(VaultError::MissingRequiredSignature.into());
    }
    if token_program.address() != &TOKEN_PROGRAM_ID {
        return Err(VaultError::InvalidTokenProgram.into());
    }
    if system_program.address() != &SYSTEM_PROGRAM_ID {
        return Err(VaultError::InvalidSystemProgram.into());
    }

    let mint_key = *mint.address();

    // Re-derive both PDAs ourselves rather than trusting the caller's account list -- this is
    // the one instruction where the canonical bump is not yet known, so `derive_program_address`
    // (which searches for it, this crate's `find_program_address` equivalent) is the correct
    // tool here, not `derive_address` with a specific bump.
    let (expected_vault_authority, vault_authority_bump) =
        Address::derive_program_address(&[VAULT_AUTHORITY_SEED, mint_key.as_ref()], program_id)
            .ok_or(VaultError::InvalidVaultAuthority)?;
    if vault_authority.address() != &expected_vault_authority {
        return Err(VaultError::InvalidVaultAuthority.into());
    }

    let (expected_vault_token_account, vault_token_bump) =
        Address::derive_program_address(&[VAULT_TOKEN_SEED, mint_key.as_ref()], program_id)
            .ok_or(VaultError::InvalidVaultTokenAccount)?;
    if vault_token_account.address() != &expected_vault_token_account {
        return Err(VaultError::InvalidVaultTokenAccount.into());
    }

    // Create `vault_authority`, signed with its own PDA seeds -- `create_account` requires the
    // account being created to co-sign, and only the owning program can produce that signature
    // for a PDA.
    let vault_authority_bump_seed = [vault_authority_bump];
    let vault_authority_seeds = [
        Seed::from(VAULT_AUTHORITY_SEED),
        Seed::from(mint_key.as_ref()),
        Seed::from(&vault_authority_bump_seed[..]),
    ];
    let vault_authority_signer = Signer::from(&vault_authority_seeds);

    CreateAccount::with_minimum_balance(
        payer,
        vault_authority,
        VaultAuthority::LEN as u64,
        program_id,
        None,
    )?
    .invoke_signed(&[vault_authority_signer])?;

    // Create `vault_token_account`, owned by the (verified) real SPL Token program, signed with
    // its own PDA seeds.
    let vault_token_bump_seed = [vault_token_bump];
    let vault_token_seeds = [
        Seed::from(VAULT_TOKEN_SEED),
        Seed::from(mint_key.as_ref()),
        Seed::from(&vault_token_bump_seed[..]),
    ];
    let vault_token_signer = Signer::from(&vault_token_seeds);

    CreateAccount::with_minimum_balance(
        payer,
        vault_token_account,
        SPL_TOKEN_ACCOUNT_LEN,
        &TOKEN_PROGRAM_ID,
        None,
    )?
    .invoke_signed(&[vault_token_signer])?;

    // Initialize it as a real SPL Token account whose token-level authority is the
    // `vault_authority` PDA -- ordinary CPI, no PDA signature required for `InitializeAccount3`
    // itself.
    InitializeAccount3::new(vault_token_account, mint, vault_authority.address()).invoke()?;

    // Write the 97-byte `VaultAuthority` state directly into the account's own data buffer --
    // no framework macro, no discriminator (see `state.rs`'s module doc comment).
    let state = VaultAuthority {
        owner: *owner.address(),
        mint: mint_key,
        vault_token_account: *vault_token_account.address(),
        bump: vault_authority_bump,
    };
    let mut data = vault_authority.try_borrow_mut()?;
    state.pack(&mut data)?;

    Ok(())
}
