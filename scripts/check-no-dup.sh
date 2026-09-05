#!/usr/bin/env bash
# CI-NODUP — asserts the Anchor `dup` constraint (opt-in to duplicate mutable accounts,
# T-13) never appears in program source.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

matches=""
if [ -d programs ]; then
    matches=$(grep -rnE '\bdup\b' programs --include='*.rs' || true)
fi

if [ -n "$matches" ]; then
    echo "check-no-dup: found the 'dup' constraint (T-13 violation):" >&2
    echo "$matches" >&2
    exit 1
fi

echo "check-no-dup: OK — no 'dup' constraint in programs/"
