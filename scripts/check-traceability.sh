#!/usr/bin/env bash
# CI-TRACE — parses docs/invariants.md and asserts that every test ID it cites, for every
# invariant already assigned to a phase this repository has completed, actually exists somewhere
# in the source tree (`docs/testing-strategy.md` §7: "A missing test fails the build";
# `docs/phases/phase-10-security.md` #35/#36). This turns the 87-invariant catalogue from
# documentation into a build-enforced contract, and does so by parsing the frozen document
# directly rather than maintaining a second, hand-written list that could drift from it
# independently (#35: "Do not maintain a separate hand-written list").
#
# Rows assigned to a phase this repository has not reached yet are skipped ON PURPOSE --
# `docs/project-status.md` states plainly that "0 of the 87 numbered invariants assigned to Phases
# 10-13 are implemented or tested yet" at various points in this repository's history, and that is
# expected, not a defect. Bump MAX_COMPLETED_PHASE as later phases land; do not remove the guard.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

MAX_COMPLETED_PHASE=12
DOC="docs/invariants.md"
# Deliberately narrow, real source paths only -- never the whole `sdk/`/`bots/` trees, whose
# `node_modules` alone are 10000+ files that would make this script slow for no benefit (no
# A-/U-/P-/I-/F- test ID in this scheme is ever defined in a dependency).
SEARCH_DIRS=(tests crates programs scripts labs sdk/ts/src sdk/ts/test bots/liquidator/src)

missing=0
checked=0
missing_list=""

# "A-ORACLE-03..11" -> A-ORACLE-03 A-ORACLE-04 ... A-ORACLE-11, zero-padded to the start's width.
# Any other token is echoed back unchanged.
expand_range() {
    local token="$1"
    if [[ "$token" =~ ^(.+-)([0-9]+)\.\.([0-9]+)$ ]]; then
        local prefix="${BASH_REMATCH[1]}"
        local start="${BASH_REMATCH[2]}"
        local end="${BASH_REMATCH[3]}"
        local width=${#start}
        local n
        for ((n = 10#$start; n <= 10#$end; n++)); do
            printf "%s%0${width}d\n" "$prefix" "$n"
        done
    else
        echo "$token"
    fi
}

while IFS= read -r line; do
    # Only rows of the per-invariant tables: "| INV-XXX | ... | ... | <Test col> | <Phase col> |"
    # (the coverage-summary table at the end of the doc has rows like "| A. Authorization | 7 | – |"
    # which do not start with INV- and are correctly excluded).
    [[ "$line" =~ ^\|[[:space:]]*(\*\*)?INV- ]] || continue

    IFS='|' read -ra cols <<< "$line"
    # cols[0] is the empty field before the leading "|"; cols[1]=ID, [2]=Invariant, [3]=Impl,
    # [4]=Test, [5]=Phase.
    test_col="${cols[4]:-}"
    phase_col="${cols[5]:-}"
    phase="$(echo "$phase_col" | tr -dc '0-9')"
    [[ -n "$phase" ]] || continue
    if (( phase > MAX_COMPLETED_PHASE )); then
        continue
    fi

    while [[ "$test_col" == *'`'*'`'* ]]; do
        rest="${test_col#*\`}"
        token="${rest%%\`*}"
        test_col="${rest#"$token"}"
        test_col="${test_col#\`}"
        [[ "$token" == *'*'* ]] && continue # wildcard IDs -- none currently cited at phase <= 10
        while IFS= read -r id; do
            [[ -z "$id" ]] && continue
            checked=$((checked + 1))
            # `command grep`, and options BEFORE the `--`/operands: BSD grep (macOS's default,
            # and whatever a wrapped `grep` shell function might shell out to) treats options
            # appearing after the file operands as filenames, not flags.
            if ! command grep -rlF --include='*.rs' --include='*.sh' --include='*.ts' \
                --exclude-dir=node_modules --exclude-dir=target --exclude-dir=.next \
                -- "$id" "${SEARCH_DIRS[@]}" >/dev/null 2>&1
            then
                missing=$((missing + 1))
                missing_list="${missing_list}  - ${id} (docs/invariants.md, phase ${phase})"$'\n'
            fi
        done < <(expand_range "$token")
    done
done < "$DOC"

if (( missing > 0 )); then
    echo "check-traceability: $missing / $checked cited test id(s) not found in the source tree:" >&2
    printf '%s' "$missing_list" >&2
    exit 1
fi

echo "check-traceability: OK — $checked test id(s) referenced by docs/invariants.md (phase <= $MAX_COMPLETED_PHASE) all exist"
