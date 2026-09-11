// `@anchor-lang/core` ships as a CJS package; Node's ESM loader (this package is `"type":
// "module"`) does not support named imports from it directly ("Named export 'BN' not found" --
// verified empirically against the installed 1.2.0 package, not assumed). The default-import +
// destructure pattern below is Node's own documented workaround for exactly this case, centralized
// here once so no other file needs to know about it.
//
// Two further empirically-verified (not assumed) facts about this coder, both load-bearing for
// every instruction builder in this package:
// 1. `coder.instruction.encode(name, args)` wants the RAW snake_case names from the IDL JSON
//    itself (e.g. `initialize_protocol`, `fee_recipient`) -- camelCase fails with "Unknown method".
// 2. Pubkey-typed args need an object with a `.toBuffer(): Buffer` method (the legacy
//    `@solana/web3.js` `PublicKey` convention this coder still expects internally), and
//    u64/u128-typed args need a `BN` instance (bn.js), not a native `bigint` -- plain values fail
//    with "src.toArrayLike is not a function". See `pubkeyArg`/`bn` below.
import pkg from '@anchor-lang/core';
import type { Address } from '@solana/kit';
import { getAddressEncoder } from '@solana/kit';

export type BorshCoder = import('@anchor-lang/core').BorshCoder;

const { BorshCoder, BN } = pkg as unknown as {
  BorshCoder: new (idl: unknown) => BorshCoder;
  BN: typeof import('bn.js');
};

export { BorshCoder, BN };
export type { Idl } from '@anchor-lang/core';

const addressEncoder = getAddressEncoder();

/** Wraps an `Address` (a plain base58 string in `@solana/kit`) in the minimal shim the coder's
 *  borsh pubkey layout actually calls: `.toBuffer()`. */
export function pubkeyArg(address: Address): { toBuffer(): Buffer } {
  const bytes = addressEncoder.encode(address);
  return { toBuffer: () => Buffer.from(bytes) };
}

/** Wraps a `bigint`/`number` for a u64/u128-typed IDL arg. */
export function bn(value: bigint | number): InstanceType<typeof BN> {
  return new BN(value.toString());
}
