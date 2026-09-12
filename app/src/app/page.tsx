'use client';

// Market list (`docs/phases/phase-09-sdk-ui.md` item 28). Uses the SDK's read models
// (`listMarketViews`) exclusively -- no hand-coded account parsing in this component.

import { useEffect, useState } from 'react';
import { listMarketViews, type MarketView } from '@aegis/sdk';
import { useAegisClient } from '@/lib/AegisProvider';
import { formatBaseUnitsToDecimal, formatPercent } from '@/lib/format';

export default function MarketsPage() {
  const { rpc, programId } = useAegisClient();
  const [markets, setMarkets] = useState<MarketView[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function load() {
    try {
      const now = BigInt(Math.floor(Date.now() / 1000));
      const views = await listMarketViews(rpc, programId, now);
      setMarkets(views);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  useEffect(() => {
    void load();
    const id = setInterval(() => void load(), 8000);
    return () => clearInterval(id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [rpc, programId]);

  return (
    <div>
      <h1>Markets</h1>
      <p className="muted">
        Local Surfpool at the RPC endpoint configured for this app. See{' '}
        <a href="/demo" className="market-link">
          the demo
        </a>{' '}
        to seed a market from a clean local validator.
      </p>
      {error && <div className="error-box">Could not load markets: {error}</div>}
      {markets && markets.length === 0 && (
        <div className="card">
          No markets found yet. Visit{' '}
          <a href="/demo" className="market-link">
            /demo
          </a>{' '}
          to seed one from this local validator.
        </div>
      )}
      {markets && markets.length > 0 && (
        <div className="card">
          <table>
            <thead>
              <tr>
                <th>Market</th>
                <th>Supplied</th>
                <th>Borrowed</th>
                <th>Available</th>
                <th>Utilization</th>
                <th>Supply APY</th>
                <th>Borrow APY</th>
                <th>LTV / LT</th>
                <th>Risk</th>
              </tr>
            </thead>
            <tbody>
              {markets.map((m) => (
                <tr key={m.address}>
                  <td>
                    <a href={`/market/${m.address}`} className="market-link mono">
                      {m.address.slice(0, 6)}…{m.address.slice(-4)}
                    </a>
                    <div className="muted" style={{ fontSize: '0.78rem' }}>
                      config #{m.market.configId}
                    </div>
                  </td>
                  <td>{formatBaseUnitsToDecimal(m.suppliedLiquidity, m.market.loanDecimals)}</td>
                  <td>{formatBaseUnitsToDecimal(m.borrowedLiquidity, m.market.loanDecimals)}</td>
                  <td>{formatBaseUnitsToDecimal(m.availableLiquidity, m.market.loanDecimals)}</td>
                  <td>{formatPercent(m.utilizationPercent)}</td>
                  <td>{formatPercent(m.supplyApyPercent)}</td>
                  <td>{formatPercent(m.borrowApyPercent)}</td>
                  <td>
                    {formatPercent(m.maxLtvPercent, 0)} / {formatPercent(m.liqThresholdPercent, 0)}
                  </td>
                  <td>
                    <div className="risk-row">
                      {m.ackFreezeAuthority && <span className="pill pill-warn">freeze authority</span>}
                      {(m.paused.supply || m.paused.borrow || m.paused.withdraw || m.paused.liquidate) && (
                        <span className="pill pill-danger">paused</span>
                      )}
                      {!m.ackFreezeAuthority &&
                        !m.paused.supply &&
                        !m.paused.borrow &&
                        !m.paused.withdraw &&
                        !m.paused.liquidate && <span className="pill pill-ok">normal</span>}
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
