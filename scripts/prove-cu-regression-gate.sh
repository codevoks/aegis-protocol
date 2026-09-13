#!/usr/bin/env bash
# Phase 11 item #32: proves `scripts/check-cu-regression.sh` actually fires on a deliberate CU
# regression, then restores the real code and proves the gate passes again. Never commits the
# regression -- `trap cleanup EXIT` restores `crates/aegis-math/src/u256.rs` from git and rebuilds
# unconditionally, even if this script is interrupted.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

U256_FILE="crates/aegis-math/src/u256.rs"
BACKUP="$(mktemp)"
cp "$U256_FILE" "$BACKUP"

# Restores from this run's own backup (not from git) -- correct regardless of whether OPT-01 is
# already committed, staged, or still a working-tree change, and never touches unrelated files.
cleanup() {
    cp "$BACKUP" "$U256_FILE"
    rm -f "$BACKUP"
    anchor build --ignore-keys >/dev/null 2>&1 || true
}
trap cleanup EXIT

echo "=== Step 1: deliberately regress (disable OPT-01's fast path in $U256_FILE) ==="
python3 - "$U256_FILE" <<'PY'
import sys
path = sys.argv[1]
text = open(path).read()
marker = (
    "        if self.hi == 0 {\n"
    "            return Some((self.lo / d, self.lo % d));\n"
    "        }\n"
)
if marker not in text:
    print("prove-cu-regression-gate: OPT-01 fast-path marker not found -- has u256.rs changed shape?", file=sys.stderr)
    sys.exit(1)
disabled = (
    "        // DELIBERATELY DISABLED by scripts/prove-cu-regression-gate.sh -- never committed.\n"
    "        if false {\n"
    "            return Some((self.lo / d, self.lo % d));\n"
    "        }\n"
)
open(path, "w").write(text.replace(marker, disabled))
PY

echo "=== Step 2: rebuild the on-chain program with the regression ==="
anchor build --ignore-keys

echo "=== Step 3: run check-cu-regression.sh -- this MUST FAIL ==="
if ./scripts/check-cu-regression.sh; then
    echo "prove-cu-regression-gate: FAILED -- the regression gate did NOT fire on a deliberate >10% regression!" >&2
    exit 1
fi
echo "prove-cu-regression-gate: gate correctly FAILED on the deliberate regression (expected, see output above)"

echo "=== Step 4: restore the real code and rebuild ==="
cp "$BACKUP" "$U256_FILE"
anchor build --ignore-keys

echo "=== Step 5: run check-cu-regression.sh again -- this MUST PASS ==="
./scripts/check-cu-regression.sh

echo "prove-cu-regression-gate: OK — gate fired on the deliberate regression and passed after restoration"
