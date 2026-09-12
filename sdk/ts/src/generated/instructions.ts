// GENERATED FILE -- do not edit by hand.
// Produced by sdk/ts/scripts/codegen.mjs from target/idl/aegis.json.
// Re-run `npm run codegen` (sdk/ts/) after `anchor build` changes the IDL; `npm run codegen:check` detects a stale commit.


import type { Address, Instruction } from '@solana/kit';

import type { FieldSpec } from '../borshValues.js';

import { buildGenericInstruction } from '../ixEngine.js';

import type { CreateMarketArgs, InitProtocolArgs } from './types.js';

export interface IxAccountMeta {
  readonly name: string;
  readonly camelName: string;
  readonly writable: boolean;
  readonly signer: boolean;
  readonly optional: boolean;
}

export interface IxDef {
  readonly name: string;
  readonly discriminator: Uint8Array;
  readonly accounts: readonly IxAccountMeta[];
  readonly argsFields: readonly FieldSpec[];
}

export const INSTRUCTIONS: Record<string, IxDef> = {
  "absorb_bad_debt": {
    name: "absorb_bad_debt",
    discriminator: Uint8Array.from([49, 38, 219, 23, 214, 120, 39, 143]),
    accounts: [
    { name: "market", camelName: "market", writable: true, signer: false, optional: false },
    { name: "position", camelName: "position", writable: true, signer: false, optional: false },
    { name: "fee_position", camelName: "feePosition", writable: true, signer: false, optional: false },
    ],
    argsFields: [],
  },
  "accrue_interest": {
    name: "accrue_interest",
    discriminator: Uint8Array.from([47, 40, 115, 198, 91, 12, 222, 49]),
    accounts: [
    { name: "market", camelName: "market", writable: true, signer: false, optional: false },
    { name: "fee_position", camelName: "feePosition", writable: true, signer: false, optional: false },
    ],
    argsFields: [],
  },
  "borrow": {
    name: "borrow",
    discriminator: Uint8Array.from([228, 253, 131, 202, 207, 116, 89, 18]),
    accounts: [
    { name: "owner", camelName: "owner", writable: false, signer: true, optional: false },
    { name: "market", camelName: "market", writable: true, signer: false, optional: false },
    { name: "position", camelName: "position", writable: true, signer: false, optional: false },
    { name: "fee_position", camelName: "feePosition", writable: true, signer: false, optional: false },
    { name: "loan_vault", camelName: "loanVault", writable: true, signer: false, optional: false },
    { name: "owner_loan_ata", camelName: "ownerLoanAta", writable: true, signer: false, optional: false },
    { name: "loan_mint", camelName: "loanMint", writable: false, signer: false, optional: false },
    { name: "loan_token_program", camelName: "loanTokenProgram", writable: false, signer: false, optional: false },
    { name: "collateral_price_update", camelName: "collateralPriceUpdate", writable: false, signer: false, optional: false },
    { name: "loan_price_update", camelName: "loanPriceUpdate", writable: false, signer: false, optional: false },
    ],
    argsFields: [{ rawName: "assets", camelName: "assets", kind: "u64" }, { rawName: "shares", camelName: "shares", kind: "u128" }],
  },
  "close_position": {
    name: "close_position",
    discriminator: Uint8Array.from([123, 134, 81, 0, 49, 68, 98, 98]),
    accounts: [
    { name: "owner", camelName: "owner", writable: true, signer: true, optional: false },
    { name: "market", camelName: "market", writable: false, signer: false, optional: false },
    { name: "position", camelName: "position", writable: true, signer: false, optional: false },
    ],
    argsFields: [],
  },
  "create_market": {
    name: "create_market",
    discriminator: Uint8Array.from([103, 226, 97, 235, 200, 188, 251, 254]),
    accounts: [
    { name: "admin", camelName: "admin", writable: true, signer: true, optional: false },
    { name: "protocol", camelName: "protocol", writable: false, signer: false, optional: false },
    { name: "collateral_mint", camelName: "collateralMint", writable: false, signer: false, optional: false },
    { name: "loan_mint", camelName: "loanMint", writable: false, signer: false, optional: false },
    { name: "collateral_token_program", camelName: "collateralTokenProgram", writable: false, signer: false, optional: false },
    { name: "loan_token_program", camelName: "loanTokenProgram", writable: false, signer: false, optional: false },
    { name: "market", camelName: "market", writable: true, signer: false, optional: false },
    { name: "collateral_vault", camelName: "collateralVault", writable: true, signer: false, optional: false },
    { name: "loan_vault", camelName: "loanVault", writable: true, signer: false, optional: false },
    { name: "fee_position", camelName: "feePosition", writable: true, signer: false, optional: false },
    { name: "system_program", camelName: "systemProgram", writable: false, signer: false, optional: false },
    ],
    argsFields: [{ rawName: "args", camelName: "args", kind: "defined", definedName: "CreateMarketArgs" }],
  },
  "deposit_collateral": {
    name: "deposit_collateral",
    discriminator: Uint8Array.from([156, 131, 142, 116, 146, 247, 162, 120]),
    accounts: [
    { name: "depositor", camelName: "depositor", writable: true, signer: true, optional: false },
    { name: "market", camelName: "market", writable: false, signer: false, optional: false },
    { name: "position", camelName: "position", writable: true, signer: false, optional: false },
    { name: "collateral_vault", camelName: "collateralVault", writable: true, signer: false, optional: false },
    { name: "depositor_collateral_ata", camelName: "depositorCollateralAta", writable: true, signer: false, optional: false },
    { name: "collateral_mint", camelName: "collateralMint", writable: false, signer: false, optional: false },
    { name: "collateral_token_program", camelName: "collateralTokenProgram", writable: false, signer: false, optional: false },
    ],
    argsFields: [{ rawName: "amount", camelName: "amount", kind: "u64" }],
  },
  "init_position": {
    name: "init_position",
    discriminator: Uint8Array.from([197, 20, 10, 1, 97, 160, 177, 91]),
    accounts: [
    { name: "payer", camelName: "payer", writable: true, signer: true, optional: false },
    { name: "market", camelName: "market", writable: false, signer: false, optional: false },
    { name: "owner", camelName: "owner", writable: false, signer: false, optional: false },
    { name: "position", camelName: "position", writable: true, signer: false, optional: false },
    { name: "system_program", camelName: "systemProgram", writable: false, signer: false, optional: false },
    ],
    argsFields: [],
  },
  "initialize_protocol": {
    name: "initialize_protocol",
    discriminator: Uint8Array.from([188, 233, 252, 106, 134, 146, 202, 91]),
    accounts: [
    { name: "payer", camelName: "payer", writable: true, signer: true, optional: false },
    { name: "protocol", camelName: "protocol", writable: true, signer: false, optional: false },
    { name: "system_program", camelName: "systemProgram", writable: false, signer: false, optional: false },
    ],
    argsFields: [{ rawName: "args", camelName: "args", kind: "defined", definedName: "InitProtocolArgs" }],
  },
  "liquidate": {
    name: "liquidate",
    discriminator: Uint8Array.from([223, 179, 226, 125, 48, 46, 39, 74]),
    accounts: [
    { name: "liquidator", camelName: "liquidator", writable: true, signer: true, optional: false },
    { name: "market", camelName: "market", writable: true, signer: false, optional: false },
    { name: "position", camelName: "position", writable: true, signer: false, optional: false },
    { name: "fee_position", camelName: "feePosition", writable: true, signer: false, optional: false },
    { name: "loan_vault", camelName: "loanVault", writable: true, signer: false, optional: false },
    { name: "collateral_vault", camelName: "collateralVault", writable: true, signer: false, optional: false },
    { name: "liquidator_loan_ata", camelName: "liquidatorLoanAta", writable: true, signer: false, optional: false },
    { name: "liquidator_collateral_ata", camelName: "liquidatorCollateralAta", writable: true, signer: false, optional: false },
    { name: "loan_mint", camelName: "loanMint", writable: false, signer: false, optional: false },
    { name: "collateral_mint", camelName: "collateralMint", writable: false, signer: false, optional: false },
    { name: "loan_token_program", camelName: "loanTokenProgram", writable: false, signer: false, optional: false },
    { name: "collateral_token_program", camelName: "collateralTokenProgram", writable: false, signer: false, optional: false },
    { name: "collateral_price_update", camelName: "collateralPriceUpdate", writable: false, signer: false, optional: false },
    { name: "loan_price_update", camelName: "loanPriceUpdate", writable: false, signer: false, optional: false },
    { name: "callback_program", camelName: "callbackProgram", writable: false, signer: false, optional: true },
    { name: "callback_collateral_account", camelName: "callbackCollateralAccount", writable: true, signer: false, optional: true },
    ],
    argsFields: [{ rawName: "repay_assets", camelName: "repayAssets", kind: "u64" }, { rawName: "seize_collateral", camelName: "seizeCollateral", kind: "u64" }, { rawName: "callback_data", camelName: "callbackData", kind: "bytes" }],
  },
  "ping": {
    name: "ping",
    discriminator: Uint8Array.from([173, 0, 94, 236, 73, 133, 225, 153]),
    accounts: [
    ],
    argsFields: [],
  },
  "repay": {
    name: "repay",
    discriminator: Uint8Array.from([234, 103, 67, 82, 208, 234, 219, 166]),
    accounts: [
    { name: "payer", camelName: "payer", writable: true, signer: true, optional: false },
    { name: "market", camelName: "market", writable: true, signer: false, optional: false },
    { name: "position", camelName: "position", writable: true, signer: false, optional: false },
    { name: "fee_position", camelName: "feePosition", writable: true, signer: false, optional: false },
    { name: "loan_vault", camelName: "loanVault", writable: true, signer: false, optional: false },
    { name: "payer_loan_ata", camelName: "payerLoanAta", writable: true, signer: false, optional: false },
    { name: "loan_mint", camelName: "loanMint", writable: false, signer: false, optional: false },
    { name: "loan_token_program", camelName: "loanTokenProgram", writable: false, signer: false, optional: false },
    ],
    argsFields: [{ rawName: "assets", camelName: "assets", kind: "u64" }, { rawName: "shares", camelName: "shares", kind: "u128" }],
  },
  "supply": {
    name: "supply",
    discriminator: Uint8Array.from([81, 67, 116, 61, 250, 209, 5, 198]),
    accounts: [
    { name: "owner", camelName: "owner", writable: true, signer: true, optional: false },
    { name: "market", camelName: "market", writable: true, signer: false, optional: false },
    { name: "position", camelName: "position", writable: true, signer: false, optional: false },
    { name: "fee_position", camelName: "feePosition", writable: true, signer: false, optional: false },
    { name: "loan_vault", camelName: "loanVault", writable: true, signer: false, optional: false },
    { name: "owner_loan_ata", camelName: "ownerLoanAta", writable: true, signer: false, optional: false },
    { name: "loan_mint", camelName: "loanMint", writable: false, signer: false, optional: false },
    { name: "loan_token_program", camelName: "loanTokenProgram", writable: false, signer: false, optional: false },
    ],
    argsFields: [{ rawName: "assets", camelName: "assets", kind: "u64" }, { rawName: "shares", camelName: "shares", kind: "u128" }],
  },
  "withdraw": {
    name: "withdraw",
    discriminator: Uint8Array.from([183, 18, 70, 156, 148, 109, 161, 34]),
    accounts: [
    { name: "owner", camelName: "owner", writable: true, signer: true, optional: false },
    { name: "market", camelName: "market", writable: true, signer: false, optional: false },
    { name: "position", camelName: "position", writable: true, signer: false, optional: false },
    { name: "fee_position", camelName: "feePosition", writable: true, signer: false, optional: false },
    { name: "loan_vault", camelName: "loanVault", writable: true, signer: false, optional: false },
    { name: "owner_loan_ata", camelName: "ownerLoanAta", writable: true, signer: false, optional: false },
    { name: "loan_mint", camelName: "loanMint", writable: false, signer: false, optional: false },
    { name: "loan_token_program", camelName: "loanTokenProgram", writable: false, signer: false, optional: false },
    ],
    argsFields: [{ rawName: "assets", camelName: "assets", kind: "u64" }, { rawName: "shares", camelName: "shares", kind: "u128" }],
  },
  "withdraw_collateral": {
    name: "withdraw_collateral",
    discriminator: Uint8Array.from([115, 135, 168, 106, 139, 214, 138, 150]),
    accounts: [
    { name: "owner", camelName: "owner", writable: false, signer: true, optional: false },
    { name: "market", camelName: "market", writable: false, signer: false, optional: false },
    { name: "position", camelName: "position", writable: true, signer: false, optional: false },
    { name: "collateral_vault", camelName: "collateralVault", writable: true, signer: false, optional: false },
    { name: "owner_collateral_ata", camelName: "ownerCollateralAta", writable: true, signer: false, optional: false },
    { name: "collateral_mint", camelName: "collateralMint", writable: false, signer: false, optional: false },
    { name: "collateral_token_program", camelName: "collateralTokenProgram", writable: false, signer: false, optional: false },
    { name: "collateral_price_update", camelName: "collateralPriceUpdate", writable: false, signer: false, optional: false },
    { name: "loan_price_update", camelName: "loanPriceUpdate", writable: false, signer: false, optional: false },
    ],
    argsFields: [{ rawName: "amount", camelName: "amount", kind: "u64" }],
  },
  "withdraw_collateral_fees": {
    name: "withdraw_collateral_fees",
    discriminator: Uint8Array.from([8, 36, 107, 152, 237, 145, 209, 244]),
    accounts: [
    { name: "admin", camelName: "admin", writable: false, signer: true, optional: false },
    { name: "protocol", camelName: "protocol", writable: false, signer: false, optional: false },
    { name: "market", camelName: "market", writable: true, signer: false, optional: false },
    { name: "collateral_vault", camelName: "collateralVault", writable: true, signer: false, optional: false },
    { name: "admin_collateral_ata", camelName: "adminCollateralAta", writable: true, signer: false, optional: false },
    { name: "collateral_mint", camelName: "collateralMint", writable: false, signer: false, optional: false },
    { name: "collateral_token_program", camelName: "collateralTokenProgram", writable: false, signer: false, optional: false },
    ],
    argsFields: [{ rawName: "amount", camelName: "amount", kind: "u64" }],
  },
};

