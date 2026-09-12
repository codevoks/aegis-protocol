// Generic, IDL-driven instruction assembly (`docs/phases/phase-09-sdk-ui.md` item 15/24: "main
// usage should be typed and safe" while "avoid requiring application code to manually know ...
// exact account order [or] instruction discriminator bytes").
//
// Every per-instruction typed builder in `generated/instructions.ts` calls this ONE function. The
// account-meta list's order, writability, signer-ness, and optional-ness come directly from the
// IDL (`INSTRUCTIONS[name].accounts`, itself copied verbatim from `anchor build`'s own per-
// instruction account list order) -- there is no second, hand-maintained account-order table
// anywhere in this SDK (contrast `bots/liquidator/src/txBuilders.ts`, which necessarily hand-wrote
// one per instruction before this codegen existed).

import { AccountRole, type Address, type Instruction } from '@solana/kit';
import { aegisCoder } from './anchorCoder.js';
import { decodeFields, encodeFields } from './borshValues.js';
import { definedTypeRegistry as registry } from './definedTypeRegistry.js';
import { INSTRUCTIONS } from './generated/index.js';

function roleFor(writable: boolean, signer: boolean): AccountRole {
  if (writable && signer) return AccountRole.WRITABLE_SIGNER;
  if (writable) return AccountRole.WRITABLE;
  if (signer) return AccountRole.READONLY_SIGNER;
  return AccountRole.READONLY;
}

/**
 * Anchor's `Option<AccountInfo>` "no account" sentinel is the invoking PROGRAM's own id -- the
 * classic Anchor optional-account convention, cross-checked directly against the installed
 * `anchor-syn` source during Phase 8 (`bots/liquidator/src/txBuilders.ts`'s
 * `noneAccountSentinel`). Reused verbatim here as the one place this SDK encodes that fact.
 */
function noneAccountSentinel(programId: Address): Address {
  return programId;
}

export function buildGenericInstruction(
  programId: Address,
  ixName: string,
  accounts: Record<string, Address | undefined>,
  args: Record<string, unknown>,
  remainingAccounts: { address: Address; writable: boolean; signer?: boolean }[] = [],
): Instruction {
  const def = INSTRUCTIONS[ixName];
  if (!def) throw new Error(`unknown Aegis instruction: ${ixName}`);

  const data = aegisCoder().instruction.encode(ixName, encodeFields(def.argsFields, args, registry));

  const metas = def.accounts.map((a) => {
    const address = accounts[a.camelName];
    if (address === undefined) {
      if (!a.optional) {
        throw new Error(`missing required account "${a.camelName}" for instruction "${ixName}"`);
      }
      return { address: noneAccountSentinel(programId), role: AccountRole.READONLY };
    }
    return { address, role: roleFor(a.writable, a.signer) };
  });

  for (const ra of remainingAccounts) {
    metas.push({
      address: ra.address,
      role: roleFor(ra.writable, ra.signer ?? false),
    });
  }

  return { programAddress: programId, accounts: metas, data: new Uint8Array(data) };
}

/** Decodes an instruction's own args back to camelCase (rarely needed by a client, but useful for
 *  tests/tooling that want to assert what a builder actually produced). */
export function decodeInstructionArgs(ixName: string, raw: Record<string, unknown>): Record<string, unknown> {
  const def = INSTRUCTIONS[ixName];
  if (!def) throw new Error(`unknown Aegis instruction: ${ixName}`);
  return decodeFields(def.argsFields, raw, registry);
}
