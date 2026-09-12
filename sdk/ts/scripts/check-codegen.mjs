#!/usr/bin/env node
// Stale-generated-code guard (`docs/phases/phase-09-sdk-ui.md` item 3): regenerates
// `sdk/ts/src/generated/` into a temp directory from the CURRENT `target/idl/aegis.json` and diffs
// it against the committed directory. A clean generated diff is deterministic; if `anchor build`
// changed the on-chain IDL and nobody re-ran `npm run codegen`, this fails loudly instead of
// silently drifting. Never rewrites the committed directory itself (a pure check must not silently
// "fix" staleness).

import { execFileSync } from 'node:child_process';
import { mkdtempSync, rmSync, readdirSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const GENERATED_DIR = path.resolve(__dirname, '../src/generated');
const CODEGEN_SCRIPT = path.join(__dirname, 'codegen.mjs');

const tmpDir = mkdtempSync(path.join(tmpdir(), 'aegis-sdk-codegen-'));

try {
  execFileSync(process.execPath, [CODEGEN_SCRIPT, tmpDir], { stdio: 'inherit' });

  const committedFiles = new Set(readdirSync(GENERATED_DIR));
  const freshFiles = new Set(readdirSync(tmpDir));
  let stale = false;

  for (const f of new Set([...committedFiles, ...freshFiles])) {
    if (!committedFiles.has(f)) {
      console.error(`check-codegen: STALE -- ${f} exists in fresh codegen but not committed`);
      stale = true;
      continue;
    }
    if (!freshFiles.has(f)) {
      console.error(`check-codegen: STALE -- ${f} is committed but codegen no longer produces it`);
      stale = true;
      continue;
    }
    const committed = readFileSync(path.join(GENERATED_DIR, f), 'utf8');
    const fresh = readFileSync(path.join(tmpDir, f), 'utf8');
    if (committed !== fresh) {
      console.error(`check-codegen: STALE -- ${f} differs from current \`target/idl/aegis.json\``);
      stale = true;
    }
  }

  if (stale) {
    console.error('\nRun `npm run codegen` (sdk/ts/) and commit the result.');
    process.exit(1);
  }
  console.log('check-codegen: OK -- sdk/ts/src/generated/ matches the current IDL exactly');
} finally {
  rmSync(tmpDir, { recursive: true, force: true });
}