export interface AbsorbBadDebtAccounts {
  market: Address;
  position: Address;
  feePosition: Address;
}

export function buildAbsorbBadDebtInstruction(programId: Address, accounts: AbsorbBadDebtAccounts): Instruction {
  return buildGenericInstruction(programId, "absorb_bad_debt", accounts as unknown as Record<string, Address | undefined>, {});
}

export interface AccrueInterestAccounts {
  market: Address;
  feePosition: Address;
}

export function buildAccrueInterestInstruction(programId: Address, accounts: AccrueInterestAccounts): Instruction {
  return buildGenericInstruction(programId, "accrue_interest", accounts as unknown as Record<string, Address | undefined>, {});
}

export interface BorrowAccounts {
  owner: Address;
  market: Address;
  position: Address;
  feePosition: Address;
  loanVault: Address;
  ownerLoanAta: Address;
  loanMint: Address;
  loanTokenProgram: Address;
  collateralPriceUpdate: Address;
  loanPriceUpdate: Address;
}

export interface BorrowArgs {
  assets: bigint;
  shares: bigint;
}

export function buildBorrowInstruction(programId: Address, accounts: BorrowAccounts, args: BorrowArgs): Instruction {
  return buildGenericInstruction(programId, "borrow", accounts as unknown as Record<string, Address | undefined>, args as unknown as Record<string, unknown>);
}

