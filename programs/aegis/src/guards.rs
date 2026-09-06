//! Small, shared preconditions reused across instruction handlers (architecture.md §2). Pause
//! guards and relation helpers grow here as later phases add pausable instructions; Phase 2 has
//! none (`initialize_protocol`, `create_market` and `init_position` are all unpausable), so this
//! module currently holds only the one check both admin-facing args and `init_position` need.

use crate::error::AegisError;
use anchor_lang::prelude::*;

/// Rejects the default `Pubkey` — used wherever an argument or account must name a real key
/// (`instruction-catalogue.md` #1's `guardian`/`fee_recipient` preconditions, #9's `owner`).
pub fn require_non_default_pubkey(key: Pubkey, err: AegisError) -> Result<()> {
    require_keys_neq!(key, Pubkey::default(), err);
    Ok(())
}

/// **Exactly one** of `assets`/`shares` must be nonzero — the shared guard for `supply`,
/// `withdraw`, `borrow` and `repay` (`economic-model.md` E-21..E-23, `instruction-catalogue.md`
/// §12-15). Both zero is rejected as `ZeroAmount`; both nonzero is rejected as `InconsistentInput`
/// — two distinct, specific errors, never one generic "invalid input".
pub fn require_exactly_one_amount(assets: u64, shares: u128) -> Result<()> {
    match (assets == 0, shares == 0) {
        (true, true) => Err(error!(AegisError::ZeroAmount)),
        (false, false) => Err(error!(AegisError::InconsistentInput)),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // U-GUARD-01 / E-23: both assets and shares zero must be rejected as ZeroAmount.
    #[test]
    fn guard_01_both_zero_is_rejected() {
        let err = require_exactly_one_amount(0, 0).unwrap_err();
        assert_eq!(err, anchor_lang::error::Error::from(AegisError::ZeroAmount));
    }

    // U-GUARD-02 / E-22: both assets and shares nonzero must be rejected as InconsistentInput.
    #[test]
    fn guard_02_both_nonzero_is_rejected() {
        let err = require_exactly_one_amount(1, 1).unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::InconsistentInput)
        );
    }

    // U-GUARD-03: exactly one nonzero (either form) must be accepted.
    #[test]
    fn guard_03_exactly_one_nonzero_is_accepted() {
        assert!(require_exactly_one_amount(1, 0).is_ok());
        assert!(require_exactly_one_amount(0, 1).is_ok());
    }
}
