'use client';

// App-wide wiring: RPC client, program id, and the local dev signer (`localSigner.ts`), exposed
// through React context so every page/component gets a single, consistent connection. No hardcoded
// production/devnet/mainnet RPC endpoint (item 26) -- the default is local Surfpool, overridable
// via `NEXT_PUBLIC_AEGIS_RPC_URL`/`NEXT_PUBLIC_AEGIS_RPC_WS_URL`/`NEXT_PUBLIC_AEGIS_PROGRAM_ID`.

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from 'react';
import {
  address,
  airdropFactory,
  createSolanaRpc,
  createSolanaRpcSubscriptions,
  devnet,
  lamports,
  type Address,
  type KeyPairSigner,
  type Rpc,
  type RpcSubscriptions,
  type SolanaRpcApi,
  type SolanaRpcSubscriptionsApi,
} from '@solana/kit';
import { AEGIS_PROGRAM_ADDRESS } from '@aegis/sdk';
import { loadOrCreateLocalSigner } from './localSigner';

const DEFAULT_RPC_URL = 'http://127.0.0.1:8899';
const DEFAULT_RPC_WS_URL = 'ws://127.0.0.1:8900';

function readPublicEnv(name: string): string | undefined {
  return (process.env as Record<string, string | undefined>)[name];
}

export interface AegisClientContextValue {
  rpc: Rpc<SolanaRpcApi>;
  rpcSubscriptions: RpcSubscriptions<SolanaRpcSubscriptionsApi>;
  rpcUrl: string;
  programId: Address;
  signer: KeyPairSigner | null;
  /** Requests a local Surfpool faucet airdrop for the connected signer -- local-only; a real
   *  cluster has no equivalent free-money endpoint, and this app never targets one. */
  airdrop: (sol: number) => Promise<void>;
  refreshBalance: () => Promise<void>;
  balanceLamports: bigint | null;
}

const AegisClientContext = createContext<AegisClientContextValue | null>(null);

export function AegisProvider({ children }: { children: ReactNode }) {
  const rpcUrl = readPublicEnv('NEXT_PUBLIC_AEGIS_RPC_URL') ?? DEFAULT_RPC_URL;
  const rpcWsUrl = readPublicEnv('NEXT_PUBLIC_AEGIS_RPC_WS_URL') ?? DEFAULT_RPC_WS_URL;
  const programId = address(readPublicEnv('NEXT_PUBLIC_AEGIS_PROGRAM_ID') ?? AEGIS_PROGRAM_ADDRESS);

  const rpc = useMemo(() => createSolanaRpc(devnet(rpcUrl)), [rpcUrl]);
  const rpcSubscriptions = useMemo(() => createSolanaRpcSubscriptions(devnet(rpcWsUrl)), [rpcWsUrl]);

  const [signer, setSigner] = useState<KeyPairSigner | null>(null);
  const [balanceLamports, setBalanceLamports] = useState<bigint | null>(null);

  useEffect(() => {
    let cancelled = false;
    loadOrCreateLocalSigner().then((s) => {
      if (!cancelled) setSigner(s);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  const refreshBalance = useCallback(async () => {
    if (!signer) return;
    try {
      const { value } = await rpc.getBalance(signer.address).send();
      setBalanceLamports(value);
    } catch {
      setBalanceLamports(null);
    }
  }, [rpc, signer]);

  useEffect(() => {
    if (signer) void refreshBalance();
  }, [signer, refreshBalance]);

  const airdrop = useCallback(
    async (sol: number) => {
      if (!signer) return;
      const airdropFn = airdropFactory({ rpc, rpcSubscriptions });
      await airdropFn({
        recipientAddress: signer.address,
        lamports: lamports(BigInt(Math.round(sol * 1_000_000_000))),
        commitment: 'confirmed',
      });
      await refreshBalance();
    },
    [rpc, rpcSubscriptions, signer, refreshBalance],
  );

  const value: AegisClientContextValue = {
    rpc,
    rpcSubscriptions,
    rpcUrl,
    programId,
    signer,
    airdrop,
    refreshBalance,
    balanceLamports,
  };

  return <AegisClientContext.Provider value={value}>{children}</AegisClientContext.Provider>;
}

export function useAegisClient(): AegisClientContextValue {
  const ctx = useContext(AegisClientContext);
  if (!ctx) throw new Error('useAegisClient must be used within <AegisProvider>');
  return ctx;
}
