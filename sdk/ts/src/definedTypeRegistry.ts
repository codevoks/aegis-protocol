// The one shared `TypeRegistry` implementation (`borshValues.ts`) used by every encode/decode call
// site (`accounts.ts`, `events.ts`, `ixEngine.ts`) to resolve a `{defined: {name}}` field's own
// `FieldSpec[]` -- e.g. an instruction's `args: CreateMarketArgs` wraps a defined struct that
// itself has fields needing the same camelCase/BN/pubkey-shim translation. Backed by the generated
// `TYPE_FIELD_SPECS` map, which covers every type in the IDL, so this file needs no changes when
// the protocol's types change -- only `npm run codegen` does.

import type { TypeRegistry } from './borshValues.js';
import { TYPE_FIELD_SPECS } from './generated/index.js';

export const definedTypeRegistry: TypeRegistry = {
  fieldsFor(definedTypeName: string) {
    const fields = TYPE_FIELD_SPECS[definedTypeName];
    if (!fields) {
      throw new Error(`no generated FieldSpec for defined type "${definedTypeName}" -- codegen may be stale`);
    }
    return fields;
  },
};
