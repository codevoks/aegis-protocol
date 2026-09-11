// Configuration for the Aegis liquidator keeper (Phase 8, docs/phases/phase-08-composability.md).
//
// No secrets live here or anywhere in this package (AGENTS.md §19). The keeper's signing key is
// either generated fresh at startup (the default -- see `loadOrGenerateSigner` in `signer.ts`) or
// loaded from a local file path named by `AEGIS_KEEPER_KEYPAIR_PATH`, which is never committed and
// is the caller's own responsibility, exactly like `solana-keygen`'s own file-based keypairs.

import { address, type Address } from '@solana/kit';

export interface KeeperConfig {
  /** JSON-RPC endpoint. Defaults to a local validator -- no hardcoded mainnet/devnet endpoint
   *  (AGENTS.md §16: "no hardcoded RPC endpoints, no devnet addresses in default paths"). */
  rpcUrl: string;
  /** WebSocket endpoint for the same node, used for confirmation subscriptions. */
  rpcSubscriptionsUrl: string;
  aegisProgramId: Address;
  exampleLiquidatorProgramId: Address;
  /** Markets to scan. In a real deployment this would come from an on-chain registry lookup or a
   *  config file; Aegis has none by design (docs/account-model.md §2), so the keeper is told
   *  which markets to watch explicitly. */
  markets: Address[];
  /** Optional path to a local keypair JSON file (the same format `solana-keygen new` produces).
   *  If unset, the keeper generates and uses an ephemeral signer for the process lifetime. */
  keypairPath: string | undefined;
  /** Minimum profit (in loan-asset base units) the keeper requires before submitting a
   *  liquidation -- purely an off-chain, advisory filter (spec #21); on-chain economics decide
   *  the actual, authoritative outcome regardless of this threshold. */
  minProfitBaseUnits: bigint;
}

export function loadConfig(): KeeperConfig {
  const rpcUrl = process.env.AEGIS_RPC_URL ?? 'http://127.0.0.1:8899';
  const rpcSubscriptionsUrl = process.env.AEGIS_RPC_WS_URL ?? 'ws://127.0.0.1:8900';
  const aegisProgramId = address(
    process.env.AEGIS_PROGRAM_ID ?? 'DbRhjkZV1QSxMj5AvrYdgVsyEz8nKhoCLnSLGSKsqaF9',
  );
  const exampleLiquidatorProgramId = address(
    process.env.AEGIS_EXAMPLE_LIQUIDATOR_PROGRAM_ID ??
      'CyexeWx6KSzkD4HtCges24DYWMt8ny4wnsjvE39iao1v',
  );
  const marketsEnv = process.env.AEGIS_MARKETS;
  const markets = marketsEnv
    ? marketsEnv.split(',').map((m) => address(m.trim()))
    : [];

  return {
    rpcUrl,
    rpcSubscriptionsUrl,
    aegisProgramId,
    exampleLiquidatorProgramId,
    markets,
    keypairPath: process.env.AEGIS_KEEPER_KEYPAIR_PATH,
    minProfitBaseUnits: BigInt(process.env.AEGIS_MIN_PROFIT_BASE_UNITS ?? '0'),
  };
}
