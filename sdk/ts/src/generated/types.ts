// GENERATED FILE -- do not edit by hand.
// Produced by sdk/ts/scripts/codegen.mjs from target/idl/aegis.json.
// Re-run `npm run codegen` (sdk/ts/) after `anchor build` changes the IDL; `npm run codegen:check` detects a stale commit.


import type { Address } from '@solana/kit';

import type { FieldSpec } from '../borshValues.js';


/**
 * `set_pending_admin` (`instruction-catalogue.md` §2-5, INV-ADM-02).
 */
export interface AdminTransferStarted {
  protocol: Address;
  currentAdmin: Address;
  pendingAdmin: Address;
}

export const ADMINTRANSFERSTARTED_FIELDS: readonly FieldSpec[] = [
  { rawName: "protocol", camelName: "protocol", kind: "pubkey" },
  { rawName: "current_admin", camelName: "currentAdmin", kind: "pubkey" },
  { rawName: "pending_admin", camelName: "pendingAdmin", kind: "pubkey" },
];

/**
 * `accept_admin`. `old_admin` is the admin that just lost authority.
 */
export interface AdminTransferred {
  protocol: Address;
  oldAdmin: Address;
  newAdmin: Address;
}

export const ADMINTRANSFERRED_FIELDS: readonly FieldSpec[] = [
  { rawName: "protocol", camelName: "protocol", kind: "pubkey" },
  { rawName: "old_admin", camelName: "oldAdmin", kind: "pubkey" },
  { rawName: "new_admin", camelName: "newAdmin", kind: "pubkey" },
];

/**
 * `economic-model.md` §8.2, `instruction-catalogue.md` §18. `absorbed_by_protocol` is the debt
 * absorbed by burning `fee_position.supply_shares` (protocol first-loss); `socialized` is the
 * residual left to fall on `total_supply_assets`/lenders. `absorbed_by_protocol + socialized ==
 * bad_assets` always.
 */
export interface BadDebtAbsorbed {
  market: Address;
  position: Address;
  badAssets: bigint;
  absorbedByProtocol: bigint;
  socialized: bigint;
  feeSharesBurned: bigint;
}

export const BADDEBTABSORBED_FIELDS: readonly FieldSpec[] = [
  { rawName: "market", camelName: "market", kind: "pubkey" },
  { rawName: "position", camelName: "position", kind: "pubkey" },
  { rawName: "bad_assets", camelName: "badAssets", kind: "u64" },
  { rawName: "absorbed_by_protocol", camelName: "absorbedByProtocol", kind: "u64" },
  { rawName: "socialized", camelName: "socialized", kind: "u64" },
  { rawName: "fee_shares_burned", camelName: "feeSharesBurned", kind: "u128" },
];

/**
 * Defined for API completeness against `instruction-catalogue.md`'s event catalogue; never
 * emitted by Phase 4's `borrow` handler, which is hard-gated to always fail before any state
 * transition (`docs/phases/phase-04-lending.md` #17: "Do not emit `Borrowed` for the hard-gated
 * unsuccessful borrow path").
 */
export interface Borrowed {
  market: Address;
  position: Address;
  owner: Address;
  assetsOut: bigint;
  sharesMinted: bigint;
}

export const BORROWED_FIELDS: readonly FieldSpec[] = [
  { rawName: "market", camelName: "market", kind: "pubkey" },
  { rawName: "position", camelName: "position", kind: "pubkey" },
  { rawName: "owner", camelName: "owner", kind: "pubkey" },
  { rawName: "assets_out", camelName: "assetsOut", kind: "u64" },
  { rawName: "shares_minted", camelName: "sharesMinted", kind: "u128" },
];

/**
 * `amount_in` is the requested transfer amount; `credited` is the measured post-CPI delta
 * actually recorded against `position.collateral_amount` (`account-model.md` §6.4). The two
 * differ exactly when the collateral mint charges a Token-2022 transfer fee.
 */
export interface CollateralDeposited {
  market: Address;
  position: Address;
  depositor: Address;
  amountIn: bigint;
  credited: bigint;
}

export const COLLATERALDEPOSITED_FIELDS: readonly FieldSpec[] = [
  { rawName: "market", camelName: "market", kind: "pubkey" },
  { rawName: "position", camelName: "position", kind: "pubkey" },
  { rawName: "depositor", camelName: "depositor", kind: "pubkey" },
  { rawName: "amount_in", camelName: "amountIn", kind: "u64" },
  { rawName: "credited", camelName: "credited", kind: "u64" },
];

/**
 * `instruction-catalogue.md` §19.
 */
export interface CollateralFeesWithdrawn {
  market: Address;
  admin: Address;
  amount: bigint;
  remainingCollateralFeeAccrued: bigint;
}

export const COLLATERALFEESWITHDRAWN_FIELDS: readonly FieldSpec[] = [
  { rawName: "market", camelName: "market", kind: "pubkey" },
  { rawName: "admin", camelName: "admin", kind: "pubkey" },
  { rawName: "amount", camelName: "amount", kind: "u64" },
  { rawName: "remaining_collateral_fee_accrued", camelName: "remainingCollateralFeeAccrued", kind: "u64" },
];

export interface CollateralWithdrawn {
  market: Address;
  position: Address;
  owner: Address;
  amount: bigint;
}

export const COLLATERALWITHDRAWN_FIELDS: readonly FieldSpec[] = [
  { rawName: "market", camelName: "market", kind: "pubkey" },
  { rawName: "position", camelName: "position", kind: "pubkey" },
  { rawName: "owner", camelName: "owner", kind: "pubkey" },
  { rawName: "amount", camelName: "amount", kind: "u64" },
];

