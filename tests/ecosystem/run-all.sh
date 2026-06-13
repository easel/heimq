#!/usr/bin/env bash
# Run all 8 ecosystem integration tests against heimq.
#
# Usage:
#   BOOTSTRAP=localhost:9094 ./tests/ecosystem/run-all.sh
#
# Each test script is independent; failures are collected and reported
# at the end. Exit code 0 only if all tests pass.
#
# Set SKIP_TESTS to a space-separated list of script prefixes to skip.
# Example: SKIP_TESTS="07 08" skips Debezium and Flink.

set -uo pipefail

BOOTSTRAP="${BOOTSTRAP:-localhost:9094}"
export BOOTSTRAP

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKIP_TESTS="${SKIP_TESTS:-}"

# Verify heimq is reachable before running any tests
if ! nc -z "$(echo "$BOOTSTRAP" | cut -d: -f1)" "$(echo "$BOOTSTRAP" | cut -d: -f2)" 2>/dev/null; then
    echo "ERROR: heimq not reachable at $BOOTSTRAP — start heimq before running ecosystem tests" >&2
    exit 1
fi

PASS=()
FAIL=()
SKIP=()

for script in "$SCRIPT_DIR"/[0-9][0-9]-*.sh; do
    name=$(basename "$script" .sh)
    prefix="${name%%-*}"

    if echo "$SKIP_TESTS" | grep -qw "$prefix"; then
        echo "SKIP: $name (in SKIP_TESTS)"
        SKIP+=("$name")
        continue
    fi

    echo ""
    echo "────────────────────────────────────────────────────"
    if bash "$script"; then
        PASS+=("$name")
    else
        FAIL+=("$name")
        echo "FAIL: $name exited non-zero" >&2
    fi
done

echo ""
echo "════════════════════════════════════════════════════"
echo "Ecosystem test results:"
echo "  PASS (${#PASS[@]}): ${PASS[*]:-none}"
echo "  FAIL (${#FAIL[@]}): ${FAIL[*]:-none}"
echo "  SKIP (${#SKIP[@]}): ${SKIP[*]:-none}"
echo "════════════════════════════════════════════════════"

[ "${#FAIL[@]}" -eq 0 ]
