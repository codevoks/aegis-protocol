'use client';

// Scripted local demo (`docs/phases/phase-09-sdk-ui.md` items 36-37, 45, I-UI-01): seeds a fresh
// protocol/market from the connected local signer, deposits collateral and borrows, warps time to
// show interest accruing with no real wall-clock wait, then scripts a price drop and a liquidation
// -- entirely against local Surfpool, offline (deterministic fixtures, no Hermes).

import { useState } from 'react';
import {
  generateKeyPairSigner,
  lamports,
  airdropFactory,
  type Address,
  type KeyPairSigner,
} from '@solana/kit';
import { getCreateAccountInstruction } from '@solana-program/system';
import {
  TOKEN_PROGRAM_ADDRESS,
  getInitializeAccount3Instruction,
  getInitializeMintInstruction,
  getMintSize,
  getMintToInstruction,
  getTokenSize,
} from '@solana-program/token';
import {
  buildAccrueInterestInstruction,
  buildBorrowInstruction,
  buildCreateMarketInstruction,
  buildDepositCollateralInstruction,
  buildInitializeProtocolInstruction,
  buildLiquidateInstruction,
  buildSignSendAndConfirm,
  buildSupplyInstruction,
  collateralVaultPda,
  fetchMarket,
  fetchPosition,
  loanVaultPda,
  marketPda,
  positionPda,
  protocolPda,
  PYTH_RECEIVER_PROGRAM_ADDRESS,
  SYSTEM_PROGRAM_ADDRESS,
  toAssetsUp,
  withInitPositionIfNeeded,
  type Market,
  type Position,
} from '@aegis/sdk';
import { useAegisClient } from '@/lib/AegisProvider';
import { getDemoMarketInfo, setDemoMarketInfo } from '@/lib/demoRegistry';
import { getOnChainUnixTimestamp, surfnetSetAccount, surfnetWarpForward } from '@/lib/surfnet';
import { encodePriceUpdateV2 } from '@/lib/priceUpdate';
import { formatBaseUnitsToDecimal } from '@/lib/format';

const WAD = 1_000_000_000_000_000_000n;

async function createMint(
  rpc: ReturnType<typeof useAegisClient>['rpc'],
  rpcSubscriptions: ReturnType<typeof useAegisClient>['rpcSubscriptions'],
  payer: KeyPairSigner,
  decimals: number,
): Promise<Address> {
  const mint = await generateKeyPairSigner();
  const space = BigInt(getMintSize());
  const rent = await rpc.getMinimumBalanceForRentExemption(space).send();
  const createIx = getCreateAccountInstruction({ payer, newAccount: mint, lamports: rent, space, programAddress: TOKEN_PROGRAM_ADDRESS });
  const initIx = getInitializeMintInstruction({ mint: mint.address, decimals, mintAuthority: payer.address, freezeAuthority: null });
  await buildSignSendAndConfirm(rpc, rpcSubscriptions, payer, [createIx, initIx]);
  return mint.address;
}

async function createTokenAccount(
  rpc: ReturnType<typeof useAegisClient>['rpc'],
  rpcSubscriptions: ReturnType<typeof useAegisClient>['rpcSubscriptions'],
  payer: KeyPairSigner,
  mint: Address,
  owner: Address,
): Promise<Address> {
  const account = await generateKeyPairSigner();
  const space = BigInt(getTokenSize());
  const rent = await rpc.getMinimumBalanceForRentExemption(space).send();
  const createIx = getCreateAccountInstruction({ payer, newAccount: account, lamports: rent, space, programAddress: TOKEN_PROGRAM_ADDRESS });
  const initIx = getInitializeAccount3Instruction({ account: account.address, mint, owner });
  await buildSignSendAndConfirm(rpc, rpcSubscriptions, payer, [createIx, initIx]);
  return account.address;
}

