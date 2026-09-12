// I-SDK-01: TS math consumes the SAME JSON vectors the real Rust `aegis-math` emits
// (`crates/aegis-test-kit/examples/phase9_vectors_dump.rs` -> `tests/vectors/*.json`). No expected
// value in this file is hand-typed -- every assertion compares `math.ts`'s output against a number
// that came out of the actual frozen Rust implementation. If `tests/vectors/*.json` is stale
// relative to `aegis-math`, `scripts/check-vectors.sh` (`npm run vectors:check`) catches it
// separately; this file only proves the TS port agrees with whatever vectors are currently
// committed.

import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import * as math from '../src/math.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const VECTORS_DIR = path.resolve(__dirname, '../../../tests/vectors');

function loadVector(name: string): any {
  return JSON.parse(readFileSync(path.join(VECTORS_DIR, `${name}.json`), 'utf8'));
}

function expectErrorOrValue(fn: () => bigint, expected: string | null, errorField: string | null) {
  if (errorField) {
    expect(() => fn()).toThrowError();
    try {
      fn();
    } catch (err) {
      expect((err as { code: string }).code).toBe(errorField);
    }
  } else {
    expect(fn().toString()).toBe(expected);
  }
}

describe('fixed.json -- mulDivFloor / mulDivCeil', () => {
  const v = loadVector('fixed');
  for (const c of v.cases) {
    it(c.name, () => {
      const a = BigInt(c.inputs.a);
      const b = BigInt(c.inputs.b);
      const d = BigInt(c.inputs.d);
      expectErrorOrValue(() => math.mulDivFloor(a, b, d), c.floor, c.floorError);
      expectErrorOrValue(() => math.mulDivCeil(a, b, d), c.ceil, c.ceilError);
    });
  }
});

describe('shares.json -- toShares*/toAssets*', () => {
  const v = loadVector('shares');
  for (const c of v.cases) {
    it(c.name, () => {
      if ('assets' in c.inputs) {
        const assets = BigInt(c.inputs.assets);
        const totalAssets = BigInt(c.inputs.totalAssets);
        const totalShares = BigInt(c.inputs.totalShares);
        if (c.toSharesDown !== null) {
          expect(math.toSharesDown(assets, totalAssets, totalShares).toString()).toBe(c.toSharesDown);
        }
        if (c.toSharesUp !== null) {
          expect(math.toSharesUp(assets, totalAssets, totalShares).toString()).toBe(c.toSharesUp);
        }
      } else {
        const shares = BigInt(c.inputs.shares);
        const totalAssets = BigInt(c.inputs.totalAssets);
        const totalShares = BigInt(c.inputs.totalShares);
        if (c.toAssetsDown !== null) {
          expect(math.toAssetsDown(shares, totalAssets, totalShares).toString()).toBe(c.toAssetsDown);
        }
        if (c.toAssetsUp !== null) {
          expect(math.toAssetsUp(shares, totalAssets, totalShares).toString()).toBe(c.toAssetsUp);
        }
      }
    });
  }
});

describe('irm.json -- utilization / borrowRate / taylor', () => {
  const v = loadVector('irm');

  for (const c of v.utilizationCases) {
    it(`utilization: ${c.name}`, () => {
      const got = math.utilization(BigInt(c.inputs.totalBorrowAssets), BigInt(c.inputs.totalSupplyAssets));
      expect(got.toString()).toBe(c.utilization);
    });
  }

  for (const c of v.rateCases) {
    it(`borrowRate: ${c.name}`, () => {
      const got = math.borrowRate(
        BigInt(c.inputs.u),
        BigInt(c.inputs.baseRatePs),
        BigInt(c.inputs.slope1Ps),
        BigInt(c.inputs.slope2Ps),
        BigInt(c.inputs.uKink),
        BigInt(c.inputs.maxRatePs),
      );
      expect(got.toString()).toBe(c.rate);
    });
  }

  for (const c of v.taylorCases) {
    it(`taylor: ${c.name}`, () => {
      const x = math.taylorX(BigInt(c.inputs.ratePerSecondWad), BigInt(c.inputs.dtSeconds));
      expect(x.toString()).toBe(c.x);
      expect(math.taylor3(x).toString()).toBe(c.growth);
    });
  }

  it('worked accrual example matches exactly, end to end', () => {
    const w = v.workedAccrual;
    const u = math.utilization(BigInt(w.inputs.totalBorrowAssets), BigInt(w.inputs.totalSupplyAssets));
    expect(u.toString()).toBe(w.utilization);
    const r = math.borrowRate(u, 0n, 1_268_391_679n, 31_709_791_983n, 800_000_000_000_000_000n, 317_097_919_837n);
    expect(r.toString()).toBe(w.ratePerSecondWad);
    const x = math.taylorX(r, BigInt(w.inputs.dtSeconds));
    expect(x.toString()).toBe(w.x);
    const growth = math.taylor3(x);
    expect(growth.toString()).toBe(w.growth);
    const interest = math.mulDivFloor(BigInt(w.inputs.totalBorrowAssets), growth, math.WAD);
    expect(interest.toString()).toBe(w.interest);
    const feeAmount = math.mulDivFloor(interest, BigInt(w.inputs.feeWad), math.WAD);
    expect(feeAmount.toString()).toBe(w.feeAmount);
  });
});

