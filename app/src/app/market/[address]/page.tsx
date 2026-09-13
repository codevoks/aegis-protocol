'use client';

// Market + position screen (`docs/phases/phase-09-sdk-ui.md` items 28-35). Displays every
// required market risk parameter and the position's health, and provides deposit/withdraw/borrow/
// repay/supply/withdraw actions -- all built through `@aegis/sdk`, never a hand-coded instruction.

import { use, useCallback, useEffect, useState } from 'react';
import { getBase64Encoder, type Address } from '@solana/kit';
import { TOKEN_PROGRAM_ADDRESS, findAssociatedTokenPda, getCreateAssociatedTokenIdempotentInstruction } from '@solana-program/token';
import {
  buildBorrowInstruction,
  buildDepositCollateralInstruction,
  buildRepayInstruction,
  buildSignSendAndConfirm,
  buildSupplyInstruction,
  buildWithdrawCollateralInstruction,
  buildWithdrawInstruction,
  buildMarketView,
  buildPositionView,
  conservativePriceBand,
  fetchMarket,
  fetchPositionIfExists,
  positionPda,
  protocolPda,
  withInitPositionIfNeeded,
  type MarketView,
  type PositionView,
} from '@aegis/sdk';
import { useAegisClient } from '@/lib/AegisProvider';
import { getDemoMarketInfo } from '@/lib/demoRegistry';
import { decodePriceUpdateV2 } from '@/lib/priceUpdate';
import { formatBaseUnitsToDecimal, formatPercent, isZeroAmount, parseDecimalToBaseUnits } from '@/lib/format';

type ActionKind = 'deposit' | 'withdrawCollateral' | 'supply' | 'withdraw' | 'borrow' | 'repay';

