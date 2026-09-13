#!/usr/bin/env bash
# CI-NOLOOP (INV-RES-04, Phase 11) — asserts every `for`/`while`/`loop` in production code under
# `programs/aegis/src` is one of the small, explicitly reviewed, structurally-bounded loops below.
# A NEW loop appearing anywhere else fails the build: it must be reviewed for boundedness and
# either rejected or added to `ALLOWED` with a one-line justification -- the same discipline
# `check-cpi-allowlist.sh` already applies to CPI targets.
#
# "Unbounded" here means "scales with attacker-controlled or unboundedly-growing state" (T-27):
# a loop over on-chain state that grows without limit, or over an instruction-data-encoded count
# with no protocol-enforced ceiling. The two production loops that exist today are NOT that:
#
#   programs/aegis/src/token/policy.rs   -- iterates a mint's OWN parsed Token-2022 extension
#                                            list at `create_market` (admin-only, one-time); bound
#                                            is Token-2022's own account-size ceiling on how many
#                                            TLV extensions a mint can carry at all, not anything
#                                            Aegis lets a caller inflate.
#   programs/aegis/src/instructions/liquidate/liquidate.rs -- iterates the liquidator's own
#                                            `remaining_accounts` in the OPTIONAL callback branch;
#                                            bound is Solana's own per-transaction account-count
#                                            ceiling (the caller pays for and constructs their own
#                                            transaction -- this cannot be used to burn compute
#                                            against anyone else's resources).
#
# `crates/aegis-math/src/u256.rs`'s division loop is a FIXED 256 iterations (`2 * LIMB_BITS`, a
# compile-time constant) regardless of input -- O(1) in every sense INV-RES-04 cares about, and is
# not even reachable from `programs/aegis/src`'s own file glob below, but is called out here for
# completeness since it is the one loop in the hot path most likely to be mistaken for unbounded.
#
# Test-module exclusion: everything from a `#[cfg(test)] mod ... {` line to end of file is skipped.
# This codebase's own consistent convention (verified across every file in `programs/aegis/src`
# and `crates/aegis-math/src`) is exactly one such test module, always the LAST item in the file --
# "from that line to EOF" is precise here, and far more robust than a brace-depth counter, which a
# stray `{`/`}` inside a string literal or format-macro message (`format!("...{}...")`) can
# silently miscount. If that convention ever changes, this check needs revisiting -- a deliberate,
# documented tradeoff, not an oversight.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

# file:line -> already-reviewed, justified above.
ALLOWED=(
    "programs/aegis/src/token/policy.rs:52"
    "programs/aegis/src/instructions/liquidate/liquidate.rs:185"
)

is_allowed() {
    local candidate="$1"
    for entry in "${ALLOWED[@]}"; do
        [[ "$candidate" == "$entry" ]] && return 0
    done
    return 1
}

scan_file() {
    awk '
        BEGIN { pending = 0; test_start = 0 }
        test_start { next }
        /#\[cfg\(test\)\]/ { pending = 1; next }
        pending && /mod[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*\{/ { test_start = 1; next }
        { pending = 0 }
        $0 ~ /^[ \t]*(for |while |loop \{)/ { print FILENAME ":" FNR ":" $0 }
    ' "$1"
}

fail=0
unreviewed=""

while IFS= read -r -d '' file; do
    while IFS= read -r hit; do
        [[ -z "$hit" ]] && continue
        rest="${hit#*:}"
        line="${rest%%:*}"
        candidate="${file}:${line}"
        if ! is_allowed "$candidate"; then
            unreviewed="${unreviewed}  ${candidate}: ${hit#*:*:}"$'\n'
            fail=1
        fi
    done < <(scan_file "$file")
done < <(find programs/aegis/src -name '*.rs' -print0)

if [ "$fail" -ne 0 ]; then
    echo "check-no-loop: FAILED -- found loop(s) in programs/aegis/src not in the reviewed allowlist:" >&2
    echo "$unreviewed" >&2
    echo "Review each for boundedness (INV-RES-04 / T-27) and add it to ALLOWED in this script with a justification, or remove it." >&2
    exit 1
fi

echo "check-no-loop: OK — every loop in programs/aegis/src is on the reviewed, structurally-bounded allowlist"
