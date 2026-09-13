//! `Deposit` (tag 1, amount `u64`): `[depositor (signer), vault_authority (readonly,
//! PDA-checked), vault_token_account (writable), depositor_token_account (writable), mint
//! (readonly), token_program (readonly)]`. Ordinary CPI, no PDA signature needed -- `depositor`
//! signs for itself.

use pinocchio::{account::AccountView, address::Address, ProgramResult};
use pinocchio_token::{instructions::TransferChecked, ID as TOKEN_PROGRAM_ID};

use crate::{
    error::VaultError,
    spl::{read_mint_decimals, read_token_account_mint},
    state::VaultAuthority,
    VAULT_AUTHORITY_SEED,
};

pub fn process(program_id: &Address, accounts: &mut [AccountView], amount: u64) -> ProgramResult {
    let [depositor, vault_authority, vault_token_account, depositor_token_account, mint, token_program] =
        accounts
    else {
        return Err(VaultError::NotEnoughAccountKeys.into());
    };

    if amount == 0 {
        return Err(VaultError::ZeroAmount.into());
    }
    if !depositor.is_signer() {
        return Err(VaultError::MissingRequiredSignature.into());
    }
    if token_program.address() != &TOKEN_PROGRAM_ID {
        return Err(VaultError::InvalidTokenProgram.into());
    }

    // `vault_authority` must be owned by this program and exactly `VaultAuthority::LEN` bytes
    // before any of its contents are trusted.
    if !vault_authority.owned_by(program_id) {
        return Err(VaultError::InvalidAccountOwner.into());
    }
    if vault_authority.data_len() != VaultAuthority::LEN {
        return Err(VaultError::InvalidAccountData.into());
    }
    let state = {
        let data = vault_authority.try_borrow()?;
        VaultAuthority::unpack(&data)?
    };

    // Re-derive the PDA from the *stored* bump (`derive_address` with `Some(bump)`, which
    // internally appends the bump as an extra seed and hashes directly -- no syscall, and no
    // `derive_program_address` bump SEARCH) and compare against the account actually passed in.
    let expected_vault_authority = Address::derive_address(
        &[VAULT_AUTHORITY_SEED, state.mint.as_ref()],
        Some(state.bump),
        program_id,
    );
    if vault_authority.address() != &expected_vault_authority {
        return Err(VaultError::InvalidVaultAuthority.into());
    }

    // The vault token account's address, as recorded in `VaultAuthority` state, must match what
    // was actually passed in.
    if vault_token_account.address() != &state.vault_token_account {
        return Err(VaultError::InvalidVaultTokenAccount.into());
    }
    if mint.address() != &state.mint {
        return Err(VaultError::MintMismatch.into());
    }
    if !vault_token_account.owned_by(&TOKEN_PROGRAM_ID) {
        return Err(VaultError::InvalidAccountOwner.into());
    }
    if !depositor_token_account.owned_by(&TOKEN_PROGRAM_ID) {
        return Err(VaultError::InvalidAccountOwner.into());
    }
    if read_token_account_mint(depositor_token_account)? != state.mint {
        return Err(VaultError::MintMismatch.into());
    }

    let decimals = read_mint_decimals(mint)?;

    TransferChecked::<&AccountView>::new(
        depositor_token_account,
        mint,
        vault_token_account,
        depositor,
        amount,
        decimals,
    )
    .invoke()
}
