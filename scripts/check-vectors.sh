#!/usr/bin/env bash
# I-SDK-01 — detects a stale tests/vectors/*.json relative to the current aegis-math
# implementation. Regenerates the vectors into a temp directory via the real Rust generator
# (crates/aegis-test-kit/examples/phase9_vectors_dump.rs) and diffs against the committed files.
# Never overwrites tests/vectors/ itself: a pure check must not silently "fix" staleness
# (docs/phases/phase-09-sdk-ui.md item 11 / AGENTS.md "no silent scope deletion").
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT

cargo run -p aegis-test-kit --example phase9_vectors_dump -- "$tmp_dir" >/dev/null

if ! diff -ru tests/vectors "$tmp_dir" >/tmp/aegis-vectors-diff.txt 2>&1; then
    echo "check-vectors: STALE — tests/vectors/*.json does not match current aegis-math output" >&2
    echo "Run 'make vectors' to regenerate, then commit the result." >&2
    cat /tmp/aegis-vectors-diff.txt >&2
    exit 1
fi

echo "check-vectors: OK — tests/vectors/*.json matches current aegis-math output exactly"