export default function MarketDetailPage({ params }: { params: Promise<{ address: string }> }) {
  const { address: marketAddressStr } = use(params);
  const marketAddress = marketAddressStr as Address;
  const { rpc, rpcSubscriptions, programId, signer } = useAegisClient();

  const [marketView, setMarketView] = useState<MarketView | null>(null);
  const [positionView, setPositionView] = useState<PositionView | null>(null);
  const [action, setAction] = useState<ActionKind>('deposit');
  const [amountInput, setAmountInput] = useState('');
  const [formError, setFormError] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(null);

  const load = useCallback(async () => {
    const now = BigInt(Math.floor(Date.now() / 1000));
    const marketAccount = await fetchMarket(rpc, marketAddress, programId);
    const mv = buildMarketView(marketAddress, marketAccount, now);
    setMarketView(mv);

    if (!signer) return;
    const posAddr = await positionPda(programId, marketAddress, signer.address);
    const position = await fetchPositionIfExists(rpc, posAddr, programId);
    if (!position) {
      setPositionView(null);
      return;
    }

    const demoInfo = getDemoMarketInfo(marketAddress);
    let prices = null;
    if (demoInfo) {
      try {
        const base64Encoder = getBase64Encoder();
        const [collateralAcc, loanAcc] = await Promise.all([
          rpc.getAccountInfo(demoInfo.collateralPriceUpdate as Address, { encoding: 'base64' }).send(),
          rpc.getAccountInfo(demoInfo.loanPriceUpdate as Address, { encoding: 'base64' }).send(),
        ]);
        if (collateralAcc.value && loanAcc.value) {
          const cData = new Uint8Array(base64Encoder.encode(collateralAcc.value.data[0]));
          const lData = new Uint8Array(base64Encoder.encode(loanAcc.value.data[0]));
          const cUpdate = decodePriceUpdateV2(cData);
          const lUpdate = decodePriceUpdateV2(lData);
          prices = {
            collateral: conservativePriceBand(cUpdate.price, cUpdate.conf, cUpdate.expo, mv.oracle.maxConfBps),
            loan: conservativePriceBand(lUpdate.price, lUpdate.conf, lUpdate.expo, mv.oracle.maxConfBps),
          };
        }
      } catch {
        prices = null; // no live price known to the demo registry for this market -- HF preview omitted
      }
    }
    setPositionView(buildPositionView(posAddr, position, mv, prices));
  }, [rpc, marketAddress, programId, signer]);

  useEffect(() => {
    void load();
    const id = setInterval(() => void load(), 6000);
    return () => clearInterval(id);
  }, [load]);

  async function ensureAta(mint: Address, owner: Address): Promise<Address> {
    const [ata] = await findAssociatedTokenPda({ mint, owner, tokenProgram: TOKEN_PROGRAM_ADDRESS });
    if (!signer) throw new Error('not connected');
    await buildSignSendAndConfirm(rpc, rpcSubscriptions, signer, [
      getCreateAssociatedTokenIdempotentInstruction({ payer: signer, ata, owner, mint, tokenProgram: TOKEN_PROGRAM_ADDRESS }),
    ]);
    return ata;
  }

  async function submit() {
    setFormError(null);
    setStatus(null);
    if (!marketView || !signer) return;
    const decimals = action === 'borrow' || action === 'repay' || action === 'supply' || action === 'withdraw'
      ? marketView.market.loanDecimals
      : marketView.market.collateralDecimals;

    if (isZeroAmount(amountInput)) {
      setFormError('Enter a nonzero amount.');
      return;
    }
    let amount: bigint;
    try {
      amount = parseDecimalToBaseUnits(amountInput, decimals);
    } catch (err) {
      setFormError(err instanceof Error ? err.message : String(err));
      return;
    }

    try {
      setStatus('building');
      const m = marketView.market;
      const positionAddr = await positionPda(programId, marketAddress, signer.address);
      // `fee_position` is derived, not a stored `Market` field (`account-model.md` §9:
      // `PDA(market, market.fee_recipient)`).
      const feePosition = await positionPda(programId, marketAddress, m.feeRecipient);

      if (action === 'deposit') {
        const ata = await ensureAta(m.collateralMint, signer.address);
        // `m.collateralVault` comes straight off the decoded on-chain `Market` account -- no PDA
        // re-derivation needed, since the vault address is already one of its stored fields.
        const ix = buildDepositCollateralInstruction(
          programId,
          {
            depositor: signer.address,
            market: marketAddress,
            position: positionAddr,
            collateralVault: m.collateralVault,
            depositorCollateralAta: ata,
            collateralMint: m.collateralMint,
            collateralTokenProgram: TOKEN_PROGRAM_ADDRESS,
          },
          { amount },
        );
        const ixs = await withInitPositionIfNeeded(rpc, programId, marketAddress, signer.address, signer.address, ix);
        setStatus('awaiting-signature');
        await buildSignSendAndConfirm(rpc, rpcSubscriptions, signer, ixs, { onStage: (s) => setStatus(s) });
      } else if (action === 'withdrawCollateral') {
        if (!positionView) throw new Error('no position');
        const demoInfo = getDemoMarketInfo(marketAddress);
        if (!demoInfo) throw new Error('price accounts unknown to this local demo -- run the seed flow first');
        const ix = buildWithdrawCollateralInstruction(
          programId,
          {
            owner: signer.address,
            protocol: await protocolPda(programId),
            market: marketAddress,
            position: positionAddr,
            collateralVault: m.collateralVault,
            ownerCollateralAta: await ensureAta(m.collateralMint, signer.address),
            collateralMint: m.collateralMint,
            collateralTokenProgram: TOKEN_PROGRAM_ADDRESS,
            collateralPriceUpdate: demoInfo.collateralPriceUpdate as Address,
            loanPriceUpdate: demoInfo.loanPriceUpdate as Address,
          },
          { amount },
        );
        await buildSignSendAndConfirm(rpc, rpcSubscriptions, signer, [ix], { onStage: (s) => setStatus(s) });
      } else if (action === 'supply') {
        const ata = await ensureAta(m.loanMint, signer.address);
        const ix = buildSupplyInstruction(
          programId,
          {
            owner: signer.address,
            protocol: await protocolPda(programId),
            market: marketAddress,
            position: positionAddr,
            feePosition,
            loanVault: m.loanVault,
            ownerLoanAta: ata,
            loanMint: m.loanMint,
            loanTokenProgram: TOKEN_PROGRAM_ADDRESS,
          },
          { assets: amount, shares: 0n },
        );
        const ixs = await withInitPositionIfNeeded(rpc, programId, marketAddress, signer.address, signer.address, ix);
        await buildSignSendAndConfirm(rpc, rpcSubscriptions, signer, ixs, { onStage: (s) => setStatus(s) });
      } else if (action === 'withdraw') {
        const ix = buildWithdrawInstruction(
          programId,
          {
            owner: signer.address,
            protocol: await protocolPda(programId),
            market: marketAddress,
            position: positionAddr,
            feePosition,
            loanVault: m.loanVault,
            ownerLoanAta: await ensureAta(m.loanMint, signer.address),
            loanMint: m.loanMint,
            loanTokenProgram: TOKEN_PROGRAM_ADDRESS,
          },
          { assets: amount, shares: 0n },
        );
        await buildSignSendAndConfirm(rpc, rpcSubscriptions, signer, [ix], { onStage: (s) => setStatus(s) });
      } else if (action === 'borrow') {
        const demoInfo = getDemoMarketInfo(marketAddress);
        if (!demoInfo) throw new Error('price accounts unknown to this local demo -- run the seed flow first');
        const ix = buildBorrowInstruction(
          programId,
          {
            owner: signer.address,
            protocol: await protocolPda(programId),
            market: marketAddress,
            position: positionAddr,
            feePosition,
            loanVault: m.loanVault,
            ownerLoanAta: await ensureAta(m.loanMint, signer.address),
            loanMint: m.loanMint,
            loanTokenProgram: TOKEN_PROGRAM_ADDRESS,
            collateralPriceUpdate: demoInfo.collateralPriceUpdate as Address,
            loanPriceUpdate: demoInfo.loanPriceUpdate as Address,
          },
          { assets: amount, shares: 0n },
        );
        await buildSignSendAndConfirm(rpc, rpcSubscriptions, signer, [ix], { onStage: (s) => setStatus(s) });
      } else if (action === 'repay') {
        const ix = buildRepayInstruction(
          programId,
          {
            payer: signer.address,
            market: marketAddress,
            position: positionAddr,
            feePosition,
            loanVault: m.loanVault,
            payerLoanAta: await ensureAta(m.loanMint, signer.address),
            loanMint: m.loanMint,
            loanTokenProgram: TOKEN_PROGRAM_ADDRESS,
          },
          { assets: amount, shares: 0n },
        );
        await buildSignSendAndConfirm(rpc, rpcSubscriptions, signer, [ix], { onStage: (s) => setStatus(s) });
      }

      setStatus('confirmed');
      setAmountInput('');
      await load();
    } catch (err) {
      setStatus('failed');
      setFormError(err instanceof Error ? err.message : String(err));
    }
  }

  if (!marketView) return <p className="muted">Loading market…</p>;
  const m = marketView.market;

  return (
    <div>
      <h1>Market {marketAddress.slice(0, 6)}…{marketAddress.slice(-4)}</h1>

      <div className="card">
        <h3>Risk parameters</h3>
        <div className="grid-2">
          <div className="stat">
            <span className="stat-label">Max LTV / Liquidation threshold</span>
            <span className="stat-value">
              {formatPercent(marketView.maxLtvPercent, 0)} / {formatPercent(marketView.liqThresholdPercent, 0)}
            </span>
          </div>
          <div className="stat">
            <span className="stat-label">Oracle staleness bound</span>
            <span className="stat-value">{marketView.oracle.maxPriceAgeSecs}s</span>
          </div>
          <div className="stat">
            <span className="stat-label">Oracle max confidence</span>
            <span className="stat-value">{(marketView.oracle.maxConfBps / 100).toFixed(2)}%</span>
          </div>
          <div className="stat">
            <span className="stat-label">Freeze authority acknowledged</span>
            <span className="stat-value">
              {marketView.ackFreezeAuthority ? (
                <span className="pill pill-warn">
                  yes — a mint&apos;s freeze authority can block seizure/withdrawal
                </span>
              ) : (
                <span className="pill pill-ok">no freeze authority on either mint</span>
              )}
            </span>
          </div>
        </div>
        <div className="risk-row">
          {marketView.paused.supply && <span className="pill pill-danger">supply paused</span>}
          {marketView.paused.borrow && <span className="pill pill-danger">borrow paused</span>}
          {marketView.paused.withdraw && <span className="pill pill-danger">withdraw paused</span>}
          {marketView.paused.liquidate && <span className="pill pill-danger">liquidate paused</span>}
        </div>
      </div>

      <div className="grid-2">
        <div className="card">
          <h3>Market</h3>
          <div className="stat">
            <span className="stat-label">Supplied</span>
            <span className="stat-value">{formatBaseUnitsToDecimal(marketView.suppliedLiquidity, m.loanDecimals)}</span>
          </div>
          <div className="stat">
            <span className="stat-label">Borrowed</span>
            <span className="stat-value">{formatBaseUnitsToDecimal(marketView.borrowedLiquidity, m.loanDecimals)}</span>
          </div>
          <div className="stat">
            <span className="stat-label">Available liquidity</span>
            <span className="stat-value">{formatBaseUnitsToDecimal(marketView.availableLiquidity, m.loanDecimals)}</span>
          </div>
          <div className="stat">
            <span className="stat-label">Utilization / Supply APY / Borrow APY</span>
            <span className="stat-value">
              {formatPercent(marketView.utilizationPercent)} / {formatPercent(marketView.supplyApyPercent)} /{' '}
              {formatPercent(marketView.borrowApyPercent)}
            </span>
          </div>
        </div>

        <div className="card">
          <h3>Your position</h3>
          {!positionView && <p className="muted">No position yet -- your first action will create one.</p>}
          {positionView && (
            <>
              <div className="stat">
                <span className="stat-label">Collateral</span>
                <span className="stat-value">
                  {formatBaseUnitsToDecimal(positionView.collateralAmount, m.collateralDecimals)}
                </span>
              </div>
              <div className="stat">
                <span className="stat-label">Debt</span>
                <span className="stat-value">{formatBaseUnitsToDecimal(positionView.debtAssets, m.loanDecimals)}</span>
              </div>
              <div className="stat">
                <span className="stat-label">Health factor</span>
                <span className="stat-value">
                  {positionView.healthFactorWad === null
                    ? 'unknown (no live price for this local demo market)'
                    : positionView.debtAssets === 0n
                      ? '∞ (no debt)'
                      : (Number(positionView.healthFactorWad) / 1e18).toFixed(4)}
                  {positionView.isLiquidatable && <span className="pill pill-danger"> liquidatable</span>}
                </span>
              </div>
              {positionView.liquidationPrice !== null && (
                <div className="stat">
                  <span className="stat-label">Liquidation price (collateral, WAD)</span>
                  <span className="stat-value mono">{positionView.liquidationPrice.toString()}</span>
                </div>
              )}
            </>
          )}
        </div>
      </div>

      <div className="card">
        <h3>Act on this market</h3>
        <div className="tabs">
          {(['deposit', 'withdrawCollateral', 'supply', 'withdraw', 'borrow', 'repay'] as ActionKind[]).map((k) => (
            <button key={k} className={action === k ? 'active' : ''} onClick={() => setAction(k)} type="button">
              {k}
            </button>
          ))}
        </div>
        <form
          className="action-form"
          onSubmit={(e) => {
            e.preventDefault();
            void submit();
          }}
        >
          <label>
            Amount
            <input
              type="text"
              value={amountInput}
              onChange={(e) => setAmountInput(e.target.value)}
              placeholder="0.0"
            />
          </label>
          {formError && <div className="error-box">{formError}</div>}
          {status && <div className="status-line">status: {status}</div>}
          <button className="btn-primary" type="submit" disabled={!signer || status === 'building' || status === 'confirming'}>
            Submit {action}
          </button>
        </form>
      </div>
    </div>
  );
}
