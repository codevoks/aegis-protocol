// Entry point: `npm run keeper`. Scans every configured market once and attempts any liquidation
// candidate it finds; run this on an interval (cron, a systemd timer, ...) for continuous
// operation -- this bot deliberately does not daemonize itself.

import { loadConfig } from './config.js';
import { loadOrGenerateSigner } from './signer.js';
import { loadAegisCoder } from './idl.js';
import { makeRpc } from './rpc.js';
import { scanAndLiquidateOneMarket, type CallbackProvider, type PriceResolver } from './keeper.js';
import { fetchMarket } from './scan.js';

async function main() {
  const config = loadConfig();
  if (config.markets.length === 0) {
    console.error(
      'No markets configured. Set AEGIS_MARKETS to a comma-separated list of Market addresses.',
    );
    process.exitCode = 1;
    return;
  }

  const coder = loadAegisCoder();
  const { rpc, rpcSubscriptions } = makeRpc(config);
  const payer = await loadOrGenerateSigner(config.keypairPath);
  console.log(`Keeper signer: ${payer.address}`);

  // Placeholder resolvers for a real deployment -- see README.md "Oracle prices" and
  // "Callback funding" for what a production implementation needs to supply here. The local demo
  // (`npm run demo`, `src/demo.ts`) supplies real, working implementations of both against its
  // own seeded fixture state.
  const priceResolver: PriceResolver = {
    async resolve() {
      throw new Error(
        'No PriceResolver configured -- see bots/liquidator/README.md "Oracle prices". ' +
          'The local demo (npm run demo) provides a working fixture implementation.',
      );
    },
  };
  const callbackProvider: CallbackProvider = {
    async build() {
      return undefined; // default policy: no callback unless a real implementation overrides this
    },
  };

  for (const market of config.markets) {
    const decoded = await fetchMarket(rpc, coder, market);
    console.log(`Scanning market ${market} (collateral=${decoded.collateralMint} loan=${decoded.loanMint})`);
    const attempts = await scanAndLiquidateOneMarket(
      rpc,
      rpcSubscriptions,
      coder,
      config.aegisProgramId,
      market,
      payer,
      payer.address, // caller must configure real ATAs in a production deployment
      payer.address,
      priceResolver,
      callbackProvider,
      config.minProfitBaseUnits,
    );
    for (const attempt of attempts) {
      if (attempt.signature) {
        console.log(
          `  liquidated ${attempt.position} (callback=${attempt.usedCallback}, repay=${attempt.repayAssets}): ${attempt.signature}`,
        );
      } else {
        console.log(
          `  skipped/failed ${attempt.position} (callback=${attempt.usedCallback}): ${attempt.error}`,
        );
      }
    }
  }
}

main().catch((err) => {
  console.error(err);
  process.exitCode = 1;
});
