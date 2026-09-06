//! Oracle abstraction (`docs/oracle-design.md`, ADR-0008). [`PriceSource`] is the trait every
//! oracle kind implements; [`require_valid_price`] is the single, shared enforcement point for
//! checks O-1..O-11 so no instruction improvises its own price validation. v1 has exactly one
//! implementer, [`pyth::PythPull`] — there is no `Mock` variant and there never will be
//! (ADR-0008 §5).
//!
//! **Call this before any state write** (INV-ORA-07): every call site in this program invokes
//! `require_valid_price` as the very first fallible operation in the handler, before
//! `accrue_mut`/`accrue_view` or any account mutation, so a failed oracle check leaves nothing
//! modified by construction.

pub mod pyth;

use crate::error::AegisError;
use crate::state::Market;
use anchor_lang::prelude::*;

/// Conservative price band for one asset, normalized to WAD (1e18) USD per whole token
/// (`oracle-design.md` §1). `lo` values COLLATERAL (rounded down); `hi` values DEBT (rounded up).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PriceBand {
    pub lo: u128,
    pub hi: u128,
    pub published_at: i64,
}

/// One oracle implementer's contract (`oracle-design.md` §1): returns a **validated** band, or a
/// specific error. Never a "best effort" price.
///
/// Takes a raw `AccountInfo`, not e.g. `Account<'info, PriceUpdateV2>` — so a v2 non-Pyth
/// implementer (Switchboard, a redundant-median composite) would not need to depend on Pyth's
/// account types at all, and so every check (including the account owner, O-1) is performed
/// explicitly by the implementer rather than delegated to a particular Anchor account wrapper.
pub trait PriceSource {
    fn read_price(
        account: &AccountInfo,
        expected_feed_id: &[u8; 32],
        now: i64,
        max_age_secs: u32,
        max_conf_bps: u16,
    ) -> Result<PriceBand>;
}

/// `Market.oracle_kind` discriminant for the Pyth pull oracle (`account-model.md` §4) — the only
/// value v1 accepts.
pub const ORACLE_KIND_PYTH_PULL: u8 = 0;

/// The single, shared enforcement point for every priced instruction (`oracle-design.md` §2,
/// INV-ORA-01). Implements O-1..O-11 in full:
/// - O-11 (the two price accounts must be distinct) is checked here, since it needs both accounts
///   at once — no single `read_price` call can see the other feed. This does not rely solely on
///   the caller passing two differently-named account arguments: it compares the actual account
///   keys.
/// - Dispatch by `market.oracle_kind` (`oracle-design.md` §1) rejects anything other than
///   [`ORACLE_KIND_PYTH_PULL`] rather than silently defaulting to a particular provider.
/// - O-1..O-10 are checked inside the dispatched `PriceSource::read_price` call, once per asset.
pub fn require_valid_price(
    market: &Market,
    collateral_price_update: &AccountInfo,
    loan_price_update: &AccountInfo,
    now: i64,
) -> Result<(PriceBand, PriceBand)> {
    // O-11: the two price accounts must be physically distinct accounts.
    require_keys_neq!(
        collateral_price_update.key(),
        loan_price_update.key(),
        AegisError::OracleDuplicatePriceAccounts
    );

    require_eq!(
        market.oracle_kind,
        ORACLE_KIND_PYTH_PULL,
        AegisError::OracleUnsupportedKind
    );

    let collateral_band = pyth::PythPull::read_price(
        collateral_price_update,
        &market.collateral_feed_id,
        now,
        market.max_price_age_secs,
        market.max_conf_bps,
    )?;
    let loan_band = pyth::PythPull::read_price(
        loan_price_update,
        &market.loan_feed_id,
        now,
        market.max_price_age_secs,
        market.max_conf_bps,
    )?;

    Ok((collateral_band, loan_band))
}
