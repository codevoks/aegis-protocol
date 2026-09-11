#!/usr/bin/env bash
# CI-CPI-ALLOWLIST (INV-RES-07 / A-CPI-01) — asserts Aegis CPIs into exactly the program classes
# `docs/composability.md` §3 names: the two token programs (via `token/transfer.rs` and
# `token/vault.rs`'s `CpiContext`-based helpers), the system program (vault creation, also
# `token/vault.rs`), and — Phase 8 only — the one liquidator-supplied `callback_program` inside
# `liquidate`, dispatched with a bare `invoke` (never `invoke_signed`, ADR-0013 §1.4: the callback
# must never receive a Market-PDA-signed CPI). A raw `invoke`/`invoke_signed` call anywhere else in
# `programs/aegis/src`, or a hardcoded external program ID (a DEX, a router, Jupiter) anywhere in
# the program, would mean an unaudited CPI target or a general-purpose CPI facility slipped in —
# both are exactly what this phase's non-scope items (#26, #27) forbid.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

fail=0

# The only file allowed to import the raw, unsigned `invoke` entry point.
raw_invoke_importers=$(grep -rl 'solana_program::program::invoke;' programs/aegis/src --include='*.rs' | sort || true)
expected_importer="programs/aegis/src/instructions/liquidate/liquidate.rs"
if [ "$raw_invoke_importers" != "$expected_importer" ]; then
    echo "check-cpi-allowlist: raw 'invoke' is imported outside the one allowed callback dispatch site:" >&2
    echo "expected: $expected_importer" >&2
    echo "found:" >&2
    echo "$raw_invoke_importers" >&2
    fail=1
fi

# That one raw invoke() call must actually exist (a positive check that it wasn't silently removed
# or replaced with invoke_signed, which would forward the Market PDA's signature to the callback).
if ! grep -q 'invoke(&ix, &account_infos)' "$expected_importer" 2>/dev/null; then
    echo "check-cpi-allowlist: expected raw invoke() callback dispatch not found in $expected_importer" >&2
    fail=1
fi

# invoke_signed must never be called directly by instruction code (the only signer PDA is the
# Market, and every legitimate use of it already goes through token/transfer.rs's
# CpiContext::with_signer, which check-collateral-transfer-paths.sh covers separately). A direct
# invoke_signed call anywhere in programs/aegis/src (including liquidate.rs) would mean some code
# path is signing a CPI outside the two audited helpers -- exactly the "invoke_signed for the
# callback" mistake ADR-0013 rules out by construction.
invoke_signed_callers=$(grep -rl 'invoke_signed(' programs/aegis/src --include='*.rs' || true)
if [ -n "$invoke_signed_callers" ]; then
    echo "check-cpi-allowlist: direct invoke_signed() call found outside the audited CpiContext helpers:" >&2
    echo "$invoke_signed_callers" >&2
    fail=1
fi

# No hardcoded external DEX/router/Jupiter program ID anywhere in the production program -- the
# callback target is always the liquidator-supplied account, never a constant (non-scope #26/#27).
if grep -rli 'jupiter\|JUP6Lkb\|raydium\|orca' programs/aegis/src --include='*.rs' | grep -v '_test' >/tmp/cpi_allowlist_hits 2>/dev/null; then
    if [ -s /tmp/cpi_allowlist_hits ]; then
        echo "check-cpi-allowlist: a specific external DEX/router appears to be hardcoded into the production program:" >&2
        cat /tmp/cpi_allowlist_hits >&2
        fail=1
    fi
fi
rm -f /tmp/cpi_allowlist_hits

if [ "$fail" -ne 0 ]; then
    exit 1
fi

echo "check-cpi-allowlist: OK — the only raw CPI target outside the token/system programs is the one opt-in, unsigned liquidation callback dispatch"
