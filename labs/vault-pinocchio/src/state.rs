//! `VaultAuthority`: the state PDA, laid out and (de)serialized by hand -- no framework macro,
//! which is the entire point of this lab (ADR-0003). 97 bytes: `owner` (32) + `mint` (32) +
//! `vault_token_account` (32) + `bump` (1). No discriminator: Anchor's 8-byte discriminator is a
//! framework convenience for safe downcasting across account *types*; a hand-rolled program has
//! exactly one account type at this seed and already proves it via the owner-program + PDA
//! checks the caller performs before ever calling `unpack`, so a discriminator would be pure
//! overhead with nothing to disambiguate.
//!
//! `unpack`/`pack` copy through plain byte slices rather than pointer-casting the account's raw
//! data buffer in place. A true zero-copy cast would need `Address` (a `#[repr(transparent)]`
//! `[u8; 32]`) validated for alignment before reinterpreting the buffer, which buys nothing here:
//! 97 bytes is copied at most once per instruction, and AGENTS.md §17 forbids optimizing without
//! a measurement that says it is worth it.

use pinocchio::address::Address;

use crate::error::VaultError;

pub struct VaultAuthority {
    pub owner: Address,
    pub mint: Address,
    pub vault_token_account: Address,
    pub bump: u8,
}

impl VaultAuthority {
    pub const LEN: usize = 32 + 32 + 32 + 1;

    pub fn pack(&self, dst: &mut [u8]) -> Result<(), VaultError> {
        if dst.len() != Self::LEN {
            return Err(VaultError::InvalidAccountData);
        }
        dst[0..32].copy_from_slice(self.owner.as_ref());
        dst[32..64].copy_from_slice(self.mint.as_ref());
        dst[64..96].copy_from_slice(self.vault_token_account.as_ref());
        dst[96] = self.bump;
        Ok(())
    }

    pub fn unpack(src: &[u8]) -> Result<Self, VaultError> {
        if src.len() != Self::LEN {
            return Err(VaultError::InvalidAccountData);
        }
        let owner = Address::new_from_array(
            src[0..32]
                .try_into()
                .map_err(|_| VaultError::InvalidAccountData)?,
        );
        let mint = Address::new_from_array(
            src[32..64]
                .try_into()
                .map_err(|_| VaultError::InvalidAccountData)?,
        );
        let vault_token_account = Address::new_from_array(
            src[64..96]
                .try_into()
                .map_err(|_| VaultError::InvalidAccountData)?,
        );
        let bump = src[96];
        Ok(Self {
            owner,
            mint,
            vault_token_account,
            bump,
        })
    }
}
