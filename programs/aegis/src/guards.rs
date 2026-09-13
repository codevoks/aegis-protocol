//! Small, shared preconditions reused across instruction handlers (architecture.md §2). Pause
//! guards and relation helpers grow here as later phases add pausable instructions; Phase 2 has
//! none (`initialize_protocol`, `create_market` and `init_position` are all unpausable), so this
//! module currently holds only the one check both admin-facing args and `init_position` need.
//!
//! Phase 12 adds [`require_pause_bit_clear`] — the one function `supply`, `withdraw`, `borrow`,
//! `withdraw_collateral` and `liquidate` call. **`repay`, `deposit_collateral`, `absorb_bad_debt`
//! and `close_position` do not import this module's pause guard at all** (INV-ADM-04,
//! `docs/phases/phase-12-governance.md` §"Security work"): the load-bearing property is that these
//! four instructions have no code path that could ever consult a pause bit, not that they consult
//! one and are excepted — an exception is a line a future change could delete.

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

/// As [`require_exactly_one_amount`], for the two `u64` inputs of `liquidate(repay_assets,
/// seize_collateral)` (`instruction-catalogue.md` §17).
pub fn require_exactly_one_u64(a: u64, b: u64) -> Result<()> {
    match (a == 0, b == 0) {
        (true, true) => Err(error!(AegisError::ZeroAmount)),
        (false, false) => Err(error!(AegisError::InconsistentInput)),
        _ => Ok(()),
    }
}

/// Phase 12, INV-ADM-03/INV-BOR-04/etc.: `bit` (one of `constants::PAUSE_*`) must be clear in
/// **both** `protocol_paused` and `market_paused` — either one setting it is enough to block the
/// operation. The one shared guard `supply`, `withdraw`, `borrow`, `withdraw_collateral` and
/// `liquidate` each call with their own bit and their own specific error, so every paused-path
/// test can assert on an exact, per-instruction error rather than one generic "paused" code.
pub fn require_pause_bit_clear(
    protocol_paused: u8,
    market_paused: u8,
    bit: u8,
    err: AegisError,
) -> Result<()> {
    // Not `require!`: that macro's error argument must be a path (e.g. `AegisError::Variant`),
    // not a value already held in a variable -- this function's whole point is picking the error
    // at runtime, per call site, so it builds the `Result` directly instead.
    if protocol_paused & bit != 0 || market_paused & bit != 0 {
        return Err(err.into());
    }
    Ok(())
}