export interface ClosePositionAccounts {
  owner: Address;
  market: Address;
  position: Address;
}

export function buildClosePositionInstruction(programId: Address, accounts: ClosePositionAccounts): Instruction {
  return buildGenericInstruction(programId, "close_position", accounts as unknown as Record<string, Address | undefined>, {});
}

export interface CreateMarketAccounts {
  admin: Address;
  protocol: Address;
  collateralMint: Address;
  loanMint: Address;
  collateralTokenProgram: Address;
  loanTokenProgram: Address;
  market: Address;
  collateralVault: Address;
  loanVault: Address;
  feePosition: Address;
  systemProgram: Address;
}

export function buildCreateMarketInstruction(programId: Address, accounts: CreateMarketAccounts, args: CreateMarketArgs): Instruction {
  return buildGenericInstruction(programId, "create_market", accounts as unknown as Record<string, Address | undefined>, { args: args });
}

export interface DepositCollateralAccounts {
  depositor: Address;
  market: Address;
  position: Address;
  collateralVault: Address;
  depositorCollateralAta: Address;
  collateralMint: Address;
  collateralTokenProgram: Address;
}

export interface DepositCollateralArgs {
  amount: bigint;
}

export function buildDepositCollateralInstruction(programId: Address, accounts: DepositCollateralAccounts, args: DepositCollateralArgs): Instruction {
  return buildGenericInstruction(programId, "deposit_collateral", accounts as unknown as Record<string, Address | undefined>, args as unknown as Record<string, unknown>);
}

