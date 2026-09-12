// I-SDK-02: for every required instruction, build with the SDK -> sign with a supplied local
// signer -> send -> confirm -> decode the resulting account/event -> assert the expected state
// transition. Runs against a real, local, OFFLINE Surfpool validator (no external RPC, no devnet,
// no mainnet, no Jupiter, no paid API -- `surfpool start --offline`) with the real, locally-built
// `aegis.so` deployed. Requires `make build` (or `anchor build`) to have produced
// `target/deploy/aegis.so` and its keypair first.
//
// Run with `npm run test:e2e` (a longer default timeout; not part of the default `npm test`, which
// covers the vector/PDA/tx-size suites that need no running validator at all).

import { spawn, execFile, type ChildProcess } from 'node:child_process';
import { promisify } from 'node:util';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import os from 'node:os';
import {
  address,
  airdropFactory,
  createSolanaRpc,
  createSolanaRpcSubscriptions,
  generateKeyPairSigner,
  lamports,
  devnet,
  type Address,
  type KeyPairSigner,
  type Rpc,
  type RpcSubscriptions,
  type SolanaRpcApi,
  type SolanaRpcSubscriptionsApi,
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
import { afterAll, beforeAll, describe, expect, it } from 'vitest';

import {
  buildAccrueInterestInstruction,
  buildBorrowInstruction,
  buildClosePositionInstruction,
  buildCreateMarketInstruction,
  buildDepositCollateralInstruction,
  buildInitializeProtocolInstruction,
  buildLiquidateInstruction,
  buildRepayInstruction,
  buildSupplyInstruction,
  buildWithdrawCollateralInstruction,
  buildWithdrawInstruction,
  withInitPositionIfNeeded,
} from '../src/ix.js';
import { buildSignSendAndConfirm } from '../src/tx.js';
import { collateralVaultPda, loanVaultPda, marketPda, positionPda, protocolPda } from '../src/pda.js';
import { fetchMarket, fetchPosition, fetchProtocol } from '../src/accounts.js';
import { SYSTEM_PROGRAM_ADDRESS } from '../src/config.js';

const execFileAsync = promisify(execFile);
const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, '../../..');

const RPC_URL = 'http://127.0.0.1:8935';
const RPC_WS_URL = 'ws://127.0.0.1:8936';
const WAD = 1_000_000_000_000_000_000n;

let surfpool: ChildProcess | undefined;
let rpc: Rpc<SolanaRpcApi>;
let rpcSubscriptions: RpcSubscriptions<SolanaRpcSubscriptionsApi>;
let programId: Address;
let priceFixtures: PriceFixtureFile;

function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function waitForHealth(timeoutMs: number) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const resp = await fetch(RPC_URL, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'getHealth' }),
      });
      if (resp.ok) return;
    } catch {
      // not up yet
    }
    await sleep(300);
  }
  throw new Error(`Surfpool did not become healthy within ${timeoutMs}ms`);
}

async function readProgramId(): Promise<Address> {
  const { stdout } = await execFileAsync('solana-keygen', [
    'pubkey',
    path.join(REPO_ROOT, 'target', 'deploy', 'aegis-keypair.json'),
  ]);
  return address(stdout.trim());
}

async function deployAegis(deployerKeypairPath: string) {
  await execFileAsync('solana', [
    'program',
    'deploy',
    path.join(REPO_ROOT, 'target', 'deploy', 'aegis.so'),
    '--program-id',
    path.join(REPO_ROOT, 'target', 'deploy', 'aegis-keypair.json'),
    '--url',
    RPC_URL,
    '--keypair',
    deployerKeypairPath,
  ]);
}

