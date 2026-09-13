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

    // ---- 6040-6059: Oracle (docs/oracle-design.md §2, checks O-1..O-11) ----
    #[msg("market.oracle_kind is not a recognized oracle kind")]
    OracleUnsupportedKind = 40,
    #[msg("O-1: price-update account is not owned by the Pyth receiver program")]
    OracleAccountOwnerMismatch,
    #[msg("O-2: price-update account failed to deserialize as a PriceUpdateV2")]
    OracleAccountInvalidData,
    #[msg("O-3: price-update account's feed_id does not match the market's configured feed")]
    OracleFeedMismatch,
    #[msg("O-4: price update is not fully verified (VerificationLevel::Full required)")]
    OracleVerificationLevelNotFull,
    #[msg("O-5: price update is older than market.max_price_age_secs")]
    OraclePriceStale,
    #[msg("O-6: price update's publish_time is unacceptably far in the future")]
    OraclePriceInFuture,
    #[msg("O-11: the collateral and loan price-update accounts must be distinct")]
    OracleDuplicatePriceAccounts,
    #[msg("O-7: oracle price is zero or negative")]
    OraclePriceNotPositive,
    #[msg("O-8: oracle confidence interval exceeds market.max_conf_bps")]
    OracleConfidenceTooWide,
    #[msg("O-9: confidence-adjusted price falls outside [MIN_PRICE_WAD, MAX_PRICE_WAD]")]
    OraclePriceOutOfBounds,
    #[msg("debt_value exceeds collateral_value * max_ltv / WAD after this operation")]
    ExceedsMaxLtv,

    // ---- 6060-6079: Solvency / health ----
    #[msg("Position debt after this operation must be exactly zero or at least market.min_debt")]
    DebtBelowMinimum = 60,

    // ---- 6080-6099: Liquidation ----
    #[msg("liq_threshold * (WAD + liq_bonus) / WAD must be strictly less than WAD (INV-LIQ-06)")]
    LiquidationBonusExceedsThresholdBound = 80,
    #[msg("INV-LIQ-01/INV-SOLV-02: position health factor is not strictly below WAD (HF == WAD is NOT liquidatable)")]
    NotLiquidatable,
    #[msg("repay_assets exceeds max_repay (close factor / dust rule / full-liquidation bound)")]
    RepayExceedsMaxRepay,
    #[msg("seize_collateral exceeds position.collateral_amount")]
    SeizeExceedsCollateral,
    #[msg("liquidate requires collateral_amount > 0 and borrow_shares > 0")]
    NothingToLiquidate,
    #[msg("absorb_bad_debt requires position.collateral_amount to be exactly zero")]
    BadDebtRequiresZeroCollateral,
    #[msg("absorb_bad_debt requires position.borrow_shares > 0")]
    BadDebtRequiresOutstandingDebt,
    #[msg("amount exceeds market.collateral_fee_accrued")]
    InsufficientCollateralFees,

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
    #[msg("exactly one of assets/shares must be nonzero, not both")]
    InconsistentInput,
    #[msg("requested assets exceed the market's free liquidity (total_supply_assets - total_borrow_assets)")]
    InsufficientLiquidity,
    #[msg("requested shares exceed the position's available share balance")]
    InsufficientShares,

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

    // ---- 6160-6179: Composability / callback (Phase 8, docs/composability.md, ADR-0013) ----
    #[msg("A liquidation callback is already in flight on this market (A-CPI-02, protocol-level guard)")]
    LiquidationCallbackReentrancy = 160,
    #[msg("callback_program was supplied but is not marked executable")]
    LiquidationCallbackNotExecutable,
    #[msg("callback_collateral_account is required exactly when callback_program is supplied, and its mint must match market.collateral_mint")]
    LiquidationCallbackAccountMismatch,
    #[msg("a remaining account supplied to the liquidation callback aliases a protected Aegis account (INV-AUTH-07)")]
    CallbackAccountNotPermitted,
    #[msg(
        "measured post-callback loan_vault delta is less than the required repayment (A-CPI-04)"
    )]
    LiquidationCallbackRepaymentShortfall,

    // ---- 6180-6199: Governance / pause / migration (Phase 12) ----
    #[msg("Signer does not match protocol.pending_admin, or no admin transfer is pending")]
    NotPendingAdmin = 180,
    #[msg("Signer is neither protocol.admin nor protocol.guardian")]
    NotAdminOrGuardian,
    #[msg("The guardian may only set pause bits, never clear them (INV-AUTH-04)")]
    GuardianCannotClearPause,
    #[msg("Pause flags contain a bit outside SUPPLY|BORROW|WITHDRAW|LIQUIDATE (INV-ADM-03)")]
    InvalidPauseBits,
    #[msg("This operation is paused, protocol-wide or for this market")]
    OperationPaused,
    #[msg("A risk-increasing parameter proposal is already staged for this market")]
    PendingParamsAlreadyStaged,
    #[msg("commit_pending_params called before pending_market_params.effective_at")]
    PendingParamsNotYetEffective,
    #[msg("new_fee_position is required and must belong to (market, args.fee_recipient) when fee_recipient is changing")]
    FeeRecipientPositionMissing,
    #[msg("pending_market_params.market does not match the supplied market")]
    PendingParamsMarketMismatch,
    #[msg("account is not the canonical PDA([b\"protocol\"]) singleton")]
    NotCanonicalProtocol,
}

impl From<aegis_math::MathError> for AegisError {
    fn from(err: aegis_math::MathError) -> Self {
        match err {
            aegis_math::MathError::Overflow => AegisError::ArithmeticOverflow,
            aegis_math::MathError::DivisionByZero => AegisError::DivisionByZero,
        }
    }
}

impl From<aegis_math::HealthError> for AegisError {
    fn from(err: aegis_math::HealthError) -> Self {
        match err {
            aegis_math::HealthError::Overflow => AegisError::ArithmeticOverflow,
            aegis_math::HealthError::DivisionByZero => AegisError::DivisionByZero,
            aegis_math::HealthError::PriceNotPositive => AegisError::OraclePriceNotPositive,
            aegis_math::HealthError::ConfidenceTooWide => AegisError::OracleConfidenceTooWide,
            aegis_math::HealthError::PriceOutOfBounds => AegisError::OraclePriceOutOfBounds,
        }
    }
}

impl From<aegis_math::LiquidationError> for AegisError {
    fn from(err: aegis_math::LiquidationError) -> Self {
        match err {
            aegis_math::LiquidationError::Overflow => AegisError::ArithmeticOverflow,
            aegis_math::LiquidationError::DivisionByZero => AegisError::DivisionByZero,
            aegis_math::LiquidationError::ZeroRepay => AegisError::ZeroAmount,
            aegis_math::LiquidationError::RepayExceedsMaxRepay => AegisError::RepayExceedsMaxRepay,
            aegis_math::LiquidationError::SeizeExceedsCollateral => {
                AegisError::SeizeExceedsCollateral
            }
        }
    }
}