export interface CreateMarketArgs {
  configId: number;
  oracleKind: number;
  collateralFeedId: Uint8Array;
  loanFeedId: Uint8Array;
  maxPriceAgeSecs: number;
  maxConfBps: number;
  maxLtv: bigint;
  liqThreshold: bigint;
  liqBonus: bigint;
  closeFactor: bigint;
  fullLiqHf: bigint;
  liqProtocolFee: bigint;
  fee: bigint;
  minDebt: bigint;
  baseRatePs: bigint;
  slope1Ps: bigint;
  slope2Ps: bigint;
  uKink: bigint;
  maxRatePs: bigint;
  ackFreezeAuthority: boolean;
}

export const CREATEMARKETARGS_FIELDS: readonly FieldSpec[] = [
  { rawName: "config_id", camelName: "configId", kind: "u16" },
  { rawName: "oracle_kind", camelName: "oracleKind", kind: "u8" },
  { rawName: "collateral_feed_id", camelName: "collateralFeedId", kind: "bytesN", len: 32 },
  { rawName: "loan_feed_id", camelName: "loanFeedId", kind: "bytesN", len: 32 },
  { rawName: "max_price_age_secs", camelName: "maxPriceAgeSecs", kind: "u32" },
  { rawName: "max_conf_bps", camelName: "maxConfBps", kind: "u16" },
  { rawName: "max_ltv", camelName: "maxLtv", kind: "u128" },
  { rawName: "liq_threshold", camelName: "liqThreshold", kind: "u128" },
  { rawName: "liq_bonus", camelName: "liqBonus", kind: "u128" },
  { rawName: "close_factor", camelName: "closeFactor", kind: "u128" },
  { rawName: "full_liq_hf", camelName: "fullLiqHf", kind: "u128" },
  { rawName: "liq_protocol_fee", camelName: "liqProtocolFee", kind: "u128" },
  { rawName: "fee", camelName: "fee", kind: "u128" },
  { rawName: "min_debt", camelName: "minDebt", kind: "u64" },
  { rawName: "base_rate_ps", camelName: "baseRatePs", kind: "u128" },
  { rawName: "slope1_ps", camelName: "slope1Ps", kind: "u128" },
  { rawName: "slope2_ps", camelName: "slope2Ps", kind: "u128" },
  { rawName: "u_kink", camelName: "uKink", kind: "u128" },
  { rawName: "max_rate_ps", camelName: "maxRatePs", kind: "u128" },
  { rawName: "ack_freeze_authority", camelName: "ackFreezeAuthority", kind: "bool" },
];

/**
 * `set_guardian`.
 */
export interface GuardianChanged {
  protocol: Address;
  admin: Address;
  oldGuardian: Address;
  newGuardian: Address;
}

export const GUARDIANCHANGED_FIELDS: readonly FieldSpec[] = [
  { rawName: "protocol", camelName: "protocol", kind: "pubkey" },
  { rawName: "admin", camelName: "admin", kind: "pubkey" },
  { rawName: "old_guardian", camelName: "oldGuardian", kind: "pubkey" },
  { rawName: "new_guardian", camelName: "newGuardian", kind: "pubkey" },
];

export interface InitProtocolArgs {
  guardian: Address;
  feeRecipient: Address;
}

export const INITPROTOCOLARGS_FIELDS: readonly FieldSpec[] = [
  { rawName: "guardian", camelName: "guardian", kind: "pubkey" },
  { rawName: "fee_recipient", camelName: "feeRecipient", kind: "pubkey" },
];

export interface InterestAccrued {
  market: Address;
  interest: bigint;
  feeAmount: bigint;
  feeShares: bigint;
  totalBorrowAssets: bigint;
  totalSupplyAssets: bigint;
}

export const INTERESTACCRUED_FIELDS: readonly FieldSpec[] = [
  { rawName: "market", camelName: "market", kind: "pubkey" },
  { rawName: "interest", camelName: "interest", kind: "u64" },
  { rawName: "fee_amount", camelName: "feeAmount", kind: "u64" },
  { rawName: "fee_shares", camelName: "feeShares", kind: "u128" },
  { rawName: "total_borrow_assets", camelName: "totalBorrowAssets", kind: "u64" },
  { rawName: "total_supply_assets", camelName: "totalSupplyAssets", kind: "u64" },
];

/**
 * `economic-model.md` §7.3, `instruction-catalogue.md` §17. `hf_before`/`hf_after` are the
 * health factor immediately before and after this liquidation (`P-LIQ-1`'s on-chain evidence
 * trail); `clamped` records whether the collateral-clamp path (`economic-model.md` §7.2) fired.
 * `callback_program` is `None` for the Phase 6 path (`I-LIQ-CB-02`) and `Some(program_id)` when
 * the Phase 8 callback branch ran — part of the permanent audit record of which liquidations used
 * external composability (docs/composability.md).
 */
export interface Liquidated {
  market: Address;
  position: Address;
  liquidator: Address;
  repayAssets: bigint;
  repayShares: bigint;
  baseSeize: bigint;
  totalSeize: bigint;
  bonusAmount: bigint;
  protocolCut: bigint;
  toLiquidator: bigint;
  clamped: boolean;
  hfBefore: bigint;
  hfAfter: bigint;
  callbackProgram: Address | null;
}

export const LIQUIDATED_FIELDS: readonly FieldSpec[] = [
  { rawName: "market", camelName: "market", kind: "pubkey" },
  { rawName: "position", camelName: "position", kind: "pubkey" },
  { rawName: "liquidator", camelName: "liquidator", kind: "pubkey" },
  { rawName: "repay_assets", camelName: "repayAssets", kind: "u64" },
  { rawName: "repay_shares", camelName: "repayShares", kind: "u128" },
  { rawName: "base_seize", camelName: "baseSeize", kind: "u64" },
  { rawName: "total_seize", camelName: "totalSeize", kind: "u64" },
  { rawName: "bonus_amount", camelName: "bonusAmount", kind: "u64" },
  { rawName: "protocol_cut", camelName: "protocolCut", kind: "u64" },
  { rawName: "to_liquidator", camelName: "toLiquidator", kind: "u64" },
  { rawName: "clamped", camelName: "clamped", kind: "bool" },
  { rawName: "hf_before", camelName: "hfBefore", kind: "u128" },
  { rawName: "hf_after", camelName: "hfAfter", kind: "u128" },
  { rawName: "callback_program", camelName: "callbackProgram", kind: "option", inner: { kind: "pubkey" } },
];

