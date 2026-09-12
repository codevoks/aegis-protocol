// Transaction builders (`docs/phases/phase-09-sdk-ui.md` item 15-16). Every user-facing Aegis
// instruction has a typed builder here, on top of the generated, IDL-derived
// `build<Pascal>Instruction` functions (`generated/instructions.ts`) -- this module adds PDA
// auto-derivation (so callers never compute a seed by hand) and first-action init-bundling.
//
// The SDK never owns or manages a user's private key (item 17): every function below returns an
// `Instruction` (or an array of them); signing is entirely the caller's responsibility, via
// whatever `TransactionSigner`/wallet adapter it has. No signer is ever constructed, stored, or
// persisted here.

import type { Address, Instruction } from '@solana/kit';
import {
  collateralVaultPda,
  loanVaultPda,
  marketPda,
  positionPda,
  protocolPda,
} from './pda.js';
import { fetchPositionIfExists } from './accounts.js';
import type { Rpc, SolanaRpcApi } from '@solana/kit';
import {
  buildAccrueInterestInstruction,
  buildBorrowInstruction,
  buildClosePositionInstruction,
  buildCreateMarketInstruction,
  buildDepositCollateralInstruction,
  buildInitPositionInstruction,
  buildInitializeProtocolInstruction,
  buildLiquidateInstruction,
  buildRepayInstruction,
  buildSupplyInstruction,
  buildWithdrawCollateralFeesInstruction,
  buildWithdrawCollateralInstruction,
  buildWithdrawInstruction,
  type CreateMarketArgs,
  type InitProtocolArgs,
} from './generated/index.js';
import { SYSTEM_PROGRAM_ADDRESS } from './config.js';

export {
  buildAccrueInterestInstruction,
  buildBorrowInstruction,
  buildClosePositionInstruction,
  buildCreateMarketInstruction,
  buildDepositCollateralInstruction,
  buildInitPositionInstruction,
  buildInitializeProtocolInstruction,
  buildLiquidateInstruction,
  buildRepayInstruction,
  buildSupplyInstruction,
  buildWithdrawCollateralFeesInstruction,
  buildWithdrawCollateralInstruction,
  buildWithdrawInstruction,
};
export type { CreateMarketArgs, InitProtocolArgs };

export interface MarketIdentity {
  programId: Address;
  collateralMint: Address;
  loanMint: Address;
  configId: number;
}

export interface MarketAddresses {
  market: Address;
  collateralVault: Address;
  loanVault: Address;
  feePosition: Address;
}

/** Derives every PDA a market-level instruction needs from its identity, plus the market's own
 *  `fee_recipient` (needed to derive `fee_position`) -- one round trip, since `fee_position`'s seed
 *  depends on the already-created market's own field. */
export async function deriveMarketAddresses(
  identity: MarketIdentity,
  feeRecipient: Address,
): Promise<MarketAddresses> {
  const market = await marketPda(identity.programId, identity.collateralMint, identity.loanMint, identity.configId);
  const [collateralVault, loanVault, feePosition] = await Promise.all([
    collateralVaultPda(identity.programId, market),
    loanVaultPda(identity.programId, market),
    positionPda(identity.programId, market, feeRecipient),
  ]);
  return { market, collateralVault, loanVault, feePosition };
}

/**
 * Bundles `init_position` with a user's first action into one array of instructions
 * (item 16), only when the position does not already exist -- checked via `fetchPositionIfExists`
 * so a returning user's second deposit does not redundantly attempt to re-`init` (which Anchor's
 * `init` would reject anyway, but this SDK should not even try). Callers append the returned
 * instructions to a single transaction message.
 */
export async function withInitPositionIfNeeded(
  rpc: Rpc<SolanaRpcApi>,
  programId: Address,
  market: Address,
  owner: Address,
  payer: Address,
  firstAction: Instruction,
): Promise<Instruction[]> {
  const position = await positionPda(programId, market, owner);
  const existing = await fetchPositionIfExists(rpc, position, programId);
  if (existing) return [firstAction];
  const initIx = buildInitPositionInstruction(programId, {
    payer,
    market,
    owner,
    position,
    systemProgram: SYSTEM_PROGRAM_ADDRESS,
  });
  return [initIx, firstAction];
}

/** Convenience: derives `position` for the caller so it does not need to be passed twice (once to
 *  derive, once to build the deposit). */
export async function buildDepositCollateralWithInitIfNeeded(
  rpc: Rpc<SolanaRpcApi>,
  programId: Address,
  accounts: {
    depositor: Address;
    market: Address;
    owner: Address;
    collateralVault: Address;
    depositorCollateralAta: Address;
    collateralMint: Address;
    collateralTokenProgram: Address;
  },
  amount: bigint,
): Promise<Instruction[]> {
  const position = await positionPda(programId, accounts.market, accounts.owner);
  const depositIx = buildDepositCollateralInstruction(
    programId,
    {
      depositor: accounts.depositor,
      market: accounts.market,
      position,
      collateralVault: accounts.collateralVault,
      depositorCollateralAta: accounts.depositorCollateralAta,
      collateralMint: accounts.collateralMint,
      collateralTokenProgram: accounts.collateralTokenProgram,
    },
    { amount },
  );
  return withInitPositionIfNeeded(rpc, programId, accounts.market, accounts.owner, accounts.depositor, depositIx);
}

/** Same bundling for a lender's first `supply` (economic-model.md's other same-position-model
 *  first action, item 16's "init + supply if architecture allows same position model" -- Aegis
 *  uses exactly one `Position` type for all three roles, `account-model.md` §5, so this is the
 *  identical mechanism as the collateral case). */
export async function buildSupplyWithInitIfNeeded(
  rpc: Rpc<SolanaRpcApi>,
  programId: Address,
  accounts: {
    owner: Address;
    market: Address;
    feePosition: Address;
    loanVault: Address;
    ownerLoanAta: Address;
    loanMint: Address;
    loanTokenProgram: Address;
  },
  assets: bigint,
  shares: bigint,
): Promise<Instruction[]> {
  const position = await positionPda(programId, accounts.market, accounts.owner);
  const supplyIx = buildSupplyInstruction(
    programId,
    {
      owner: accounts.owner,
      market: accounts.market,
      position,
      feePosition: accounts.feePosition,
      loanVault: accounts.loanVault,
      ownerLoanAta: accounts.ownerLoanAta,
      loanMint: accounts.loanMint,
      loanTokenProgram: accounts.loanTokenProgram,
    },
    { assets, shares },
  );
  return withInitPositionIfNeeded(rpc, programId, accounts.market, accounts.owner, accounts.owner, supplyIx);
}

export { protocolPda };
