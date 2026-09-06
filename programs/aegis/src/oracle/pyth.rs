//! `PythPull` — the sole [`PriceSource`] implementer (ADR-0008 §1). Uses the real
//! `pyth-solana-receiver-sdk` 2.0.0 (`docs/ecosystem-research.md` RV-3/RV-4, resolved
//! 2026-09-06) to deserialize and validate a `PriceUpdateV2` account.
//!
//! Reading a price update is an **account read, not a CPI** — the Pyth receiver program
//! (`rec5EKMGg6MxZYaMdyBfgwp4d5rB9T1VQH5pJv5LtFJ`) never needs to be deployed or invoked; only its
//! program ID needs to match the account's `owner` field.

use super::{PriceBand, PriceSource};
use crate::constants::MAX_FUTURE_PRICE_SKEW_SECS;
use crate::error::AegisError;
use anchor_lang::prelude::*;
use pyth_solana_receiver_sdk::error::GetPriceError as PythGetPriceError;
use pyth_solana_receiver_sdk::price_update::{PriceUpdateV2, VerificationLevel};

pub struct PythPull;

impl PriceSource for PythPull {
    fn read_price(
        account: &AccountInfo,
        expected_feed_id: &[u8; 32],
        now: i64,
        max_age_secs: u32,
        max_conf_bps: u16,
    ) -> Result<PriceBand> {
        // O-1: explicit owner check. `Account<'info, PriceUpdateV2>` would perform this
        // automatically via its `Owner` impl, but `PriceSource::read_price`'s frozen signature
        // takes a raw `AccountInfo` (`oracle-design.md` §1), so this repeats the check by hand --
        // deliberately, so it survives a future refactor away from any particular account
        // wrapper (`docs/phases/phase-05-oracle.md` "Explicit owner validation").
        require_keys_eq!(
            *account.owner,
            pyth_solana_receiver_sdk::ID,
            AegisError::OracleAccountOwnerMismatch
        );

        // O-2: discriminator check + full Borsh deserialize of the real SDK account type. A
        // malformed or wrong-discriminator buffer is rejected with a clean Aegis error rather
        // than an opaque deserialization panic.
        let price_update = {
            let data = account
                .try_borrow_data()
                .map_err(|_| error!(AegisError::OracleAccountInvalidData))?;
            let mut slice: &[u8] = &data;
            PriceUpdateV2::try_deserialize(&mut slice)
                .map_err(|_| error!(AegisError::OracleAccountInvalidData))?
        };

        // O-4, explicit: re-checked below via the SDK's own `get_price_no_older_than`, but
        // asserted here by hand too -- oracle-design.md's implementation note calls this "easy to
        // omit and rarely tested," and the spec wants this defense to survive a future SDK or
        // wrapper change (`docs/phases/phase-05-oracle.md` "VerificationLevel").
        require!(
            price_update.verification_level == VerificationLevel::Full,
            AegisError::OracleVerificationLevelNotFull
        );

        // O-3, explicit, for the same reason.
        require!(
            price_update.price_message.feed_id == *expected_feed_id,
            AegisError::OracleFeedMismatch
        );

        // O-3 + O-4 + O-5 together, via the SDK's own recommended API
        // (docs.pyth.network best practices; `oracle-design.md` implementation notes: "Use
        // get_price_no_older_than(&Clock, max_age, &feed_id) -- it performs O-3 and O-5
        // together"). `Clock { unix_timestamp: now, ..Clock::default() }` mirrors the SDK's own
        // test suite's pattern for driving this call under a controlled clock (only
        // `unix_timestamp` is read by this method).
        let clock = Clock {
            unix_timestamp: now,
            ..Clock::default()
        };
        let price = price_update
            .get_price_no_older_than(&clock, max_age_secs as u64, expected_feed_id)
            .map_err(|e| match e {
                PythGetPriceError::PriceTooOld => error!(AegisError::OraclePriceStale),
                PythGetPriceError::MismatchedFeedId => error!(AegisError::OracleFeedMismatch),
                PythGetPriceError::InsufficientVerificationLevel => {
                    error!(AegisError::OracleVerificationLevelNotFull)
                }
                _ => error!(AegisError::OracleAccountInvalidData),
            })?;

        // O-6: reject a publish_time unacceptably far in the future -- a feed/clock anomaly, not
        // covered by the SDK's own staleness check (which only bounds age from below `now`).
        require!(
            price.publish_time <= now.saturating_add(MAX_FUTURE_PRICE_SKEW_SECS),
            AegisError::OraclePriceInFuture
        );

        // O-7, O-8, O-9, O-10: price positivity, confidence bound, sanity bounds, and
        // exponent-driven overflow safety -- pure numeric checks, in aegis-math (Tier 1,
        // `AGENTS.md` §9).
        let band = aegis_math::conservative_price_band(
            price.price,
            price.conf,
            price.exponent,
            max_conf_bps,
        )
        .map_err(AegisError::from)?;

        Ok(PriceBand {
            lo: band.lo,
            hi: band.hi,
            published_at: price.publish_time,
        })
    }
}
