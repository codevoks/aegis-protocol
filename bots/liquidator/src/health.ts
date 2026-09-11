// Off-chain health-factor estimate, mirroring `crates/aegis-math/src/health.rs`'s formula in
// plain BigInt (exact integer arithmetic, no floating point, no precision loss -- the same
// discipline `aegis-math` uses on-chain, even though this code never runs on-chain).
//
// ADVISORY ONLY (spec #21). This exists to pick liquidation *candidates* cheaply, off-chain,
// before spending a transaction. The on-chain `liquidate` instruction re-validates the oracle and
// re-computes health from scratch under its own strict rules (INV-ORA-07, INV-LIQ-01) --
// nothing here is trusted for the actual liquidation decision. A position this module thinks is
// unhealthy may turn out healthy on-chain (a stale local price, another liquidator having already
// acted, a race with the position owner's own repay) and vice-versa; the keeper must treat an
// on-chain rejection as a normal, expected outcome, never a bug.

export const WAD = 10n ** 18n;

export interface MarketSnapshot {
  liqThreshold: bigint; // WAD
  collateralDecimals: number;
  loanDecimals: number;
  totalBorrowAssets: bigint;
  totalBorrowShares: bigint;
}

export interface PositionSnapshot {
  collateralAmount: bigint;
  borrowShares: bigint;
}

export interface PriceBand {
  /** Conservative low price (WAD USD per whole collateral token), used for collateral valuation. */
  collateralPriceLo: bigint;
  /** Conservative high price (WAD USD per whole loan token), used for debt valuation. */
  loanPriceHi: bigint;
}

function pow10(n: number): bigint {
  return 10n ** BigInt(n);
}

/** `to_assets_up`: ceil(shares * total_assets / total_shares), 0 if total_shares is 0. */
function debtAssetsUp(position: PositionSnapshot, market: MarketSnapshot): bigint {
  if (market.totalBorrowShares === 0n) return 0n;
  const numerator = position.borrowShares * market.totalBorrowAssets;
  return (numerator + market.totalBorrowShares - 1n) / market.totalBorrowShares;
}

/** `collateral_value`: floor(amount * price_lo / 10^decimals) -- WAD USD value. */
function collateralValue(amount: bigint, priceLo: bigint, decimals: number): bigint {
  return (amount * priceLo) / pow10(decimals);
}

/** `debt_value`: ceil(amount * price_hi / 10^decimals) -- WAD USD value. */
function debtValue(amount: bigint, priceHi: bigint, decimals: number): bigint {
  const denom = pow10(decimals);
  const numerator = amount * priceHi;
  return (numerator + denom - 1n) / denom;
}

/**
 * `health_factor`: `collateral_value * liq_threshold / debt_value` (WAD). `Infinity`-as-max-bigint
 * when there is no debt (never liquidatable) -- mirrors `aegis-math`'s own treatment of the
 * zero-debt case as maximally healthy.
 */
export function estimateHealthFactor(
  market: MarketSnapshot,
  position: PositionSnapshot,
  band: PriceBand,
): bigint {
  const debtAssets = debtAssetsUp(position, market);
  if (debtAssets === 0n) return BigInt(Number.MAX_SAFE_INTEGER); // no debt -> never liquidatable
  const cv = collateralValue(position.collateralAmount, band.collateralPriceLo, market.collateralDecimals);
  const dv = debtValue(debtAssets, band.loanPriceHi, market.loanDecimals);
  if (dv === 0n) return BigInt(Number.MAX_SAFE_INTEGER);
  // `economic-model.md` §6.3: HF = floor(collateral_value * liq_threshold / debt_value). Both
  // `cv` and `dv` are already WAD-scaled USD values and `liqThreshold` is a WAD-scaled ratio, so
  // this single division yields a WAD-scaled HF directly -- no separate `/ WAD` step.
  return (cv * market.liqThreshold) / dv;
}

/** Strict `HF < WAD` (INV-LIQ-01) -- the same boundary the on-chain check uses, never `<=`. */
export function isLikelyLiquidatable(hf: bigint): boolean {
  return hf < WAD;
}
