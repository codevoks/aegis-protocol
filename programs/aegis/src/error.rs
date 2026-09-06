//! The single `AegisError` enum (Anchor 1.0 permits exactly one `#[error_code]`), organized in
//! the banded layout from `architecture.md` §8 so error codes stay stable and greppable.
//!
//! Explicit discriminants (`= N`) pin each variant to `6000 + N`; variants without an explicit
//! discriminant continue from the previous one (`anchor-syn`'s error-code parser), which is how
//! the bands below are kept aligned to the table in `architecture.md` §8 without restating every
//! number.

use anchor_lang::prelude::*;

#[error_code]
pub enum AegisError {
    // ---- 6000-6019: Authorization / account validation ----
    #[msg("Signer does not match protocol.admin")]
    NotProtocolAdmin = 0,
    #[msg("Signer does not match position.owner")]
    NotPositionOwner,
    #[msg("Position does not belong to the supplied market")]
    PositionMarketMismatch,

    // ---- 6020-6039: Arithmetic / rounding ----
    #[msg("Arithmetic overflow")]
    ArithmeticOverflow = 20,
    #[msg("Division by zero")]
    DivisionByZero,

    // ---- 6040-6059: Oracle ----
    #[msg("This operation requires oracle-backed valuation, which does not exist before Phase 5")]
    OracleNotYetAvailable = 40,

    // ---- 6060-6079: Solvency / health ---- (unused before Phase 4; band reserved)

    // ---- 6080-6099: Liquidation ----
    #[msg("liq_threshold * (WAD + liq_bonus) / WAD must be strictly less than WAD (INV-LIQ-06)")]
    LiquidationBonusExceedsThresholdBound = 80,

    // ---- 6100-6119: Token / extension policy ----
    #[msg("Mint account owner does not match the supplied token program")]
    TokenProgramMintMismatch = 100,
    #[msg("Token-2022 extension is not on the positive allowlist")]
    UnsupportedTokenExtension,
    #[msg("A transfer-fee mint cannot be used as the loan asset")]
    TransferFeeNotAllowedForLoanAsset,
    #[msg("Mint has a freeze authority that was not acknowledged")]
    FreezeAuthorityNotAcknowledged,
    #[msg("Mint account data could not be parsed as a Token or Token-2022 mint")]
    InvalidMintAccountData,
    #[msg("Mint account does not match the market's pinned mint for this asset")]
    VaultMintMismatch,
    #[msg("Token account does not match the market's canonical vault for this asset")]
    VaultMismatch,
    #[msg("Token program does not match the market's pinned token program for this asset")]
    TokenProgramMismatch,
    #[msg("Measured post-CPI vault balance decreased instead of increasing")]
    VaultAccountingError,
    #[msg("amount must be greater than zero")]
    ZeroAmount,
    #[msg("amount exceeds position.collateral_amount")]
    InsufficientCollateral,

    // ---- 6120-6139: Configuration / bounds ----
    #[msg("guardian and fee_recipient must not be the default Pubkey")]
    DefaultPubkeyNotAllowed = 120,
    #[msg("collateral_mint and loan_mint must differ")]
    SameCollateralAndLoanMint,
    #[msg("require 0 < max_ltv < liq_threshold < WAD")]
    InvalidMaxLtvOrThreshold,
    #[msg("liq_bonus must be in [0, MAX_LIQ_BONUS]")]
    InvalidLiqBonus,
    #[msg("close_factor must be in [MIN_CLOSE_FACTOR, WAD]")]
    InvalidCloseFactor,
    #[msg("full_liq_hf must be in (0, WAD]")]
    InvalidFullLiqHf,
    #[msg("liq_protocol_fee must be in [0, MAX_LIQ_PROTOCOL_FEE]")]
    InvalidLiqProtocolFee,
    #[msg("fee must be in [0, MAX_FEE]")]
    InvalidFee,
    #[msg("min_debt must be greater than zero")]
    InvalidMinDebt,
    #[msg("IRM parameters violate 0 < u_kink < WAD, max_rate_ps > 0, or rate <= max_rate_ps")]
    InvalidIrmParams,
    #[msg("max_price_age_secs must be in [1, 3600]")]
    InvalidMaxPriceAge,
    #[msg("max_conf_bps must be in [1, 2000]")]
    InvalidMaxConfBps,

    // ---- 6140-6159: Lifecycle / state ----
    #[msg("position owner must not be the default Pubkey")]
    InvalidPositionOwner = 140,
    #[msg("close_position requires supply_shares, borrow_shares and collateral_amount to be exactly zero")]
    PositionNotEmpty,
}

impl From<aegis_math::MathError> for AegisError {
    fn from(err: aegis_math::MathError) -> Self {
        match err {
            aegis_math::MathError::Overflow => AegisError::ArithmeticOverflow,
            aegis_math::MathError::DivisionByZero => AegisError::DivisionByZero,
        }
    }
}
