#!/usr/bin/env bash
# Phase 11 CI check — compares a fresh CU measurement (via the real Mollusk benchmark harness,
# `tests/bench.rs::cu_benchmark_suite`) against the committed `benchmarks/cu.json` baseline.
#
# Rule (docs/phases/phase-11-performance.md #30, docs/performance-strategy.md §5): a regression of
# MORE THAN 10% on ANY benchmarked instruction/scenario fails the build. "More than" is exact: a
# scenario that regressed by precisely +10.00% PASSES; anything strictly above +10% FAILS. This is
# tested directly (not just implemented) by `scripts/prove-cu-regression-gate.sh`, which
# deliberately regresses a scenario, shows this script failing, restores the code, and shows it
# passing again (docs/phases/phase-11-performance.md #32).
#
# Never overwrites the committed baseline: this script only READS benchmarks/cu.json and writes
# its fresh measurement to a scratch temp file (via `AEGIS_BENCH_OUTPUT`), so a regression cannot
# "fix itself" by silently regenerating the baseline (item #33's "baseline update discipline").
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

BASELINE="benchmarks/cu.json"
if [ ! -f "$BASELINE" ]; then
    echo "check-cu-regression: $BASELINE not found -- run 'make bench' first to establish a baseline" >&2
    exit 1
fi

SCRATCH="$(mktemp)"
RUN_LOG="$(mktemp)"
trap 'rm -f "$SCRATCH" "$RUN_LOG"' EXIT

echo "check-cu-regression: measuring current CU via cu_benchmark_suite..."
if ! AEGIS_BENCH_WRITE=1 AEGIS_BENCH_OUTPUT="$SCRATCH" \
    cargo test --test bench --offline cu_benchmark_suite -- --nocapture >"$RUN_LOG" 2>&1; then
    echo "check-cu-regression: FAILED -- cu_benchmark_suite did not run successfully:" >&2
    tail -n 60 "$RUN_LOG" >&2
    exit 1
fi

if [ ! -s "$SCRATCH" ]; then
    echo "check-cu-regression: FAILED -- cu_benchmark_suite produced no output at $SCRATCH" >&2
    exit 1
fi

failed=0
count=$(jq '.measurements | length' "$BASELINE")

for i in $(seq 0 $((count - 1))); do
    instr=$(jq -r ".measurements[$i].instruction" "$BASELINE")
    scenario=$(jq -r ".measurements[$i].scenario" "$BASELINE")
    token=$(jq -r ".measurements[$i].token_program" "$BASELINE")
    baseline_cu=$(jq -r ".measurements[$i].cu" "$BASELINE")

    current_cu=$(jq -r --arg i "$instr" --arg s "$scenario" --arg t "$token" \
        '[.measurements[] | select(.instruction==$i and .scenario==$s and .token_program==$t) | .cu] | first // empty' \
        "$SCRATCH")

    if [ -z "$current_cu" ]; then
        echo "check-cu-regression: FAILED -- baseline scenario '$instr/$scenario/$token' is missing from the current measurement (renamed, removed, or the harness changed?)" >&2
        failed=1
        continue
    fi

    # current > baseline * 1.10, computed in integer arithmetic (no floating point) as
    # current * 100 > baseline * 110 -- exactly +10.00% is NOT a regression by this rule.
    threshold=$((baseline_cu * 110))
    actual=$((current_cu * 100))

    if [ "$actual" -gt "$threshold" ]; then
        if [ "$baseline_cu" -gt 0 ]; then
            pct=$(((current_cu - baseline_cu) * 100 / baseline_cu))
        else
            pct="inf"
        fi
        echo "check-cu-regression: FAILED -- $instr/$scenario/$token regressed from $baseline_cu to $current_cu CU (+${pct}%, exceeds the 10% threshold)" >&2
        failed=1
    fi
done

if [ "$failed" -ne 0 ]; then
    echo "check-cu-regression: FAILED -- one or more scenarios regressed by more than 10% against the committed $BASELINE baseline" >&2
    exit 1
fi

echo "check-cu-regression: OK — no benchmarked scenario regressed by more than 10% against the committed baseline ($count scenarios checked)"
