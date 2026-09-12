// SDK configuration. No hardcoded production/devnet/mainnet RPC endpoint anywhere in this file or
// this package (AGENTS.md §16, ADR-0010): the default is always local Surfpool, and every value is
// overridable by the caller (env var when running under Node, or an explicit argument in the
// browser, where `process.env` does not exist).

import { address, type Address } from '@solana/kit';
import { AEGIS_PROGRAM_ADDRESS } from './generated/idl.js';

export interface AegisClientConfig {
  /** JSON-RPC HTTP endpoint. Default: a local validator. */
  rpcUrl: string;
  /** JSON-RPC WebSocket endpoint (confirmation subscriptions). Default: a local validator. */
  rpcSubscriptionsUrl: string;
  /** The Aegis program's on-chain address -- read from the committed IDL snapshot
   *  (`generated/idl.json`'s own `address` field, itself written by `anchor build`) unless
   *  overridden, so a redeploy at a different local address does not require an SDK code change. */
  programId: Address;
}

const DEFAULT_RPC_URL = 'http://127.0.0.1:8899';
const DEFAULT_RPC_WS_URL = 'ws://127.0.0.1:8900';

function readEnv(name: string): string | undefined {
  // `process` does not exist in a browser bundle; guarded so this module is safe to import from
  // the Next.js app as well as from Node scripts/tests.
  return typeof process !== 'undefined' ? process.env?.[name] : undefined;
}

/** Loads config from environment variables where available (Node: env vars named
 *  `AEGIS_RPC_URL` / `AEGIS_RPC_WS_URL` / `AEGIS_PROGRAM_ID`), falling back to local-Surfpool
 *  defaults. Callers in a browser context should build `AegisClientConfig` directly instead. */
export function loadConfig(overrides: Partial<AegisClientConfig> = {}): AegisClientConfig {
  return {
    rpcUrl: overrides.rpcUrl ?? readEnv('AEGIS_RPC_URL') ?? DEFAULT_RPC_URL,
    rpcSubscriptionsUrl:
      overrides.rpcSubscriptionsUrl ?? readEnv('AEGIS_RPC_WS_URL') ?? DEFAULT_RPC_WS_URL,
    programId:
      overrides.programId ?? address(readEnv('AEGIS_PROGRAM_ID') ?? AEGIS_PROGRAM_ADDRESS),
  };
}

export const SYSTEM_PROGRAM_ADDRESS = address('11111111111111111111111111111111');
export const PYTH_RECEIVER_PROGRAM_ADDRESS = address(
  'rec5EKMGg6MxZYaMdyBfgwp4d5rB9T1VQH5pJv5LtFJ',
);
