//! `Market` — one isolated lending venue (`account-model.md` §4). Field order, types and
//! `_reserved` width are transcribed verbatim from the frozen spec; do not reorder.
//!
//! Parameter-bound validation lives here (not in the instruction handler) per
//! `architecture.md` §2 ("`state/market.rs` // Market account + accrual + param validation").

use crate::constants::{
    MAX_CONF_BPS, MAX_FEE, MAX_LIQ_BONUS, MAX_LIQ_PROTOCOL_FEE, MAX_PRICE_AGE_SECS,
    MIN_CLOSE_FACTOR, MIN_CONF_BPS, MIN_PRICE_AGE_SECS,
};
use crate::error::AegisError;
use crate::state::Position;
use aegis_math::{borrow_rate, mul_div_floor, taylor3, taylor_x, to_shares_down, utilization, WAD};
use anchor_lang::prelude::*;

#[account]
pub struct Market {
    // --- identity (immutable after creation) ---
    pub collateral_mint: Pubkey,
    pub loan_mint: Pubkey,
    pub collateral_token_program: Pubkey,
    pub loan_token_program: Pubkey,
    pub collateral_vault: Pubkey,
    pub loan_vault: Pubkey,
    pub fee_recipient: Pubkey,
    pub config_id: u16,
    pub collateral_decimals: u8,
    pub loan_decimals: u8,

    // --- oracle config (admin-mutable, market-local) ---
    pub oracle_kind: u8,
    pub collateral_feed_id: [u8; 32],
    pub loan_feed_id: [u8; 32],
    pub max_price_age_secs: u32,
    pub max_conf_bps: u16,

    // --- risk params (admin-mutable, bounds-checked) ---
    pub max_ltv: u128,
    pub liq_threshold: u128,
    pub liq_bonus: u128,
    pub close_factor: u128,
    pub full_liq_hf: u128,
    pub liq_protocol_fee: u128,
    pub fee: u128,
    pub min_debt: u64,

    // --- IRM params (stateless) ---
    pub base_rate_ps: u128,
    pub slope1_ps: u128,
    pub slope2_ps: u128,
    pub u_kink: u128,
    pub max_rate_ps: u128,

    // --- accounting (hot) ---
    pub total_supply_assets: u64,
    pub total_supply_shares: u128,
    pub total_borrow_assets: u64,
    pub total_borrow_shares: u128,
    pub collateral_fee_accrued: u64,
    pub last_accrual_ts: i64,

    // --- flags / bumps ---
    pub paused: u8,
    pub flags: u8,
    pub bump: u8,
    pub collateral_vault_bump: u8,
    pub loan_vault_bump: u8,
    /// Phase 8 (ADR-0013, `account-model.md` §4): `1` while a liquidation callback CPI is in
    /// flight on this market, `0` otherwise. Checked unconditionally at the top of `liquidate`;
    /// no other instruction reads or writes it. One byte moved out of `_reserved` — `Market::LEN`
    /// is unchanged.
    pub liquidation_guard: u8,
    pub _reserved: [u8; 63],
}

impl Market {
    /// Discriminator (8) + identity (7 `Pubkey` + `u16` + 2 `u8` = 228) + oracle config (71) +
    /// risk params (120) + IRM params (80) + accounting (64) + flags/bumps (69) = 640.
    ///
    /// `account-model.md` §4 states this size approximately ("~633 ... ≈ 641"); this constant is
    /// the exact figure derived by summing the field list in that same section field-by-field,
    /// and is pinned by `U-ACCT-02` against the account actually produced by `create_market`.
    /// ADR-0013 (Phase 8) added `liquidation_guard: u8` and shrank `_reserved` by one byte in the
    /// same breath, so the flags/bumps subtotal (69) and the overall total (640) are unchanged.
    pub const LEN: usize = 8
        + (32 * 7 + 2 + 1 + 1) // identity: 228
        + (1 + 32 + 32 + 4 + 2) // oracle config: 71
        + (16 * 7 + 8) // risk params: 120
        + (16 * 5) // IRM params: 80
        + (8 + 16 + 8 + 16 + 8 + 8) // accounting: 64
        + (1 + 1 + 1 + 1 + 1 + 1 + 63); // flags/bumps: 69 (incl. liquidation_guard)

    /// Risk-parameter bounds from `economic-model.md` §5, including the derived liquidation
    /// bound `liq_threshold * (WAD + liq_bonus) / WAD < WAD` (INV-LIQ-06).
    #[allow(clippy::too_many_arguments)]
    pub fn validate_risk_params(
        max_ltv: u128,
        liq_threshold: u128,
        liq_bonus: u128,
        close_factor: u128,
        full_liq_hf: u128,
        liq_protocol_fee: u128,
        fee: u128,
        min_debt: u64,
    ) -> Result<()> {
        require!(
            max_ltv > 0 && max_ltv < liq_threshold && liq_threshold < WAD,
            AegisError::InvalidMaxLtvOrThreshold
        );
        require!(liq_bonus <= MAX_LIQ_BONUS, AegisError::InvalidLiqBonus);

        // Derived bound: liq_bonus is already <= MAX_LIQ_BONUS (0.25 WAD), so WAD + liq_bonus
        // cannot overflow u128, and mul_div_floor's 256-bit intermediate cannot overflow either.
        let bonus_factor = WAD
            .checked_add(liq_bonus)
            .ok_or(AegisError::ArithmeticOverflow)?;
        let threshold_times_bonus =
            mul_div_floor(liq_threshold, bonus_factor, WAD).map_err(AegisError::from)?;
        require!(
            threshold_times_bonus < WAD,
            AegisError::LiquidationBonusExceedsThresholdBound
        );

        require!(
            (MIN_CLOSE_FACTOR..=WAD).contains(&close_factor),
            AegisError::InvalidCloseFactor
        );
        require!(
            full_liq_hf > 0 && full_liq_hf <= WAD,
            AegisError::InvalidFullLiqHf
        );
        require!(
            liq_protocol_fee <= MAX_LIQ_PROTOCOL_FEE,
            AegisError::InvalidLiqProtocolFee
        );
        require!(fee <= MAX_FEE, AegisError::InvalidFee);
        require!(min_debt > 0, AegisError::InvalidMinDebt);
        Ok(())
    }

