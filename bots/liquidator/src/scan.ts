// Account discovery: fetch a `Market` and all `Position`s that belong to it, decoded with the
// real, `anchor build`-generated IDL coder (never a hand-maintained layout guess).
//
// Two more empirically-verified (not assumed) facts about this coder's DECODE side, mirroring
// `anchorCore.ts`'s notes on its encode side: decoded field names are the raw snake_case IDL names
// (`collateral_mint`, `liq_threshold`, ...), Pubkey-typed fields decode to legacy
// `@solana/web3.js`-style `PublicKey` instances (`.toString()` gives the base58 address), and
// u64/u128-typed fields decode to `BN` instances (`.toString()` gives the decimal string) --
// never plain strings/bigints.

import { getBase64Encoder, type Address, type Rpc, type SolanaRpcApi } from '@solana/kit';
import type { BorshCoder } from './anchorCore.js';

function pk(value: unknown): Address {
  return (value as { toString(): string }).toString() as Address;
}
function big(value: unknown): bigint {
  return BigInt((value as { toString(): string }).toString());
}

export interface DecodedMarket {
  address: Address;
  collateralMint: Address;
  loanMint: Address;
  collateralTokenProgram: Address;
  loanTokenProgram: Address;
  collateralVault: Address;
  loanVault: Address;
  feeRecipient: Address;
  configId: number;
  collateralDecimals: number;
  loanDecimals: number;
  collateralFeedId: Uint8Array;
  loanFeedId: Uint8Array;
  liqThreshold: bigint;
  totalBorrowAssets: bigint;
  totalBorrowShares: bigint;
  liquidationGuard: number;
}

export interface DecodedPosition {
  address: Address;
  market: Address;
  owner: Address;
  supplyShares: bigint;
  borrowShares: bigint;
  collateralAmount: bigint;
}

const base64Encoder = getBase64Encoder();

async function fetchAccountData(
  rpc: Rpc<SolanaRpcApi>,
  addr: Address,
): Promise<Uint8Array | null> {
  const resp = await rpc.getAccountInfo(addr, { encoding: 'base64' }).send();
  if (!resp.value) return null;
  const [b64] = resp.value.data;
  return new Uint8Array(base64Encoder.encode(b64));
}

export async function fetchMarket(
  rpc: Rpc<SolanaRpcApi>,
  coder: BorshCoder,
  marketAddress: Address,
): Promise<DecodedMarket> {
  const data = await fetchAccountData(rpc, marketAddress);
  if (!data) throw new Error(`Market account ${marketAddress} does not exist`);
  const decoded = coder.accounts.decode<Record<string, unknown>>('Market', Buffer.from(data));
  return {
    address: marketAddress,
    collateralMint: pk(decoded.collateral_mint),
    loanMint: pk(decoded.loan_mint),
    collateralTokenProgram: pk(decoded.collateral_token_program),
    loanTokenProgram: pk(decoded.loan_token_program),
    collateralVault: pk(decoded.collateral_vault),
    loanVault: pk(decoded.loan_vault),
    feeRecipient: pk(decoded.fee_recipient),
    configId: Number(decoded.config_id),
    collateralDecimals: Number(decoded.collateral_decimals),
    loanDecimals: Number(decoded.loan_decimals),
    collateralFeedId: Uint8Array.from(decoded.collateral_feed_id as number[]),
    loanFeedId: Uint8Array.from(decoded.loan_feed_id as number[]),
    liqThreshold: big(decoded.liq_threshold),
    totalBorrowAssets: big(decoded.total_borrow_assets),
    totalBorrowShares: big(decoded.total_borrow_shares),
    liquidationGuard: Number(decoded.liquidation_guard),
  };
}

/**
 * Scans for every `Position` belonging to `market` via `getProgramAccounts` with a `memcmp`
 * filter on the `market` field (offset 8 for the discriminator, then `market: Pubkey` is
 * `Position`'s first field -- `programs/aegis/src/state/position.rs`).
 */
export async function scanPositions(
  rpc: Rpc<SolanaRpcApi>,
  coder: BorshCoder,
  aegisProgramId: Address,
  market: Address,
): Promise<DecodedPosition[]> {
  const resp = await rpc
    .getProgramAccounts(aegisProgramId, {
      encoding: 'base64',
      filters: [{ memcmp: { offset: 8n, bytes: market, encoding: 'base58' } }],
    })
    .send();

  const positions: DecodedPosition[] = [];
  for (const { pubkey, account } of resp) {
    const [b64] = account.data;
    const data = Buffer.from(base64Encoder.encode(b64));
    let decoded: Record<string, unknown>;
    try {
      decoded = coder.accounts.decode<Record<string, unknown>>('Position', data);
    } catch {
      continue; // not a Position account (e.g. Market/Protocol also live under this program id)
    }
    positions.push({
      address: pubkey,
      market: pk(decoded.market),
      owner: pk(decoded.owner),
      supplyShares: big(decoded.supply_shares),
      borrowShares: big(decoded.borrow_shares),
      collateralAmount: big(decoded.collateral_amount),
    });
  }
  return positions;
}