export interface Market {
  collateralMint: Address;
  loanMint: Address;
  collateralTokenProgram: Address;
  loanTokenProgram: Address;
  collateralVault: Address;
  loanVault: Address;
  feeRecipient: Address;
  configId: number;
  collateralDecimals: number;
  loanDecimals: number;
  oracleKind: number;
  collateralFeedId: Uint8Array;
  loanFeedId: Uint8Array;
  maxPriceAgeSecs: number;
  maxConfBps: number;
  maxLtv: bigint;
  liqThreshold: bigint;
  liqBonus: bigint;
  closeFactor: bigint;
  fullLiqHf: bigint;
  liqProtocolFee: bigint;
  fee: bigint;
  minDebt: bigint;
  baseRatePs: bigint;
  slope1Ps: bigint;
  slope2Ps: bigint;
  uKink: bigint;
  maxRatePs: bigint;
  totalSupplyAssets: bigint;
  totalSupplyShares: bigint;
  totalBorrowAssets: bigint;
  totalBorrowShares: bigint;
  collateralFeeAccrued: bigint;
  lastAccrualTs: bigint;
  paused: number;
  flags: number;
  bump: number;
  collateralVaultBump: number;
  loanVaultBump: number;
  liquidationGuard: number;
  Reserved: Uint8Array;
}

export const MARKET_FIELDS: readonly FieldSpec[] = [
  { rawName: "collateral_mint", camelName: "collateralMint", kind: "pubkey" },
  { rawName: "loan_mint", camelName: "loanMint", kind: "pubkey" },
  { rawName: "collateral_token_program", camelName: "collateralTokenProgram", kind: "pubkey" },
  { rawName: "loan_token_program", camelName: "loanTokenProgram", kind: "pubkey" },
  { rawName: "collateral_vault", camelName: "collateralVault", kind: "pubkey" },
  { rawName: "loan_vault", camelName: "loanVault", kind: "pubkey" },
  { rawName: "fee_recipient", camelName: "feeRecipient", kind: "pubkey" },
  { rawName: "config_id", camelName: "configId", kind: "u16" },
  { rawName: "collateral_decimals", camelName: "collateralDecimals", kind: "u8" },
  { rawName: "loan_decimals", camelName: "loanDecimals", kind: "u8" },
  { rawName: "oracle_kind", camelName: "oracleKind", kind: "u8" },
  { rawName: "collateral_feed_id", camelName: "collateralFeedId", kind: "bytesN", len: 32 },
  { rawName: "loan_feed_id", camelName: "loanFeedId", kind: "bytesN", len: 32 },
  { rawName: "max_price_age_secs", camelName: "maxPriceAgeSecs", kind: "u32" },
  { rawName: "max_conf_bps", camelName: "maxConfBps", kind: "u16" },
  { rawName: "max_ltv", camelName: "maxLtv", kind: "u128" },
  { rawName: "liq_threshold", camelName: "liqThreshold", kind: "u128" },
  { rawName: "liq_bonus", camelName: "liqBonus", kind: "u128" },
  { rawName: "close_factor", camelName: "closeFactor", kind: "u128" },
  { rawName: "full_liq_hf", camelName: "fullLiqHf", kind: "u128" },
  { rawName: "liq_protocol_fee", camelName: "liqProtocolFee", kind: "u128" },
  { rawName: "fee", camelName: "fee", kind: "u128" },
  { rawName: "min_debt", camelName: "minDebt", kind: "u64" },
  { rawName: "base_rate_ps", camelName: "baseRatePs", kind: "u128" },
  { rawName: "slope1_ps", camelName: "slope1Ps", kind: "u128" },
  { rawName: "slope2_ps", camelName: "slope2Ps", kind: "u128" },
  { rawName: "u_kink", camelName: "uKink", kind: "u128" },
  { rawName: "max_rate_ps", camelName: "maxRatePs", kind: "u128" },
  { rawName: "total_supply_assets", camelName: "totalSupplyAssets", kind: "u64" },
  { rawName: "total_supply_shares", camelName: "totalSupplyShares", kind: "u128" },
  { rawName: "total_borrow_assets", camelName: "totalBorrowAssets", kind: "u64" },
  { rawName: "total_borrow_shares", camelName: "totalBorrowShares", kind: "u128" },
  { rawName: "collateral_fee_accrued", camelName: "collateralFeeAccrued", kind: "u64" },
  { rawName: "last_accrual_ts", camelName: "lastAccrualTs", kind: "i64" },
  { rawName: "paused", camelName: "paused", kind: "u8" },
  { rawName: "flags", camelName: "flags", kind: "u8" },
  { rawName: "bump", camelName: "bump", kind: "u8" },
  { rawName: "collateral_vault_bump", camelName: "collateralVaultBump", kind: "u8" },
  { rawName: "loan_vault_bump", camelName: "loanVaultBump", kind: "u8" },
  { rawName: "liquidation_guard", camelName: "liquidationGuard", kind: "u8" },
  { rawName: "_reserved", camelName: "Reserved", kind: "bytesN", len: 63 },
];

/**
 * The full parameter snapshot for a newly created market — the permanent audit record of the
 * market's risk configuration and the exact Token-2022 extension inventory that was accepted for
 * each mint (`token-compatibility.md` §6 step 7).
 */
