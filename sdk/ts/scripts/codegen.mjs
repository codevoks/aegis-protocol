#!/usr/bin/env node
// Phase 9 IDL codegen (`docs/phases/phase-09-sdk-ui.md` item 2-3).
//
// Reads the CURRENT Anchor-1.x-generated IDL (`target/idl/aegis.json`, produced by `anchor build`
// -- Program Metadata layout, never a hand-maintained copy) and derives every generated TypeScript
// artifact under `sdk/ts/src/generated/` from it: account/instruction/event discriminators, field
// layouts, typed interfaces, and error codes. Nothing here is hand-typed from memory of the
// program's shape -- if the IDL changes, re-running this script changes the output, and
// `check-codegen.mjs` / `make codegen-check` catches a stale commit.
//
// Determinism: this script's output depends only on the bytes of `target/idl/aegis.json`. Given an
// unchanged IDL, re-running it produces byte-identical files.
//
// Run with: `npm run codegen` (from `sdk/ts/`), or `make codegen` from the repo root.

import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, '../../..');
const IDL_PATH = path.join(REPO_ROOT, 'target', 'idl', 'aegis.json');
// Overridable so `check-codegen.mjs` can regenerate into a scratch directory without touching the
// committed one, while every other caller (`npm run codegen`, `make codegen`) gets the real path.
const OUT_DIR = process.argv[2]
  ? path.resolve(process.argv[2])
  : path.resolve(__dirname, '../src/generated');

function loadIdl() {
  let raw;
  try {
    raw = readFileSync(IDL_PATH, 'utf8');
  } catch (err) {
    throw new Error(
      `Could not read ${IDL_PATH} -- run \`anchor build\` (or \`make build\`) first so the real ` +
        `IDL exists before running codegen. (${err.message})`,
    );
  }
  return JSON.parse(raw);
}

// --- naming ---

function snakeToCamel(s) {
  return s.replace(/_([a-z0-9])/g, (_, c) => c.toUpperCase());
}

function pascalCase(s) {
  const camel = snakeToCamel(s);
  return camel.charAt(0).toUpperCase() + camel.slice(1);
}

// --- IDL type -> {tsType, kind} ---
//
// `kind` drives the generic runtime encode/decode helpers in `src/borshValues.ts`: it is a schema,
// not a re-implementation of borsh itself (`@anchor-lang/core`'s BorshCoder still does the actual
// byte-level (de)serialization -- this is only the camelCase<->snake_case, Address<->pubkey-shim,
// and bigint<->BN translation layer between this SDK's ergonomic types and the coder's expected
// shapes, which the coder does not do for us).
function resolveType(t, typesByName) {
  if (typeof t === 'string') {
    switch (t) {
      case 'pubkey':
      case 'publicKey':
        return { tsType: 'Address', kind: 'pubkey' };
      case 'bool':
        return { tsType: 'boolean', kind: 'bool' };
      case 'u8':
      case 'u16':
      case 'u32':
      case 'i8':
      case 'i16':
      case 'i32':
        return { tsType: 'number', kind: t };
      case 'u64':
      case 'u128':
      case 'i64':
      case 'i128':
        return { tsType: 'bigint', kind: t };
      case 'bytes':
        return { tsType: 'Uint8Array', kind: 'bytes' };
      case 'string':
        return { tsType: 'string', kind: 'string' };
      default:
        throw new Error(`unhandled primitive IDL type: ${t}`);
    }
  }
  if (t.array) {
    const [elem, len] = t.array;
    if (elem === 'u8') {
      return { tsType: 'Uint8Array', kind: 'bytesN', len };
    }
    const inner = resolveType(elem, typesByName);
    return { tsType: `${inner.tsType}[]`, kind: 'array', inner, len };
  }
  if (t.vec) {
    const inner = resolveType(t.vec, typesByName);
    return { tsType: `${inner.tsType}[]`, kind: 'vec', inner };
  }
  if (t.option) {
    const inner = resolveType(t.option, typesByName);
    return { tsType: `${inner.tsType} | null`, kind: 'option', inner };
  }
  if (t.defined) {
    const name = t.defined.name;
    if (!typesByName.has(name)) throw new Error(`unknown defined type: ${name}`);
    return { tsType: name, kind: 'defined', name };
  }
  throw new Error(`unhandled IDL type shape: ${JSON.stringify(t)}`);
}

function fieldsOf(idlType, typesByName) {
  const fields = idlType.type.fields ?? [];
  return fields.map((f) => {
    const resolved = resolveType(f.type, typesByName);
    return { rawName: f.name, camelName: snakeToCamel(f.name), ...resolved };
  });
}

// --- emit helpers ---

