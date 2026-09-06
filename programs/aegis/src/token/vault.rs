//! Vault (custody token account) creation for `create_market`.
//!
//! Deliberately **not** Anchor's `#[account(init, token::mint = ..., ...)]` sugar: that sugar
//! sizes a Token-2022 account correctly for whatever extensions the *mint* requires, but it has
//! no way to also request `ImmutableOwner` — an extension Aegis chooses to add to its own vaults
//! that the mint does not require (`token-compatibility.md` §2, §5.4). Creating the account by
//! hand gives full control over that extra step while reusing the same sizing algorithm Anchor
//! itself uses (`ExtensionType::try_calculate_account_len`), so vault length is never hardcoded.

use crate::error::AegisError;
use crate::token::policy::vault_account_extensions;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::program_pack::Pack;
use anchor_lang::system_program::{create_account, CreateAccount};
use anchor_spl::token_2022;
use anchor_spl::token_interface::spl_token_2022::extension::ExtensionType;
use anchor_spl::token_interface::spl_token_2022::state::Account as SplTokenAccount;
use anchor_spl::token_interface::{
    initialize_account3, initialize_immutable_owner, InitializeAccount3, InitializeImmutableOwner,
};

/// The exact account length for a vault of `token_program_id` holding a mint with
/// `mint_extension_types`. Never hardcoded to 165 (`token-compatibility.md` §5.4, §9 of the phase
/// spec): legacy SPL Token accounts are always `SplTokenAccount::LEN` (165, no extensions
/// possible), but a Token-2022 vault's length depends on what the mint requires plus the
/// `ImmutableOwner` extension Aegis always adds.
pub fn vault_account_space(
    token_program_id: &Pubkey,
    mint_extension_types: &[ExtensionType],
) -> Result<usize> {
    if *token_program_id == token_2022::ID {
        let extensions = vault_account_extensions(mint_extension_types);
        ExtensionType::try_calculate_account_len::<SplTokenAccount>(&extensions)
            .map_err(|_| error!(AegisError::InvalidMintAccountData))
    } else {
        Ok(SplTokenAccount::LEN)
    }
}

/// Creates and initializes one vault token account at a PDA this program controls.
///
/// Order matters for Token-2022 (SPL Token-2022 design constraint): the account must be
/// allocated at its final size and assigned to the token program *before* any extension-init
/// instruction, and `InitializeAccount3` — which marks the account `Initialized` — must be the
/// last step. `ImmutableOwner` needs an explicit init call; any extension the mint itself
/// requires on holder accounts (e.g. `TransferFeeAmount`) is populated automatically by the
/// token program during `InitializeAccount3` provided the account was sized for it.
#[allow(clippy::too_many_arguments)]
pub fn create_vault<'info>(
    vault: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    mint_extension_types: &[ExtensionType],
    vault_signer_seeds: &[&[u8]],
) -> Result<()> {
    let token_program_id = token_program.key();
    let space = vault_account_space(&token_program_id, mint_extension_types)?;
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(space);

    create_account(
        CpiContext::new(
            system_program.key(),
            CreateAccount {
                from: payer.clone(),
                to: vault.clone(),
            },
        )
        .with_signer(&[vault_signer_seeds]),
        lamports,
        space as u64,
        &token_program_id,
    )?;

    if token_program_id == token_2022::ID {
        initialize_immutable_owner(CpiContext::new(
            token_program.key(),
            InitializeImmutableOwner {
                account: vault.clone(),
            },
        ))?;
    }

    initialize_account3(CpiContext::new(
        token_program.key(),
        InitializeAccount3 {
            account: vault.clone(),
            mint: mint.clone(),
            authority: authority.clone(),
        },
    ))?;

    Ok(())
}
