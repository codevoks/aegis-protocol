// GENERATED FILE -- do not edit by hand.
// Produced by sdk/ts/scripts/codegen.mjs from target/idl/aegis.json.
// Re-run `npm run codegen` (sdk/ts/) after `anchor build` changes the IDL; `npm run codegen:check` detects a stale commit.


import type { Address } from '@solana/kit';

import type { FieldSpec } from '../borshValues.js';


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

export interface Protocol {
  admin: Address;
  pendingAdmin: Address;
  guardian: Address;
  feeRecipient: Address;
  paused: number;
  bump: number;
  Reserved: Uint8Array;
}

export const PROTOCOL_FIELDS: readonly FieldSpec[] = [
  { rawName: "admin", camelName: "admin", kind: "pubkey" },
  { rawName: "pending_admin", camelName: "pendingAdmin", kind: "pubkey" },
  { rawName: "guardian", camelName: "guardian", kind: "pubkey" },
  { rawName: "fee_recipient", camelName: "feeRecipient", kind: "pubkey" },
  { rawName: "paused", camelName: "paused", kind: "u8" },
  { rawName: "bump", camelName: "bump", kind: "u8" },
  { rawName: "_reserved", camelName: "Reserved", kind: "bytesN", len: 64 },
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
  "BadDebtAbsorbed": BADDEBTABSORBED_FIELDS,
  "Borrowed": BORROWED_FIELDS,
  "CollateralDeposited": COLLATERALDEPOSITED_FIELDS,
  "CollateralFeesWithdrawn": COLLATERALFEESWITHDRAWN_FIELDS,
  "CollateralWithdrawn": COLLATERALWITHDRAWN_FIELDS,
  "CreateMarketArgs": CREATEMARKETARGS_FIELDS,
  "InitProtocolArgs": INITPROTOCOLARGS_FIELDS,
  "InterestAccrued": INTERESTACCRUED_FIELDS,
  "Liquidated": LIQUIDATED_FIELDS,
  "Market": MARKET_FIELDS,
  "MarketCreated": MARKETCREATED_FIELDS,
  "Position": POSITION_FIELDS,
  "PositionClosed": POSITIONCLOSED_FIELDS,
  "PositionInitialized": POSITIONINITIALIZED_FIELDS,
  "Protocol": PROTOCOL_FIELDS,
  "ProtocolInitialized": PROTOCOLINITIALIZED_FIELDS,
  "Repaid": REPAID_FIELDS,
  "Supplied": SUPPLIED_FIELDS,
  "Withdrawn": WITHDRAWN_FIELDS,
};
