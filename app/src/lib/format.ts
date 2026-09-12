// Safe token-unit <-> decimal-string conversion (`docs/phases/phase-09-sdk-ui.md` item 41). Never
// uses floating-point multiplication to convert a user-typed decimal amount to base units -- that
// is exactly the class of bug that silently loses or fabricates precision on real token amounts.
// `bigint` end to end; `Number` only ever appears in already-formatted, human-facing strings.

/** Parses a user-typed decimal string (e.g. "12.5") into exact base units for a mint with `decimals`
 *  places. Throws with a specific message on: empty input, a negative sign, more fractional digits
 *  than the mint supports, or non-digit characters -- never silently rounds or truncates. */
export function parseDecimalToBaseUnits(input: string, decimals: number): bigint {
  const trimmed = input.trim();
  if (trimmed === '') throw new Error('Amount is required.');
  if (trimmed.startsWith('-')) throw new Error('Amount cannot be negative.');
  if (!/^\d*\.?\d*$/.test(trimmed) || trimmed === '.') {
    throw new Error('Amount must be a plain decimal number (e.g. 12.5).');
  }
  const [wholeRaw, fracRaw = ''] = trimmed.split('.');
  const whole = wholeRaw === '' ? '0' : wholeRaw;
  if (fracRaw.length > decimals) {
    throw new Error(`Amount has more than ${decimals} decimal place${decimals === 1 ? '' : 's'} for this asset.`);
  }
  const fracPadded = fracRaw.padEnd(decimals, '0');
  const combined = `${whole}${fracPadded}`.replace(/^0+(?=\d)/, '');
  return BigInt(combined === '' ? '0' : combined);
}

/** Formats exact base units back to a human decimal string, e.g. `1_500_000n` at 6 decimals ->
 *  "1.5". Deterministic and exact -- string manipulation on the base-unit digits, never a floating
 *  division. Trailing fractional zeros are trimmed for readability; a whole-number result never
 *  shows a trailing ".". */
export function formatBaseUnitsToDecimal(amount: bigint, decimals: number): string {
  const negative = amount < 0n;
  const abs = negative ? -amount : amount;
  const s = abs.toString().padStart(decimals + 1, '0');
  const whole = s.slice(0, s.length - decimals) || '0';
  const frac = decimals > 0 ? s.slice(s.length - decimals) : '';
  const fracTrimmed = frac.replace(/0+$/, '');
  const out = fracTrimmed ? `${whole}.${fracTrimmed}` : whole;
  return negative ? `-${out}` : out;
}

/** Zero validated against a decimals count -- rejects "0", "0.0", "" alike as a single named check,
 *  since several flows (borrow, withdraw, supply) must reject a zero amount before ever building a
 *  transaction (item 42). */
export function isZeroAmount(input: string): boolean {
  const trimmed = input.trim();
  if (trimmed === '' || trimmed === '.') return true;
  return /^0*\.?0*$/.test(trimmed);
}

/** UI-only percentage formatting (never fed back into a transaction). */
export function formatPercent(value: number, fractionDigits = 2): string {
  if (!Number.isFinite(value)) return '—';
  return `${value.toFixed(fractionDigits)}%`;
}
