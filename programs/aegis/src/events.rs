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