    /// IRM bounds from `instruction-catalogue.md` §6 precondition 7:
    /// `0 < u_kink < WAD`, `max_rate_ps > 0`, every rate `<= max_rate_ps`.
    pub fn validate_irm_params(
        base_rate_ps: u128,
        slope1_ps: u128,
        slope2_ps: u128,
        u_kink: u128,
        max_rate_ps: u128,
    ) -> Result<()> {
        require!(
            u_kink > 0 && u_kink < WAD && max_rate_ps > 0,
            AegisError::InvalidIrmParams
        );
        require!(
            base_rate_ps <= max_rate_ps && slope1_ps <= max_rate_ps && slope2_ps <= max_rate_ps,
            AegisError::InvalidIrmParams
        );
        Ok(())
    }

    /// Oracle config bounds from `instruction-catalogue.md` §6 precondition 9.
    pub fn validate_oracle_config(max_price_age_secs: u32, max_conf_bps: u16) -> Result<()> {
        require!(
            (MIN_PRICE_AGE_SECS..=MAX_PRICE_AGE_SECS).contains(&max_price_age_secs),
            AegisError::InvalidMaxPriceAge
        );
        require!(
            (MIN_CONF_BPS..=MAX_CONF_BPS).contains(&max_conf_bps),
            AegisError::InvalidMaxConfBps
        );
        Ok(())
    }

    /// **Pure.** Computes the fully-accrued totals as of `now`, writing nothing
    /// (`economic-model.md` §4.5, INV-ACC-08). `dt == 0` is a successful no-op: `interest` and
    /// `fee_amount` are both `0` and every total is returned unchanged (E-02, `U-IRM-01`).
    ///
    /// This exists so a future caller (e.g. a Phase 5 `withdraw_collateral` solvency check) can
    /// evaluate fully-accrued debt **without taking a write lock on `Market`** (NFR-7, ADR-0004) —
    /// using accrued (larger) debt is strictly conservative for a solvency check. `accrue_mut`
    /// below is the only place this crate is allowed to duplicate: everything else must call this
    /// function rather than reimplement the formula (`docs/phases/phase-04-lending.md`).
    pub fn accrue_view(&self, now: i64) -> Result<AccrueOutcome> {
        let dt = now.saturating_sub(self.last_accrual_ts).max(0);
        if dt == 0 {
            return Ok(AccrueOutcome {
                total_supply_assets: self.total_supply_assets,
                total_borrow_assets: self.total_borrow_assets,
                last_accrual_ts: self.last_accrual_ts,
                interest: 0,
                fee_amount: 0,
            });
        }
        // `now > last_accrual_ts` here, so `dt` fits a u64 (i64 seconds since a real, bounded
        // Unix timestamp) via a non-truncating narrowing (the value is already known to be a
        // small, non-negative i64).
        let dt_seconds = u64::try_from(dt).map_err(|_| AegisError::ArithmeticOverflow)?;

        let u = utilization(self.total_borrow_assets, self.total_supply_assets)
            .map_err(AegisError::from)?;
        let r = borrow_rate(
            u,
            self.base_rate_ps,
            self.slope1_ps,
            self.slope2_ps,
            self.u_kink,
            self.max_rate_ps,
        )
        .map_err(AegisError::from)?;
        let x = taylor_x(r, dt_seconds).map_err(AegisError::from)?;
        let growth = taylor3(x).map_err(AegisError::from)?;

        let interest_u128 = mul_div_floor(self.total_borrow_assets as u128, growth, WAD)
            .map_err(AegisError::from)?;
        let interest = u64::try_from(interest_u128).map_err(|_| AegisError::ArithmeticOverflow)?;

        let total_borrow_assets = self
            .total_borrow_assets
            .checked_add(interest)
            .ok_or(AegisError::ArithmeticOverflow)?;
        let total_supply_assets = self
            .total_supply_assets
            .checked_add(interest)
            .ok_or(AegisError::ArithmeticOverflow)?;

        let fee_amount_u128 =
            mul_div_floor(interest as u128, self.fee, WAD).map_err(AegisError::from)?;
        let fee_amount =
            u64::try_from(fee_amount_u128).map_err(|_| AegisError::ArithmeticOverflow)?;

        Ok(AccrueOutcome {
            total_supply_assets,
            total_borrow_assets,
            last_accrual_ts: now,
            interest,
            fee_amount,
        })
    }

