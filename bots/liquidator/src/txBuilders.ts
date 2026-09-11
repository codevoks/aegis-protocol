// Builds Aegis instructions from the real IDL coder. Account lists are constructed explicitly, in
// the exact order each instruction's Anchor `#[derive(Accounts)]` struct declares them -- the
// coder only encodes instruction *data*, not the account-meta list (Anchor's TS `Program.methods`
// convenience wrapper would do that, but ADR-0011 deliberately uses `@anchor-lang/core` only for
// its coder, not its Provider/Program layer, to stay on `@solana/kit` for everything
// RPC/transaction/signing-related). See `anchorCore.ts` for the two empirically-verified encoding
// conventions this file depends on (raw snake_case names, `BN`/`.toBuffer()` argument shims).

import { AccountRole, type Address, type Instruction } from '@solana/kit';
import { bn, pubkeyArg, type BorshCoder } from './anchorCore.js';

function acc(address: Address, role: AccountRole) {
  return { address, role };
}

/** Anchor's `Option<Pubkey>` "no account" sentinel is the invoking PROGRAM's own id (the classic
 *  Anchor optional-account convention this repo's own `anchor-derive-accounts` codegen uses --
 *  see `anchor-syn`'s `__client_accounts.rs`, cross-checked directly against the installed crate
 *  source during Phase 8 implementation). */
function noneAccountSentinel(programId: Address): Address {
  return programId;
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
  /** `undefined` reproduces Phase 6 behavior exactly (`I-LIQ-CB-02`). */
  callback?: {
    program: Address;
    collateralAccount: Address;
    /** Forwarded verbatim as the callback's own CPI instruction data -- Aegis defines no schema
     *  for it (`docs/instruction-catalogue.md` §17). */
    data: Uint8Array;
    /** Becomes `ctx.remaining_accounts` on the Aegis instruction, forwarded verbatim into the
     *  callback's CPI account list (ADR-0013 §1.4). Never include Market/liquidator/Position/
     *  fee_position/vault/ATA addresses here -- Aegis rejects any that alias a protected key
     *  (`CallbackAccountNotPermitted`). */
    remainingAccounts: { address: Address; writable: boolean }[];
  };
}

export function buildLiquidateInstruction(
  coder: BorshCoder,
  aegisProgramId: Address,
  accounts: LiquidateAccounts,
  repayAssets: bigint,
  seizeCollateral: bigint,
): Instruction {
  const hasCallback = accounts.callback !== undefined;
  const data = coder.instruction.encode('liquidate', {
    repay_assets: bn(repayAssets),
    seize_collateral: bn(seizeCollateral),
    callback_data: Buffer.from(accounts.callback?.data ?? new Uint8Array()),
  });

  const metas = [
    acc(accounts.liquidator, AccountRole.WRITABLE_SIGNER),
    acc(accounts.market, AccountRole.WRITABLE),
    acc(accounts.position, AccountRole.WRITABLE),
    acc(accounts.feePosition, AccountRole.WRITABLE),
    acc(accounts.loanVault, AccountRole.WRITABLE),
    acc(accounts.collateralVault, AccountRole.WRITABLE),
    acc(accounts.liquidatorLoanAta, AccountRole.WRITABLE),
    acc(accounts.liquidatorCollateralAta, AccountRole.WRITABLE),
    acc(accounts.loanMint, AccountRole.READONLY),
    acc(accounts.collateralMint, AccountRole.READONLY),
    acc(accounts.loanTokenProgram, AccountRole.READONLY),
    acc(accounts.collateralTokenProgram, AccountRole.READONLY),
    acc(accounts.collateralPriceUpdate, AccountRole.READONLY),
    acc(accounts.loanPriceUpdate, AccountRole.READONLY),
    hasCallback
      ? acc(accounts.callback!.program, AccountRole.READONLY)
      : acc(noneAccountSentinel(aegisProgramId), AccountRole.READONLY),
    hasCallback
      ? acc(accounts.callback!.collateralAccount, AccountRole.WRITABLE)
      : acc(noneAccountSentinel(aegisProgramId), AccountRole.READONLY),
    ...(accounts.callback?.remainingAccounts.map((a) =>
      acc(a.address, a.writable ? AccountRole.WRITABLE : AccountRole.READONLY),
    ) ?? []),
  ];

  return { programAddress: aegisProgramId, accounts: metas, data: new Uint8Array(data) };
}

export function buildInitializeProtocolInstruction(
  coder: BorshCoder,
  aegisProgramId: Address,
  payer: Address,
  protocol: Address,
  systemProgram: Address,
  guardian: Address,
  feeRecipient: Address,
): Instruction {
  const data = coder.instruction.encode('initialize_protocol', {
    args: { guardian: pubkeyArg(guardian), fee_recipient: pubkeyArg(feeRecipient) },
  });
  return {
    programAddress: aegisProgramId,
    accounts: [
      acc(payer, AccountRole.WRITABLE_SIGNER),
      acc(protocol, AccountRole.WRITABLE),
      acc(systemProgram, AccountRole.READONLY),
    ],
    data: new Uint8Array(data),
  };
}

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

