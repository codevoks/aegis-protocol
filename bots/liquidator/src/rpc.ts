import { createSolanaRpc, createSolanaRpcSubscriptions, devnet } from '@solana/kit';
import type { KeeperConfig } from './config.js';

/**
 * `devnet(...)` here is purely a TYPE-BRANDING helper (`@solana/rpc-types`), not a real cluster
 * selection -- it makes the resulting RPC's TS type include test-cluster-only methods like
 * `requestAirdrop`, which both the local demo (`src/demo.ts`) and `airdropFactory` need. The
 * actual endpoint is still whatever `config.rpcUrl`/`config.rpcSubscriptionsUrl` say (a local
 * validator by default, AGENTS.md §16: no hardcoded devnet/mainnet address).
 */
export function makeRpc(config: KeeperConfig) {
  const rpc = createSolanaRpc(devnet(config.rpcUrl));
  const rpcSubscriptions = createSolanaRpcSubscriptions(devnet(config.rpcSubscriptionsUrl));
  return { rpc, rpcSubscriptions };
}