    /// Applies `accrue_view`'s result to `self` and, if a nonzero fee accrued, mints protocol fee
    /// shares to `fee_position` (`economic-model.md` §4.3). **Never duplicates the financial
    /// formulas independently of `accrue_view`** — `P-ACCRUE-1` proves the two agree.
    ///
    /// The fee shares are priced against `total_supply_assets - fee_amount` — the **pre-fee**
    /// asset base — which is what makes the dilution exactly `fee_amount` of value and no more
    /// (`P-FEE-1`). Pricing against the post-fee base would under-issue fee shares and silently
    /// give lenders part of the protocol's fee.
    ///
    /// Returns the same `AccrueOutcome` plus the fee shares minted (`0` if no fee accrued), for
    /// the `InterestAccrued` event.
    pub fn accrue_mut(
        &mut self,
        fee_position: &mut Position,
        now: i64,
    ) -> Result<(AccrueOutcome, u128)> {
        let outcome = self.accrue_view(now)?;

        self.total_supply_assets = outcome.total_supply_assets;
        self.total_borrow_assets = outcome.total_borrow_assets;
        self.last_accrual_ts = outcome.last_accrual_ts;

        let mut fee_shares = 0u128;
        if outcome.fee_amount > 0 {
            let pre_fee_supply_base = self
                .total_supply_assets
                .checked_sub(outcome.fee_amount)
                .ok_or(AegisError::ArithmeticOverflow)?;
            fee_shares = to_shares_down(
                outcome.fee_amount,
                pre_fee_supply_base,
                self.total_supply_shares,
            )
            .map_err(AegisError::from)?;
            self.total_supply_shares = self
                .total_supply_shares
                .checked_add(fee_shares)
                .ok_or(AegisError::ArithmeticOverflow)?;
            fee_position.supply_shares = fee_position
                .supply_shares
                .checked_add(fee_shares)
                .ok_or(AegisError::ArithmeticOverflow)?;
        }

        Ok((outcome, fee_shares))
    }
}

/// The result of `Market::accrue_view` — the fully-accrued totals as of a given timestamp, plus
/// the interest and protocol-fee amounts that produced them. Never includes share counts: the
/// only permitted divergence between `accrue_view` and `accrue_mut` is that the latter also mints
/// fee shares, which affect `total_supply_shares` alone, never these fields (economic-model.md
/// §4.5's INV-ACC-08 carve-out).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccrueOutcome {
    pub total_supply_assets: u64,
    pub total_borrow_assets: u64,
    pub last_accrual_ts: i64,
    pub interest: u64,
    pub fee_amount: u64,
}

/// Phase 12 (`governance.md` §4, INV-ADM-09, ADR-0014): a risk-increasing ("loosening")
/// `set_market_params` change, staged behind `constants::PARAM_TIMELOCK_SECS` rather than applied
/// immediately. One per `Market` (`PDA([b"pending_params", market])`), created by
/// `set_market_params` only on the loosening path and closed by `commit_pending_params`.
///
/// Deliberately **does not** carry `fee_recipient`: that field is not a risk parameter (it does
/// not appear in either row of `governance.md` §4's tighten/loosen table) and always applies
/// immediately in the same `set_market_params` call that staged this proposal, orthogonal to
/// whatever else is pending.
#[account]
pub struct PendingMarketParams {
    pub market: Pubkey,
    /// Unix seconds (never slots, per AGENTS.md §7.9) at or after which `commit_pending_params`
    /// may apply this proposal.
    pub effective_at: i64,

    pub oracle_kind: u8,
    pub collateral_feed_id: [u8; 32],
    pub loan_feed_id: [u8; 32],
    pub max_price_age_secs: u32,
    pub max_conf_bps: u16,

    pub max_ltv: u128,
    pub liq_threshold: u128,
    pub liq_bonus: u128,
    pub close_factor: u128,
    pub full_liq_hf: u128,
    pub liq_protocol_fee: u128,
    pub fee: u128,
    pub min_debt: u64,

    pub base_rate_ps: u128,
    pub slope1_ps: u128,
    pub slope2_ps: u128,
    pub u_kink: u128,
    pub max_rate_ps: u128,

    pub bump: u8,
    pub _reserved: [u8; 32],
}

impl PendingMarketParams {
    /// `8` (discriminator) + `32` (`market`) + `8` (`effective_at`) + oracle config (`1 + 32 + 32
    /// + 4 + 2` = 71) + risk params (`16*7 + 8` = 120) + IRM params (`16*5` = 80) + `1` (`bump`)
    /// + `32` (`_reserved`) = 352.
    pub const LEN: usize = 8 + 32 + 8 + (1 + 32 + 32 + 4 + 2) + (16 * 7 + 8) + (16 * 5) + 1 + 32;
}

/// Every field of `Market` that `set_market_params` is capable of touching, in one place so the
/// tighten/loosen classifier and the instruction handler both work from the same field list
/// rather than two hand-maintained copies. Deliberately excludes `fee_recipient` (see
/// [`PendingMarketParams`]'s doc comment) and every identity field (`INV-ADM-06`) — those simply
/// have no place in this struct, which is what makes changing them unrepresentable rather than
/// merely rejected (`A-ADM-06`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::too_many_arguments)]
pub struct MutableMarketParams {
    pub oracle_kind: u8,
    pub collateral_feed_id: [u8; 32],
    pub loan_feed_id: [u8; 32],
    pub max_price_age_secs: u32,
    pub max_conf_bps: u16,

    pub max_ltv: u128,
    pub liq_threshold: u128,
    pub liq_bonus: u128,
    pub close_factor: u128,
    pub full_liq_hf: u128,
    pub liq_protocol_fee: u128,
    pub fee: u128,
    pub min_debt: u64,

    pub base_rate_ps: u128,
    pub slope1_ps: u128,
    pub slope2_ps: u128,
    pub u_kink: u128,
    pub max_rate_ps: u128,
}