export interface MarketCreated {
  market: Address;
  collateralMint: Address;
  loanMint: Address;
  collateralTokenProgram: Address;
  loanTokenProgram: Address;
  collateralVault: Address;
  loanVault: Address;
  feeRecipient: Address;
  feePosition: Address;
  configId: number;
  collateralDecimals: number;
  loanDecimals: number;
  oracleKind: number;
  collateralFeedId: Uint8Array;
  loanFeedId: Uint8Array;
  maxPriceAgeSecs: number;
  maxConfBps: number;
  maxLtv: bigint;
  liqThreshold: bigint;
  liqBonus: bigint;
  closeFactor: bigint;
  fullLiqHf: bigint;
  liqProtocolFee: bigint;
  fee: bigint;
  minDebt: bigint;
  baseRatePs: bigint;
  slope1Ps: bigint;
  slope2Ps: bigint;
  uKink: bigint;
  maxRatePs: bigint;
  flags: number;
  collateralExtensions: number[];
  loanExtensions: number[];
}

export const MARKETCREATED_FIELDS: readonly FieldSpec[] = [
  { rawName: "market", camelName: "market", kind: "pubkey" },
  { rawName: "collateral_mint", camelName: "collateralMint", kind: "pubkey" },
  { rawName: "loan_mint", camelName: "loanMint", kind: "pubkey" },
  { rawName: "collateral_token_program", camelName: "collateralTokenProgram", kind: "pubkey" },
  { rawName: "loan_token_program", camelName: "loanTokenProgram", kind: "pubkey" },
  { rawName: "collateral_vault", camelName: "collateralVault", kind: "pubkey" },
  { rawName: "loan_vault", camelName: "loanVault", kind: "pubkey" },
  { rawName: "fee_recipient", camelName: "feeRecipient", kind: "pubkey" },
  { rawName: "fee_position", camelName: "feePosition", kind: "pubkey" },
  { rawName: "config_id", camelName: "configId", kind: "u16" },
  { rawName: "collateral_decimals", camelName: "collateralDecimals", kind: "u8" },
  { rawName: "loan_decimals", camelName: "loanDecimals", kind: "u8" },
  { rawName: "oracle_kind", camelName: "oracleKind", kind: "u8" },
  { rawName: "collateral_feed_id", camelName: "collateralFeedId", kind: "bytesN", len: 32 },
  { rawName: "loan_feed_id", camelName: "loanFeedId", kind: "bytesN", len: 32 },
  { rawName: "max_price_age_secs", camelName: "maxPriceAgeSecs", kind: "u32" },
  { rawName: "max_conf_bps", camelName: "maxConfBps", kind: "u16" },
  { rawName: "max_ltv", camelName: "maxLtv", kind: "u128" },
  { rawName: "liq_threshold", camelName: "liqThreshold", kind: "u128" },
  { rawName: "liq_bonus", camelName: "liqBonus", kind: "u128" },
  { rawName: "close_factor", camelName: "closeFactor", kind: "u128" },
  { rawName: "full_liq_hf", camelName: "fullLiqHf", kind: "u128" },
  { rawName: "liq_protocol_fee", camelName: "liqProtocolFee", kind: "u128" },
  { rawName: "fee", camelName: "fee", kind: "u128" },
  { rawName: "min_debt", camelName: "minDebt", kind: "u64" },
  { rawName: "base_rate_ps", camelName: "baseRatePs", kind: "u128" },
  { rawName: "slope1_ps", camelName: "slope1Ps", kind: "u128" },
  { rawName: "slope2_ps", camelName: "slope2Ps", kind: "u128" },
  { rawName: "u_kink", camelName: "uKink", kind: "u128" },
  { rawName: "max_rate_ps", camelName: "maxRatePs", kind: "u128" },
  { rawName: "flags", camelName: "flags", kind: "u8" },
  { rawName: "collateral_extensions", camelName: "collateralExtensions", kind: "vec", inner: { kind: "u16" } },
  { rawName: "loan_extensions", camelName: "loanExtensions", kind: "vec", inner: { kind: "u16" } },
];

/**
 * `set_market_params`'s immediate-application path (tightening, or a pure `fee_recipient`
 * change) — a full before/after snapshot of every field the instruction is capable of touching,
 * mirroring `MarketCreated`'s audit-record style (`instruction-catalogue.md` §7).
 */
export interface MarketParamsUpdated {
  market: Address;
  admin: Address;
  oldFeeRecipient: Address;
  newFeeRecipient: Address;
  oldOracleKind: number;
  newOracleKind: number;
  oldCollateralFeedId: Uint8Array;
  newCollateralFeedId: Uint8Array;
  oldLoanFeedId: Uint8Array;
  newLoanFeedId: Uint8Array;
  oldMaxPriceAgeSecs: number;
  newMaxPriceAgeSecs: number;
  oldMaxConfBps: number;
  newMaxConfBps: number;
  oldMaxLtv: bigint;
  newMaxLtv: bigint;
  oldLiqThreshold: bigint;
  newLiqThreshold: bigint;
  oldLiqBonus: bigint;
  newLiqBonus: bigint;
  oldCloseFactor: bigint;
  newCloseFactor: bigint;
  oldFullLiqHf: bigint;
  newFullLiqHf: bigint;
  oldLiqProtocolFee: bigint;
  newLiqProtocolFee: bigint;
  oldFee: bigint;
  newFee: bigint;
  oldMinDebt: bigint;
  newMinDebt: bigint;
  oldBaseRatePs: bigint;
  newBaseRatePs: bigint;
  oldSlope1Ps: bigint;
  newSlope1Ps: bigint;
  oldSlope2Ps: bigint;
  newSlope2Ps: bigint;
  oldUKink: bigint;
  newUKink: bigint;
  oldMaxRatePs: bigint;
  newMaxRatePs: bigint;
}

