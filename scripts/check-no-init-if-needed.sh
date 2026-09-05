#!/usr/bin/env bash
# CI-NOINITIF — asserts `init_if_needed` (the constraint) and `init-if-needed` (the
# anchor-lang feature) never appear (INV-LIFE-01). Scoped to source and manifests, not
# docs/, which legitimately discusses the ban by name.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

matches=$(grep -rnE 'init_if_needed|init-if-needed' \
    programs crates Cargo.toml \
    --include='*.rs' --include='Cargo.toml' 2>/dev/null || true)

if [ -n "$matches" ]; then
    echo "check-no-init-if-needed: found a banned init_if_needed reference (INV-LIFE-01 violation):" >&2
    echo "$matches" >&2
    exit 1
fi

echo "check-no-init-if-needed: OK — no init_if_needed constraint or feature in use"
