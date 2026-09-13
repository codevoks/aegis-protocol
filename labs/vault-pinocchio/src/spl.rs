//! Manual, minimal reads of the two raw SPL Token (classic) account layouts this lab touches.
//! No `spl-token`/`spl-token-interface` dependency in the on-chain crate itself (that would
//! defeat the "zero-dependency-by-design" point of the lab) -- just the two fixed byte offsets
//! this primitive actually needs, read directly out of the account's own data buffer.

use pinocchio::{account::AccountView, address::Address, error::ProgramError};

use crate::error::VaultError;

/// SPL Token `Account` layout: `mint` (32) + `owner` (32) + `amount` (8) + ... = 165 bytes total.
/// The mint is always the first 32 bytes, regardless of anything else in the layout.
pub fn read_token_account_mint(account: &AccountView) -> Result<Address, ProgramError> {
    let data = account.try_borrow()?;
    let bytes: [u8; 32] = data
        .get(0..32)
        .ok_or(VaultError::InvalidAccountData)?
        .try_into()
        .map_err(|_| VaultError::InvalidAccountData)?;
    Ok(Address::new_from_array(bytes))
}

/// SPL Token `Mint` layout: `mint_authority: COption<Pubkey>` (4-byte tag + 32-byte value) +
/// `supply: u64` (8) + `decimals: u8` -- so `decimals` sits at byte offset 4 + 32 + 8 = 44.
pub fn read_mint_decimals(mint: &AccountView) -> Result<u8, ProgramError> {
    let data = mint.try_borrow()?;
    data.get(44)
        .copied()
        .ok_or_else(|| ProgramError::from(VaultError::InvalidAccountData))
}
