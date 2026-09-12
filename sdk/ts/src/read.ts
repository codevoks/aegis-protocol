// Read models (`docs/phases/phase-09-sdk-ui.md` item 7; `architecture.md` §5). Ergonomic, typed
// reads for protocol/market/position state plus every derived, user-facing quantity the UI needs.
//
// All protocol-critical computation goes through `math.ts` (bigint, exact). The one exception,
// clearly isolated per item 13, is `ratePerSecondToApyNumber` -- a UI-only floating-point display
// conversion that is never fed back into any transaction builder in `ix.ts`.

import type { Address, Rpc, SolanaRpcApi } from '@solana/kit';
import * as math from './math.js';
import { fetchMarket, fetchPosition, fetchProtocol, discoverMarkets } from './accounts.js';
import type { Market, Position, Protocol } from './generated/index.js';

// --- accrue_view, ported for read models (economic-model.md §4.5): a pure, non-mutating
// projection of a Market's accounting totals to "now", exactly mirroring
// `programs/aegis/src/state/market.rs`'s `accrue_view` so the UI never displays stale
// pre-accrual numbers. Validated indirectly by the same `worked_accrual` vector `math.test.ts`
// exercises directly on its constituent primitives (utilization/borrowRate/taylorX/taylor3).

export interface AccruedTotals {
  totalSupplyAssets: bigint;
  totalBorrowAssets: bigint;
  utilizationWad: bigint;
  borrowRatePerSecondWad: bigint;
}

export function accrueView(market: Market, nowUnixSeconds: bigint): AccruedTotals {
  const dt = nowUnixSeconds > market.lastAccrualTs ? nowUnixSeconds - market.lastAccrualTs : 0n;
  const u = math.utilization(market.totalBorrowAssets, market.totalSupplyAssets);
  const r = math.borrowRate(
    u,
    market.baseRatePs,
    market.slope1Ps,
    market.slope2Ps,
    market.uKink,
    market.maxRatePs,
  );
  if (dt === 0n) {
    return {
      totalSupplyAssets: market.totalSupplyAssets,
      totalBorrowAssets: market.totalBorrowAssets,
      utilizationWad: u,
      borrowRatePerSecondWad: r,
    };
  }
  const x = math.taylorX(r, dt);
  const growth = math.taylor3(x);
  const interest = math.mulDivFloor(market.totalBorrowAssets, growth, math.WAD);
  return {
    totalSupplyAssets: market.totalSupplyAssets + interest,
    totalBorrowAssets: market.totalBorrowAssets + interest,
    utilizationWad: u,
    borrowRatePerSecondWad: r,
  };
}

/** Suppliers' per-second rate: the fraction of borrower interest (net of the protocol fee) that
 *  flows to lenders, pro-rata -- a standard derived quantity (Compound/Aave-style), computed
 *  exactly in WAD via `mulDivFloor` (never a float). */
export function supplyRatePerSecond(market: Market, utilizationWad: bigint, borrowRatePs: bigint): bigint {
  const grossToSuppliers = math.mulDivFloor(borrowRatePs, utilizationWad, math.WAD);
  const netOfFee = math.mulDivFloor(grossToSuppliers, math.WAD - market.fee, math.WAD);
  return netOfFee;
}

/** UI-ONLY: converts a WAD per-second rate to a human compounded-APY percentage number (e.g.
 *  `5.42` for 5.42%). Uses floating point deliberately -- this is a display string, never fed back
 *  into `ix.ts` or any protocol-critical computation (item 13). */
export function ratePerSecondToApyPercent(ratePerSecondWad: bigint): number {
  const perSecond = Number(ratePerSecondWad) / Number(math.WAD);
  const secondsPerYear = Number(math.SECONDS_PER_YEAR);
  return (Math.pow(1 + perSecond, secondsPerYear) - 1) * 100;
}

export interface OracleRiskParams {
  maxPriceAgeSecs: number;
  maxConfBps: number;
}

export interface MarketView {
  address: Address;
  market: Market;
  now: bigint;
  accrued: AccruedTotals;

  suppliedLiquidity: bigint; // total_supply_assets, accrued
  borrowedLiquidity: bigint; // total_borrow_assets, accrued
  availableLiquidity: bigint; // supplied - borrowed
  utilizationPercent: number; // UI-only
  supplyApyPercent: number; // UI-only
  borrowApyPercent: number; // UI-only

  maxLtvPercent: number; // UI-only
  liqThresholdPercent: number; // UI-only

  ackFreezeAuthority: boolean;
  oracle: OracleRiskParams;
  paused: {
    supply: boolean;
    borrow: boolean;
    withdraw: boolean;
    liquidate: boolean;
  };
}

const PAUSE_SUPPLY = 0b0001;
const PAUSE_BORROW = 0b0010;
const PAUSE_WITHDRAW = 0b0100;
const PAUSE_LIQUIDATE = 0b1000;
const FLAG_ACK_FREEZE_AUTHORITY = 0b01;

