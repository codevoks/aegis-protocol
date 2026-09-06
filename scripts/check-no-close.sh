#!/usr/bin/env bash
# CI-NOCLOSE — asserts no `close = ` Anchor constraint ever targets a `Market` or `Protocol`
# account (INV-LIFE-04: neither is ever closable by any instruction — there is no safe moment to
# close a market with outstanding positions, and Protocol is a singleton for the program's whole
# life). `Position` *is* closable (`close_position`, Phase 3) and must not trip this guard.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

violations=""
while IFS=: read -r file line; do
    window=$(sed -n "${line},$((line + 6))p" "$file")
    if echo "$window" | grep -qE "Account<'info,[[:space:]]*(Market|Protocol)>"; then
        violations="${violations}${file}:${line}"$'\n'
    fi
done < <(grep -rnE 'close[[:space:]]*=' programs/aegis/src --include='*.rs' | cut -d: -f1,2)

if [ -n "$violations" ]; then
    echo "check-no-close: found a 'close' constraint on Market or Protocol (INV-LIFE-04 violation):" >&2
    printf '%s' "$violations" >&2
    exit 1
fi

echo "check-no-close: OK — no close constraint targets Market or Protocol"