function header(sourceIdlPath) {
  return (
    `// GENERATED FILE -- do not edit by hand.\n` +
    `// Produced by sdk/ts/scripts/codegen.mjs from ${sourceIdlPath}.\n` +
    `// Re-run \`npm run codegen\` (sdk/ts/) after \`anchor build\` changes the IDL; ` +
    `\`npm run codegen:check\` detects a stale commit.\n\n`
  );
}

function emitFieldSpec(f) {
  const base = `{ rawName: ${JSON.stringify(f.rawName)}, camelName: ${JSON.stringify(f.camelName)}, kind: ${JSON.stringify(f.kind)}`;
  if (f.kind === 'defined') return `${base}, definedName: ${JSON.stringify(f.name)} }`;
  if (f.kind === 'array' || f.kind === 'vec') {
    return `${base}, inner: ${emitFieldSpecInner(f.inner)} }`;
  }
  if (f.kind === 'option') {
    return `${base}, inner: ${emitFieldSpecInner(f.inner)} }`;
  }
  if (f.kind === 'bytesN') return `${base}, len: ${f.len} }`;
  return `${base} }`;
}

function emitFieldSpecInner(inner) {
  if (inner.kind === 'defined') return `{ kind: ${JSON.stringify(inner.kind)}, definedName: ${JSON.stringify(inner.name)} }`;
  if (inner.kind === 'bytesN') return `{ kind: ${JSON.stringify(inner.kind)}, len: ${inner.len} }`;
  return `{ kind: ${JSON.stringify(inner.kind)} }`;
}

function emitInterface(name, fields, docs) {
  const lines = [];
  if (docs && docs.length) lines.push('/**', ...docs.map((d) => ` * ${d}`), ' */');
  lines.push(`export interface ${name} {`);
  for (const f of fields) {
    lines.push(`  ${f.camelName}: ${f.tsType};`);
  }
  lines.push('}');
  return lines.join('\n');
}

function emitFieldSpecsConst(constName, fields) {
  const lines = [`export const ${constName}: readonly FieldSpec[] = [`];
  for (const f of fields) lines.push(`  ${emitFieldSpec(f)},`);
  lines.push('];');
  return lines.join('\n');
}

