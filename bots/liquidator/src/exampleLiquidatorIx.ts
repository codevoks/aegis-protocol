// `labs/example-liquidator` is built with `cargo build-sbf` directly (Anchor CLI's own notion of
// "the workspace" only discovers programs under `programs/`, per `Makefile`'s own note), so it has
// no generated IDL JSON to load a coder from. Its one instruction's discriminator is computed the
// same way Anchor's own `#[program]` macro computes it: the first 8 bytes of
// `sha256("global:<snake_case_instruction_name>")` -- this is Anchor's documented, stable sighash
// convention (unchanged since Anchor's discriminator scheme was introduced), not a guess.

import { createHash } from 'node:crypto';

function anchorSighash(instructionName: string): Buffer {
  const hash = createHash('sha256').update(`global:${instructionName}`).digest();
  return hash.subarray(0, 8);
}

/** `handle_liquidation(rate_wad: u128)` instruction data: 8-byte discriminator + 16-byte
 *  little-endian u128. */
export function HandleLiquidationData(rateWad: bigint): Uint8Array {
  const discriminator = anchorSighash('handle_liquidation');
  const rateBytes = Buffer.alloc(16);
  rateBytes.writeBigUInt64LE(rateWad & 0xffffffffffffffffn, 0);
  rateBytes.writeBigUInt64LE(rateWad >> 64n, 8);
  return new Uint8Array(Buffer.concat([discriminator, rateBytes]));
}
