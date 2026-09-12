// Typed account decoders (`docs/phases/phase-09-sdk-ui.md` item 6). Every decode goes through the
// shared `BorshCoder` (generated straight from the IDL, never a hand-rolled layout) and is preceded
// by an explicit account-owner check -- an account merely *shaped* like a `Market` but owned by a
// different program must be rejected, not silently decoded (a `getProgramAccounts` result is
// trusted data from the RPC's point of view, but this SDK does not trust it further than that).
//
// No `any`: every decoded value is typed via the generated `Market`/`Position`/`Protocol`
// interfaces (`generated/types.ts`), produced by the schema-driven `decodeFields` in
// `borshValues.ts` rather than a loose object cast.

import { getBase64Encoder, type Address, type Rpc, type SolanaRpcApi } from '@solana/kit';
import type { Base64EncodedBytes } from '@solana/rpc-types';
import { aegisCoder } from './anchorCoder.js';
import { decodeFields } from './borshValues.js';
import { definedTypeRegistry as registry } from './definedTypeRegistry.js';
import {
  ACCOUNT_DISCRIMINATORS,
  MARKET_FIELD_SPEC,
  POSITION_FIELD_SPEC,
  PROTOCOL_FIELD_SPEC,
  type Market,
  type Position,
  type Protocol,
} from './generated/index.js';

export class AccountNotFoundError extends Error {
  constructor(address: Address) {
    super(`account ${address} does not exist`);
    this.name = 'AccountNotFoundError';
  }
}

/** The account exists but is not owned by the Aegis program -- either a genuinely different
 *  account, or a spoofing attempt. Never decoded. */
export class AccountOwnerMismatchError extends Error {
  constructor(address: Address, expected: Address, actual: Address) {
    super(`account ${address} is owned by ${actual}, expected the Aegis program ${expected}`);
    this.name = 'AccountOwnerMismatchError';
  }
}

/** The account is owned by the right program but its bytes do not match the expected account's
 *  discriminator/layout -- e.g. a `Position` fetched where a `Market` was expected. */
export class AccountLayoutMismatchError extends Error {
  constructor(address: Address, expectedKind: string) {
    super(`account ${address} is not a valid ${expectedKind} (discriminator/layout mismatch)`);
    this.name = 'AccountLayoutMismatchError';
  }
}

function checkDiscriminator(address: Address, data: Uint8Array, kind: 'Market' | 'Position' | 'Protocol') {
  const expected = ACCOUNT_DISCRIMINATORS[kind];
  if (data.length < 8 || !expected.every((b, i) => data[i] === b)) {
    throw new AccountLayoutMismatchError(address, kind);
  }
}

export function decodeMarket(address: Address, data: Uint8Array): Market {
  checkDiscriminator(address, data, 'Market');
  const raw = aegisCoder().accounts.decode<Record<string, unknown>>('Market', Buffer.from(data));
  return decodeFields(MARKET_FIELD_SPEC, raw, registry) as unknown as Market;
}

export function decodePosition(address: Address, data: Uint8Array): Position {
  checkDiscriminator(address, data, 'Position');
  const raw = aegisCoder().accounts.decode<Record<string, unknown>>('Position', Buffer.from(data));
  return decodeFields(POSITION_FIELD_SPEC, raw, registry) as unknown as Position;
}

export function decodeProtocol(address: Address, data: Uint8Array): Protocol {
  checkDiscriminator(address, data, 'Protocol');
  const raw = aegisCoder().accounts.decode<Record<string, unknown>>('Protocol', Buffer.from(data));
  return decodeFields(PROTOCOL_FIELD_SPEC, raw, registry) as unknown as Protocol;
}

const base64Encoder = getBase64Encoder();

interface FetchedAccount {
  owner: Address;
  data: Uint8Array;
}

async function fetchAccount(
  rpc: Rpc<SolanaRpcApi>,
  address: Address,
): Promise<FetchedAccount | null> {
  const resp = await rpc.getAccountInfo(address, { encoding: 'base64' }).send();
  if (!resp.value) return null;
  const [b64] = resp.value.data;
  return { owner: resp.value.owner, data: new Uint8Array(base64Encoder.encode(b64)) };
}