function main() {
  mkdirSync(OUT_DIR, { recursive: true });
  const idl = loadIdl();
  const idlRelPath = path.relative(REPO_ROOT, IDL_PATH);

  // 1. Committed IDL snapshot -- the single source of truth this SDK's generated code and runtime
  //    coder both read, so the package is self-contained (does not require a fresh local
  //    `anchor build` merely to import it), while still being regenerable and diffable against a
  //    fresh `target/idl/aegis.json` by `check-codegen.mjs`.
  writeFileSync(path.join(OUT_DIR, 'idl.json'), JSON.stringify(idl, null, 2) + '\n');
  writeFileSync(
    path.join(OUT_DIR, 'idl.ts'),
    header(idlRelPath) +
      `import idlJson from './idl.json' with { type: 'json' };\n` +
      `import type { Idl } from '@anchor-lang/core';\n\n` +
      `/** The exact IDL \`anchor build\` produced, committed as a snapshot (see codegen.mjs). */\n` +
      `export const AEGIS_IDL = idlJson as unknown as Idl;\n` +
      `export const AEGIS_PROGRAM_ADDRESS = ${JSON.stringify(idl.address)};\n`,
  );

  const typesByName = new Map(idl.types.map((t) => [t.name, t]));

  // 2. types.ts -- every `types` entry (event payloads, instruction arg structs, and the three
  //    account structs themselves) as a typed interface plus a runtime field-spec array.
  {
    const parts = [
      header(idlRelPath),
      `import type { Address } from '@solana/kit';\n`,
      `import type { FieldSpec } from '../borshValues.js';\n`,
    ];
    for (const t of idl.types) {
      const fields = fieldsOf(t, typesByName);
      parts.push('\n' + emitInterface(t.name, fields, t.docs));
      parts.push('\n' + emitFieldSpecsConst(`${t.name.toUpperCase()}_FIELDS`, fields));
    }
    // One registry covering every defined type (`{defined: {name}}` field), so a `{defined: ...}`
    // reference at ANY nesting depth -- an instruction's own `args: SomeArgs` struct, an event
    // field, or (should the IDL ever add one) a struct field nested inside another struct -- can be
    // resolved generically by `borshValues.ts`'s `encodeFields`/`decodeFields` without a second,
    // hand-written case per call site.
    parts.push('\nexport const TYPE_FIELD_SPECS: Record<string, readonly FieldSpec[]> = {');
    for (const t of idl.types) {
      parts.push(`  ${JSON.stringify(t.name)}: ${t.name.toUpperCase()}_FIELDS,`);
    }
    parts.push('};');
    writeFileSync(path.join(OUT_DIR, 'types.ts'), parts.join('\n') + '\n');
  }

  // 3. accounts.ts -- Protocol/Market/Position: discriminators + re-exported typed interfaces.
  {
    const parts = [
      header(idlRelPath),
      `import type { FieldSpec } from '../borshValues.js';\n`,
      `export type { Protocol, Market, Position } from './types.js';\n`,
      `export { PROTOCOL_FIELDS as PROTOCOL_FIELD_SPEC, MARKET_FIELDS as MARKET_FIELD_SPEC, POSITION_FIELDS as POSITION_FIELD_SPEC } from './types.js';\n`,
    ];
    parts.push('\nexport interface AccountDiscriminator {');
    parts.push('  name: string;');
    parts.push('  discriminator: Uint8Array;');
    parts.push('}\n');
    parts.push('export const ACCOUNT_DISCRIMINATORS: Record<string, Uint8Array> = {');
    for (const a of idl.accounts) {
      parts.push(`  ${JSON.stringify(a.name)}: Uint8Array.from([${a.discriminator.join(', ')}]),`);
    }
    parts.push('};');
    writeFileSync(path.join(OUT_DIR, 'accounts.ts'), parts.join('\n') + '\n');
  }

  // 4. events.ts -- event name -> discriminator + typed payload re-exports + field specs (for
  //    generic camelCase decoding via `borshValues.ts`, the same mechanism `accounts.ts` uses).
  {
    const eventTypeNames = idl.events.map((e) => e.name);
    const fieldConstNames = eventTypeNames.map((n) => `${n.toUpperCase()}_FIELDS`);
    const parts = [
      header(idlRelPath),
      `import type { FieldSpec } from '../borshValues.js';\n`,
      `export type { ${eventTypeNames.join(', ')} } from './types.js';`,
      `import { ${fieldConstNames.join(', ')} } from './types.js';\n`,
    ];
    parts.push('export const EVENT_DISCRIMINATORS: Record<string, Uint8Array> = {');
    for (const e of idl.events) {
      parts.push(`  ${JSON.stringify(e.name)}: Uint8Array.from([${e.discriminator.join(', ')}]),`);
    }
    parts.push('};\n');
    parts.push('export const EVENT_FIELD_SPECS: Record<string, readonly FieldSpec[]> = {');
    for (const e of idl.events) {
      parts.push(`  ${JSON.stringify(e.name)}: ${e.name.toUpperCase()}_FIELDS,`);
    }
    parts.push('};');
    writeFileSync(path.join(OUT_DIR, 'events.ts'), parts.join('\n') + '\n');
  }

  // 5. errors.ts -- error code -> {name, msg}, derived straight from the IDL's own error metadata
  //    (never a hand-copied duplicate of programs/aegis/src/error.rs).
  {
    const parts = [header(idlRelPath)];
    parts.push('export interface AegisErrorDef {');
    parts.push('  code: number;');
    parts.push('  name: string;');
    parts.push('  msg: string;');
    parts.push('}\n');
    parts.push('export const AEGIS_ERRORS: Record<number, AegisErrorDef> = {');
    for (const e of idl.errors) {
      parts.push(`  ${e.code}: { code: ${e.code}, name: ${JSON.stringify(e.name)}, msg: ${JSON.stringify(e.msg ?? '')} },`);
    }
    parts.push('};');
    writeFileSync(path.join(OUT_DIR, 'errors.ts'), parts.join('\n') + '\n');
  }

  // 6. instructions.ts -- per-instruction discriminator, args field spec, and account-meta schema
  //    (name, writable, signer, optional) taken VERBATIM from the IDL's own per-instruction account
  //    list order -- this is what lets `ixEngine.ts`'s generic builder assemble a correctly-ordered,
  //    correctly-flagged AccountMeta list without a second, hand-maintained per-instruction account
  //    array (contrast `bots/liquidator/src/txBuilders.ts`, which necessarily hand-wrote these
  //    before this codegen existed). Also emits, per instruction, a typed `<Pascal>Accounts`
  //    interface, a typed `<Pascal>Args` interface (when the instruction takes arguments), and a
  //    typed `build<Pascal>Instruction(programId, accounts, args?)` wrapper around the one shared
  //    engine function -- so application code never passes a bare string/object where a typo would
  //    only be caught at runtime.
  {
    const parts = [
      header(idlRelPath),
      `import type { Address, Instruction } from '@solana/kit';\n`,
      `import type { FieldSpec } from '../borshValues.js';\n`,
      `import { buildGenericInstruction } from '../ixEngine.js';\n`,
      `import type { CreateMarketArgs, InitProtocolArgs } from './types.js';\n`,
      `export interface IxAccountMeta {\n  readonly name: string;\n  readonly camelName: string;\n  readonly writable: boolean;\n  readonly signer: boolean;\n  readonly optional: boolean;\n}\n`,
      `export interface IxDef {\n  readonly name: string;\n  readonly discriminator: Uint8Array;\n  readonly accounts: readonly IxAccountMeta[];\n  readonly argsFields: readonly FieldSpec[];\n}\n`,
    ];
    parts.push('export const INSTRUCTIONS: Record<string, IxDef> = {');
    for (const ix of idl.instructions) {
      const accEntries = ix.accounts.map((a) => {
        const camel = snakeToCamel(a.name);
        return `    { name: ${JSON.stringify(a.name)}, camelName: ${JSON.stringify(camel)}, writable: ${!!a.writable}, signer: ${!!a.signer}, optional: ${!!a.optional} },`;
      });
      const argFields = (ix.args ?? []).map((a) => {
        const resolved = resolveType(a.type, typesByName);
        return { rawName: a.name, camelName: snakeToCamel(a.name), ...resolved };
      });
      parts.push(`  ${JSON.stringify(ix.name)}: {`);
      parts.push(`    name: ${JSON.stringify(ix.name)},`);
      parts.push(`    discriminator: Uint8Array.from([${ix.discriminator.join(', ')}]),`);
      parts.push('    accounts: [');
      parts.push(...accEntries);
      parts.push('    ],');
      parts.push(`    argsFields: [${argFields.map(emitFieldSpec).join(', ')}],`);
      parts.push('  },');
    }
    parts.push('};\n');

    // Per-instruction typed Accounts/Args interfaces and builder functions.
    for (const ix of idl.instructions) {
      if (ix.name === 'ping') continue; // toolchain-proof only; not part of the SDK's public surface
      const pascal = pascalCase(ix.name);

      parts.push(`export interface ${pascal}Accounts {`);
      for (const a of ix.accounts) {
        const camel = snakeToCamel(a.name);
        parts.push(`  ${camel}${a.optional ? '?' : ''}: Address;`);
      }
      parts.push('}\n');

      const argFields = (ix.args ?? []).map((a) => {
        const resolved = resolveType(a.type, typesByName);
        return { rawName: a.name, camelName: snakeToCamel(a.name), ...resolved };
      });
      let argsTypeName = null;
      if (argFields.length === 1 && argFields[0].kind === 'defined') {
        // The common Aegis shape: a single `args: CreateMarketArgs`-style struct.
        argsTypeName = argFields[0].tsType;
      } else if (argFields.length > 0) {
        argsTypeName = `${pascal}Args`;
        parts.push(`export interface ${argsTypeName} {`);
        for (const f of argFields) parts.push(`  ${f.camelName}: ${f.tsType};`);
        parts.push('}\n');
      }

      const argsParam = argsTypeName ? `, args: ${argsTypeName}` : '';
      const argsPassed = argFields.length === 1 && argFields[0].kind === 'defined'
        ? `{ ${argFields[0].camelName}: args }`
        : argFields.length > 0
          ? `args as unknown as Record<string, unknown>`
          : '{}';
      const remainingParam = ix.name === 'liquidate'
        ? ', remainingAccounts: { address: Address; writable: boolean }[] = []'
        : '';
      const remainingArg = ix.name === 'liquidate' ? ', remainingAccounts' : '';

      parts.push(
        `export function build${pascal}Instruction(programId: Address, accounts: ${pascal}Accounts${argsParam}${remainingParam}): Instruction {`,
      );
      parts.push(
        `  return buildGenericInstruction(programId, ${JSON.stringify(ix.name)}, accounts as unknown as Record<string, Address | undefined>, ${argsPassed}${remainingArg});`,
      );
      parts.push('}\n');
    }

    writeFileSync(path.join(OUT_DIR, 'instructions.ts'), parts.join('\n') + '\n');
  }

  // 7. index.ts barrel.
  {
    const content =
      header(idlRelPath) +
      [
        `export * from './idl.js';`,
        `export * from './types.js';`,
        `export * from './accounts.js';`,
        `export * from './events.js';`,
        `export * from './errors.js';`,
        `export * from './instructions.js';`,
      ].join('\n') +
      '\n';
    writeFileSync(path.join(OUT_DIR, 'index.ts'), content);
  }

  console.log(`Codegen complete. Wrote ${OUT_DIR}`);
  console.log(`  ${idl.instructions.length} instructions, ${idl.accounts.length} accounts, ${idl.events.length} events, ${idl.errors.length} errors, ${idl.types.length} types.`);
}

main();