describe('health.json -- scale/band/valuation/HF', () => {
  const v = loadVector('health');

  for (const c of v.scaleCases) {
    it(`scale: ${c.name}`, () => {
      const raw = BigInt(c.inputs.raw);
      const expo = c.inputs.expo;
      if (c.floor !== null) expect(math.scaleToWadFloor(raw, expo).toString()).toBe(c.floor);
      if (c.ceil !== null) expect(math.scaleToWadCeil(raw, expo).toString()).toBe(c.ceil);
    });
  }

  for (const c of v.bandCases) {
    it(`band: ${c.name}`, () => {
      const price = BigInt(c.inputs.price);
      const conf = BigInt(c.inputs.conf);
      const expo = c.inputs.expo;
      const maxConfBps = c.inputs.maxConfBps;
      if (c.error) {
        expect(() => math.conservativePriceBand(price, conf, expo, maxConfBps)).toThrowError();
        try {
          math.conservativePriceBand(price, conf, expo, maxConfBps);
        } catch (err) {
          expect((err as { code: string }).code).toBe(c.error);
        }
      } else {
        const band = math.conservativePriceBand(price, conf, expo, maxConfBps);
        expect(band.lo.toString()).toBe(c.lo);
        expect(band.hi.toString()).toBe(c.hi);
      }
    });
  }

  it('worked examples: healthy and crashed positions match exactly', () => {
    const { healthy, crashed } = v.workedExamples;

    const cv1 = math.collateralValue(
      BigInt(healthy.inputs.collateralAmount),
      BigInt(healthy.inputs.priceCLo),
      healthy.inputs.collateralDecimals,
    );
    const dv1 = math.debtValue(
      BigInt(healthy.inputs.debtAssets),
      BigInt(healthy.inputs.priceLHi),
      healthy.inputs.loanDecimals,
    );
    expect(cv1.toString()).toBe(healthy.collateralValue);
    expect(dv1.toString()).toBe(healthy.debtValue);
    const hf1 = math.healthFactor(cv1, BigInt(healthy.inputs.liqThreshold), dv1);
    expect(hf1.toString()).toBe(healthy.healthFactor);
    expect(math.isWithinMaxLtv(cv1, dv1, BigInt(healthy.inputs.maxLtv))).toBe(healthy.isWithinMaxLtv);

    const cv2 = math.collateralValue(
      BigInt(crashed.inputs.collateralAmount),
      BigInt(crashed.inputs.priceCLo),
      crashed.inputs.collateralDecimals,
    );
    const dv2 = math.debtValue(
      BigInt(crashed.inputs.debtAssets),
      BigInt(crashed.inputs.priceLHi),
      crashed.inputs.loanDecimals,
    );
    expect(cv2.toString()).toBe(crashed.collateralValue);
    expect(dv2.toString()).toBe(crashed.debtValue);
    const hf2 = math.healthFactor(cv2, BigInt(crashed.inputs.liqThreshold), dv2);
    expect(hf2.toString()).toBe(crashed.healthFactor);
  });
});

