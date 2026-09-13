//! `#[event]` definitions for Phase 2 (architecture.md §2). Every event is emitted exactly once,
//! from the successful transition that produces it, and carries enough content to be a real audit
//! record rather than a bare "something happened" marker.

use anchor_lang::prelude::*;

#[event]
pub struct ProtocolInitialized {
    pub protocol: Pubkey,
    pub admin: Pubkey,
    pub guardian: Pubkey,
    pub fee_recipient: Pubkey,
}

/// The full parameter snapshot for a newly created market — the permanent audit record of the
/// market's risk configuration and the exact Token-2022 extension inventory that was accepted for
/// each mint (`token-compatibility.md` §6 step 7).
#[event]
pub struct MarketCreated {
    pub market: Pubkey,
    pub collateral_mint: Pubkey,
    pub loan_mint: Pubkey,
    pub collateral_token_program: Pubkey,
    pub loan_token_program: Pubkey,
    pub collateral_vault: Pubkey,
    pub loan_vault: Pubkey,
    pub fee_recipient: Pubkey,
    pub fee_position: Pubkey,
    pub config_id: u16,
    pub collateral_decimals: u8,
    pub loan_decimals: u8,

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

    pub flags: u8,
    /// Token-2022 extension discriminants (`ExtensionType as u16`) accepted for the collateral
    /// mint. Empty for a classic SPL Token mint.
    pub collateral_extensions: Vec<u16>,
    /// As above, for the loan mint.
    pub loan_extensions: Vec<u16>,
}

#[event]
pub struct PositionInitialized {
    pub market: Pubkey,
    pub position: Pubkey,
    pub owner: Pubkey,
}

/// `amount_in` is the requested transfer amount; `credited` is the measured post-CPI delta
/// actually recorded against `position.collateral_amount` (`account-model.md` §6.4). The two
/// differ exactly when the collateral mint charges a Token-2022 transfer fee.
#[event]
pub struct CollateralDeposited {
    pub market: Pubkey,
    pub position: Pubkey,
    pub depositor: Pubkey,
    pub amount_in: u64,
    pub credited: u64,
}

#[event]
pub struct CollateralWithdrawn {
    pub market: Pubkey,
    pub position: Pubkey,
    pub owner: Pubkey,
    pub amount: u64,
}

#[event]
pub struct PositionClosed {
    pub market: Pubkey,
    pub position: Pubkey,
    pub owner: Pubkey,
}

/// `assets_in` is the requested/computed transfer amount; `credited` is the measured post-CPI
/// delta actually recorded (`account-model.md` §6.4) — loan assets are policy-restricted to
/// fee-free mints, so the two are expected to be equal, but this is verified, never assumed
/// (`instruction-catalogue.md` §12).
#[event]
pub struct Supplied {
    pub market: Pubkey,
    pub position: Pubkey,
    pub owner: Pubkey,
    pub assets_in: u64,
    pub credited: u64,
    pub shares_minted: u128,
}

#[event]
pub struct Withdrawn {
    pub market: Pubkey,
    pub position: Pubkey,
    pub owner: Pubkey,
    pub assets_out: u64,
    pub shares_burned: u128,
}

/// Defined for API completeness against `instruction-catalogue.md`'s event catalogue; never
/// emitted by Phase 4's `borrow` handler, which is hard-gated to always fail before any state
/// transition (`docs/phases/phase-04-lending.md` #17: "Do not emit `Borrowed` for the hard-gated
/// unsuccessful borrow path").
#[event]
pub struct Borrowed {
    pub market: Pubkey,
    pub position: Pubkey,
    pub owner: Pubkey,
    pub assets_out: u64,
    pub shares_minted: u128,
}

#[event]
pub struct Repaid {
    pub market: Pubkey,
    pub position: Pubkey,
    pub payer: Pubkey,
    pub assets_in: u64,
    pub credited: u64,
    pub shares_burned: u128,
}

#[event]
pub struct InterestAccrued {
    pub market: Pubkey,
    pub interest: u64,
    pub fee_amount: u64,
    pub fee_shares: u128,
    pub total_borrow_assets: u64,
    pub total_supply_assets: u64,
}

/// `economic-model.md` §7.3, `instruction-catalogue.md` §17. `hf_before`/`hf_after` are the
/// health factor immediately before and after this liquidation (`P-LIQ-1`'s on-chain evidence
/// trail); `clamped` records whether the collateral-clamp path (`economic-model.md` §7.2) fired.
/// `callback_program` is `None` for the Phase 6 path (`I-LIQ-CB-02`) and `Some(program_id)` when
/// the Phase 8 callback branch ran — part of the permanent audit record of which liquidations used
/// external composability (docs/composability.md).
#[event]
pub struct Liquidated {
    pub market: Pubkey,
    pub position: Pubkey,
    pub liquidator: Pubkey,
    pub repay_assets: u64,
    pub repay_shares: u128,
    pub base_seize: u64,
    pub total_seize: u64,
    pub bonus_amount: u64,
    pub protocol_cut: u64,
    pub to_liquidator: u64,
    pub clamped: bool,
    pub hf_before: u128,
    pub hf_after: u128,
    pub callback_program: Option<Pubkey>,
}

