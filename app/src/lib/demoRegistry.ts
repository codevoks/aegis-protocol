'use client';

// Aegis has no on-chain registry mapping a market's configured `feed_id` to the specific account
// address currently holding that feed's posted price (`docs/account-model.md` §2: by design --
// resolving a feed id to a live price account is inherently an off-chain concern, normally Hermes's
// job). This local-only registry lets the demo/seed flow record which price-update addresses it
// injected for a given market, so the position/market pages can look them up again to compute a
// health-factor preview. Entirely a demo convenience; never part of `@aegis/sdk` itself.

export interface DemoMarketInfo {
  collateralPriceUpdate: string;
  loanPriceUpdate: string;
  collateralFeedId: string; // hex
  loanFeedId: string; // hex
}

const STORAGE_KEY = 'aegis-demo-market-registry-v1';

function readAll(): Record<string, DemoMarketInfo> {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    return raw ? JSON.parse(raw) : {};
  } catch {
    return {};
  }
}

function writeAll(value: Record<string, DemoMarketInfo>): void {
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(value));
  } catch {
    // best-effort
  }
}

export function getDemoMarketInfo(marketAddress: string): DemoMarketInfo | null {
  return readAll()[marketAddress] ?? null;
}

export function setDemoMarketInfo(marketAddress: string, info: DemoMarketInfo): void {
  const all = readAll();
  all[marketAddress] = info;
  writeAll(all);
}