export const MARKETPARAMSUPDATED_FIELDS: readonly FieldSpec[] = [
  { rawName: "market", camelName: "market", kind: "pubkey" },
  { rawName: "admin", camelName: "admin", kind: "pubkey" },
  { rawName: "old_fee_recipient", camelName: "oldFeeRecipient", kind: "pubkey" },
  { rawName: "new_fee_recipient", camelName: "newFeeRecipient", kind: "pubkey" },
  { rawName: "old_oracle_kind", camelName: "oldOracleKind", kind: "u8" },
  { rawName: "new_oracle_kind", camelName: "newOracleKind", kind: "u8" },
  { rawName: "old_collateral_feed_id", camelName: "oldCollateralFeedId", kind: "bytesN", len: 32 },
  { rawName: "new_collateral_feed_id", camelName: "newCollateralFeedId", kind: "bytesN", len: 32 },
  { rawName: "old_loan_feed_id", camelName: "oldLoanFeedId", kind: "bytesN", len: 32 },
  { rawName: "new_loan_feed_id", camelName: "newLoanFeedId", kind: "bytesN", len: 32 },
  { rawName: "old_max_price_age_secs", camelName: "oldMaxPriceAgeSecs", kind: "u32" },
  { rawName: "new_max_price_age_secs", camelName: "newMaxPriceAgeSecs", kind: "u32" },
  { rawName: "old_max_conf_bps", camelName: "oldMaxConfBps", kind: "u16" },
  { rawName: "new_max_conf_bps", camelName: "newMaxConfBps", kind: "u16" },
  { rawName: "old_max_ltv", camelName: "oldMaxLtv", kind: "u128" },
  { rawName: "new_max_ltv", camelName: "newMaxLtv", kind: "u128" },
  { rawName: "old_liq_threshold", camelName: "oldLiqThreshold", kind: "u128" },
  { rawName: "new_liq_threshold", camelName: "newLiqThreshold", kind: "u128" },
  { rawName: "old_liq_bonus", camelName: "oldLiqBonus", kind: "u128" },
  { rawName: "new_liq_bonus", camelName: "newLiqBonus", kind: "u128" },
  { rawName: "old_close_factor", camelName: "oldCloseFactor", kind: "u128" },
  { rawName: "new_close_factor", camelName: "newCloseFactor", kind: "u128" },
  { rawName: "old_full_liq_hf", camelName: "oldFullLiqHf", kind: "u128" },
  { rawName: "new_full_liq_hf", camelName: "newFullLiqHf", kind: "u128" },
  { rawName: "old_liq_protocol_fee", camelName: "oldLiqProtocolFee", kind: "u128" },
  { rawName: "new_liq_protocol_fee", camelName: "newLiqProtocolFee", kind: "u128" },
  { rawName: "old_fee", camelName: "oldFee", kind: "u128" },
  { rawName: "new_fee", camelName: "newFee", kind: "u128" },
  { rawName: "old_min_debt", camelName: "oldMinDebt", kind: "u64" },
  { rawName: "new_min_debt", camelName: "newMinDebt", kind: "u64" },
  { rawName: "old_base_rate_ps", camelName: "oldBaseRatePs", kind: "u128" },
  { rawName: "new_base_rate_ps", camelName: "newBaseRatePs", kind: "u128" },
  { rawName: "old_slope1_ps", camelName: "oldSlope1Ps", kind: "u128" },
  { rawName: "new_slope1_ps", camelName: "newSlope1Ps", kind: "u128" },
  { rawName: "old_slope2_ps", camelName: "oldSlope2Ps", kind: "u128" },
  { rawName: "new_slope2_ps", camelName: "newSlope2Ps", kind: "u128" },
  { rawName: "old_u_kink", camelName: "oldUKink", kind: "u128" },
  { rawName: "new_u_kink", camelName: "newUKink", kind: "u128" },
  { rawName: "old_max_rate_ps", camelName: "oldMaxRatePs", kind: "u128" },
  { rawName: "new_max_rate_ps", camelName: "newMaxRatePs", kind: "u128" },
];

/**
 * `set_market_pause`.
 */
export interface MarketPauseSet {
  market: Address;
  authority: Address;
  oldPaused: number;
  newPaused: number;
}

export const MARKETPAUSESET_FIELDS: readonly FieldSpec[] = [
  { rawName: "market", camelName: "market", kind: "pubkey" },
  { rawName: "authority", camelName: "authority", kind: "pubkey" },
  { rawName: "old_paused", camelName: "oldPaused", kind: "u8" },
  { rawName: "new_paused", camelName: "newPaused", kind: "u8" },
];

/**
 * `set_market_params`'s staged path (a risk-increasing/"loosening" change): the proposal is
 * recorded, not applied. Carries the same field set as [`MarketParamsUpdated`] minus
 * `fee_recipient` (which is never staged — it is orthogonal to risk and always applies
 * immediately) plus `effective_at`.
 */
export interface ParamsStaged {
  market: Address;
  pendingMarketParams: Address;
  admin: Address;
  effectiveAt: bigint;
  oracleKind: number;
  collateralFeedId: Uint8Array;
  loanFeedId: Uint8Array;
  maxPriceAgeSecs: number;
  maxConfBps: number;
  maxLtv: bigint;
  liqThreshold: bigint;
  liqBonus: bigint;
  closeFactor: bigint;
  fullLiqHf: bigint;
  liqProtocolFee: bigint;
  fee: bigint;
  minDebt: bigint;
  baseRatePs: bigint;
  slope1Ps: bigint;
  slope2Ps: bigint;
  uKink: bigint;
  maxRatePs: bigint;
}

