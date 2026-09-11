// The keeper loop: scan -> filter candidates (off-chain, advisory) -> build -> send -> handle
// races/staleness/rejection as ordinary outcomes (spec #21). On-chain Aegis remains authoritative
// for every liquidation decision; nothing here shortcuts the oracle or health checks it performs.

import type { Address, KeyPairSigner, Rpc, RpcSubscriptions, SolanaRpcApi, SolanaRpcSubscriptionsApi } from '@solana/kit';
import type { BorshCoder } from '@anchor-lang/core';
import { fetchMarket, scanPositions, type DecodedMarket, type DecodedPosition } from './scan.js';
import { estimateHealthFactor, isLikelyLiquidatable, type PriceBand } from './health.js';
import { buildLiquidateInstruction, type LiquidateAccounts } from './txBuilders.js';
import { buildSignAndSend } from './send.js';

/** Resolves the two oracle price-update accounts a `liquidate` call needs for a market. Kept
 *  behind an interface (ADR-0011) so the required, offline path can supply a local fixture while
 *  a real deployment would fetch fresh Pyth Hermes updates and post them -- that network step is
 *  deliberately out of this bot's scope; see `bots/liquidator/README.md`. */
export interface PriceResolver {
  resolve(market: DecodedMarket): Promise<{
    collateralPriceUpdate: Address;
    loanPriceUpdate: Address;
    band: PriceBand;
  }>;
}

export interface CallbackProvider {
  /** Builds the optional callback for a candidate liquidation, or `undefined` to liquidate
   *  without one (the ordinary, pre-funded path). A real keeper would choose this based on its
   *  own loan-asset balance; this bot's default policy is: use the callback whenever the
   *  keeper's own balance can't cover the repayment. */
  build(
    market: DecodedMarket,
    position: DecodedPosition,
    repayAssets: bigint,
  ): Promise<LiquidateAccounts['callback'] | undefined>;
}

export interface LiquidationAttempt {
  position: Address;
  usedCallback: boolean;
  repayAssets: bigint;
  signature?: string;
  error?: string;
}

export async function scanAndLiquidateOneMarket(
  rpc: Rpc<SolanaRpcApi>,
  rpcSubscriptions: RpcSubscriptions<SolanaRpcSubscriptionsApi>,
  coder: BorshCoder,
  aegisProgramId: Address,
  marketAddress: Address,
  payer: KeyPairSigner,
  liquidatorLoanAta: Address,
  liquidatorCollateralAta: Address,
  priceResolver: PriceResolver,
  callbackProvider: CallbackProvider,
  minProfitBaseUnits: bigint,
): Promise<LiquidationAttempt[]> {
  const market = await fetchMarket(rpc, coder, marketAddress);
  const positions = await scanPositions(rpc, coder, aegisProgramId, marketAddress);
  const { collateralPriceUpdate, loanPriceUpdate, band } = await priceResolver.resolve(market);

  const attempts: LiquidationAttempt[] = [];

  for (const position of positions) {
    if (position.borrowShares === 0n || position.collateralAmount === 0n) continue;

    // Advisory-only candidate filter (spec #21) -- staleness, a race with the owner's own repay,
    // or another liquidator winning first all just mean this candidate turns out to be wrong by
    // the time the transaction lands; on-chain Aegis is what actually decides.
    const hf = estimateHealthFactor(
      { ...market, liqThreshold: market.liqThreshold },
      position,
      band,
    );
    if (!isLikelyLiquidatable(hf)) continue;

    // Advisory full-repay estimate for candidate sizing only -- the real `max_repay` bound
    // (close factor / dust rule / full-liquidation threshold) is computed on-chain
    // (`economic-model.md` §7.1); this bot does not need to reproduce it exactly to pick a
    // reasonable trial amount, since the chain will clamp or reject it as needed.
    const debtAssets =
      market.totalBorrowShares === 0n
        ? 0n
        : (position.borrowShares * market.totalBorrowAssets + market.totalBorrowShares - 1n) /
          market.totalBorrowShares;
    if (debtAssets === 0n) continue;

    const repayAssets = debtAssets;
    if (repayAssets < minProfitBaseUnits) continue;

    const callback = await callbackProvider.build(market, position, repayAssets);

    const accounts: LiquidateAccounts = {
      liquidator: payer.address,
      market: market.address,
      position: position.address,
      feePosition: await deriveFeePosition(rpc, coder, aegisProgramId, market),
      loanVault: market.loanVault,
      collateralVault: market.collateralVault,
      liquidatorLoanAta,
      liquidatorCollateralAta,
      loanMint: market.loanMint,
      collateralMint: market.collateralMint,
      loanTokenProgram: market.loanTokenProgram,
      collateralTokenProgram: market.collateralTokenProgram,
      collateralPriceUpdate,
      loanPriceUpdate,
      callback,
    };

    const ix = buildLiquidateInstruction(coder, aegisProgramId, accounts, repayAssets, 0n);

    try {
      const signature = await buildSignAndSend(rpc, rpcSubscriptions, payer, [ix]);
      attempts.push({
        position: position.address,
        usedCallback: callback !== undefined,
        repayAssets,
        signature,
      });
    } catch (err) {
      // Expected in normal operation: the position may have become healthy, another liquidator
      // may have already acted, the oracle price may have moved, or the trial repay amount may
      // exceed the on-chain max_repay bound. None of these are bugs in this keeper.
      attempts.push({
        position: position.address,
        usedCallback: callback !== undefined,
        repayAssets,
        error: err instanceof Error ? err.message : String(err),
      });
    }
  }

  return attempts;
}

async function deriveFeePosition(
  rpc: Rpc<SolanaRpcApi>,
  _coder: BorshCoder,
  aegisProgramId: Address,
  market: DecodedMarket,
): Promise<Address> {
  const { positionPda } = await import('./pda.js');
  return positionPda(aegisProgramId, market.address, market.feeRecipient);
}