export interface InitPositionAccounts {
  payer: Address;
  market: Address;
  owner: Address;
  position: Address;
  systemProgram: Address;
}

export function buildInitPositionInstruction(programId: Address, accounts: InitPositionAccounts): Instruction {
  return buildGenericInstruction(programId, "init_position", accounts as unknown as Record<string, Address | undefined>, {});
}

export interface InitializeProtocolAccounts {
  payer: Address;
  protocol: Address;
  systemProgram: Address;
}

export function buildInitializeProtocolInstruction(programId: Address, accounts: InitializeProtocolAccounts, args: InitProtocolArgs): Instruction {
  return buildGenericInstruction(programId, "initialize_protocol", accounts as unknown as Record<string, Address | undefined>, { args: args });
}

export interface LiquidateAccounts {
  liquidator: Address;
  market: Address;
  position: Address;
  feePosition: Address;
  loanVault: Address;
  collateralVault: Address;
  liquidatorLoanAta: Address;
  liquidatorCollateralAta: Address;
  loanMint: Address;
  collateralMint: Address;
  loanTokenProgram: Address;
  collateralTokenProgram: Address;
  collateralPriceUpdate: Address;
  loanPriceUpdate: Address;
  callbackProgram?: Address;
  callbackCollateralAccount?: Address;
}

