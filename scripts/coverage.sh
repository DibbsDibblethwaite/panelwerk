#!/usr/bin/env bash
# Test coverage gate (CLAUDE.md rule 9): 100 % of lines, functions and
# regions in both crates. Exclusions, each agreed with Ian and listed here
# with its reason: none.
#
# Add --html for a browsable report in target/llvm-cov/html.
#
# Speed, as on DSPWerkbank, Kör and Rödalog: the tests run under
# cargo-nextest in one pool across the cores, and the coverage build is at
# opt-level 1 (this script only; debug assertions and overflow checks stay
# on). Needs `cargo install --locked cargo-nextest cargo-llvm-cov`.
set -euo pipefail
cd "$(dirname "$0")/.."
export CARGO_PROFILE_DEV_OPT_LEVEL=1
# Each checkout keeps its own coverage build: cargo-llvm-cov clears the
# profiles in its directory before a run, so two runs sharing one would
# spoil each other's counts.
export CARGO_LLVM_COV_TARGET_DIR="$PWD/target/llvm-cov-target"
# The hook passes --quiet, which nextest doesn't take; the hook discards the
# output anyway, so it's dropped here.
args=()
for a in "$@"; do [[ "$a" == --quiet ]] || args+=("$a"); done
# The tests, then the report: a workspace with no library code yet (the
# crates are empty until docs/proposal.md's step 1) has no regions at all,
# which llvm-cov's gate counts as a failure, so that case passes, said.
cargo llvm-cov nextest --workspace --no-tests=pass --no-report
regions=$(cargo llvm-cov report --json --summary-only | grep -oE '"regions":\{"count":[0-9]+' | tail -1 | grep -oE '[0-9]+$')
if [[ "$regions" == 0 ]]; then
    echo "coverage: no library code yet, nothing to cover"
    exit 0
fi
cargo llvm-cov report --fail-under-lines 100 --fail-under-functions 100 --fail-under-regions 100 "${args[@]}"
