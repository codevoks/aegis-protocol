// Error decoding (`docs/phases/phase-09-sdk-ui.md` item 22): maps Aegis program error codes to
// human-readable, typed errors using the generated `AEGIS_ERRORS` table (derived straight from the
// IDL's own `errors` array -- never a hand-copied duplicate of `programs/aegis/src/error.rs`).

import { AEGIS_ERRORS, type AegisErrorDef } from './generated/index.js';

export class AegisProgramError extends Error {
  readonly code: number;
  readonly errorName: string;
  constructor(def: AegisErrorDef) {
    super(`Aegis error ${def.code} (${def.name}): ${def.msg}`);
    this.name = 'AegisProgramError';
    this.code = def.code;
    this.errorName = def.name;
  }
}

/** Looks up an Anchor custom-program error code (the numeric code inside
 *  `{InstructionError: [i, {Custom: code}]}`, or the `0x....` in "custom program error: 0x..."). */
export function lookupAegisError(code: number): AegisErrorDef | undefined {
  return AEGIS_ERRORS[code];
}

/** Best-effort extraction of a custom program error code from the shapes commonly seen from
 *  `@solana/kit` RPC responses and thrown `SolanaError`s: a JSON-RPC `TransactionError` of the form
 *  `{InstructionError: [index, {Custom: code}]}`, or a message containing `custom program error:
 *  0x<hex>`. Returns `undefined` if neither shape is found -- callers should fall back to
 *  displaying the raw error in that case, never guess. */
export function extractCustomProgramErrorCode(err: unknown): number | undefined {
  if (err && typeof err === 'object') {
    const withContext = err as { context?: { err?: unknown }; err?: unknown };
    const txErr = withContext.context?.err ?? withContext.err;
    if (Array.isArray(txErr)) {
      // legacy positional shape: [index, {Custom: code}] is not standard; guard anyway
    } else if (txErr && typeof txErr === 'object' && 'InstructionError' in txErr) {
      const instrErr = (txErr as { InstructionError: [number, unknown] }).InstructionError;
      const inner = instrErr?.[1];
      if (inner && typeof inner === 'object' && 'Custom' in inner) {
        return (inner as { Custom: number }).Custom;
      }
    }
  }
  const message = err instanceof Error ? err.message : String(err);
  const match = message.match(/custom program error: 0x([0-9a-fA-F]+)/);
  if (match) return parseInt(match[1], 16);
  return undefined;
}

/** Decodes a thrown/returned error into an `AegisProgramError` when it carries a recognized Aegis
 *  custom error code, else returns the original error untouched -- callers should always check
 *  which one they got back rather than assuming decode always succeeds. */
export function decodeError(err: unknown): AegisProgramError | unknown {
  const code = extractCustomProgramErrorCode(err);
  if (code === undefined) return err;
  const def = lookupAegisError(code);
  if (!def) return err;
  return new AegisProgramError(def);
}