export const PARAMSSTAGED_FIELDS: readonly FieldSpec[] = [
  { rawName: "market", camelName: "market", kind: "pubkey" },
  { rawName: "pending_market_params", camelName: "pendingMarketParams", kind: "pubkey" },
  { rawName: "admin", camelName: "admin", kind: "pubkey" },
  { rawName: "effective_at", camelName: "effectiveAt", kind: "i64" },
  { rawName: "oracle_kind", camelName: "oracleKind", kind: "u8" },
  { rawName: "collateral_feed_id", camelName: "collateralFeedId", kind: "bytesN", len: 32 },
  { rawName: "loan_feed_id", camelName: "loanFeedId", kind: "bytesN", len: 32 },
  { rawName: "max_price_age_secs", camelName: "maxPriceAgeSecs", kind: "u32" },
  { rawName: "max_conf_bps", camelName: "maxConfBps", kind: "u16" },
  { rawName: "max_ltv", camelName: "maxLtv", kind: "u128" },
  { rawName: "liq_threshold", camelName: "liqThreshold", kind: "u128" },
  { rawName: "liq_bonus", camelName: "liqBonus", kind: "u128" },
  { rawName: "close_factor", camelName: "closeFactor", kind: "u128" },
  { rawName: "full_liq_hf", camelName: "fullLiqHf", kind: "u128" },
  { rawName: "liq_protocol_fee", camelName: "liqProtocolFee", kind: "u128" },
  { rawName: "fee", camelName: "fee", kind: "u128" },
  { rawName: "min_debt", camelName: "minDebt", kind: "u64" },
  { rawName: "base_rate_ps", camelName: "baseRatePs", kind: "u128" },
  { rawName: "slope1_ps", camelName: "slope1Ps", kind: "u128" },
  { rawName: "slope2_ps", camelName: "slope2Ps", kind: "u128" },
  { rawName: "u_kink", camelName: "uKink", kind: "u128" },
  { rawName: "max_rate_ps", camelName: "maxRatePs", kind: "u128" },
];

/**
 * Phase 12 (`governance.md` §4, INV-ADM-09, ADR-0014): a risk-increasing ("loosening")
 * `set_market_params` change, staged behind `constants::PARAM_TIMELOCK_SECS` rather than applied
 * immediately. One per `Market` (`PDA([b"pending_params", market])`), created by
 * `set_market_params` only on the loosening path and closed by `commit_pending_params`.
 * 
 * Deliberately **does not** carry `fee_recipient`: that field is not a risk parameter (it does
 * not appear in either row of `governance.md` §4's tighten/loosen table) and always applies
 * immediately in the same `set_market_params` call that staged this proposal, orthogonal to
 * whatever else is pending.
 */
export interface PendingMarketParams {
  market: Address;
  effectiveAt: bigint;
  oracleKind: number;
  collateralFeedId: Uint8Array;
  loanFeedId: Uint8Array;
  maxPriceAgeSecs: number;
  maxConfBps: number;
  maxLtv: bigint;
  liqThreshold: bigint;
  liqBonus: bigint;
  closeFactor: bigint;
  fullLiqHf: bigint;
  liqProtocolFee: bigint;
  fee: bigint;
  minDebt: bigint;
  baseRatePs: bigint;
  slope1Ps: bigint;
  slope2Ps: bigint;
  uKink: bigint;
  maxRatePs: bigint;
  bump: number;
  Reserved: Uint8Array;
}

export const PENDINGMARKETPARAMS_FIELDS: readonly FieldSpec[] = [
  { rawName: "market", camelName: "market", kind: "pubkey" },
  { rawName: "effective_at", camelName: "effectiveAt", kind: "i64" },
  { rawName: "oracle_kind", camelName: "oracleKind", kind: "u8" },
  { rawName: "collateral_feed_id", camelName: "collateralFeedId", kind: "bytesN", len: 32 },
  { rawName: "loan_feed_id", camelName: "loanFeedId", kind: "bytesN", len: 32 },
  { rawName: "max_price_age_secs", camelName: "maxPriceAgeSecs", kind: "u32" },
  { rawName: "max_conf_bps", camelName: "maxConfBps", kind: "u16" },
  { rawName: "max_ltv", camelName: "maxLtv", kind: "u128" },
  { rawName: "liq_threshold", camelName: "liqThreshold", kind: "u128" },
  { rawName: "liq_bonus", camelName: "liqBonus", kind: "u128" },
  { rawName: "close_factor", camelName: "closeFactor", kind: "u128" },
  { rawName: "full_liq_hf", camelName: "fullLiqHf", kind: "u128" },
  { rawName: "liq_protocol_fee", camelName: "liqProtocolFee", kind: "u128" },
  { rawName: "fee", camelName: "fee", kind: "u128" },
  { rawName: "min_debt", camelName: "minDebt", kind: "u64" },
  { rawName: "base_rate_ps", camelName: "baseRatePs", kind: "u128" },
  { rawName: "slope1_ps", camelName: "slope1Ps", kind: "u128" },
  { rawName: "slope2_ps", camelName: "slope2Ps", kind: "u128" },
  { rawName: "u_kink", camelName: "uKink", kind: "u128" },
  { rawName: "max_rate_ps", camelName: "maxRatePs", kind: "u128" },
  { rawName: "bump", camelName: "bump", kind: "u8" },
  { rawName: "_reserved", camelName: "Reserved", kind: "bytesN", len: 32 },
];

export interface Position {
  market: Address;
  owner: Address;
  supplyShares: bigint;
  borrowShares: bigint;
  collateralAmount: bigint;
  bump: number;
  Reserved: Uint8Array;
}

export const POSITION_FIELDS: readonly FieldSpec[] = [
  { rawName: "market", camelName: "market", kind: "pubkey" },
  { rawName: "owner", camelName: "owner", kind: "pubkey" },
  { rawName: "supply_shares", camelName: "supplyShares", kind: "u128" },
  { rawName: "borrow_shares", camelName: "borrowShares", kind: "u128" },
  { rawName: "collateral_amount", camelName: "collateralAmount", kind: "u64" },
  { rawName: "bump", camelName: "bump", kind: "u8" },
  { rawName: "_reserved", camelName: "Reserved", kind: "bytesN", len: 32 },
];