async function fetchAndDecode<T>(
  rpc: Rpc<SolanaRpcApi>,
  address: Address,
  programId: Address,
  decode: (address: Address, data: Uint8Array) => T,
): Promise<T> {
  const account = await fetchAccount(rpc, address);
  if (!account) throw new AccountNotFoundError(address);
  if (account.owner !== programId) {
    throw new AccountOwnerMismatchError(address, programId, account.owner);
  }
  return decode(address, account.data);
}

export async function fetchMarket(
  rpc: Rpc<SolanaRpcApi>,
  address: Address,
  programId: Address,
): Promise<Market> {
  return fetchAndDecode(rpc, address, programId, decodeMarket);
}

export async function fetchPosition(
  rpc: Rpc<SolanaRpcApi>,
  address: Address,
  programId: Address,
): Promise<Position> {
  return fetchAndDecode(rpc, address, programId, decodePosition);
}

export async function fetchProtocol(
  rpc: Rpc<SolanaRpcApi>,
  address: Address,
  programId: Address,
): Promise<Protocol> {
  return fetchAndDecode(rpc, address, programId, decodeProtocol);
}

/** Returns `null` instead of throwing when the account does not exist yet -- the common case for
 *  "does this user have a Position in this market" checks that drive init-bundling (`ix.ts`). */
export async function fetchPositionIfExists(
  rpc: Rpc<SolanaRpcApi>,
  address: Address,
  programId: Address,
): Promise<Position | null> {
  try {
    return await fetchPosition(rpc, address, programId);
  } catch (err) {
    if (err instanceof AccountNotFoundError) return null;
    throw err;
  }
}

/** Every `Market` account the Aegis program owns, decoded (item 7: "market list/discovery where
 *  architecture supports it"). Aegis has no on-chain market registry by design
 *  (`docs/account-model.md` §2) -- markets are discovered via `getProgramAccounts` filtered by the
 *  `Market` account discriminator, exactly the same mechanism `bots/liquidator/src/scan.ts` already
 *  uses for `Position` (Phase 8), generalized to filter on discriminator instead of a `market`
 *  field so it works with no market address known in advance. */
export async function discoverMarkets(
  rpc: Rpc<SolanaRpcApi>,
  programId: Address,
): Promise<{ address: Address; market: Market }[]> {
  // `bytes` is a branded `Base64EncodedBytes` string type at the type level (`@solana/rpc-types`);
  // a plain string is the correct runtime value (the RPC transport only ever sees JSON), so this
  // cast is a type-level formality, not an unchecked escape.
  const discriminator = Buffer.from(ACCOUNT_DISCRIMINATORS.Market).toString('base64') as Base64EncodedBytes;
  const resp = await rpc
    .getProgramAccounts(programId, {
      encoding: 'base64',
      filters: [{ memcmp: { offset: 0n, bytes: discriminator, encoding: 'base64' } }],
    })
    .send();
  const out: { address: Address; market: Market }[] = [];
  for (const { pubkey, account } of resp) {
    const [b64] = account.data;
    const data = new Uint8Array(base64Encoder.encode(b64));
    out.push({ address: pubkey, market: decodeMarket(pubkey, data) });
  }
  return out;
}

/** Every `Position` belonging to `market`, discovered via `getProgramAccounts` with a `memcmp`
 *  filter on the `market` field (offset 8 for the discriminator, then `market: Pubkey` is
 *  `Position`'s first field) -- identical technique to `bots/liquidator/src/scan.ts`. */
export async function discoverPositions(
  rpc: Rpc<SolanaRpcApi>,
  programId: Address,
  market: Address,
): Promise<{ address: Address; position: Position }[]> {
  const resp = await rpc
    .getProgramAccounts(programId, {
      encoding: 'base64',
      filters: [{ memcmp: { offset: 8n, bytes: market, encoding: 'base58' } }],
    })
    .send();
  const out: { address: Address; position: Position }[] = [];
  for (const { pubkey, account } of resp) {
    const [b64] = account.data;
    const data = new Uint8Array(base64Encoder.encode(b64));
    try {
      out.push({ address: pubkey, position: decodePosition(pubkey, data) });
    } catch {
      continue; // not actually a Position (e.g. a Market also matched the raw byte filter)
    }
  }
  return out;
}