describe('liquidation.json -- maxRepay / isLiquidatable / compute*', () => {
  const v = loadVector('liquidation');

  function params(): math.LiquidationParams {
    return {
      collateralDecimals: v.params.collateralDecimals,
      loanDecimals: v.params.loanDecimals,
      priceCLo: BigInt(v.params.priceCLo),
      priceLHi: BigInt(v.params.priceLHi),
      liqBonus: BigInt(v.params.liqBonus),
      liqProtocolFee: BigInt(v.params.liqProtocolFee),
      closeFactor: BigInt(v.params.closeFactor),
      fullLiqHf: BigInt(v.params.fullLiqHf),
      minDebt: BigInt(v.params.minDebt),
    };
  }

  for (const c of v.isLiquidatableCases) {
    it(`isLiquidatable: ${c.name}`, () => {
      expect(math.isLiquidatable(BigInt(c.hf))).toBe(c.isLiquidatable);
    });
  }

  it('maxRepay: full liquidation band', () => {
    const c = v.maxRepayCases[0];
    const got = math.maxRepay(BigInt(c.inputs.debtAssets), BigInt(c.inputs.hf), params());
    expect(got.toString()).toBe(c.maxRepay);
  });

  it('maxRepay: close-factor band', () => {
    const c = v.maxRepayCases[1];
    const p = { ...params(), closeFactor: BigInt(c.inputs.closeFactor), minDebt: BigInt(c.inputs.minDebt) };
    const got = math.maxRepay(BigInt(c.inputs.debtAssets), BigInt(c.inputs.hf), p);
    expect(got.toString()).toBe(c.maxRepay);
  });

  it('maxRepay: dust rule forces full repayment', () => {
    const c = v.maxRepayCases[2];
    const p = { ...params(), closeFactor: BigInt(c.inputs.closeFactor), minDebt: BigInt(c.inputs.minDebt) };
    const got = math.maxRepay(BigInt(c.inputs.debtAssets), BigInt(c.inputs.hf), p);
    expect(got.toString()).toBe(c.maxRepay);
  });

  function expectOutcome(got: math.LiquidationOutcome, expected: any) {
    expect(got.repayAssets.toString()).toBe(expected.repayAssets);
    expect(got.totalSeize.toString()).toBe(expected.totalSeize);
    expect(got.baseSeize.toString()).toBe(expected.baseSeize);
    expect(got.bonusAmount.toString()).toBe(expected.bonusAmount);
    expect(got.protocolCut.toString()).toBe(expected.protocolCut);
    expect(got.toLiquidator.toString()).toBe(expected.toLiquidator);
    expect(got.clamped).toBe(expected.clamped);
  }

  it('worked liquidation: sufficient collateral, no clamp', () => {
    const w = v.worked;
    const got = math.computeLiquidationByRepay(
      params(),
      BigInt(w.debtAssets),
      BigInt(w.collateralAmountSufficient),
      BigInt(w.hf),
      BigInt(w.debtAssets),
    );
    expectOutcome(got, w.byRepaySufficient);
  });

  it('worked liquidation: collateral clamp fires, repay recomputed upward', () => {
    const w = v.worked;
    const got = math.computeLiquidationByRepay(
      params(),
      BigInt(w.debtAssets),
      BigInt(w.collateralAmountInsufficient),
      BigInt(w.hf),
      BigInt(w.debtAssets),
    );
    expectOutcome(got, w.byRepayClamped);
  });

  it('compute_liquidation_by_seize matches compute_liquidation_by_repay at the same point', () => {
    const w = v.worked;
    const byRepay = math.computeLiquidationByRepay(
      params(),
      BigInt(w.debtAssets),
      BigInt(w.collateralAmountSufficient),
      BigInt(w.hf),
      BigInt(w.debtAssets),
    );
    const bySeize = math.computeLiquidationBySeize(
      params(),
      BigInt(w.debtAssets),
      BigInt(w.collateralAmountSufficient),
      BigInt(w.hf),
      byRepay.totalSeize,
    );
    expectOutcome(bySeize, w.bySeizeMatchesByRepay);
  });

  it('a healthy position (HF == WAD) is rejected before any liquidation math runs', () => {
    const w = v.healthyPositionRejected;
    expect(math.isLiquidatable(BigInt(w.hf))).toBe(false);
  });
});