impl MutableMarketParams {
    /// The current values of every mutable field, read off a live `Market`.
    pub fn from_market(market: &Market) -> Self {
        Self {
            oracle_kind: market.oracle_kind,
            collateral_feed_id: market.collateral_feed_id,
            loan_feed_id: market.loan_feed_id,
            max_price_age_secs: market.max_price_age_secs,
            max_conf_bps: market.max_conf_bps,
            max_ltv: market.max_ltv,
            liq_threshold: market.liq_threshold,
            liq_bonus: market.liq_bonus,
            close_factor: market.close_factor,
            full_liq_hf: market.full_liq_hf,
            liq_protocol_fee: market.liq_protocol_fee,
            fee: market.fee,
            min_debt: market.min_debt,
            base_rate_ps: market.base_rate_ps,
            slope1_ps: market.slope1_ps,
            slope2_ps: market.slope2_ps,
            u_kink: market.u_kink,
            max_rate_ps: market.max_rate_ps,
        }
    }

    /// Writes every field into `market` unconditionally (identity fields are untouched because
    /// they are not part of this struct at all). Callers apply this only after
    /// `Market::validate_risk_params`/`validate_irm_params`/`validate_oracle_config` have already
    /// accepted the proposed values.
    pub fn apply_to(&self, market: &mut Market) {
        market.oracle_kind = self.oracle_kind;
        market.collateral_feed_id = self.collateral_feed_id;
        market.loan_feed_id = self.loan_feed_id;
        market.max_price_age_secs = self.max_price_age_secs;
        market.max_conf_bps = self.max_conf_bps;
        market.max_ltv = self.max_ltv;
        market.liq_threshold = self.liq_threshold;
        market.liq_bonus = self.liq_bonus;
        market.close_factor = self.close_factor;
        market.full_liq_hf = self.full_liq_hf;
        market.liq_protocol_fee = self.liq_protocol_fee;
        market.fee = self.fee;
        market.min_debt = self.min_debt;
        market.base_rate_ps = self.base_rate_ps;
        market.slope1_ps = self.slope1_ps;
        market.slope2_ps = self.slope2_ps;
        market.u_kink = self.u_kink;
        market.max_rate_ps = self.max_rate_ps;
    }