export function buildMarketView(address: Address, market: Market, nowUnixSeconds: bigint): MarketView {
  const accrued = accrueView(market, nowUnixSeconds);
  const supplyRatePs = supplyRatePerSecond(market, accrued.utilizationWad, accrued.borrowRatePerSecondWad);

  return {
    address,
    market,
    now: nowUnixSeconds,
    accrued,
    suppliedLiquidity: accrued.totalSupplyAssets,
    borrowedLiquidity: accrued.totalBorrowAssets,
    availableLiquidity: accrued.totalSupplyAssets - accrued.totalBorrowAssets,
    utilizationPercent: Number(accrued.utilizationWad) / Number(math.WAD) * 100,
    supplyApyPercent: ratePerSecondToApyPercent(supplyRatePs),
    borrowApyPercent: ratePerSecondToApyPercent(accrued.borrowRatePerSecondWad),
    maxLtvPercent: Number(market.maxLtv) / Number(math.WAD) * 100,
    liqThresholdPercent: Number(market.liqThreshold) / Number(math.WAD) * 100,
    ackFreezeAuthority: (market.flags & FLAG_ACK_FREEZE_AUTHORITY) !== 0,
    oracle: { maxPriceAgeSecs: market.maxPriceAgeSecs, maxConfBps: market.maxConfBps },
    paused: {
      supply: (market.paused & PAUSE_SUPPLY) !== 0,
      borrow: (market.paused & PAUSE_BORROW) !== 0,
      withdraw: (market.paused & PAUSE_WITHDRAW) !== 0,
      liquidate: (market.paused & PAUSE_LIQUIDATE) !== 0,
    },
  };
}

export interface PositionView {
  address: Address;
  position: Position;
  market: MarketView;

  collateralAmount: bigint;
  debtAssets: bigint; // to_assets_up(borrow_shares, ...) against accrued totals
  supplyAssets: bigint; // to_assets_down(supply_shares, ...) against accrued totals

  collateralValueWad: bigint | null; // null if no price band supplied
  debtValueWad: bigint | null;
  healthFactorWad: bigint | null;
  isLiquidatable: boolean | null;
  currentLtvPercent: number | null; // UI-only
  liquidationPrice: bigint | null; // WAD, collateral price at which HF == WAD
}

/** A conservative price band for both assets of a market, as `math.ts`'s
 *  `conservativePriceBand` produces from a raw oracle quote -- the same shape `oracle.ts`'s
 *  `PriceQuote` feeds into via that function. */
export interface MarketPriceBands {
  collateral: math.PriceBandWad;
  loan: math.PriceBandWad;
}

export function buildPositionView(
  address: Address,
  position: Position,
  marketView: MarketView,
  prices: MarketPriceBands | null,
): PositionView {
  // `totalBorrowShares` is unaffected by accrual (only the asset totals grow, economic-model.md
  // §4.2) -- the on-chain stored share count is still the correct denominator against the
  // accrued asset total.
  const debtAssets = math.toAssetsUp(
    position.borrowShares,
    marketView.accrued.totalBorrowAssets,
    marketView.market.totalBorrowShares,
  );
  const supplyAssets = math.toAssetsDown(
    position.supplyShares,
    marketView.accrued.totalSupplyAssets,
    marketView.market.totalSupplyShares,
  );

  let collateralValueWad: bigint | null = null;
  let debtValueWad: bigint | null = null;
  let healthFactorWad: bigint | null = null;
  let isLiquidatable: boolean | null = null;
  let currentLtvPercent: number | null = null;
  let liquidationPrice: bigint | null = null;

  if (prices) {
    collateralValueWad = math.collateralValue(
      position.collateralAmount,
      prices.collateral.lo,
      marketView.market.collateralDecimals,
    );
    debtValueWad = math.debtValue(debtAssets, prices.loan.hi, marketView.market.loanDecimals);
    healthFactorWad = math.healthFactor(collateralValueWad, marketView.market.liqThreshold, debtValueWad);
    isLiquidatable = math.isLiquidatable(healthFactorWad);
    currentLtvPercent =
      collateralValueWad === 0n ? null : (Number(debtValueWad) / Number(collateralValueWad)) * 100;
    liquidationPrice = math.liquidationPrice(
      debtValueWad,
      position.collateralAmount,
      marketView.market.collateralDecimals,
      marketView.market.liqThreshold,
    );
  }

  return {
    address,
    position,
    market: marketView,
    collateralAmount: position.collateralAmount,
    debtAssets,
    supplyAssets,
    collateralValueWad,
    debtValueWad,
    healthFactorWad,
    isLiquidatable,
    currentLtvPercent,
    liquidationPrice,
  };
}

// --- convenience fetch + view in one call ---

export async function readMarketView(
  rpc: Rpc<SolanaRpcApi>,
  programId: Address,
  marketAddress: Address,
  nowUnixSeconds: bigint,
): Promise<MarketView> {
  const market = await fetchMarket(rpc, marketAddress, programId);
  return buildMarketView(marketAddress, market, nowUnixSeconds);
}

export async function readPositionView(
  rpc: Rpc<SolanaRpcApi>,
  programId: Address,
  positionAddress: Address,
  marketView: MarketView,
  prices: MarketPriceBands | null,
): Promise<PositionView> {
  const position = await fetchPosition(rpc, positionAddress, programId);
  return buildPositionView(positionAddress, position, marketView, prices);
}

export async function readProtocolView(
  rpc: Rpc<SolanaRpcApi>,
  programId: Address,
  protocolAddress: Address,
): Promise<Protocol> {
  return fetchProtocol(rpc, protocolAddress, programId);
}

/** Every market Aegis knows about, as full views (item 7: "market list/discovery"). Aegis has no
 *  on-chain registry by design, so this discovers via `getProgramAccounts` (`accounts.ts`). */
export async function listMarketViews(
  rpc: Rpc<SolanaRpcApi>,
  programId: Address,
  nowUnixSeconds: bigint,
): Promise<MarketView[]> {
  const found = await discoverMarkets(rpc, programId);
  return found.map(({ address, market }) => buildMarketView(address, market, nowUnixSeconds));
}