/// `economic-model.md` §8.2, `instruction-catalogue.md` §18. `absorbed_by_protocol` is the debt
/// absorbed by burning `fee_position.supply_shares` (protocol first-loss); `socialized` is the
/// residual left to fall on `total_supply_assets`/lenders. `absorbed_by_protocol + socialized ==
/// bad_assets` always.
#[event]
pub struct BadDebtAbsorbed {
    pub market: Pubkey,
    pub position: Pubkey,
    pub bad_assets: u64,
    pub absorbed_by_protocol: u64,
    pub socialized: u64,
    pub fee_shares_burned: u128,
}

/// `instruction-catalogue.md` §19.
#[event]
pub struct CollateralFeesWithdrawn {
    pub market: Pubkey,
    pub admin: Pubkey,
    pub amount: u64,
    pub remaining_collateral_fee_accrued: u64,
}

// ---- Phase 12: governance, pause, and migration events ----

/// `set_pending_admin` (`instruction-catalogue.md` §2-5, INV-ADM-02).
#[event]
pub struct AdminTransferStarted {
    pub protocol: Pubkey,
    pub current_admin: Pubkey,
    pub pending_admin: Pubkey,
}

/// `accept_admin`. `old_admin` is the admin that just lost authority.
#[event]
pub struct AdminTransferred {
    pub protocol: Pubkey,
    pub old_admin: Pubkey,
    pub new_admin: Pubkey,
}

/// `set_guardian`.
#[event]
pub struct GuardianChanged {
    pub protocol: Pubkey,
    pub admin: Pubkey,
    pub old_guardian: Pubkey,
    pub new_guardian: Pubkey,
}

/// `set_protocol_pause`. `authority` is whichever of `admin`/`guardian` signed.
#[event]
pub struct ProtocolPauseSet {
    pub protocol: Pubkey,
    pub authority: Pubkey,
    pub old_paused: u8,
    pub new_paused: u8,
}

/// `set_market_pause`.
#[event]
pub struct MarketPauseSet {
    pub market: Pubkey,
    pub authority: Pubkey,
    pub old_paused: u8,
    pub new_paused: u8,
}

/// `set_market_params`'s immediate-application path (tightening, or a pure `fee_recipient`
/// change) — a full before/after snapshot of every field the instruction is capable of touching,
/// mirroring `MarketCreated`'s audit-record style (`instruction-catalogue.md` §7).
#[event]
pub struct MarketParamsUpdated {
    pub market: Pubkey,
    pub admin: Pubkey,

    pub old_fee_recipient: Pubkey,
    pub new_fee_recipient: Pubkey,

    pub old_oracle_kind: u8,
    pub new_oracle_kind: u8,
    pub old_collateral_feed_id: [u8; 32],
    pub new_collateral_feed_id: [u8; 32],
    pub old_loan_feed_id: [u8; 32],
    pub new_loan_feed_id: [u8; 32],
    pub old_max_price_age_secs: u32,
    pub new_max_price_age_secs: u32,
    pub old_max_conf_bps: u16,
    pub new_max_conf_bps: u16,

    pub old_max_ltv: u128,
    pub new_max_ltv: u128,
    pub old_liq_threshold: u128,
    pub new_liq_threshold: u128,
    pub old_liq_bonus: u128,
    pub new_liq_bonus: u128,
    pub old_close_factor: u128,
    pub new_close_factor: u128,
    pub old_full_liq_hf: u128,
    pub new_full_liq_hf: u128,
    pub old_liq_protocol_fee: u128,
    pub new_liq_protocol_fee: u128,
    pub old_fee: u128,
    pub new_fee: u128,
    pub old_min_debt: u64,
    pub new_min_debt: u64,

    pub old_base_rate_ps: u128,
    pub new_base_rate_ps: u128,
    pub old_slope1_ps: u128,
    pub new_slope1_ps: u128,
    pub old_slope2_ps: u128,
    pub new_slope2_ps: u128,
    pub old_u_kink: u128,
    pub new_u_kink: u128,
    pub old_max_rate_ps: u128,
    pub new_max_rate_ps: u128,
}

/// `set_market_params`'s staged path (a risk-increasing/"loosening" change): the proposal is
/// recorded, not applied. Carries the same field set as [`MarketParamsUpdated`] minus
/// `fee_recipient` (which is never staged — it is orthogonal to risk and always applies
/// immediately) plus `effective_at`.
#[event]
pub struct ParamsStaged {
    pub market: Pubkey,
    pub pending_market_params: Pubkey,
    pub admin: Pubkey,
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
}

/// `commit_pending_params`, once the timelock has elapsed and the staged proposal has been
/// re-validated and applied.
#[event]
pub struct StagedParamsCommitted {
    pub market: Pubkey,
    pub pending_market_params: Pubkey,
    pub effective_at: i64,
}

/// `migrate_protocol_v2` (`INV-UPG-01..03`, ADR-0014). Emitted once, after a successful
/// `Migration<'info, ProtocolV1, Protocol>::migrate` call.
#[event]
pub struct ProtocolMigrated {
    pub protocol: Pubkey,
    pub admin: Pubkey,
    pub schema_version: u8,
}
