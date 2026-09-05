#!/usr/bin/env bash
# CI-NOFLOAT — asserts no f32/f64 appears anywhere in the on-chain crate or aegis-math
# (INV-ACC-10). Deliberately crude: a grep, not a type-system proof, but cheap and
# unambiguous (docs/testing-strategy.md §9).
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

paths=(programs crates/aegis-math)
matches=""
for path in "${paths[@]}"; do
    if [ -d "$path" ]; then
        found=$(grep -rnE '\bf32\b|\bf64\b' "$path" --include='*.rs' || true)
        if [ -n "$found" ]; then
            matches="${matches}${found}
"
        fi
    fi
done

if [ -n "$matches" ]; then
    echo "check-no-float: found floating-point types (INV-ACC-10 violation):" >&2
    echo "$matches" >&2
    exit 1
fi

echo "check-no-float: OK — no f32/f64 in programs/ or crates/aegis-math/"
