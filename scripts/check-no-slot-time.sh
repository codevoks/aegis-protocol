#!/usr/bin/env bash
# CI-NOSLOT — asserts `Clock::slot` / a `.slot` field access is never used for wall-clock
# time (INV-ORA-06); only `unix_timestamp` may be used. Slot times are not constant
# (SIMD-0525), so any slot-based staleness window is unsafe.
#
# Implemented as two plain (non-PCRE) greps chained together, so it runs the same under
# BSD grep (macOS) and GNU grep (CI), rather than relying on -P lookaheads.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

matches=""
if [ -d programs ]; then
    matches=$(grep -rniE 'clock' programs --include='*.rs' 2>/dev/null \
        | grep -iE '\.slot\b' || true)
fi

if [ -n "$matches" ]; then
    echo "check-no-slot-time: found slot-based time usage (INV-ORA-06 violation):" >&2
    echo "$matches" >&2
    exit 1
fi

echo "check-no-slot-time: OK — no Clock.slot usage in programs/"