export interface PositionClosed {
  market: Address;
  position: Address;
  owner: Address;
}

export const POSITIONCLOSED_FIELDS: readonly FieldSpec[] = [
  { rawName: "market", camelName: "market", kind: "pubkey" },
  { rawName: "position", camelName: "position", kind: "pubkey" },
  { rawName: "owner", camelName: "owner", kind: "pubkey" },
];

export interface PositionInitialized {
  market: Address;
  position: Address;
  owner: Address;
}

export const POSITIONINITIALIZED_FIELDS: readonly FieldSpec[] = [
  { rawName: "market", camelName: "market", kind: "pubkey" },
  { rawName: "position", camelName: "position", kind: "pubkey" },
  { rawName: "owner", camelName: "owner", kind: "pubkey" },
];

/**
 * Phase 12 (`INV-UPG-01..03`, ADR-0014): the current live schema. `schema_version` is the
 * explicit version marker required by the migration design (distinct from, and in addition to,
 * the Anchor discriminator that already distinguishes this type from [`ProtocolV1`] at the byte
 * level) — `1` for every account created by `migrate_protocol_v2` or by `initialize_protocol`
 * from this point forward.
 */
export interface Protocol {
  admin: Address;
  pendingAdmin: Address;
  guardian: Address;
  feeRecipient: Address;
  paused: number;
  bump: number;
  schemaVersion: number;
  Reserved: Uint8Array;
}

export const PROTOCOL_FIELDS: readonly FieldSpec[] = [
  { rawName: "admin", camelName: "admin", kind: "pubkey" },
  { rawName: "pending_admin", camelName: "pendingAdmin", kind: "pubkey" },
  { rawName: "guardian", camelName: "guardian", kind: "pubkey" },
  { rawName: "fee_recipient", camelName: "feeRecipient", kind: "pubkey" },
  { rawName: "paused", camelName: "paused", kind: "u8" },
  { rawName: "bump", camelName: "bump", kind: "u8" },
  { rawName: "schema_version", camelName: "schemaVersion", kind: "u8" },
  { rawName: "_reserved", camelName: "Reserved", kind: "bytesN", len: 63 },
];

export interface ProtocolInitialized {
  protocol: Address;
  admin: Address;
  guardian: Address;
  feeRecipient: Address;
}

export const PROTOCOLINITIALIZED_FIELDS: readonly FieldSpec[] = [
  { rawName: "protocol", camelName: "protocol", kind: "pubkey" },
  { rawName: "admin", camelName: "admin", kind: "pubkey" },
  { rawName: "guardian", camelName: "guardian", kind: "pubkey" },
  { rawName: "fee_recipient", camelName: "feeRecipient", kind: "pubkey" },
];

/**
 * `migrate_protocol_v2` (`INV-UPG-01..03`, ADR-0014). Emitted once, after a successful
 * `Migration<'info, ProtocolV1, Protocol>::migrate` call.
 */
export interface ProtocolMigrated {
  protocol: Address;
  admin: Address;
  schemaVersion: number;
}

export const PROTOCOLMIGRATED_FIELDS: readonly FieldSpec[] = [
  { rawName: "protocol", camelName: "protocol", kind: "pubkey" },
  { rawName: "admin", camelName: "admin", kind: "pubkey" },
  { rawName: "schema_version", camelName: "schemaVersion", kind: "u8" },
];

/**
 * `set_protocol_pause`. `authority` is whichever of `admin`/`guardian` signed.
 */
export interface ProtocolPauseSet {
  protocol: Address;
  authority: Address;
  oldPaused: number;
  newPaused: number;
}

export const PROTOCOLPAUSESET_FIELDS: readonly FieldSpec[] = [
  { rawName: "protocol", camelName: "protocol", kind: "pubkey" },
  { rawName: "authority", camelName: "authority", kind: "pubkey" },
  { rawName: "old_paused", camelName: "oldPaused", kind: "u8" },
  { rawName: "new_paused", camelName: "newPaused", kind: "u8" },
];

export interface Repaid {
  market: Address;
  position: Address;
  payer: Address;
  assetsIn: bigint;
  credited: bigint;
  sharesBurned: bigint;
}

export const REPAID_FIELDS: readonly FieldSpec[] = [
  { rawName: "market", camelName: "market", kind: "pubkey" },
  { rawName: "position", camelName: "position", kind: "pubkey" },
  { rawName: "payer", camelName: "payer", kind: "pubkey" },
  { rawName: "assets_in", camelName: "assetsIn", kind: "u64" },
  { rawName: "credited", camelName: "credited", kind: "u64" },
  { rawName: "shares_burned", camelName: "sharesBurned", kind: "u128" },
];

export interface SetMarketParamsArgs {
  oracleKind: number;
  collateralFeedId: Uint8Array;
  loanFeedId: Uint8Array;
  maxPriceAgeSecs: number;
  maxConfBps: number;
  maxLtv: bigint;
  liqThreshold: bigint;
  liqBonus: bigint;
  closeFactor: bigint;
  fullLiqHf: bigint;
  liqProtocolFee: bigint;
  fee: bigint;
  minDebt: bigint;
  baseRatePs: bigint;
  slope1Ps: bigint;
  slope2Ps: bigint;
  uKink: bigint;
  maxRatePs: bigint;
  feeRecipient: Address;
}

