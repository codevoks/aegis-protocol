// `@anchor-lang/core` ships as a CJS package; Node's ESM loader (this package is `"type": "module"`)
// does not support named imports from it directly ("Named export 'BN' not found" -- verified
// empirically against the installed 1.2.0 package during Phase 8, `bots/liquidator/src/
// anchorCore.ts`, which this file mirrors exactly for the SDK). The default-import + destructure
// pattern below is Node's own documented workaround for exactly this case.
//
// Two further empirically-verified (not assumed) facts about this coder, both load-bearing for
// every instruction/account/event encode-decode in this SDK:
// 1. `coder.instruction.encode(name, args)` / `coder.accounts.decode(name, data)` want the RAW
//    snake_case names from the IDL JSON itself (`initialize_protocol`, `collateral_mint`, ...).
// 2. Pubkey-typed values need an object with a `.toBuffer(): Buffer` method (the legacy
//    `@solana/web3.js` `PublicKey` convention this coder still expects internally) on encode, and
//    decode to such an object (`.toString()` gives the base58 address) -- never a plain string.
//    u64/u128-typed values need a `BN` instance (bn.js) on encode and decode to one -- never a
//    native `bigint` directly. `borshValues.ts` wraps both directions generically from the
//    generated `FieldSpec[]` schema so no call site needs to know this.
import * as pkg from '@anchor-lang/core';
import type { Address } from '@solana/kit';
import { getAddressEncoder } from '@solana/kit';
import { AEGIS_IDL } from './generated/idl.js';

export type BorshCoder = import('@anchor-lang/core').BorshCoder;

type AnchorCoreModule = {
  BorshCoder: new (idl: unknown) => BorshCoder;
  BN: typeof import('bn.js');
};

// Different bundlers synthesize CJS/ESM interop differently for a default import of a CJS
// package: plain Node ESM and `tsx`/`vitest` put the real exports directly on `pkg`, while
// webpack's production SSR bundle (verified empirically building the Next.js app) instead nests
// them under `pkg.default`. Rather than assume one shape, use whichever actually has `BorshCoder`
// at runtime -- deliberately untyped here (the two candidate shapes are not expressible as one
// sound static type), re-typed as `AnchorCoreModule` immediately after the unwrap.
function unwrapAnchorCore(imported: unknown): AnchorCoreModule {
  const candidate = imported as Record<string, unknown>;
  const inner = candidate?.BorshCoder ? candidate : (candidate?.default as Record<string, unknown> | undefined);
  if (!inner?.BorshCoder) {
    throw new Error('@anchor-lang/core: could not locate BorshCoder export in either module shape');
  }
  return inner as unknown as AnchorCoreModule;
}

const { BorshCoder, BN } = unwrapAnchorCore(pkg);

export { BorshCoder, BN };
export type { Idl } from '@anchor-lang/core';

const addressEncoder = getAddressEncoder();

/** Wraps an `Address` (a plain base58 string in `@solana/kit`) in the minimal shim the coder's
 *  borsh pubkey layout actually calls: `.toBuffer()`. */
export function pubkeyArg(address: Address): { toBuffer(): Buffer } {
  const bytes = addressEncoder.encode(address);
  return { toBuffer: () => Buffer.from(bytes) };
}

/** Wraps a `bigint`/`number` for a u64/u128/i64/i128-typed IDL value. */
export function bn(value: bigint | number): InstanceType<typeof BN> {
  return new BN(value.toString());
}

let cachedCoder: BorshCoder | undefined;

/** The one shared coder instance for this SDK, built from the committed IDL snapshot
 *  (`generated/idl.json`) -- never a hand-maintained duplicate of the account/instruction/event
 *  layouts or discriminators. */
export function aegisCoder(): BorshCoder {
  if (!cachedCoder) cachedCoder = new BorshCoder(AEGIS_IDL);
  return cachedCoder;
}
