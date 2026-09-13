// GENERATED FILE -- do not edit by hand.
// Produced by sdk/ts/scripts/codegen.mjs from target/idl/aegis.json.
// Re-run `npm run codegen` (sdk/ts/) after `anchor build` changes the IDL; `npm run codegen:check` detects a stale commit.


import type { FieldSpec } from '../borshValues.js';

export type { Protocol, Market, Position } from './types.js';

export { PROTOCOL_FIELDS as PROTOCOL_FIELD_SPEC, MARKET_FIELDS as MARKET_FIELD_SPEC, POSITION_FIELDS as POSITION_FIELD_SPEC } from './types.js';


export interface AccountDiscriminator {
  name: string;
  discriminator: Uint8Array;
}

export const ACCOUNT_DISCRIMINATORS: Record<string, Uint8Array> = {
  "Market": Uint8Array.from([219, 190, 213, 55, 0, 227, 198, 154]),
  "PendingMarketParams": Uint8Array.from([34, 82, 61, 75, 226, 224, 192, 191]),
  "Position": Uint8Array.from([170, 188, 143, 228, 122, 64, 247, 208]),
  "Protocol": Uint8Array.from([45, 39, 101, 43, 115, 72, 131, 40]),
};
