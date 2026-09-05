#!/usr/bin/env bash
# CI check — asserts `overflow-checks = true` is present inside `[profile.release]` in the
# workspace Cargo.toml (T-16). Release builds do not check arithmetic overflow by default;
# without this line, every arithmetic safety argument in docs/economic-model.md is void in
# the deployed artifact.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

MANIFEST="Cargo.toml"

if [ ! -f "$MANIFEST" ]; then
    echo "check-overflow-checks: $MANIFEST not found" >&2
    exit 1
fi

# Extract the [profile.release] table body: everything after the header line, up to the
# next line that starts a new table (a line beginning with '[').
release_block=$(awk '
    /^\[profile\.release\]/ { in_block = 1; next }
    /^\[/ { in_block = 0 }
    in_block { print }
' "$MANIFEST")

if [ -z "$release_block" ]; then
    echo "check-overflow-checks: no [profile.release] table found in $MANIFEST (T-16 violation)" >&2
    exit 1
fi

if ! echo "$release_block" | grep -qE '^\s*overflow-checks\s*=\s*true\s*$'; then
    echo "check-overflow-checks: [profile.release] does not set overflow-checks = true (T-16 violation)" >&2
    exit 1
fi

echo "check-overflow-checks: OK — overflow-checks = true is set in [profile.release]"