export const SETMARKETPARAMSARGS_FIELDS: readonly FieldSpec[] = [
  { rawName: "oracle_kind", camelName: "oracleKind", kind: "u8" },
  { rawName: "collateral_feed_id", camelName: "collateralFeedId", kind: "bytesN", len: 32 },
  { rawName: "loan_feed_id", camelName: "loanFeedId", kind: "bytesN", len: 32 },
  { rawName: "max_price_age_secs", camelName: "maxPriceAgeSecs", kind: "u32" },
  { rawName: "max_conf_bps", camelName: "maxConfBps", kind: "u16" },
  { rawName: "max_ltv", camelName: "maxLtv", kind: "u128" },
  { rawName: "liq_threshold", camelName: "liqThreshold", kind: "u128" },
  { rawName: "liq_bonus", camelName: "liqBonus", kind: "u128" },
  { rawName: "close_factor", camelName: "closeFactor", kind: "u128" },
  { rawName: "full_liq_hf", camelName: "fullLiqHf", kind: "u128" },
  { rawName: "liq_protocol_fee", camelName: "liqProtocolFee", kind: "u128" },
  { rawName: "fee", camelName: "fee", kind: "u128" },
  { rawName: "min_debt", camelName: "minDebt", kind: "u64" },
  { rawName: "base_rate_ps", camelName: "baseRatePs", kind: "u128" },
  { rawName: "slope1_ps", camelName: "slope1Ps", kind: "u128" },
  { rawName: "slope2_ps", camelName: "slope2Ps", kind: "u128" },
  { rawName: "u_kink", camelName: "uKink", kind: "u128" },
  { rawName: "max_rate_ps", camelName: "maxRatePs", kind: "u128" },
  { rawName: "fee_recipient", camelName: "feeRecipient", kind: "pubkey" },
];

/**
 * `commit_pending_params`, once the timelock has elapsed and the staged proposal has been
 * re-validated and applied.
 */
export interface StagedParamsCommitted {
  market: Address;
  pendingMarketParams: Address;
  effectiveAt: bigint;
}

export const STAGEDPARAMSCOMMITTED_FIELDS: readonly FieldSpec[] = [
  { rawName: "market", camelName: "market", kind: "pubkey" },
  { rawName: "pending_market_params", camelName: "pendingMarketParams", kind: "pubkey" },
  { rawName: "effective_at", camelName: "effectiveAt", kind: "i64" },
];

/**
 * `assets_in` is the requested/computed transfer amount; `credited` is the measured post-CPI
 * delta actually recorded (`account-model.md` §6.4) — loan assets are policy-restricted to
 * fee-free mints, so the two are expected to be equal, but this is verified, never assumed
 * (`instruction-catalogue.md` §12).
 */
export interface Supplied {
  market: Address;
  position: Address;
  owner: Address;
  assetsIn: bigint;
  credited: bigint;
  sharesMinted: bigint;
}

export const SUPPLIED_FIELDS: readonly FieldSpec[] = [
  { rawName: "market", camelName: "market", kind: "pubkey" },
  { rawName: "position", camelName: "position", kind: "pubkey" },
  { rawName: "owner", camelName: "owner", kind: "pubkey" },
  { rawName: "assets_in", camelName: "assetsIn", kind: "u64" },
  { rawName: "credited", camelName: "credited", kind: "u64" },
  { rawName: "shares_minted", camelName: "sharesMinted", kind: "u128" },
];

export interface Withdrawn {
  market: Address;
  position: Address;
  owner: Address;
  assetsOut: bigint;
  sharesBurned: bigint;
}

export const WITHDRAWN_FIELDS: readonly FieldSpec[] = [
  { rawName: "market", camelName: "market", kind: "pubkey" },
  { rawName: "position", camelName: "position", kind: "pubkey" },
  { rawName: "owner", camelName: "owner", kind: "pubkey" },
  { rawName: "assets_out", camelName: "assetsOut", kind: "u64" },
  { rawName: "shares_burned", camelName: "sharesBurned", kind: "u128" },
];

export const TYPE_FIELD_SPECS: Record<string, readonly FieldSpec[]> = {
  "AdminTransferStarted": ADMINTRANSFERSTARTED_FIELDS,
  "AdminTransferred": ADMINTRANSFERRED_FIELDS,
  "BadDebtAbsorbed": BADDEBTABSORBED_FIELDS,
  "Borrowed": BORROWED_FIELDS,
  "CollateralDeposited": COLLATERALDEPOSITED_FIELDS,
  "CollateralFeesWithdrawn": COLLATERALFEESWITHDRAWN_FIELDS,
  "CollateralWithdrawn": COLLATERALWITHDRAWN_FIELDS,
  "CreateMarketArgs": CREATEMARKETARGS_FIELDS,
  "GuardianChanged": GUARDIANCHANGED_FIELDS,
  "InitProtocolArgs": INITPROTOCOLARGS_FIELDS,
  "InterestAccrued": INTERESTACCRUED_FIELDS,
  "Liquidated": LIQUIDATED_FIELDS,
  "Market": MARKET_FIELDS,
  "MarketCreated": MARKETCREATED_FIELDS,
  "MarketParamsUpdated": MARKETPARAMSUPDATED_FIELDS,
  "MarketPauseSet": MARKETPAUSESET_FIELDS,
  "ParamsStaged": PARAMSSTAGED_FIELDS,
  "PendingMarketParams": PENDINGMARKETPARAMS_FIELDS,
  "Position": POSITION_FIELDS,
  "PositionClosed": POSITIONCLOSED_FIELDS,
  "PositionInitialized": POSITIONINITIALIZED_FIELDS,
  "Protocol": PROTOCOL_FIELDS,
  "ProtocolInitialized": PROTOCOLINITIALIZED_FIELDS,
  "ProtocolMigrated": PROTOCOLMIGRATED_FIELDS,
  "ProtocolPauseSet": PROTOCOLPAUSESET_FIELDS,
  "Repaid": REPAID_FIELDS,
  "SetMarketParamsArgs": SETMARKETPARAMSARGS_FIELDS,
  "StagedParamsCommitted": STAGEDPARAMSCOMMITTED_FIELDS,
  "Supplied": SUPPLIED_FIELDS,
  "Withdrawn": WITHDRAWN_FIELDS,
};