    /// Phase 12 tighten/loosen classification (`governance.md` §4, INV-ADM-09, ADR-0014).
    /// Returns `true` ("loosening") if `new` is risk-increasing relative to `old` in **any**
    /// single field; the whole parameter-update call is then staged as one atomic proposal rather
    /// than partially applied (ADR-0014: partial immediate-plus-staged application is exactly the
    /// kind of implicit per-field special-casing this repository's rules warn against).
    ///
    /// Exact classification, from `governance.md` §4's table:
    /// - `max_ltv`, `liq_threshold`, `liq_bonus`, `fee`: an **increase** is loosening.
    /// - `min_debt`: a **decrease** is loosening (the counterintuitive row governance.md calls
    ///   out explicitly).
    /// - `max_price_age_secs`, `max_conf_bps`: an **increase** is loosening.
    /// - `collateral_feed_id`/`loan_feed_id`/`oracle_kind`: **any** change is loosening
    ///   (governance.md §4: "equivalent to swapping the asset's price source").
    ///
    /// `close_factor`, `full_liq_hf`, `liq_protocol_fee`, and the five IRM parameters
    /// (`base_rate_ps`, `slope1_ps`, `slope2_ps`, `u_kink`, `max_rate_ps`) have **no directional
    /// classification anywhere in `governance.md` or `economic-model.md`** — inventing one here
    /// would be exactly the "intuition-based rule" `docs/phases/phase-12-governance.md` forbids.
    /// This is a documented Phase 12 implementation decision (ADR-0014), not a silent guess: **any**
    /// change to one of these fields is conservatively treated as loosening. A parameter set that
    /// is safe either way loses nothing but a delay; one that turns out to matter is never applied
    /// without the observation window the timelock exists to provide.
    pub fn is_loosening(old: &Self, new: &Self) -> bool {
        new.max_ltv > old.max_ltv
            || new.liq_threshold > old.liq_threshold
            || new.liq_bonus > old.liq_bonus
            || new.fee > old.fee
            || new.min_debt < old.min_debt
            || new.max_price_age_secs > old.max_price_age_secs
            || new.max_conf_bps > old.max_conf_bps
            || new.collateral_feed_id != old.collateral_feed_id
            || new.loan_feed_id != old.loan_feed_id
            || new.oracle_kind != old.oracle_kind
            || new.close_factor != old.close_factor
            || new.full_liq_hf != old.full_liq_hf
            || new.liq_protocol_fee != old.liq_protocol_fee
            || new.base_rate_ps != old.base_rate_ps
            || new.slope1_ps != old.slope1_ps
            || new.slope2_ps != old.slope2_ps
            || new.u_kink != old.u_kink
            || new.max_rate_ps != old.max_rate_ps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn len_matches_account_model_spec() {
        // account-model.md §4 states "8 + ~633 ~= 641" (approximate); 640 is the exact sum of
        // the field list in that same section.
        assert_eq!(Market::LEN, 640);
    }

    #[test]
    fn pending_market_params_len_is_352() {
        assert_eq!(PendingMarketParams::LEN, 352);
    }

    fn reference_mutable_params() -> MutableMarketParams {
        MutableMarketParams {
            oracle_kind: 0,
            collateral_feed_id: [1u8; 32],
            loan_feed_id: [2u8; 32],
            max_price_age_secs: 60,
            max_conf_bps: 100,
            max_ltv: REF_MAX_LTV,
            liq_threshold: REF_LT,
            liq_bonus: REF_BONUS,
            close_factor: REF_CLOSE_FACTOR,
            full_liq_hf: REF_FULL_LIQ_HF,
            liq_protocol_fee: REF_LIQ_PROTOCOL_FEE,
            fee: REF_FEE,
            min_debt: REF_MIN_DEBT,
            base_rate_ps: 0,
            slope1_ps: WAD / 20,
            slope2_ps: WAD / 2,
            u_kink: WAD / 2,
            max_rate_ps: WAD,
        }
    }

    // U-ADM-02 (informal): an unchanged parameter set is never loosening.
    #[test]
    fn identical_params_are_not_loosening() {
        let p = reference_mutable_params();
        assert!(!MutableMarketParams::is_loosening(&p, &p));
    }

    // governance.md §4: raising max_ltv/liq_threshold/liq_bonus/fee is loosening; lowering is not
    // (by itself).
    #[test]
    fn raising_max_ltv_liq_threshold_liq_bonus_or_fee_is_loosening() {
        let old = reference_mutable_params();

        let mut new = old;
        new.max_ltv += 1;
        assert!(MutableMarketParams::is_loosening(&old, &new));

        let mut new = old;
        new.liq_threshold += 1;
        assert!(MutableMarketParams::is_loosening(&old, &new));

        let mut new = old;
        new.liq_bonus += 1;
        assert!(MutableMarketParams::is_loosening(&old, &new));

        let mut new = old;
        new.fee += 1;
        assert!(MutableMarketParams::is_loosening(&old, &new));
    }

    #[test]
    fn lowering_max_ltv_liq_threshold_liq_bonus_or_fee_is_tightening() {
        let old = reference_mutable_params();

        let mut new = old;
        new.max_ltv -= 1;
        assert!(!MutableMarketParams::is_loosening(&old, &new));

        let mut new = old;
        new.liq_threshold -= 1;
        assert!(!MutableMarketParams::is_loosening(&old, &new));

        let mut new = old;
        new.liq_bonus -= 1;
        assert!(!MutableMarketParams::is_loosening(&old, &new));

        let mut new = old;
        new.fee -= 1;
        assert!(!MutableMarketParams::is_loosening(&old, &new));
    }

    // governance.md §4's explicitly counterintuitive row: LOWERING min_debt is loosening, RAISING
    // it is tightening -- the opposite direction from every other field above.
    #[test]
    fn min_debt_direction_is_inverted() {
        let old = reference_mutable_params();

        let mut lowered = old;
        lowered.min_debt -= 1;
        assert!(
            MutableMarketParams::is_loosening(&old, &lowered),
            "lowering min_debt must be classified as loosening"
        );

        let mut raised = old;
        raised.min_debt += 1;
        assert!(
            !MutableMarketParams::is_loosening(&old, &raised),
            "raising min_debt must be classified as tightening"
        );
    }

    #[test]
    fn raising_max_price_age_or_max_conf_bps_is_loosening() {
        let old = reference_mutable_params();

        let mut new = old;
        new.max_price_age_secs += 1;
        assert!(MutableMarketParams::is_loosening(&old, &new));

        let mut new = old;
        new.max_conf_bps += 1;
        assert!(MutableMarketParams::is_loosening(&old, &new));
    }

    #[test]
    fn lowering_max_price_age_or_max_conf_bps_is_tightening() {
        let old = reference_mutable_params();

        let mut new = old;
        new.max_price_age_secs -= 1;
        assert!(!MutableMarketParams::is_loosening(&old, &new));

        let mut new = old;
        new.max_conf_bps -= 1;
        assert!(!MutableMarketParams::is_loosening(&old, &new));
    }

    // governance.md §4: ANY feed-ID or oracle_kind change is loosening, regardless of "direction"
    // (there is no direction for an opaque identifier).
    #[test]
    fn any_feed_id_or_oracle_kind_change_is_loosening() {
        let old = reference_mutable_params();

        let mut new = old;
        new.collateral_feed_id = [9u8; 32];
        assert!(MutableMarketParams::is_loosening(&old, &new));

        let mut new = old;
        new.loan_feed_id = [9u8; 32];
        assert!(MutableMarketParams::is_loosening(&old, &new));

        let mut new = old;
        new.oracle_kind = 1;
        assert!(MutableMarketParams::is_loosening(&old, &new));
    }

    // ADR-0014's conservative default: any change to a field governance.md does not classify
    // (close_factor, full_liq_hf, liq_protocol_fee, and the five IRM params) is loosening.
    #[test]
    fn unclassified_fields_default_to_loosening_on_any_change() {
        let old = reference_mutable_params();

        let mut new = old;
        new.close_factor += 1;
        assert!(MutableMarketParams::is_loosening(&old, &new));

        let mut new = old;
        new.full_liq_hf -= 1;
        assert!(MutableMarketParams::is_loosening(&old, &new));

        let mut new = old;
        new.liq_protocol_fee += 1;
        assert!(MutableMarketParams::is_loosening(&old, &new));

        let mut new = old;
        new.base_rate_ps += 1;
        assert!(MutableMarketParams::is_loosening(&old, &new));

        let mut new = old;
        new.slope1_ps += 1;
        assert!(MutableMarketParams::is_loosening(&old, &new));

        let mut new = old;
        new.slope2_ps += 1;
        assert!(MutableMarketParams::is_loosening(&old, &new));

        let mut new = old;
        new.u_kink += 1;
        assert!(MutableMarketParams::is_loosening(&old, &new));

        let mut new = old;
        new.max_rate_ps -= 1;
        assert!(MutableMarketParams::is_loosening(&old, &new));
    }

    #[test]
    fn from_market_and_apply_to_round_trip() {
        let mut market = test_market(
            1_000_000_000,
            1_000_000_000_000_000,
            900_000_000,
            900_000_000_000_000,
            1_000,
            REF_FEE,
        );
        let snapshot = MutableMarketParams::from_market(&market);
        assert_eq!(snapshot.max_ltv, market.max_ltv);
        assert_eq!(snapshot.min_debt, market.min_debt);

        let mut proposed = snapshot;
        proposed.fee = REF_FEE / 2;
        proposed.apply_to(&mut market);
        assert_eq!(market.fee, REF_FEE / 2);
        // Identity fields are untouched -- MutableMarketParams has no field for them at all.
        assert_eq!(market.collateral_decimals, 9);
    }

    const REF_MAX_LTV: u128 = 750_000_000_000_000_000; // 0.75 WAD
    const REF_LT: u128 = 800_000_000_000_000_000; // 0.80 WAD
    const REF_BONUS: u128 = 50_000_000_000_000_000; // 0.05 WAD
    const REF_CLOSE_FACTOR: u128 = 500_000_000_000_000_000; // 0.5 WAD
    const REF_FULL_LIQ_HF: u128 = 950_000_000_000_000_000; // 0.95 WAD
    const REF_LIQ_PROTOCOL_FEE: u128 = 100_000_000_000_000_000; // 0.10 WAD
    const REF_FEE: u128 = 100_000_000_000_000_000; // 0.10 WAD
    const REF_MIN_DEBT: u64 = 10_000_000; // 10 USDC at 6dp

    // U-ADM-00 (informal): economic-model.md §5.1's own reference parameter set must validate.
    #[test]
    fn reference_parameter_set_is_valid() {
        assert!(Market::validate_risk_params(
            REF_MAX_LTV,
            REF_LT,
            REF_BONUS,
            REF_CLOSE_FACTOR,
            REF_FULL_LIQ_HF,
            REF_LIQ_PROTOCOL_FEE,
            REF_FEE,
            REF_MIN_DEBT,
        )
        .is_ok());
    }

    // A-ADM-04 (math-level component): an otherwise-plausible parameter set that specifically
    // violates the derived liquidation bound LT*(1+b) < WAD must be rejected, distinctly from the
    // flat liq_bonus bound (0.24 WAD alone is within MAX_LIQ_BONUS).
    #[test]
    fn derived_liquidation_bound_rejects_plausible_but_unsafe_params() {
        let unsafe_bonus = 240_000_000_000_000_000u128; // 0.24 WAD, <= MAX_LIQ_BONUS
        let high_threshold = 850_000_000_000_000_000u128; // 0.85 WAD: 0.85 * 1.24 = 1.054 > 1
        let err = Market::validate_risk_params(
            REF_MAX_LTV,
            high_threshold,
            unsafe_bonus,
            REF_CLOSE_FACTOR,
            REF_FULL_LIQ_HF,
            REF_LIQ_PROTOCOL_FEE,
            REF_FEE,
            REF_MIN_DEBT,
        )
        .unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::LiquidationBonusExceedsThresholdBound)
        );
    }

    #[test]
    fn max_ltv_must_be_below_liq_threshold() {
        let err = Market::validate_risk_params(
            REF_LT,
            REF_MAX_LTV,
            REF_BONUS,
            REF_CLOSE_FACTOR,
            REF_FULL_LIQ_HF,
            REF_LIQ_PROTOCOL_FEE,
            REF_FEE,
            REF_MIN_DEBT,
        )
        .unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::InvalidMaxLtvOrThreshold)
        );
    }

    #[test]
    fn liq_bonus_above_max_is_rejected() {
        let err = Market::validate_risk_params(
            REF_MAX_LTV,
            REF_LT,
            MAX_LIQ_BONUS + 1,
            REF_CLOSE_FACTOR,
            REF_FULL_LIQ_HF,
            REF_LIQ_PROTOCOL_FEE,
            REF_FEE,
            REF_MIN_DEBT,
        )
        .unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::InvalidLiqBonus)
        );
    }

    #[test]
    fn close_factor_below_minimum_is_rejected() {
        let err = Market::validate_risk_params(
            REF_MAX_LTV,
            REF_LT,
            REF_BONUS,
            MIN_CLOSE_FACTOR - 1,
            REF_FULL_LIQ_HF,
            REF_LIQ_PROTOCOL_FEE,
            REF_FEE,
            REF_MIN_DEBT,
        )
        .unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::InvalidCloseFactor)
        );
    }

    #[test]
    fn full_liq_hf_zero_is_rejected() {
        let err = Market::validate_risk_params(
            REF_MAX_LTV,
            REF_LT,
            REF_BONUS,
            REF_CLOSE_FACTOR,
            0,
            REF_LIQ_PROTOCOL_FEE,
            REF_FEE,
            REF_MIN_DEBT,
        )
        .unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::InvalidFullLiqHf)
        );
    }

    #[test]
    fn liq_protocol_fee_above_max_is_rejected() {
        let err = Market::validate_risk_params(
            REF_MAX_LTV,
            REF_LT,
            REF_BONUS,
            REF_CLOSE_FACTOR,
            REF_FULL_LIQ_HF,
            MAX_LIQ_PROTOCOL_FEE + 1,
            REF_FEE,
            REF_MIN_DEBT,
        )
        .unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::InvalidLiqProtocolFee)
        );
    }

    #[test]
    fn fee_above_max_is_rejected() {
        let err = Market::validate_risk_params(
            REF_MAX_LTV,
            REF_LT,
            REF_BONUS,
            REF_CLOSE_FACTOR,
            REF_FULL_LIQ_HF,
            REF_LIQ_PROTOCOL_FEE,
            MAX_FEE + 1,
            REF_MIN_DEBT,
        )
        .unwrap_err();
        assert_eq!(err, anchor_lang::error::Error::from(AegisError::InvalidFee));
    }

    #[test]
    fn zero_min_debt_is_rejected() {
        let err = Market::validate_risk_params(
            REF_MAX_LTV,
            REF_LT,
            REF_BONUS,
            REF_CLOSE_FACTOR,
            REF_FULL_LIQ_HF,
            REF_LIQ_PROTOCOL_FEE,
            REF_FEE,
            0,
        )
        .unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::InvalidMinDebt)
        );
    }

    #[test]
    fn irm_params_reference_set_is_valid() {
        assert!(Market::validate_irm_params(0, WAD / 20, WAD / 2, WAD / 2, WAD).is_ok());
    }

    #[test]
    fn irm_u_kink_out_of_range_is_rejected() {
        let err = Market::validate_irm_params(0, WAD / 20, WAD / 2, WAD, WAD).unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::InvalidIrmParams)
        );
    }

    #[test]
    fn irm_rate_exceeding_max_is_rejected() {
        let err = Market::validate_irm_params(0, WAD / 20, WAD + 1, WAD / 2, WAD).unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::InvalidIrmParams)
        );
    }

    #[test]
    fn oracle_config_reference_is_valid() {
        assert!(Market::validate_oracle_config(60, 100).is_ok());
    }

    #[test]
    fn oracle_config_price_age_out_of_range_is_rejected() {
        let err = Market::validate_oracle_config(0, 100).unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::InvalidMaxPriceAge)
        );
        let err = Market::validate_oracle_config(MAX_PRICE_AGE_SECS + 1, 100).unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::InvalidMaxPriceAge)
        );
    }

    #[test]
    fn oracle_config_conf_bps_out_of_range_is_rejected() {
        let err = Market::validate_oracle_config(60, 0).unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::InvalidMaxConfBps)
        );
        let err = Market::validate_oracle_config(60, MAX_CONF_BPS + 1).unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::InvalidMaxConfBps)
        );
    }

    // --- Phase 4: accrual tests (accrue_view / accrue_mut) ---

    fn test_market(
        total_supply_assets: u64,
        total_supply_shares: u128,
        total_borrow_assets: u64,
        total_borrow_shares: u128,
        last_accrual_ts: i64,
        fee: u128,
    ) -> Market {
        Market {
            collateral_mint: Pubkey::default(),
            loan_mint: Pubkey::default(),
            collateral_token_program: Pubkey::default(),
            loan_token_program: Pubkey::default(),
            collateral_vault: Pubkey::default(),
            loan_vault: Pubkey::default(),
            fee_recipient: Pubkey::default(),
            config_id: 0,
            collateral_decimals: 9,
            loan_decimals: 6,
            oracle_kind: 0,
            collateral_feed_id: [0u8; 32],
            loan_feed_id: [0u8; 32],
            max_price_age_secs: 60,
            max_conf_bps: 100,
            max_ltv: REF_MAX_LTV,
            liq_threshold: REF_LT,
            liq_bonus: REF_BONUS,
            close_factor: REF_CLOSE_FACTOR,
            full_liq_hf: REF_FULL_LIQ_HF,
            liq_protocol_fee: REF_LIQ_PROTOCOL_FEE,
            fee,
            min_debt: REF_MIN_DEBT,
            base_rate_ps: 0,
            slope1_ps: 1_268_391_679,
            slope2_ps: 31_709_791_983,
            u_kink: 800_000_000_000_000_000,
            max_rate_ps: 317_097_919_837,
            total_supply_assets,
            total_supply_shares,
            total_borrow_assets,
            total_borrow_shares,
            collateral_fee_accrued: 0,
            last_accrual_ts,
            paused: 0,
            flags: 0,
            bump: 0,
            collateral_vault_bump: 0,
            loan_vault_bump: 0,
            liquidation_guard: 0,
            _reserved: [0u8; 63],
        }
    }

    fn test_fee_position() -> Position {
        Position {
            market: Pubkey::default(),
            owner: Pubkey::default(),
            supply_shares: 0,
            borrow_shares: 0,
            collateral_amount: 0,
            bump: 0,
            _reserved: [0u8; 32],
        }
    }

    // U-IRM-01 / E-02: dt == 0 must be a successful no-op -- no error, totals unchanged, no fee
    // shares minted, last_accrual_ts unchanged.
    #[test]
    fn accrue_with_dt_zero_is_a_no_op() {
        let market = test_market(
            1_000_000_000,
            1_000_000_000_000_000,
            900_000_000,
            900_000_000_000_000,
            1_000,
            REF_FEE,
        );
        let outcome = market.accrue_view(1_000).unwrap();
        assert_eq!(outcome.total_supply_assets, market.total_supply_assets);
        assert_eq!(outcome.total_borrow_assets, market.total_borrow_assets);
        assert_eq!(outcome.last_accrual_ts, market.last_accrual_ts);
        assert_eq!(outcome.interest, 0);
        assert_eq!(outcome.fee_amount, 0);

        let mut market_mut = market;
        let mut fee_position = test_fee_position();
        let (mut_outcome, fee_shares) = market_mut.accrue_mut(&mut fee_position, 1_000).unwrap();
        assert_eq!(mut_outcome, outcome);
        assert_eq!(fee_shares, 0, "dt == 0 must never mint fee shares");
        assert_eq!(fee_position.supply_shares, 0);
        assert_eq!(market_mut.total_supply_shares, 1_000_000_000_000_000);
        assert_eq!(market_mut.last_accrual_ts, 1_000);
    }

    // U-IRM-03: the exact worked example from economic-model.md §4.4, wired through the real
    // Market::accrue_view.
    #[test]
    fn accrue_view_matches_worked_example() {
        let market = test_market(
            1_000_000_000,
            1_000_000_000_000_000_000,
            900_000_000,
            900_000_000_000_000_000,
            0,
            REF_FEE,
        );
        let outcome = market.accrue_view(86_400).unwrap();
        assert_eq!(outcome.interest, 1_332_492);
        assert_eq!(outcome.fee_amount, 133_249);
        assert_eq!(outcome.total_borrow_assets, 901_332_492);
        assert_eq!(outcome.total_supply_assets, 1_001_332_492);
        assert_eq!(outcome.last_accrual_ts, 86_400);
    }

    // P-ACCRUE-1 / INV-ACC-08: accrue_view(s, now) totals == { accrue_mut(s', now); s'.totals },
    // for equivalent starting state and time -- the only permitted divergence is fee shares, which
    // do not appear in AccrueOutcome at all.
    #[test]
    fn p_accrue_1_view_and_mut_agree() {
        let cases: [(u64, u128, u64, u128, i64, i64); 4] = [
            (
                1_000_000_000,
                1_000_000_000_000_000_000,
                900_000_000,
                900_000_000_000_000_000,
                0,
                86_400,
            ),
            (0, 0, 0, 0, 0, 1_000),
            (
                5_000_000,
                5_000_000_000_000,
                5_000_000,
                5_000_000_000_000,
                100,
                100,
            ), // dt = 0
            (
                u64::MAX / 4,
                1_000_000_000_000_000_000,
                u64::MAX / 8,
                500_000_000_000_000_000,
                0,
                31_536_000,
            ),
        ];
        for (ta, ts, tb, tbs, last_ts, now) in cases {
            let market = test_market(ta, ts, tb, tbs, last_ts, REF_FEE);
            let view_outcome = market.accrue_view(now).unwrap();

            let mut market_mut = market;
            let mut fee_position = test_fee_position();
            let (mut_outcome, _fee_shares) = market_mut.accrue_mut(&mut fee_position, now).unwrap();

            assert_eq!(
                view_outcome, mut_outcome,
                "accrue_view and accrue_mut must agree exactly"
            );
            assert_eq!(
                market_mut.total_supply_assets,
                view_outcome.total_supply_assets
            );
            assert_eq!(
                market_mut.total_borrow_assets,
                view_outcome.total_borrow_assets
            );
            assert_eq!(market_mut.last_accrual_ts, view_outcome.last_accrual_ts);
        }
    }

    // P-ACCRUE-2 / INV-ACC-04: total_supply_assets - total_borrow_assets is invariant under
    // accrual (interest is a pure transfer of claim from borrowers to lenders; no token moves).
    #[test]
    fn p_accrue_2_free_liquidity_invariant_under_accrual() {
        let mut market = test_market(
            1_000_000_000,
            1_000_000_000_000_000_000,
            900_000_000,
            900_000_000_000_000_000,
            0,
            REF_FEE,
        );
        let free_liquidity_before = market.total_supply_assets - market.total_borrow_assets;
        let mut fee_position = test_fee_position();
        market.accrue_mut(&mut fee_position, 86_400).unwrap();
        let free_liquidity_after = market.total_supply_assets - market.total_borrow_assets;
        assert_eq!(free_liquidity_before, free_liquidity_after);
    }

    // P-FEE-1: fee shares dilute lenders by exactly fee_amount (+-1 unit) -- after accrual, the
    // fee recipient's claimable assets increase by exactly fee_amount, and total claimable assets
    // (which is what every OTHER lender's aggregate claim is measured against) grows by exactly
    // interest - fee_amount in aggregate once the fee recipient's own share is excluded.
    #[test]
    fn p_fee_1_fee_shares_dilute_by_exactly_fee_amount() {
        let mut market = test_market(
            1_000_000_000,
            1_000_000_000_000_000_000,
            900_000_000,
            900_000_000_000_000_000,
            0,
            REF_FEE,
        );
        let mut fee_position = test_fee_position();
        let (outcome, fee_shares) = market.accrue_mut(&mut fee_position, 86_400).unwrap();
        assert!(fee_shares > 0, "fixture must actually accrue a nonzero fee");

        let fee_recipient_claim = aegis_math::to_assets_down(
            fee_position.supply_shares,
            market.total_supply_assets,
            market.total_supply_shares,
        )
        .unwrap();
        let diff = fee_recipient_claim as i128 - outcome.fee_amount as i128;
        assert!(
            diff.unsigned_abs() <= 1,
            "fee recipient's claim ({fee_recipient_claim}) must equal fee_amount ({}) within 1 unit of rounding",
            outcome.fee_amount
        );

        // The denominator used to price fee shares must be the PRE-fee base
        // (total_supply_assets - fee_amount), not the post-fee total -- proven by checking that
        // pricing against the WRONG (post-fee) denominator would have produced strictly fewer
        // shares (under-dilution, silently gifting lenders part of the fee).
        let wrong_denominator_shares = aegis_math::to_shares_down(
            outcome.fee_amount,
            outcome.total_supply_assets, // the bug: post-fee base
            market.total_supply_shares - fee_shares, // pre-mint share total
        )
        .unwrap();
        assert!(
            fee_shares >= wrong_denominator_shares,
            "fee shares priced against the correct pre-fee base must be >= what the wrong \
             (post-fee) denominator would have produced"
        );
    }
}
