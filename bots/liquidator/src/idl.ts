// Loads the real, `anchor build`-generated IDL (target/idl/aegis.json) -- never a hand-maintained
// copy, so a change to the program's accounts/instructions is caught by re-running `anchor build`
// rather than silently drifting (the same principle ADR-0011 states for aegis-math re-implementations).

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { BorshCoder, type Idl } from './anchorCore.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

/** Repo-relative path to the real, generated IDL -- run `make build` (or `anchor build`) first. */
export const AEGIS_IDL_PATH = path.resolve(__dirname, '../../../target/idl/aegis.json');

export function loadAegisIdl(): Idl {
  let raw: string;
  try {
    raw = readFileSync(AEGIS_IDL_PATH, 'utf8');
  } catch (err) {
    throw new Error(
      `Could not read ${AEGIS_IDL_PATH} -- run \`make build\` (or \`anchor build\`) first so the ` +
        `real IDL exists. (${(err as Error).message})`,
    );
  }
  return JSON.parse(raw) as Idl;
}

export function loadAegisCoder(): BorshCoder {
  return new BorshCoder(loadAegisIdl());
}