export default function DemoPage() {
  const { rpc, rpcSubscriptions, rpcUrl, programId, signer, airdrop } = useAegisClient();
  const [log, setLog] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const [marketAddress, setMarketAddress] = useState<Address | null>(null);
  const [borrowerPosition, setBorrowerPosition] = useState<Address | null>(null);
  const [borrower, setBorrower] = useState<KeyPairSigner | null>(null);
  const [marketState, setMarketState] = useState<Market | null>(null);
  const [positionState, setPositionState] = useState<Position | null>(null);

  function appendLog(line: string) {
    setLog((l) => [...l, line]);
  }

  async function withBusy(fn: () => Promise<void>) {
    setBusy(true);
    try {
      await fn();
    } catch (err) {
      appendLog(`ERROR: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setBusy(false);
    }
  }

  async function refreshState(market: Address, position: Address) {
    setMarketState(await fetchMarket(rpc, market, programId));
    setPositionState(await fetchPosition(rpc, position, programId));
  }

  async function step1SeedMarketAndOpenPosition() {
    if (!signer) return;
    await withBusy(async () => {
      appendLog('Airdropping SOL to the connected wallet (admin + lender)...');
      await airdrop(50);

      appendLog('Creating collateral (9dp) and loan (6dp) mints...');
      const collateralMint = await createMint(rpc, rpcSubscriptions, signer, 9);
      const loanMint = await createMint(rpc, rpcSubscriptions, signer, 6);

      appendLog('Initializing protocol...');
      const protocol = await protocolPda(programId);
      // The fee recipient must be a DIFFERENT identity from the connected wallet: the connected
      // wallet also acts as lender below, and Aegis's fee shares live in that same recipient's
      // `Position` (`account-model.md` §9). If the fee recipient and the lender were the same
      // address, `supply`'s `position` and `fee_position` accounts would resolve to the identical
      // PDA, which Anchor 1.0 rejects outright as a duplicate mutable account
      // (`ConstraintDuplicateMutableAccount`) -- exactly the protection AGENTS.md §1 describes.
      // This address never needs to sign anything, so it is never funded or persisted.
      const feeRecipient = (await generateKeyPairSigner()).address;
      await buildSignSendAndConfirm(rpc, rpcSubscriptions, signer, [
        buildInitializeProtocolInstruction(
          programId,
          { payer: signer.address, protocol, systemProgram: SYSTEM_PROGRAM_ADDRESS },
          { guardian: signer.address, feeRecipient },
        ),
      ]);

      appendLog('Creating market (SOL-like collateral / USDC-like loan)...');
      const configId = 0;
      const market = await marketPda(programId, collateralMint, loanMint, configId);
      const collateralVault = await collateralVaultPda(programId, market);
      const loanVault = await loanVaultPda(programId, market);
      const feePosition = await positionPda(programId, market, feeRecipient);

      const collateralFeedId = crypto.getRandomValues(new Uint8Array(32));
      const loanFeedId = crypto.getRandomValues(new Uint8Array(32));

      await buildSignSendAndConfirm(rpc, rpcSubscriptions, signer, [
        buildCreateMarketInstruction(
          programId,
          {
            admin: signer.address,
            protocol,
            collateralMint,
            loanMint,
            collateralTokenProgram: TOKEN_PROGRAM_ADDRESS,
            loanTokenProgram: TOKEN_PROGRAM_ADDRESS,
            market,
            collateralVault,
            loanVault,
            feePosition,
            systemProgram: SYSTEM_PROGRAM_ADDRESS,
          },
          {
            configId,
            oracleKind: 0,
            collateralFeedId,
            loanFeedId,
            maxPriceAgeSecs: 3600,
            maxConfBps: 100,
            maxLtv: (WAD * 75n) / 100n,
            liqThreshold: (WAD * 80n) / 100n,
            liqBonus: (WAD * 5n) / 100n,
            closeFactor: WAD / 2n,
            fullLiqHf: (WAD * 95n) / 100n,
            liqProtocolFee: WAD / 10n,
            fee: WAD / 10n,
            minDebt: 10_000_000n,
            baseRatePs: 0n,
            slope1Ps: 1_268_391_679n,
            slope2Ps: 31_709_791_983n,
            uKink: (WAD * 80n) / 100n,
            maxRatePs: 317_097_919_837n,
            ackFreezeAuthority: false,
          },
        ),
      ]);

      appendLog('Lender (connected wallet) supplies 1,000,000 loan-asset units...');
      const lenderLoanAta = await createTokenAccount(rpc, rpcSubscriptions, signer, loanMint, signer.address);
      await buildSignSendAndConfirm(rpc, rpcSubscriptions, signer, [
        getMintToInstruction({ mint: loanMint, token: lenderLoanAta, mintAuthority: signer, amount: 1_000_000_000_000n }),
      ]);
      const supplyIx = buildSupplyInstruction(
        programId,
        {
          owner: signer.address,
          protocol,
          market,
          position: await positionPda(programId, market, signer.address),
          feePosition,
          loanVault,
          ownerLoanAta: lenderLoanAta,
          loanMint,
          loanTokenProgram: TOKEN_PROGRAM_ADDRESS,
        },
        { assets: 1_000_000_000_000n, shares: 0n },
      );
      const supplyIxs = await withInitPositionIfNeeded(rpc, programId, market, signer.address, signer.address, supplyIx);
      await buildSignSendAndConfirm(rpc, rpcSubscriptions, signer, supplyIxs);

      appendLog('Generating an ephemeral borrower identity for this demo...');
      const demoBorrower = await generateKeyPairSigner();
      const airdropFn = airdropFactory({ rpc, rpcSubscriptions });
      await airdropFn({ recipientAddress: demoBorrower.address, lamports: lamports(10_000_000_000n), commitment: 'confirmed' });

      appendLog('Borrower deposits 10 collateral-asset units and borrows 900 loan-asset units...');
      const borrowerCollateralAta = await createTokenAccount(rpc, rpcSubscriptions, signer, collateralMint, demoBorrower.address);
      await buildSignSendAndConfirm(rpc, rpcSubscriptions, signer, [
        getMintToInstruction({ mint: collateralMint, token: borrowerCollateralAta, mintAuthority: signer, amount: 10_000_000_000n }),
      ]);
      const bPosition = await positionPda(programId, market, demoBorrower.address);
      const depositIx = buildDepositCollateralInstruction(
        programId,
        {
          depositor: demoBorrower.address,
          market,
          position: bPosition,
          collateralVault,
          depositorCollateralAta: borrowerCollateralAta,
          collateralMint,
          collateralTokenProgram: TOKEN_PROGRAM_ADDRESS,
        },
        { amount: 10_000_000_000n },
      );
      const depositIxs = await withInitPositionIfNeeded(rpc, programId, market, demoBorrower.address, demoBorrower.address, depositIx);
      await buildSignSendAndConfirm(rpc, rpcSubscriptions, demoBorrower, depositIxs);

      const nowSecs = await getOnChainUnixTimestamp(rpcUrl);
      const collateralPriceUpdate = await generateKeyPairSigner();
      const loanPriceUpdate = await generateKeyPairSigner();
      await surfnetSetAccount(
        rpcUrl,
        collateralPriceUpdate.address,
        PYTH_RECEIVER_PROGRAM_ADDRESS,
        Buffer.from(encodePriceUpdateV2(collateralFeedId, 15_000_000_000n, 0n, -8, nowSecs)).toString('base64'),
        1_000_000_000,
      );
      await surfnetSetAccount(
        rpcUrl,
        loanPriceUpdate.address,
        PYTH_RECEIVER_PROGRAM_ADDRESS,
        Buffer.from(encodePriceUpdateV2(loanFeedId, 100_000_000n, 0n, -8, nowSecs)).toString('base64'),
        1_000_000_000,
      );
      setDemoMarketInfo(market, {
        collateralPriceUpdate: collateralPriceUpdate.address,
        loanPriceUpdate: loanPriceUpdate.address,
        collateralFeedId: Buffer.from(collateralFeedId).toString('hex'),
        loanFeedId: Buffer.from(loanFeedId).toString('hex'),
      });

      const borrowerLoanAta = await createTokenAccount(rpc, rpcSubscriptions, signer, loanMint, demoBorrower.address);
      await buildSignSendAndConfirm(rpc, rpcSubscriptions, demoBorrower, [
        buildBorrowInstruction(
          programId,
          {
            owner: demoBorrower.address,
            protocol,
            market,
            position: bPosition,
            feePosition,
            loanVault,
            ownerLoanAta: borrowerLoanAta,
            loanMint,
            loanTokenProgram: TOKEN_PROGRAM_ADDRESS,
            collateralPriceUpdate: collateralPriceUpdate.address,
            loanPriceUpdate: loanPriceUpdate.address,
          },
          { assets: 900_000_000n, shares: 0n },
        ),
      ]);

      setMarketAddress(market);
      setBorrowerPosition(bPosition);
      setBorrower(demoBorrower);
      await refreshState(market, bPosition);
      appendLog('Borrower position opened: 10.000000000 collateral units, 900.000000 debt units. HF healthy.');
    });
  }

  async function step2WarpTimeAndAccrue() {
    if (!marketAddress || !signer) return;
    await withBusy(async () => {
      appendLog('Warping the local validator clock forward by 30 days (no real wall-clock wait)...');
      await surfnetWarpForward(rpcUrl, 30 * 24 * 3600);
      appendLog('Calling accrue_interest...');
      const marketBefore = await fetchMarket(rpc, marketAddress, programId);
      await buildSignSendAndConfirm(rpc, rpcSubscriptions, signer, [
        buildAccrueInterestInstruction(programId, {
          market: marketAddress,
          feePosition: await positionPda(programId, marketAddress, marketBefore.feeRecipient),
        }),
      ]);
      await refreshState(marketAddress, borrowerPosition!);
      const marketAfter = await fetchMarket(rpc, marketAddress, programId);
      appendLog(
        `Interest accrued: total_borrow_assets ${marketBefore.totalBorrowAssets} -> ${marketAfter.totalBorrowAssets} (both, since interest is a lender/borrower claim transfer).`,
      );
    });
  }

  async function step3CrashPriceAndLiquidate() {
    if (!marketAddress || !borrowerPosition || !borrower || !signer) return;
    await withBusy(async () => {
      const info = getDemoMarketInfo(marketAddress);
      if (!info) throw new Error('Demo market info missing -- run step 1 first');
      appendLog('Scripted price drop: collateral asset falls from $150.00 to $95.00...');
      const nowSecs = await getOnChainUnixTimestamp(rpcUrl);
      await surfnetSetAccount(
        rpcUrl,
        info.collateralPriceUpdate,
        PYTH_RECEIVER_PROGRAM_ADDRESS,
        Buffer.from(
          encodePriceUpdateV2(Uint8Array.from(Buffer.from(info.collateralFeedId, 'hex')), 9_500_000_000n, 20_000_000n, -8, nowSecs),
        ).toString('base64'),
        1_000_000_000,
      );
      await surfnetSetAccount(
        rpcUrl,
        info.loanPriceUpdate,
        PYTH_RECEIVER_PROGRAM_ADDRESS,
        Buffer.from(
          encodePriceUpdateV2(Uint8Array.from(Buffer.from(info.loanFeedId, 'hex')), 100_020_000n, 20_000n, -8, nowSecs),
        ).toString('base64'),
        1_000_000_000,
      );
      appendLog('Position is now unhealthy. Liquidating (connected wallet acts as liquidator)...');

      const market = await fetchMarket(rpc, marketAddress, programId);
      const liquidatorLoanAta = await createTokenAccount(rpc, rpcSubscriptions, signer, market.loanMint, signer.address);
      await buildSignSendAndConfirm(rpc, rpcSubscriptions, signer, [
        getMintToInstruction({ mint: market.loanMint, token: liquidatorLoanAta, mintAuthority: signer, amount: 2_000_000_000n }),
      ]);
      const liquidatorCollateralAta = await createTokenAccount(rpc, rpcSubscriptions, signer, market.collateralMint, signer.address);

      const position = await fetchPosition(rpc, borrowerPosition, programId);
      // Repay the exact currently-accrued debt (not a hardcoded 900 USDC-equivalent) so the
      // liquidation fully closes the position rather than leaving dust from interest that accrued
      // since the position was opened -- `toAssetsUp` mirrors `to_assets_up`'s own rounding
      // direction (economic-model.md §3.1), matching what `liquidate`'s `max_repay` will accept in
      // the full-liquidation band (`HF < full_liq_hf`, where `max_repay == debt_assets` exactly).
      const debtToRepay =
        position.borrowShares > 0n
          ? toAssetsUp(position.borrowShares, market.totalBorrowAssets, market.totalBorrowShares)
          : 0n;

      await buildSignSendAndConfirm(rpc, rpcSubscriptions, signer, [
        buildLiquidateInstruction(
          programId,
          {
            liquidator: signer.address,
            protocol: await protocolPda(programId),
            market: marketAddress,
            position: borrowerPosition,
            feePosition: await positionPda(programId, marketAddress, market.feeRecipient),
            loanVault: await loanVaultPda(programId, marketAddress),
            collateralVault: await collateralVaultPda(programId, marketAddress),
            liquidatorLoanAta,
            liquidatorCollateralAta,
            loanMint: market.loanMint,
            collateralMint: market.collateralMint,
            loanTokenProgram: TOKEN_PROGRAM_ADDRESS,
            collateralTokenProgram: TOKEN_PROGRAM_ADDRESS,
            collateralPriceUpdate: info.collateralPriceUpdate as Address,
            loanPriceUpdate: info.loanPriceUpdate as Address,
          },
          { repayAssets: debtToRepay, seizeCollateral: 0n, callbackData: new Uint8Array() },
        ),
      ]);
      await refreshState(marketAddress, borrowerPosition);
      const positionAfter = await fetchPosition(rpc, borrowerPosition, programId);
      appendLog(
        positionAfter.borrowShares === 0n
          ? 'Liquidation confirmed. Position debt is now 0.'
          : `Liquidation confirmed. ${positionAfter.borrowShares} borrow shares remain (interest accrued between the read above and execution) -- repay or liquidate again to close fully.`,
      );
    });
  }

  return (
    <div>
      <h1>Local demo</h1>
      <p className="muted">
        Runs entirely against the local Surfpool validator this app is configured for -- no
        external network, no Hermes, no devnet. Each step is a real, confirmed transaction.
      </p>

      <div className="card">
        <h3>1. Seed a market and open a borrower position</h3>
        <button className="btn-primary" disabled={busy || !signer} onClick={() => void step1SeedMarketAndOpenPosition()}>
          Run step 1
        </button>
      </div>

      <div className="card">
        <h3>2. Warp time forward 30 days and accrue interest</h3>
        <button className="btn-primary" disabled={busy || !marketAddress} onClick={() => void step2WarpTimeAndAccrue()}>
          Run step 2
        </button>
      </div>

      <div className="card">
        <h3>3. Crash the collateral price and liquidate</h3>
        <button className="btn-primary" disabled={busy || !marketAddress} onClick={() => void step3CrashPriceAndLiquidate()}>
          Run step 3
        </button>
      </div>

      {marketState && positionState && (
        <div className="card">
          <h3>Current state</h3>
          <div className="grid-2">
            <div className="stat">
              <span className="stat-label">Market total borrow assets</span>
              <span className="stat-value">
                {formatBaseUnitsToDecimal(marketState.totalBorrowAssets, marketState.loanDecimals)}
              </span>
            </div>
            <div className="stat">
              <span className="stat-label">Position collateral</span>
              <span className="stat-value">
                {formatBaseUnitsToDecimal(positionState.collateralAmount, marketState.collateralDecimals)}
              </span>
            </div>
            <div className="stat">
              <span className="stat-label">Position borrow shares</span>
              <span className="stat-value mono">{positionState.borrowShares.toString()}</span>
            </div>
            {marketAddress && (
              <div className="stat">
                <span className="stat-label">Market address</span>
                <a className="market-link mono" href={`/market/${marketAddress}`}>
                  {marketAddress}
                </a>
              </div>
            )}
          </div>
        </div>
      )}

      <div className="card">
        <h3>Log</h3>
        <div className="status-line" style={{ whiteSpace: 'pre-wrap' }}>
          {log.length === 0 ? 'Nothing run yet.' : log.join('\n')}
        </div>
      </div>
    </div>
  );
}