async function surfnetSetAccount(pubkey: Address, owner: Address, dataBase64: string, lamportsN: number) {
  const dataHex = Buffer.from(dataBase64, 'base64').toString('hex');
  const resp = await fetch(RPC_URL, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      jsonrpc: '2.0',
      id: 1,
      method: 'surfnet_setAccount',
      params: [pubkey, { lamports: lamportsN, data: dataHex, owner, executable: false }],
    }),
  });
  const json = (await resp.json()) as { error?: { message: string } };
  if (json.error) throw new Error(`surfnet_setAccount failed: ${json.error.message}`);
}

// Deterministic Pyth `PriceUpdateV2` fixtures (ADR-0008: test-fixture account injection, never a
// mock program). Rather than reimplementing Pyth's account layout a second time in TypeScript
// (the classic drift risk this repository's own conventions exist to avoid), this test reuses the
// exact same byte-exact Rust fixture builder every Rust test already uses
// (`aegis_test_kit::PriceFixture`, via the existing `phase8_price_fixture_dump` example that
// `bots/liquidator/src/demo.ts` also consumes) and injects the resulting bytes over RPC with
// `surfnet_setAccount` -- identical technique, one real source of truth for the byte layout.
interface SurfnetAccountFixture {
  pubkey: string;
  owner: string;
  lamports: number;
  dataBase64: string;
}
interface PriceFixtureFile {
  collateralFeedId: string;
  loanFeedId: string;
  valid: { collateralPriceUpdate: SurfnetAccountFixture; loanPriceUpdate: SurfnetAccountFixture };
  crash: { collateralPriceUpdate: SurfnetAccountFixture; loanPriceUpdate: SurfnetAccountFixture };
}

const FIXTURES_PATH = path.join(__dirname, '..', 'fixtures', 'prices.json');

async function regeneratePriceFixtures(): Promise<PriceFixtureFile> {
  await execFileAsync('cargo', [
    'run',
    '-p',
    'aegis-test-kit',
    '--example',
    'phase8_price_fixture_dump',
    '--',
    FIXTURES_PATH,
  ], { cwd: REPO_ROOT });
  return JSON.parse(readFileSync(FIXTURES_PATH, 'utf8')) as PriceFixtureFile;
}

async function injectFixture(f: SurfnetAccountFixture) {
  await surfnetSetAccount(address(f.pubkey), address(f.owner), f.dataBase64, f.lamports);
}

async function createMint(payer: KeyPairSigner, decimals: number): Promise<Address> {
  const mint = await generateKeyPairSigner();
  const space = BigInt(getMintSize());
  const rent = await rpc.getMinimumBalanceForRentExemption(space).send();
  const createIx = getCreateAccountInstruction({
    payer,
    newAccount: mint,
    lamports: rent,
    space,
    programAddress: TOKEN_PROGRAM_ADDRESS,
  });
  const initIx = getInitializeMintInstruction({
    mint: mint.address,
    decimals,
    mintAuthority: payer.address,
    freezeAuthority: null,
  });
  await buildSignSendAndConfirm(rpc, rpcSubscriptions, payer, [createIx, initIx]);
  return mint.address;
}

async function createTokenAccount(payer: KeyPairSigner, mint: Address, owner: Address): Promise<Address> {
  const account = await generateKeyPairSigner();
  const space = BigInt(getTokenSize());
  const rent = await rpc.getMinimumBalanceForRentExemption(space).send();
  const createIx = getCreateAccountInstruction({
    payer,
    newAccount: account,
    lamports: rent,
    space,
    programAddress: TOKEN_PROGRAM_ADDRESS,
  });
  const initIx = getInitializeAccount3Instruction({ account: account.address, mint, owner });
  await buildSignSendAndConfirm(rpc, rpcSubscriptions, payer, [createIx, initIx]);
  return account.address;
}

async function mintTo(mintAuthority: KeyPairSigner, mint: Address, destination: Address, amount: bigint) {
  const ix = getMintToInstruction({ mint, token: destination, mintAuthority, amount });
  await buildSignSendAndConfirm(rpc, rpcSubscriptions, mintAuthority, [ix]);
}

