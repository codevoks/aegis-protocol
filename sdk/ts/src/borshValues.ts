// Generic camelCase-typed-value <-> `@anchor-lang/core` BorshCoder wire-shape conversion, driven
// entirely by the `FieldSpec[]` arrays codegen emits into `generated/{types,instructions}.ts`.
//
// Why this exists: `BorshCoder` (the shared coder every encode/decode in this SDK goes through --
// never a hand-rolled borsh layout, `docs/phases/phase-09-sdk-ui.md` item 2) expects and returns
// snake_case field names, `BN` instances for u64/u128 fields, and `.toBuffer()`-shimmed objects for
// pubkey fields (see `bots/liquidator/src/anchorCore.ts`'s empirically-verified notes, which this
// module generalizes into a reusable, schema-driven pair of functions instead of one hand-written
// conversion per instruction/account).

import { getAddressEncoder, type Address } from '@solana/kit';
import { bn, pubkeyArg, BN } from './anchorCoder.js';

export type FieldKind =
  | 'pubkey'
  | 'bool'
  | 'u8'
  | 'u16'
  | 'u32'
  | 'i8'
  | 'i16'
  | 'i32'
  | 'u64'
  | 'u128'
  | 'i64'
  | 'i128'
  | 'bytes'
  | 'bytesN'
  | 'string'
  | 'array'
  | 'vec'
  | 'option'
  | 'defined';

export interface FieldSpec {
  readonly rawName: string;
  readonly camelName: string;
  readonly kind: FieldKind;
  readonly len?: number;
  readonly definedName?: string;
  readonly inner?: { kind: FieldKind; len?: number; definedName?: string };
}

const addressEncoder = getAddressEncoder();

function addressFromBytesLike(value: unknown): Address {
  return (value as { toString(): string }).toString() as Address;
}

/** Converts one leaf value from this SDK's camelCase/Address/bigint shape to the coder's expected
 *  wire shape, given only its `kind` (used both for top-level fields and for `array`/`vec`/`option`
 *  inner elements, which is why it takes a bare kind rather than a full `FieldSpec`). */
function encodeLeaf(kind: FieldKind, value: unknown, definedName: string | undefined, registry: TypeRegistry): unknown {
  switch (kind) {
    case 'pubkey':
      return pubkeyArg(value as Address);
    case 'u64':
    case 'u128':
    case 'i64':
    case 'i128':
      return bn(value as bigint);
    case 'bytesN':
      return Array.from(value as Uint8Array);
    case 'bytes':
      return Buffer.from(value as Uint8Array);
    case 'defined':
      return encodeFields(registry.fieldsFor(definedName!), value as Record<string, unknown>, registry);
    default:
      return value;
  }
}

function decodeLeaf(kind: FieldKind, value: unknown, definedName: string | undefined, registry: TypeRegistry): unknown {
  switch (kind) {
    case 'pubkey':
      return addressFromBytesLike(value);
    case 'u64':
    case 'u128':
    case 'i64':
    case 'i128':
      return BigInt((value as { toString(): string }).toString());
    case 'bytesN':
    case 'bytes':
      return Uint8Array.from(value as ArrayLike<number>);
    case 'defined':
      return decodeFields(registry.fieldsFor(definedName!), value as Record<string, unknown>, registry);
    default:
      return value;
  }
}

/** Looks up a defined type's own `FieldSpec[]` by name, so nested `{defined: {...}}` fields (there
 *  are none directly nested in Aegis's current IDL, but the mechanism is general) resolve without
 *  a second hand-written case. Supplied by `generated/index.ts` at call sites that need it. */
export interface TypeRegistry {
  fieldsFor(definedTypeName: string): readonly FieldSpec[];
}

export function encodeFields(
  fields: readonly FieldSpec[],
  camelObj: Record<string, unknown>,
  registry: TypeRegistry,
): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const f of fields) {
    const v = camelObj[f.camelName];
    if (f.kind === 'array' || f.kind === 'vec') {
      out[f.rawName] = (v as unknown[]).map((e) => encodeLeaf(f.inner!.kind, e, f.inner!.definedName, registry));
    } else if (f.kind === 'option') {
      out[f.rawName] = v === null || v === undefined ? null : encodeLeaf(f.inner!.kind, v, f.inner!.definedName, registry);
    } else {
      out[f.rawName] = encodeLeaf(f.kind, v, f.definedName, registry);
    }
  }
  return out;
}

export function decodeFields(
  fields: readonly FieldSpec[],
  rawObj: Record<string, unknown>,
  registry: TypeRegistry,
): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const f of fields) {
    const v = rawObj[f.rawName];
    if (f.kind === 'array' || f.kind === 'vec') {
      out[f.camelName] = (v as unknown[]).map((e) => decodeLeaf(f.inner!.kind, e, f.inner!.definedName, registry));
    } else if (f.kind === 'option') {
      out[f.camelName] = v === null || v === undefined ? null : decodeLeaf(f.inner!.kind, v, f.inner!.definedName, registry);
    } else {
      out[f.camelName] = decodeLeaf(f.kind, v, f.definedName, registry);
    }
  }
  return out;
}

// Re-exported so callers that only need the BN class (e.g. constructing raw coder args outside a
// FieldSpec-described struct) do not need a second import path.
export { BN };
