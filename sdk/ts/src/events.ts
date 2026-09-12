// Typed event decoding (`docs/phases/phase-09-sdk-ui.md` item 23): the UI/demo can present
// transaction results ("Supplied 900 USDC", "Liquidated 9.97 SOL") without scraping raw logs with
// brittle regexes -- Anchor's own `emit!` mechanism logs each event as a `Program data: <base64>`
// line, which `@anchor-lang/core`'s `BorshCoder.events.decode` parses given that base64 payload;
// this module adds the camelCase field mapping via the same generic `decodeFields` machinery
// `accounts.ts` uses, driven by the generated `EVENT_FIELD_SPECS`.

import { aegisCoder } from './anchorCoder.js';
import { decodeFields } from './borshValues.js';
import { definedTypeRegistry as registry } from './definedTypeRegistry.js';
import { EVENT_FIELD_SPECS } from './generated/index.js';

export interface DecodedEvent {
  name: string;
  data: Record<string, unknown>;
}

const PROGRAM_DATA_PREFIX = 'Program data: ';

/** Decodes every Aegis event found in a transaction's log lines, in order. Non-Aegis / malformed
 *  `Program data:` lines (e.g. from a different program's CPI in the same transaction) are skipped
 *  rather than thrown on, since a transaction can legitimately contain logs this SDK does not own. */
export function decodeEventsFromLogs(logs: readonly string[]): DecodedEvent[] {
  const coder = aegisCoder();
  const out: DecodedEvent[] = [];
  for (const line of logs) {
    if (!line.startsWith(PROGRAM_DATA_PREFIX)) continue;
    const base64 = line.slice(PROGRAM_DATA_PREFIX.length).trim();
    let decoded: { name: string; data: Record<string, unknown> } | null;
    try {
      decoded = coder.events.decode(base64) as typeof decoded;
    } catch {
      continue;
    }
    if (!decoded) continue;
    const fields = EVENT_FIELD_SPECS[decoded.name];
    if (!fields) continue; // decoded to some other program's event shape by coincidence
    out.push({ name: decoded.name, data: decodeFields(fields, decoded.data, registry) });
  }
  return out;
}

/** Convenience: the first decoded event of a given name, or `undefined`. Most Aegis instructions
 *  emit exactly one event, so this is the common case UI code wants. */
export function findEvent<T = Record<string, unknown>>(
  logs: readonly string[],
  name: string,
): T | undefined {
  const found = decodeEventsFromLogs(logs).find((e) => e.name === name);
  return found?.data as T | undefined;
}
