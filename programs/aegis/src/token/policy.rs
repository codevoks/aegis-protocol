//! Token-2022 extension policy engine (`docs/token-compatibility.md`).
//!
//! **Positive allowlist.** Every extension type found on a mint is matched explicitly; anything
//! not matched — including any future extension Token-2022 ships tomorrow — is rejected by the
//! catch-all arm. This is deliberate: a blocklist would silently accept an unrecognized
//! extension, which is exactly the failure mode `A-TOK-05` exists to catch.
//!
//! A classic SPL Token mint has no TLV data at all, so `StateWithExtensions::<Mint>::unpack`
//! trivially returns an empty extension list for it — the same code path handles both token
//! programs uniformly, per `token-compatibility.md` §6 step 2.

use crate::error::AegisError;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::spl_token_2022::extension::{
    BaseStateWithExtensions, ExtensionType, StateWithExtensions,
};
use anchor_spl::token_interface::spl_token_2022::state::Mint as SplMint;

/// Which side of a market a mint is being validated for. The loan asset is held to a stricter
/// standard than collateral (`token-compatibility.md` §1) because loan-asset accounting drives
/// share pricing for every lender in the market, while collateral accounting is per-position.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MintRole {
    Collateral,
    Loan,
}

/// The result of evaluating one mint's extension inventory against the policy.
pub struct MintPolicyOutcome {
    pub has_freeze_authority: bool,
    /// Only ever `true` for `MintRole::Collateral` — `TransferFeeConfig` is rejected outright as
    /// the loan asset (`token-compatibility.md` §4), so a `Loan`-role outcome can never set this.
    pub has_transfer_fee: bool,
    /// The mint's full Token-2022 extension inventory, for the `MarketCreated` audit record
    /// (`token-compatibility.md` §6 step 7). Empty for a classic SPL Token mint.
    pub extension_types: Vec<ExtensionType>,
}

/// Parses `mint_info`'s TLV extension list and enforces the positive allowlist and per-role
/// restrictions from `token-compatibility.md` §2 and §4. Does **not** check the mint's owning
/// program against a pinned token program — callers must do that separately (T-11).
pub fn evaluate_mint(mint_info: &AccountInfo, role: MintRole) -> Result<MintPolicyOutcome> {
    let data = mint_info.try_borrow_data()?;
    let state = StateWithExtensions::<SplMint>::unpack(&data)
        .map_err(|_| error!(AegisError::InvalidMintAccountData))?;

    let extension_types = state
        .get_extension_types()
        .map_err(|_| error!(AegisError::InvalidMintAccountData))?;

    let mut has_transfer_fee = false;
    for extension_type in &extension_types {
        match extension_type {
            // Tier A — display/organizational metadata only, or UI-scaling that never touches
            // raw base-unit accounting (token-compatibility.md §2).
            ExtensionType::MetadataPointer
            | ExtensionType::TokenMetadata
            | ExtensionType::GroupPointer
            | ExtensionType::TokenGroup
            | ExtensionType::GroupMemberPointer
            | ExtensionType::TokenGroupMember
            | ExtensionType::InterestBearingConfig
            | ExtensionType::ScaledUiAmount => {}
            // Tier B — alters transfer amounts, but measured-delta accounting handles it
            // correctly. Collateral only; rejected as the loan asset (token-compatibility.md §4).
            ExtensionType::TransferFeeConfig => {
                if role == MintRole::Loan {
                    return Err(error!(AegisError::TransferFeeNotAllowedForLoanAsset));
                }
                has_transfer_fee = true;
            }
            // Tier C (TransferHook, PermanentDelegate, MintCloseAuthority,
            // DefaultAccountState, Pausable, NonTransferable, ConfidentialTransfer*, ...) and any
            // extension not on this list at all: rejected. Fail closed on the unknown case.
            _ => return Err(error!(AegisError::UnsupportedTokenExtension)),
        }
    }

    let has_freeze_authority = state.base.freeze_authority.is_some();

    Ok(MintPolicyOutcome {
        has_freeze_authority,
        has_transfer_fee,
        extension_types,
    })
}

/// The Token-2022 account-level extensions Aegis's own vault must carry: whatever the mint's own
/// extensions require on any account holding it (e.g. `TransferFeeAmount` for a
/// `TransferFeeConfig` mint), plus `ImmutableOwner`, which Aegis sets on every Token-2022 vault it
/// creates (`token-compatibility.md` §2, §5.4).
pub fn vault_account_extensions(mint_extension_types: &[ExtensionType]) -> Vec<ExtensionType> {
    let mut extensions = ExtensionType::get_required_init_account_extensions(mint_extension_types);
    if !extensions.contains(&ExtensionType::ImmutableOwner) {
        extensions.push(ExtensionType::ImmutableOwner);
    }
    extensions
}

#[cfg(test)]
mod tests {
    use super::*;

    // U-LIFE-02-adjacent sanity check: the extension→account-extension mapping is deterministic
    // and always includes ImmutableOwner, even for a mint with no extensions at all.
    #[test]
    fn vault_extensions_always_include_immutable_owner() {
        let extensions = vault_account_extensions(&[]);
        assert_eq!(extensions, vec![ExtensionType::ImmutableOwner]);
    }

    #[test]
    fn transfer_fee_mint_requires_transfer_fee_amount_and_immutable_owner() {
        let extensions = vault_account_extensions(&[ExtensionType::TransferFeeConfig]);
        assert!(extensions.contains(&ExtensionType::TransferFeeAmount));
        assert!(extensions.contains(&ExtensionType::ImmutableOwner));
        assert_eq!(extensions.len(), 2);
    }
}