/// Phase 12: `set_protocol_pause`/`set_market_pause` share this one authorization+bit rule.
/// `signer` must be `admin` or `guardian`; if `signer == admin`, any bit combination within
/// `PAUSE_ALL_BITS` is accepted (the admin may set or clear freely, INV-ADM-04's table). If
/// `signer == guardian` (and is not simultaneously `admin`), `new_paused` must be a **superset**
/// of `old_paused` — no bit the guardian did not already see set may be cleared (`A-AUTH-04`,
/// INV-AUTH-04). Undefined bits are rejected unconditionally, before the admin/guardian branch,
/// so `A-ADM-05` fails the same way regardless of who is calling.
pub fn require_authorized_pause_change(
    signer: Pubkey,
    admin: Pubkey,
    guardian: Pubkey,
    old_paused: u8,
    new_paused: u8,
) -> Result<()> {
    require!(
        new_paused & !crate::constants::PAUSE_ALL_BITS == 0,
        AegisError::InvalidPauseBits
    );

    if signer == admin {
        return Ok(());
    }
    require_keys_eq!(signer, guardian, AegisError::NotAdminOrGuardian);
    // Every bit already set must stay set; the guardian may only add bits, never remove one
    // (`new_paused & old_paused == old_paused` is exactly "old_paused is a subset of new_paused").
    require!(
        new_paused & old_paused == old_paused,
        AegisError::GuardianCannotClearPause
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{PAUSE_ALL_BITS, PAUSE_BORROW, PAUSE_SUPPLY};

    fn key(byte: u8) -> Pubkey {
        Pubkey::new_from_array([byte; 32])
    }

    #[test]
    fn pause_bit_clear_in_both_accounts_is_ok() {
        assert!(require_pause_bit_clear(0, 0, PAUSE_SUPPLY, AegisError::OperationPaused).is_ok());
        assert!(
            require_pause_bit_clear(PAUSE_BORROW, 0, PAUSE_SUPPLY, AegisError::OperationPaused)
                .is_ok(),
            "an unrelated bit set elsewhere must not block this bit's check"
        );
    }

    #[test]
    fn pause_bit_set_on_protocol_blocks_with_the_caller_supplied_error() {
        let err =
            require_pause_bit_clear(PAUSE_SUPPLY, 0, PAUSE_SUPPLY, AegisError::OperationPaused)
                .unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::OperationPaused)
        );
    }

    #[test]
    fn pause_bit_set_on_market_alone_also_blocks() {
        let err =
            require_pause_bit_clear(0, PAUSE_SUPPLY, PAUSE_SUPPLY, AegisError::OperationPaused)
                .unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::OperationPaused)
        );
    }

    // A-ADM-05 (component): a bit outside PAUSE_ALL_BITS must be rejected regardless of caller.
    #[test]
    fn undefined_pause_bit_is_rejected_for_admin_and_guardian() {
        let admin = key(1);
        let guardian = key(2);
        let undefined_bit = 0b1000_0000u8;

        let err =
            require_authorized_pause_change(admin, admin, guardian, 0, undefined_bit).unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::InvalidPauseBits)
        );

        let err = require_authorized_pause_change(guardian, admin, guardian, 0, undefined_bit)
            .unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::InvalidPauseBits)
        );
    }

    // Admin may set AND clear freely within the defined bits.
    #[test]
    fn admin_may_set_and_clear_freely() {
        let admin = key(1);
        let guardian = key(2);
        assert!(require_authorized_pause_change(admin, admin, guardian, 0, PAUSE_ALL_BITS).is_ok());
        assert!(require_authorized_pause_change(admin, admin, guardian, PAUSE_ALL_BITS, 0).is_ok());
    }

    // A-AUTH-04: the guardian may set new bits, but clearing even one already-set bit must fail
    // with the exact GuardianCannotClearPause error -- never a generic err.
    #[test]
    fn a_auth_04_guardian_can_set_but_not_clear() {
        let admin = key(1);
        let guardian = key(2);

        // Setting an additional bit (superset of old_paused) succeeds.
        assert!(require_authorized_pause_change(
            guardian,
            admin,
            guardian,
            PAUSE_SUPPLY,
            PAUSE_ALL_BITS
        )
        .is_ok());

        // Clearing one previously-set bit must fail with the specific error.
        let err = require_authorized_pause_change(
            guardian,
            admin,
            guardian,
            PAUSE_ALL_BITS,
            PAUSE_SUPPLY,
        )
        .unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::GuardianCannotClearPause)
        );
    }

    // A random third party is neither admin nor guardian.
    #[test]
    fn random_signer_is_rejected() {
        let admin = key(1);
        let guardian = key(2);
        let attacker = key(3);
        let err = require_authorized_pause_change(attacker, admin, guardian, 0, PAUSE_SUPPLY)
            .unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::NotAdminOrGuardian)
        );
    }

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

    // `liquidate(repay_assets, seize_collateral)`'s own exactly-one-of guard.
    #[test]
    fn require_exactly_one_u64_rejects_both_zero_and_both_nonzero() {
        assert_eq!(
            require_exactly_one_u64(0, 0).unwrap_err(),
            anchor_lang::error::Error::from(AegisError::ZeroAmount)
        );
        assert_eq!(
            require_exactly_one_u64(1, 1).unwrap_err(),
            anchor_lang::error::Error::from(AegisError::InconsistentInput)
        );
        assert!(require_exactly_one_u64(1, 0).is_ok());
        assert!(require_exactly_one_u64(0, 1).is_ok());
    }
}