export interface LiquidateArgs {
  repayAssets: bigint;
  seizeCollateral: bigint;
  callbackData: Uint8Array;
}

export function buildLiquidateInstruction(programId: Address, accounts: LiquidateAccounts, args: LiquidateArgs, remainingAccounts: { address: Address; writable: boolean }[] = []): Instruction {
  return buildGenericInstruction(programId, "liquidate", accounts as unknown as Record<string, Address | undefined>, args as unknown as Record<string, unknown>, remainingAccounts);
}

export interface RepayAccounts {
  payer: Address;
  market: Address;
  position: Address;
  feePosition: Address;
  loanVault: Address;
  payerLoanAta: Address;
  loanMint: Address;
  loanTokenProgram: Address;
}

export interface RepayArgs {
  assets: bigint;
  shares: bigint;
}

export function buildRepayInstruction(programId: Address, accounts: RepayAccounts, args: RepayArgs): Instruction {
  return buildGenericInstruction(programId, "repay", accounts as unknown as Record<string, Address | undefined>, args as unknown as Record<string, unknown>);
}

export interface SupplyAccounts {
  owner: Address;
  market: Address;
  position: Address;
  feePosition: Address;
  loanVault: Address;
  ownerLoanAta: Address;
  loanMint: Address;
  loanTokenProgram: Address;
}

export interface SupplyArgs {
  assets: bigint;
  shares: bigint;
}

export function buildSupplyInstruction(programId: Address, accounts: SupplyAccounts, args: SupplyArgs): Instruction {
  return buildGenericInstruction(programId, "supply", accounts as unknown as Record<string, Address | undefined>, args as unknown as Record<string, unknown>);
}

export interface WithdrawAccounts {
  owner: Address;
  market: Address;
  position: Address;
  feePosition: Address;
  loanVault: Address;
  ownerLoanAta: Address;
  loanMint: Address;
  loanTokenProgram: Address;
}

export interface WithdrawArgs {
  assets: bigint;
  shares: bigint;
}

export function buildWithdrawInstruction(programId: Address, accounts: WithdrawAccounts, args: WithdrawArgs): Instruction {
  return buildGenericInstruction(programId, "withdraw", accounts as unknown as Record<string, Address | undefined>, args as unknown as Record<string, unknown>);
}

export interface WithdrawCollateralAccounts {
  owner: Address;
  market: Address;
  position: Address;
  collateralVault: Address;
  ownerCollateralAta: Address;
  collateralMint: Address;
  collateralTokenProgram: Address;
  collateralPriceUpdate: Address;
  loanPriceUpdate: Address;
}

export interface WithdrawCollateralArgs {
  amount: bigint;
}

export function buildWithdrawCollateralInstruction(programId: Address, accounts: WithdrawCollateralAccounts, args: WithdrawCollateralArgs): Instruction {
  return buildGenericInstruction(programId, "withdraw_collateral", accounts as unknown as Record<string, Address | undefined>, args as unknown as Record<string, unknown>);
}

export interface WithdrawCollateralFeesAccounts {
  admin: Address;
  protocol: Address;
  market: Address;
  collateralVault: Address;
  adminCollateralAta: Address;
  collateralMint: Address;
  collateralTokenProgram: Address;
}

export interface WithdrawCollateralFeesArgs {
  amount: bigint;
}

export function buildWithdrawCollateralFeesInstruction(programId: Address, accounts: WithdrawCollateralFeesAccounts, args: WithdrawCollateralFeesArgs): Instruction {
  return buildGenericInstruction(programId, "withdraw_collateral_fees", accounts as unknown as Record<string, Address | undefined>, args as unknown as Record<string, unknown>);
}

