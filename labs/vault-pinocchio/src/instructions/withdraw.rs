//! `Withdraw` (tag 2, amount `u64`): `[owner (signer, must equal vault_authority.owner),
//! vault_authority (readonly, PDA re-verified from the stored bump), vault_token_account
//! (writable), recipient_token_account (writable), mint (readonly), token_program (readonly)]`.
//! Signed CPI: only `vault_authority`'s own PDA signature can move tokens out of the vault.

use pinocchio::{
    account::AccountView,
    address::Address,
    cpi::{Seed, Signer},
    ProgramResult,
};
use pinocchio_token::{instructions::TransferChecked, ID as TOKEN_PROGRAM_ID};

use crate::{
    error::VaultError,
    spl::{read_mint_decimals, read_token_account_mint},
    state::VaultAuthority,
    VAULT_AUTHORITY_SEED,
};

pub fn process(program_id: &Address, accounts: &mut [AccountView], amount: u64) -> ProgramResult {
    let [owner, vault_authority, vault_token_account, recipient_token_account, mint, token_program] =
        accounts
    else {
        return Err(VaultError::NotEnoughAccountKeys.into());
    };

    if amount == 0 {
        return Err(VaultError::ZeroAmount.into());
    }
    if !owner.is_signer() {
        return Err(VaultError::MissingRequiredSignature.into());
    }
    if token_program.address() != &TOKEN_PROGRAM_ID {
        return Err(VaultError::InvalidTokenProgram.into());
    }

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

    // The signer must be exactly the vault's recorded owner -- this is the one check that
    // stands between an attacker and every token in the vault.
    if owner.address() != &state.owner {
        return Err(VaultError::OwnerMismatch.into());
    }

    // Re-derive the PDA from the *stored* bump via `derive_address(seeds, Some(bump), ..)` --
    // never a bump SEARCH (`derive_program_address`) on this hot path (phase-11-performance.md's
    // own guidance: the bump was already found once, at `Initialize`, and is now trusted state).
    let expected_vault_authority = Address::derive_address(
        &[VAULT_AUTHORITY_SEED, state.mint.as_ref()],
        Some(state.bump),
        program_id,
    );
    if vault_authority.address() != &expected_vault_authority {
        return Err(VaultError::InvalidVaultAuthority.into());
    }

    if vault_token_account.address() != &state.vault_token_account {
        return Err(VaultError::InvalidVaultTokenAccount.into());
    }
    if mint.address() != &state.mint {
        return Err(VaultError::MintMismatch.into());
    }
    if !vault_token_account.owned_by(&TOKEN_PROGRAM_ID) {
        return Err(VaultError::InvalidAccountOwner.into());
    }
    if !recipient_token_account.owned_by(&TOKEN_PROGRAM_ID) {
        return Err(VaultError::InvalidAccountOwner.into());
    }
    if read_token_account_mint(recipient_token_account)? != state.mint {
        return Err(VaultError::MintMismatch.into());
    }

    let decimals = read_mint_decimals(mint)?;

    let bump_seed = [state.bump];
    let seeds = [
        Seed::from(VAULT_AUTHORITY_SEED),
        Seed::from(state.mint.as_ref()),
        Seed::from(&bump_seed[..]),
    ];
    let signer = Signer::from(&seeds);

    TransferChecked::<&AccountView>::new(
        vault_token_account,
        mint,
        recipient_token_account,
        vault_authority,
        amount,
        decimals,
    )
    .invoke_signed(&[signer])
}