describe('I-SDK-02 -- build/sign/send/confirm/decode against local Surfpool', () => {
  beforeAll(async () => {
    surfpool = spawn(
      'surfpool',
      ['start', '--offline', '--port', '8935', '--ws-port', '8936', '--no-tui'],
      { stdio: 'ignore', detached: true },
    );
    await waitForHealth(20_000);

    programId = await readProgramId();
    const deployerKeypairPath = path.join(os.tmpdir(), `aegis-sdk-e2e-deployer-${process.pid}.json`);
    await execFileAsync('solana-keygen', ['new', '--no-bip39-passphrase', '--silent', '--force', '-o', deployerKeypairPath]);
    await execFileAsync('solana', ['airdrop', '20', '--url', RPC_URL, '--keypair', deployerKeypairPath]);
    await deployAegis(deployerKeypairPath);

    rpc = createSolanaRpc(devnet(RPC_URL));
    rpcSubscriptions = createSolanaRpcSubscriptions(devnet(RPC_WS_URL));

    // Regenerate with a fresh `publish_time` so the fixture's staleness (O-5, max_price_age_secs)
    // is satisfied relative to whenever this test actually runs.
    priceFixtures = await regeneratePriceFixtures();
  }, 60_000);

  afterAll(() => {
    if (surfpool?.pid) {
      try {
        process.kill(-surfpool.pid);
      } catch {
        // already gone
      }
    }
  });

  it('runs the full lifecycle: init -> market -> position -> deposit -> borrow -> repay -> withdraw -> liquidate', async () => {
    const airdrop = airdropFactory({ rpc, rpcSubscriptions });

    const admin = await generateKeyPairSigner();
    await airdrop({ recipientAddress: admin.address, lamports: lamports(100_000_000_000n), commitment: 'confirmed' });

    // --- initialize_protocol ---
    const guardian = (await generateKeyPairSigner()).address;
    const feeRecipient = (await generateKeyPairSigner()).address;
    const protocol = await protocolPda(programId);
    const { events: initEvents } = await buildSignSendAndConfirm(rpc, rpcSubscriptions, admin, [
      buildInitializeProtocolInstruction(
        programId,
        { payer: admin.address, protocol, systemProgram: SYSTEM_PROGRAM_ADDRESS },
        { guardian, feeRecipient },
      ),
    ]);
    expect(initEvents.some((e) => e.name === 'ProtocolInitialized')).toBe(true);
    const protocolAccount = await fetchProtocol(rpc, protocol, programId);
    expect(protocolAccount.admin).toBe(admin.address);
    expect(protocolAccount.guardian).toBe(guardian);

    // --- create_market ---
    const collateralMint = await createMint(admin, 9);
    const loanMint = await createMint(admin, 6);
    const configId = 0;
    const market = await marketPda(programId, collateralMint, loanMint, configId);
    const collateralVault = await collateralVaultPda(programId, market);
    const loanVault = await loanVaultPda(programId, market);
    const feePosition = await positionPda(programId, market, feeRecipient);

    const collateralFeedId = Uint8Array.from(Buffer.from(priceFixtures.collateralFeedId, 'hex'));
    const loanFeedId = Uint8Array.from(Buffer.from(priceFixtures.loanFeedId, 'hex'));

    const { events: marketEvents } = await buildSignSendAndConfirm(rpc, rpcSubscriptions, admin, [
      buildCreateMarketInstruction(
        programId,
        {
          admin: admin.address,
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
          slope1Ps: 0n,
          slope2Ps: 0n,
          uKink: (WAD * 80n) / 100n,
          maxRatePs: WAD,
          ackFreezeAuthority: false,
        },
      ),
    ]);
    expect(marketEvents.some((e) => e.name === 'MarketCreated')).toBe(true);
    const marketAccountAfterCreate = await fetchMarket(rpc, market, programId);
    expect(marketAccountAfterCreate.collateralMint).toBe(collateralMint);
    expect(marketAccountAfterCreate.totalSupplyAssets).toBe(0n);

    // --- lender: init_position (bundled) + supply ---
    const lender = await generateKeyPairSigner();
    await airdrop({ recipientAddress: lender.address, lamports: lamports(10_000_000_000n), commitment: 'confirmed' });
    const lenderLoanAta = await createTokenAccount(admin, loanMint, lender.address);
    await mintTo(admin, loanMint, lenderLoanAta, 1_000_000_000_000n);

    const supplyIx = buildSupplyInstruction(
      programId,
      {
        owner: lender.address,
        market,
        position: await positionPda(programId, market, lender.address),
        feePosition,
        loanVault,
        ownerLoanAta: lenderLoanAta,
        loanMint,
        loanTokenProgram: TOKEN_PROGRAM_ADDRESS,
      },
      { assets: 1_000_000_000_000n, shares: 0n },
    );
    const supplyIxs = await withInitPositionIfNeeded(rpc, programId, market, lender.address, lender.address, supplyIx);
    const { events: supplyEvents } = await buildSignSendAndConfirm(rpc, rpcSubscriptions, lender, supplyIxs);
    expect(supplyEvents.some((e) => e.name === 'PositionInitialized')).toBe(true);
    expect(supplyEvents.some((e) => e.name === 'Supplied')).toBe(true);

    const marketAfterSupply = await fetchMarket(rpc, market, programId);
    expect(marketAfterSupply.totalSupplyAssets).toBe(1_000_000_000_000n);

    // --- borrower: init_position (bundled) + deposit_collateral, then borrow ---
    const borrower = await generateKeyPairSigner();
    await airdrop({ recipientAddress: borrower.address, lamports: lamports(10_000_000_000n), commitment: 'confirmed' });
    const borrowerCollateralAta = await createTokenAccount(admin, collateralMint, borrower.address);
    await mintTo(admin, collateralMint, borrowerCollateralAta, 10_000_000_000n);
    const borrowerPosition = await positionPda(programId, market, borrower.address);

    const depositIx = buildDepositCollateralInstruction(
      programId,
      {
        depositor: borrower.address,
        market,
        position: borrowerPosition,
        collateralVault,
        depositorCollateralAta: borrowerCollateralAta,
        collateralMint,
        collateralTokenProgram: TOKEN_PROGRAM_ADDRESS,
      },
      { amount: 10_000_000_000n },
    );
    const depositIxs = await withInitPositionIfNeeded(rpc, programId, market, borrower.address, borrower.address, depositIx);
    const { events: depositEvents } = await buildSignSendAndConfirm(rpc, rpcSubscriptions, borrower, depositIxs);
    expect(depositEvents.some((e) => e.name === 'CollateralDeposited')).toBe(true);

    const positionAfterDeposit = await fetchPosition(rpc, borrowerPosition, programId);
    expect(positionAfterDeposit.collateralAmount).toBe(10_000_000_000n);

    // Inject deterministic Pyth price fixtures (ADR-0008's fixture-injection pattern, over RPC via
    // `surfnet_setAccount` -- the same cheat-code `bots/liquidator/src/surfnet.ts` uses, and the
    // same fixture bytes, generated by the real `aegis_test_kit::PriceFixture`, that
    // `bots/liquidator/src/demo.ts` already injects for the Phase 8 keeper demo). Both scenarios
    // share the same two account addresses throughout (a real Pyth pull account's data is updated
    // in place, not re-created at a new address).
    const collateralPriceUpdate = address(priceFixtures.valid.collateralPriceUpdate.pubkey);
    const loanPriceUpdate = address(priceFixtures.valid.loanPriceUpdate.pubkey);

    // $150.00/SOL, $1.00/USDC -- healthy borrow.
    await injectFixture(priceFixtures.valid.collateralPriceUpdate);
    await injectFixture(priceFixtures.valid.loanPriceUpdate);

    const borrowerLoanAta = await createTokenAccount(admin, loanMint, borrower.address);
    const { events: borrowEvents } = await buildSignSendAndConfirm(rpc, rpcSubscriptions, borrower, [
      buildBorrowInstruction(
        programId,
        {
          owner: borrower.address,
          market,
          position: borrowerPosition,
          feePosition,
          loanVault,
          ownerLoanAta: borrowerLoanAta,
          loanMint,
          loanTokenProgram: TOKEN_PROGRAM_ADDRESS,
          collateralPriceUpdate,
          loanPriceUpdate,
        },
        { assets: 900_000_000n, shares: 0n },
      ),
    ]);
    expect(borrowEvents.some((e) => e.name === 'Borrowed')).toBe(true);

    const positionAfterBorrow = await fetchPosition(rpc, borrowerPosition, programId);
    expect(positionAfterBorrow.borrowShares > 0n).toBe(true);

    // --- accrue_interest (permissionless, observable) ---
    const { events: accrueEvents } = await buildSignSendAndConfirm(rpc, rpcSubscriptions, borrower, [
      buildAccrueInterestInstruction(programId, { market, feePosition }),
    ]);
    expect(accrueEvents.some((e) => e.name === 'InterestAccrued')).toBe(true);

    // --- repay in full ---
    const marketBeforeRepay = await fetchMarket(rpc, market, programId);
    await mintTo(admin, loanMint, borrowerLoanAta, 100_000_000n); // top up for accrued interest
    const { events: repayEvents } = await buildSignSendAndConfirm(rpc, rpcSubscriptions, borrower, [
      buildRepayInstruction(
        programId,
        {
          payer: borrower.address,
          market,
          position: borrowerPosition,
          feePosition,
          loanVault,
          payerLoanAta: borrowerLoanAta,
          loanMint,
          loanTokenProgram: TOKEN_PROGRAM_ADDRESS,
        },
        { assets: 0n, shares: positionAfterBorrow.borrowShares },
      ),
    ]);
    expect(repayEvents.some((e) => e.name === 'Repaid')).toBe(true);
    const positionAfterRepay = await fetchPosition(rpc, borrowerPosition, programId);
    expect(positionAfterRepay.borrowShares).toBe(0n);
    void marketBeforeRepay;

    // --- withdraw_collateral (debt is now zero) ---
    const { events: withdrawCollateralEvents } = await buildSignSendAndConfirm(rpc, rpcSubscriptions, borrower, [
      buildWithdrawCollateralInstruction(
        programId,
        {
          owner: borrower.address,
          market,
          position: borrowerPosition,
          collateralVault,
          ownerCollateralAta: borrowerCollateralAta,
          collateralMint,
          collateralTokenProgram: TOKEN_PROGRAM_ADDRESS,
          collateralPriceUpdate,
          loanPriceUpdate,
        },
        { amount: 10_000_000_000n },
      ),
    ]);
    expect(withdrawCollateralEvents.some((e) => e.name === 'CollateralWithdrawn')).toBe(true);
    const positionAfterWithdrawCollateral = await fetchPosition(rpc, borrowerPosition, programId);
    expect(positionAfterWithdrawCollateral.collateralAmount).toBe(0n);

    // --- close_position ---
    const { events: closeEvents } = await buildSignSendAndConfirm(rpc, rpcSubscriptions, borrower, [
      buildClosePositionInstruction(programId, { owner: borrower.address, market, position: borrowerPosition }),
    ]);
    expect(closeEvents.some((e) => e.name === 'PositionClosed')).toBe(true);

    // --- lender: withdraw ---
    const marketBeforeLenderWithdraw = await fetchMarket(rpc, market, programId);
    const { events: withdrawEvents } = await buildSignSendAndConfirm(rpc, rpcSubscriptions, lender, [
      buildWithdrawInstruction(
        programId,
        {
          owner: lender.address,
          market,
          position: await positionPda(programId, market, lender.address),
          feePosition,
          loanVault,
          ownerLoanAta: lenderLoanAta,
          loanMint,
          loanTokenProgram: TOKEN_PROGRAM_ADDRESS,
        },
        { assets: 0n, shares: 1n },
      ),
    ]);
    expect(withdrawEvents.some((e) => e.name === 'Withdrawn')).toBe(true);
    void marketBeforeLenderWithdraw;

    // --- liquidate: a second borrower, driven underwater by a scripted price drop ---
    const borrower2 = await generateKeyPairSigner();
    await airdrop({ recipientAddress: borrower2.address, lamports: lamports(10_000_000_000n), commitment: 'confirmed' });
    const borrower2CollateralAta = await createTokenAccount(admin, collateralMint, borrower2.address);
    await mintTo(admin, collateralMint, borrower2CollateralAta, 10_000_000_000n);
    const borrower2Position = await positionPda(programId, market, borrower2.address);
    const deposit2Ix = buildDepositCollateralInstruction(
      programId,
      {
        depositor: borrower2.address,
        market,
        position: borrower2Position,
        collateralVault,
        depositorCollateralAta: borrower2CollateralAta,
        collateralMint,
        collateralTokenProgram: TOKEN_PROGRAM_ADDRESS,
      },
      { amount: 10_000_000_000n },
    );
    const deposit2Ixs = await withInitPositionIfNeeded(rpc, programId, market, borrower2.address, borrower2.address, deposit2Ix);
    await buildSignSendAndConfirm(rpc, rpcSubscriptions, borrower2, deposit2Ixs);

    const borrower2LoanAta = await createTokenAccount(admin, loanMint, borrower2.address);
    await buildSignSendAndConfirm(rpc, rpcSubscriptions, borrower2, [
      buildBorrowInstruction(
        programId,
        {
          owner: borrower2.address,
          market,
          position: borrower2Position,
          feePosition,
          loanVault,
          ownerLoanAta: borrower2LoanAta,
          loanMint,
          loanTokenProgram: TOKEN_PROGRAM_ADDRESS,
          collateralPriceUpdate,
          loanPriceUpdate,
        },
        { assets: 900_000_000n, shares: 0n },
      ),
    ]);

    // Crash: SOL falls to $95.00 -- the position becomes liquidatable (economic-model.md §6.5).
    await injectFixture(priceFixtures.crash.collateralPriceUpdate);
    await injectFixture(priceFixtures.crash.loanPriceUpdate);

    const liquidator = await generateKeyPairSigner();
    await airdrop({ recipientAddress: liquidator.address, lamports: lamports(10_000_000_000n), commitment: 'confirmed' });
    const liquidatorLoanAta = await createTokenAccount(admin, loanMint, liquidator.address);
    await mintTo(admin, loanMint, liquidatorLoanAta, 2_000_000_000n);
    const liquidatorCollateralAta = await createTokenAccount(admin, collateralMint, liquidator.address);

    const { events: liquidateEvents } = await buildSignSendAndConfirm(rpc, rpcSubscriptions, liquidator, [
      buildLiquidateInstruction(
        programId,
        {
          liquidator: liquidator.address,
          market,
          position: borrower2Position,
          feePosition,
          loanVault,
          collateralVault,
          liquidatorLoanAta,
          liquidatorCollateralAta,
          loanMint,
          collateralMint,
          loanTokenProgram: TOKEN_PROGRAM_ADDRESS,
          collateralTokenProgram: TOKEN_PROGRAM_ADDRESS,
          collateralPriceUpdate,
          loanPriceUpdate,
        },
        { repayAssets: 900_000_000n, seizeCollateral: 0n, callbackData: new Uint8Array() },
      ),
    ]);
    expect(liquidateEvents.some((e) => e.name === 'Liquidated')).toBe(true);
    const liquidatedPosition = await fetchPosition(rpc, borrower2Position, programId);
    expect(liquidatedPosition.borrowShares).toBe(0n);
  }, 90_000);
});