export function buildCreateMarketInstruction(
  coder: BorshCoder,
  aegisProgramId: Address,
  accounts: {
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
  },
  args: CreateMarketArgs,
): Instruction {
  const data = coder.instruction.encode('create_market', {
    args: {
      config_id: args.configId,
      oracle_kind: args.oracleKind,
      collateral_feed_id: Array.from(args.collateralFeedId),
      loan_feed_id: Array.from(args.loanFeedId),
      max_price_age_secs: args.maxPriceAgeSecs,
      max_conf_bps: args.maxConfBps,
      max_ltv: bn(args.maxLtv),
      liq_threshold: bn(args.liqThreshold),
      liq_bonus: bn(args.liqBonus),
      close_factor: bn(args.closeFactor),
      full_liq_hf: bn(args.fullLiqHf),
      liq_protocol_fee: bn(args.liqProtocolFee),
      fee: bn(args.fee),
      min_debt: bn(args.minDebt),
      base_rate_ps: bn(args.baseRatePs),
      slope1_ps: bn(args.slope1Ps),
      slope2_ps: bn(args.slope2Ps),
      u_kink: bn(args.uKink),
      max_rate_ps: bn(args.maxRatePs),
      ack_freeze_authority: args.ackFreezeAuthority,
    },
  });
  return {
    programAddress: aegisProgramId,
    accounts: [
      acc(accounts.admin, AccountRole.WRITABLE_SIGNER),
      acc(accounts.protocol, AccountRole.READONLY),
      acc(accounts.collateralMint, AccountRole.READONLY),
      acc(accounts.loanMint, AccountRole.READONLY),
      acc(accounts.collateralTokenProgram, AccountRole.READONLY),
      acc(accounts.loanTokenProgram, AccountRole.READONLY),
      acc(accounts.market, AccountRole.WRITABLE),
      acc(accounts.collateralVault, AccountRole.WRITABLE),
      acc(accounts.loanVault, AccountRole.WRITABLE),
      acc(accounts.feePosition, AccountRole.WRITABLE),
      acc(accounts.systemProgram, AccountRole.READONLY),
    ],
    data: new Uint8Array(data),
  };
}

export function buildInitPositionInstruction(
  coder: BorshCoder,
  aegisProgramId: Address,
  payer: Address,
  market: Address,
  owner: Address,
  position: Address,
  systemProgram: Address,
): Instruction {
  const data = coder.instruction.encode('init_position', {});
  return {
    programAddress: aegisProgramId,
    accounts: [
      acc(payer, AccountRole.WRITABLE_SIGNER),
      acc(market, AccountRole.READONLY),
      acc(owner, AccountRole.READONLY),
      acc(position, AccountRole.WRITABLE),
      acc(systemProgram, AccountRole.READONLY),
    ],
    data: new Uint8Array(data),
  };
}

export function buildDepositCollateralInstruction(
  coder: BorshCoder,
  aegisProgramId: Address,
  accounts: {
    depositor: Address;
    market: Address;
    position: Address;
    collateralVault: Address;
    depositorCollateralAta: Address;
    collateralMint: Address;
    collateralTokenProgram: Address;
  },
  amount: bigint,
): Instruction {
  const data = coder.instruction.encode('deposit_collateral', { amount: bn(amount) });
  return {
    programAddress: aegisProgramId,
    accounts: [
      acc(accounts.depositor, AccountRole.WRITABLE_SIGNER),
      acc(accounts.market, AccountRole.READONLY),
      acc(accounts.position, AccountRole.WRITABLE),
      acc(accounts.collateralVault, AccountRole.WRITABLE),
      acc(accounts.depositorCollateralAta, AccountRole.WRITABLE),
      acc(accounts.collateralMint, AccountRole.READONLY),
      acc(accounts.collateralTokenProgram, AccountRole.READONLY),
    ],
    data: new Uint8Array(data),
  };
}

export function buildSupplyInstruction(
  coder: BorshCoder,
  aegisProgramId: Address,
  accounts: {
    owner: Address;
    market: Address;
    position: Address;
    feePosition: Address;
    loanVault: Address;
    ownerLoanAta: Address;
    loanMint: Address;
    loanTokenProgram: Address;
  },
  assets: bigint,
  shares: bigint,
): Instruction {
  const data = coder.instruction.encode('supply', { assets: bn(assets), shares: bn(shares) });
  return {
    programAddress: aegisProgramId,
    accounts: [
      acc(accounts.owner, AccountRole.WRITABLE_SIGNER),
      acc(accounts.market, AccountRole.WRITABLE),
      acc(accounts.position, AccountRole.WRITABLE),
      acc(accounts.feePosition, AccountRole.WRITABLE),
      acc(accounts.loanVault, AccountRole.WRITABLE),
      acc(accounts.ownerLoanAta, AccountRole.WRITABLE),
      acc(accounts.loanMint, AccountRole.READONLY),
      acc(accounts.loanTokenProgram, AccountRole.READONLY),
    ],
    data: new Uint8Array(data),
  };
}

export function buildBorrowInstruction(
  coder: BorshCoder,
  aegisProgramId: Address,
  accounts: {
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
  },
  assets: bigint,
  shares: bigint,
): Instruction {
  const data = coder.instruction.encode('borrow', { assets: bn(assets), shares: bn(shares) });
  return {
    programAddress: aegisProgramId,
    accounts: [
      acc(accounts.owner, AccountRole.WRITABLE_SIGNER),
      acc(accounts.market, AccountRole.WRITABLE),
      acc(accounts.position, AccountRole.WRITABLE),
      acc(accounts.feePosition, AccountRole.WRITABLE),
      acc(accounts.loanVault, AccountRole.WRITABLE),
      acc(accounts.ownerLoanAta, AccountRole.WRITABLE),
      acc(accounts.loanMint, AccountRole.READONLY),
      acc(accounts.loanTokenProgram, AccountRole.READONLY),
      acc(accounts.collateralPriceUpdate, AccountRole.READONLY),
      acc(accounts.loanPriceUpdate, AccountRole.READONLY),
    ],
    data: new Uint8Array(data),
  };
}
