// Local, no-network demonstration (`docs/phases/phase-08-composability.md` "Demo", spec #22):
// spins up a PLAIN (non-forking) local Surfpool validator, deploys the three real built
// programs, bootstraps a protocol/market/unhealthy-position from scratch with deterministic
// fixtures, then runs the SAME keeper logic `src/index.ts` uses to find and execute a callback
// liquidation for a liquidator holding ZERO loan-asset balance.
//
// No devnet, no mainnet RPC, no Jupiter, no paid API -- `surfpool start --offline` guarantees the
// validator itself makes no outbound network calls (verified: this is a different, additional
// guarantee from "no fork", since `--network`/`--rpc-url` fork mode is what would need network).
//
// Run with: `npm run demo` (after `make build` at the repo root so target/deploy/*.so exist).

import { spawn, type ChildProcess } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import os from 'node:os';
import {
  address,
  airdropFactory,
  createSolanaRpc,
  devnet,
  createSolanaRpcSubscriptions,
  generateKeyPairSigner,
  lamports,
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

import { loadAegisCoder } from './idl.js';
import {
  collateralVaultPda,
  exampleLiquidatorAuthorityPda,
  loanVaultPda,
  marketPda,
  positionPda,
  protocolPda,
} from './pda.js';
import { buildSignAndSend } from './send.js';
import { surfnetSetAccount, type SurfnetAccount } from './surfnet.js';
import {
  buildBorrowInstruction,
  buildCreateMarketInstruction,
  buildDepositCollateralInstruction,
  buildInitPositionInstruction,
  buildInitializeProtocolInstruction,
  buildSupplyInstruction,
} from './txBuilders.js';
import { scanAndLiquidateOneMarket, type CallbackProvider, type PriceResolver } from './keeper.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, '../../..');

const RPC_URL = 'http://127.0.0.1:8899';
const RPC_WS_URL = 'ws://127.0.0.1:8900';
const SYSTEM_PROGRAM = address('11111111111111111111111111111111');

const PYTH_RECEIVER_ID = address('rec5EKMGg6MxZYaMdyBfgwp4d5rB9T1VQH5pJv5LtFJ');

/**
 * Reads a program's deployed address from its `target/deploy/*-keypair.json` file rather than
 * hardcoding the source's `declare_id!` constant, so this demo keeps working if a program is ever
 * redeployed at a new local address without a matching source change.
 *
 * This mismatch is exactly what Phase 8 found and fixed here: `aegis-keypair.json`'s real pubkey
 * did not match `declare_id!` in `programs/aegis/src/lib.rs` (a stale, uncommitted local artifact
 * from before this session -- `target/` is gitignored, so the keypair's history is unknown). This
 * is NOT cosmetic: Anchor's `#[program]` dispatcher checks the actual invocation address against
 * the compiled-in `crate::ID` on every call and rejects a mismatch with `DeclaredProgramIdMismatch`
 * -- confirmed empirically the hard way, when the very first real instruction sent to the deployed
 * program failed with exactly that error. `anchor keys sync` corrected `declare_id!` (and
 * `Anchor.toml`) to match the existing local keypair, `DbRhjkZV1QSxMj5AvrYdgVsyEz8nKhoCLnSLGSKsqaF9`,
 * and the program was rebuilt. See the Phase 8 final report for the full account of this fix.
 */
function readProgramId(keypairName: string): Promise<Address> {
  return new Promise((resolve, reject) => {
    const child = spawn('solana-keygen', [
      'pubkey',
      path.join(REPO_ROOT, 'target', 'deploy', keypairName),
    ]);
    let out = '';
    child.stdout.on('data', (d) => (out += d.toString()));
    child.on('exit', (code) => {
      if (code === 0) resolve(address(out.trim()));
      else reject(new Error(`solana-keygen pubkey ${keypairName} exited with code ${code}`));
    });
  });
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function waitForHealth(rpcUrl: string, timeoutMs: number): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const resp = await fetch(rpcUrl, {
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

function runCommand(cmd: string, args: string[]): Promise<void> {
  return new Promise((resolve, reject) => {
    const child = spawn(cmd, args, { stdio: 'inherit' });
    child.on('exit', (code) => {
      if (code === 0) resolve();
      else reject(new Error(`${cmd} ${args.join(' ')} exited with code ${code}`));
    });
  });
}

/** `deployerKeypairPath` is a throwaway, freshly-generated-and-funded local file (never
 *  committed, never reused across runs) -- it pays deploy fees/rent; it is NOT either program's
 *  own identity, which is pinned separately via `--program-id`. */
async function deployProgram(
  soName: string,
  programKeypairName: string,
  deployerKeypairPath: string,
): Promise<void> {
  const soPath = path.join(REPO_ROOT, 'target', 'deploy', soName);
  const programKeypairPath = path.join(REPO_ROOT, 'target', 'deploy', programKeypairName);
  await runCommand('solana', [
    'program',
    'deploy',
    soPath,
    '--program-id',
    programKeypairPath,
    '--url',
    RPC_URL,
    '--keypair',
    deployerKeypairPath,
  ]);
}

async function main() {
  console.log('=== Aegis Phase 8 liquidator keeper -- local demo ===');
  console.log('(no network, no fork -- surfpool --offline)\n');

  console.log('[1] Starting a plain local Surfpool validator (offline)...');
  const surfpool: ChildProcess = spawn(
    'surfpool',
    ['start', '--offline', '--port', '8899', '--ws-port', '8900', '--no-tui'],
    { stdio: 'ignore', detached: true },
  );
  const cleanup = () => {
    if (surfpool.pid) {
      try {
        process.kill(-surfpool.pid);
      } catch {
        // already gone
      }
    }
  };
  process.on('exit', cleanup);
  process.on('SIGINT', () => {
    cleanup();
    process.exit(130);
  });

  try {
    await waitForHealth(RPC_URL, 20_000);
    console.log('    Surfpool is healthy.');

    const AEGIS_PROGRAM_ID = await readProgramId('aegis-keypair.json');
    const EXAMPLE_LIQUIDATOR_PROGRAM_ID = await readProgramId('example_liquidator-keypair.json');
    console.log(`    aegis program id             = ${AEGIS_PROGRAM_ID}`);
    console.log(`    example-liquidator program id = ${EXAMPLE_LIQUIDATOR_PROGRAM_ID}`);

    console.log('[2] Deploying aegis.so and example_liquidator.so...');
    // A throwaway deploy payer -- generated fresh every run, funded by this local validator's own
    // genesis/faucet, never committed (AGENTS.md §19). Separate from either program's own identity
    // keypair (target/deploy/*-keypair.json), which only need to sign as the resulting program
    // address, not pay for anything.
    const deployerKeypairPath = path.join(os.tmpdir(), `aegis-demo-deployer-${process.pid}.json`);
    await runCommand('solana-keygen', [
      'new',
      '--no-bip39-passphrase',
      '--silent',
      '--force',
      '-o',
      deployerKeypairPath,
    ]);
    await runCommand('solana', [
      'airdrop',
      '20',
      '--url',
      RPC_URL,
      '--keypair',
      deployerKeypairPath,
    ]);
    await deployProgram('aegis.so', 'aegis-keypair.json', deployerKeypairPath);
    await deployProgram('example_liquidator.so', 'example_liquidator-keypair.json', deployerKeypairPath);

    const rpc = createSolanaRpc(devnet(RPC_URL));
    const rpcSubscriptions = createSolanaRpcSubscriptions(devnet(RPC_WS_URL));
    const coder = loadAegisCoder();
    const airdrop = airdropFactory({ rpc, rpcSubscriptions });

    const admin = await generateKeyPairSigner();
    await airdrop({
      recipientAddress: admin.address,
      lamports: lamports(100_000_000_000n),
      commitment: 'confirmed',
    });
    console.log(`    admin = ${admin.address}`);

    console.log('[3] Creating collateral (9dp) and loan (6dp) mints...');
    const collateralMint = await createMint(rpc, rpcSubscriptions, admin, 9);
    const loanMint = await createMint(rpc, rpcSubscriptions, admin, 6);
    console.log(`    collateralMint = ${collateralMint}`);
    console.log(`    loanMint       = ${loanMint}`);

    console.log('[4] Protocol + market setup...');
    const guardian = (await generateKeyPairSigner()).address;
    const feeRecipient = (await generateKeyPairSigner()).address;
    const protocol = await protocolPda(AEGIS_PROGRAM_ID);
    const initProtocolIx = buildInitializeProtocolInstruction(
      coder,
      AEGIS_PROGRAM_ID,
      admin.address,
      protocol,
      SYSTEM_PROGRAM,
      guardian,
      feeRecipient,
    );
    await buildSignAndSend(rpc, rpcSubscriptions, admin, [initProtocolIx]);

    const configId = 0;
    const market = await marketPda(AEGIS_PROGRAM_ID, collateralMint, loanMint, configId);
    const collateralVault = await collateralVaultPda(AEGIS_PROGRAM_ID, market);
    const loanVault = await loanVaultPda(AEGIS_PROGRAM_ID, market);
    const feePosition = await positionPda(AEGIS_PROGRAM_ID, market, feeRecipient);

    const WAD_N = 1_000_000_000_000_000_000n;
    const createMarketIx = buildCreateMarketInstruction(
      coder,
      AEGIS_PROGRAM_ID,
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
        systemProgram: SYSTEM_PROGRAM,
      },
      {
        configId,
        oracleKind: 0,
        collateralFeedId: new Uint8Array(32).fill(0xaa),
        loanFeedId: new Uint8Array(32).fill(0xbb),
        // Generous on purpose for this local demo only: deploying 2 programs + several setup
        // transactions against a real (if local) validator takes real wall-clock seconds, unlike
        // LiteSVM's instantly-warpable clock. `docs/oracle-design.md`'s real staleness bound
        // (`instruction-catalogue.md` §6, `[1, 3600]`) is a market-configured, per-deployment
        // choice, not a hardcoded protocol constant -- 3600s is within its allowed range.
        maxPriceAgeSecs: 3600,
        maxConfBps: 100,
        maxLtv: (WAD_N * 75n) / 100n,
        liqThreshold: (WAD_N * 80n) / 100n,
        liqBonus: (WAD_N * 5n) / 100n,
        closeFactor: WAD_N / 2n,
        fullLiqHf: (WAD_N * 95n) / 100n,
        liqProtocolFee: WAD_N / 10n,
        fee: WAD_N / 10n,
        minDebt: 10_000_000n,
        baseRatePs: 0n,
        slope1Ps: 0n,
        slope2Ps: 0n,
        uKink: (WAD_N * 80n) / 100n,
        maxRatePs: WAD_N,
        ackFreezeAuthority: false,
      },
    );
    await buildSignAndSend(rpc, rpcSubscriptions, admin, [createMarketIx]);
    console.log(`    market = ${market}`);

    console.log('[5] Lender supplies liquidity...');
    const lender = await generateKeyPairSigner();
    await airdrop({ recipientAddress: lender.address, lamports: lamports(10_000_000_000n), commitment: 'confirmed' });
    const lenderLoanAta = await createTokenAccount(rpc, rpcSubscriptions, admin, loanMint, lender.address);
    await mintTo(rpc, rpcSubscriptions, admin, loanMint, lenderLoanAta, 1_000_000_000_000n);
    const lenderPosition = await positionPda(AEGIS_PROGRAM_ID, market, lender.address);
    await buildSignAndSend(rpc, rpcSubscriptions, lender, [
      buildInitPositionInstruction(coder, AEGIS_PROGRAM_ID, lender.address, market, lender.address, lenderPosition, SYSTEM_PROGRAM),
    ]);
    await buildSignAndSend(rpc, rpcSubscriptions, lender, [
      buildSupplyInstruction(
        coder,
        AEGIS_PROGRAM_ID,
        { owner: lender.address, market, position: lenderPosition, feePosition, loanVault, ownerLoanAta: lenderLoanAta, loanMint, loanTokenProgram: TOKEN_PROGRAM_ADDRESS },
        1_000_000_000_000n,
        0n,
      ),
    ]);
    console.log('    lender supplied 1,000,000.000000 USDC');

    console.log('[6] Borrower deposits collateral and borrows at $150.00/SOL...');
    const borrower = await generateKeyPairSigner();
    await airdrop({ recipientAddress: borrower.address, lamports: lamports(10_000_000_000n), commitment: 'confirmed' });
    const borrowerCollateralAta = await createTokenAccount(rpc, rpcSubscriptions, admin, collateralMint, borrower.address);
    await mintTo(rpc, rpcSubscriptions, admin, collateralMint, borrowerCollateralAta, 10_000_000_000n);
    const borrowerPosition = await positionPda(AEGIS_PROGRAM_ID, market, borrower.address);
    await buildSignAndSend(rpc, rpcSubscriptions, borrower, [
      buildInitPositionInstruction(coder, AEGIS_PROGRAM_ID, borrower.address, market, borrower.address, borrowerPosition, SYSTEM_PROGRAM),
    ]);
    await buildSignAndSend(rpc, rpcSubscriptions, borrower, [
      buildDepositCollateralInstruction(
        coder,
        AEGIS_PROGRAM_ID,
        { depositor: borrower.address, market, position: borrowerPosition, collateralVault, depositorCollateralAta: borrowerCollateralAta, collateralMint, collateralTokenProgram: TOKEN_PROGRAM_ADDRESS },
        10_000_000_000n,
      ),
    ]);
    const borrowerLoanAta = await createTokenAccount(rpc, rpcSubscriptions, admin, loanMint, borrower.address);

    const priceFixtures = JSON.parse(
      readFileSync(path.join(__dirname, '..', 'fixtures', 'prices.json'), 'utf8'),
    ) as PriceFixtureFile;
    await injectPriceFixture(RPC_URL, priceFixtures.valid.collateralPriceUpdate);
    await injectPriceFixture(RPC_URL, priceFixtures.valid.loanPriceUpdate);
    const collateralPriceUpdate = address(priceFixtures.valid.collateralPriceUpdate.pubkey);
    const loanPriceUpdate = address(priceFixtures.valid.loanPriceUpdate.pubkey);

    await buildSignAndSend(rpc, rpcSubscriptions, borrower, [
      buildBorrowInstruction(
        coder,
        AEGIS_PROGRAM_ID,
        { owner: borrower.address, market, position: borrowerPosition, feePosition, loanVault, ownerLoanAta: borrowerLoanAta, loanMint, loanTokenProgram: TOKEN_PROGRAM_ADDRESS, collateralPriceUpdate, loanPriceUpdate },
        900_000_000n,
        0n,
      ),
    ]);
    console.log('    borrower deposited 10.000000000 SOL, borrowed 900.000000 USDC');

    console.log('\n[7] SOL crashes to $95.00 -- the position is now liquidatable.');
    await injectPriceFixture(RPC_URL, priceFixtures.crash.collateralPriceUpdate);
    await injectPriceFixture(RPC_URL, priceFixtures.crash.loanPriceUpdate);

    console.log('[8] A liquidator with ZERO loan-asset balance runs the keeper...');
    const liquidator = await generateKeyPairSigner();
    await airdrop({ recipientAddress: liquidator.address, lamports: lamports(10_000_000_000n), commitment: 'confirmed' });
    const liquidatorLoanAta = await createTokenAccount(rpc, rpcSubscriptions, admin, loanMint, liquidator.address);
    const liquidatorCollateralAta = await createTokenAccount(rpc, rpcSubscriptions, admin, collateralMint, liquidator.address);

    const exampleLiquidatorAuthority = await exampleLiquidatorAuthorityPda(EXAMPLE_LIQUIDATOR_PROGRAM_ID);
    const callbackCollateralAccount = await createTokenAccount(rpc, rpcSubscriptions, admin, collateralMint, exampleLiquidatorAuthority);
    const loanReserve = await createTokenAccount(rpc, rpcSubscriptions, admin, loanMint, exampleLiquidatorAuthority);
    await mintTo(rpc, rpcSubscriptions, admin, loanMint, loanReserve, 2_000_000_000n);
    console.log(`    liquidator loan-asset balance: 0 (relies entirely on the callback)`);
    console.log(`    example-liquidator reserve pre-funded: 2,000.000000 USDC`);

    const priceResolver: PriceResolver = {
      async resolve() {
        return {
          collateralPriceUpdate,
          loanPriceUpdate,
          band: {
            collateralPriceLo: 9_500_000_000n * 10n ** 10n - 20_000_000n * 10n ** 10n,
            loanPriceHi: 100_000_000n * 10n ** 10n + 20_000n * 10n ** 10n,
          },
        };
      },
    };
    const RATE_WAD = 100_000_000_000_000_000_000n; // $100/SOL deterministic local rate
    const callbackProvider: CallbackProvider = {
      async build(_market, _position, _repayAssets) {
        const { HandleLiquidationData } = await import('./exampleLiquidatorIx.js');
        return {
          program: EXAMPLE_LIQUIDATOR_PROGRAM_ID,
          collateralAccount: callbackCollateralAccount,
          data: HandleLiquidationData(RATE_WAD),
          remainingAccounts: [
            { address: exampleLiquidatorAuthority, writable: false },
            { address: loanReserve, writable: true },
          ],
        };
      },
    };

    const attempts = await scanAndLiquidateOneMarket(
      rpc,
      rpcSubscriptions,
      coder,
      AEGIS_PROGRAM_ID,
      market,
      liquidator,
      liquidatorLoanAta,
      liquidatorCollateralAta,
      priceResolver,
      callbackProvider,
      0n,
    );

    console.log('\n=== Keeper result ===');
    for (const attempt of attempts) {
      if (attempt.signature) {
        console.log(
          `  LIQUIDATED position ${attempt.position} via callback=${attempt.usedCallback}, repay=${attempt.repayAssets}`,
        );
        console.log(`  signature: ${attempt.signature}`);
      } else {
        console.log(`  skipped/failed ${attempt.position}: ${attempt.error}`);
      }
    }
    if (!attempts.some((a) => a.signature)) {
      throw new Error('Demo failed: the keeper did not successfully liquidate any position.');
    }
    console.log('\nDemo complete: the keeper found and executed a callback liquidation for a');
    console.log('liquidator holding zero loan-asset balance, entirely against a local, offline');
    console.log('Surfpool validator.');
  } finally {
    cleanup();
  }
}

interface PriceFixtureFile {
  valid: { collateralPriceUpdate: SurfnetAccount; loanPriceUpdate: SurfnetAccount };
  crash: { collateralPriceUpdate: SurfnetAccount; loanPriceUpdate: SurfnetAccount };
}

async function injectPriceFixture(rpcUrl: string, account: SurfnetAccount): Promise<void> {
  await surfnetSetAccount(rpcUrl, { ...account, owner: PYTH_RECEIVER_ID.toString() });
}

async function createMint(
  rpc: ReturnType<typeof createSolanaRpc<ReturnType<typeof devnet>>>,
  rpcSubscriptions: ReturnType<typeof createSolanaRpcSubscriptions<ReturnType<typeof devnet>>>,
  payer: KeyPairSigner,
  decimals: number,
): Promise<Address> {
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
  await buildSignAndSend(rpc, rpcSubscriptions, payer, [createIx, initIx]);
  return mint.address;
}

async function createTokenAccount(
  rpc: ReturnType<typeof createSolanaRpc<ReturnType<typeof devnet>>>,
  rpcSubscriptions: ReturnType<typeof createSolanaRpcSubscriptions<ReturnType<typeof devnet>>>,
  payer: KeyPairSigner,
  mint: Address,
  owner: Address,
): Promise<Address> {
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
  await buildSignAndSend(rpc, rpcSubscriptions, payer, [createIx, initIx]);
  return account.address;
}

async function mintTo(
  rpc: ReturnType<typeof createSolanaRpc<ReturnType<typeof devnet>>>,
  rpcSubscriptions: ReturnType<typeof createSolanaRpcSubscriptions<ReturnType<typeof devnet>>>,
  mintAuthority: KeyPairSigner,
  mint: Address,
  destination: Address,
  amount: bigint,
): Promise<void> {
  const ix = getMintToInstruction({ mint, token: destination, mintAuthority, amount });
  await buildSignAndSend(rpc, rpcSubscriptions, mintAuthority, [ix]);
}

main().catch((err) => {
  console.error('\nDemo FAILED:', err);
  process.exitCode = 1;
});
