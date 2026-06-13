#!/usr/bin/env bash
# Run the rdkafka compatibility test suite against heimq.
#
# These tests are marked #[ignore] in integration.rs because rdkafka can
# segfault when run under `cargo test` (signal handler conflicts). Running
# each test in its own process avoids the issue.
#
# Usage:
#   ./scripts/compatibility-test.sh [-- <cargo-test-filter>]
#
# Environment:
#   HEIMQ_LOG  — set to 1 to show heimq server logs during tests

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "==> Building heimq (debug)..."
cargo build -p heimq --bin heimq 2>&1

echo "==> Running rdkafka integration tests (one process per test)..."

# Collect all ignored test names that mention compatibility-test.sh
mapfile -t TESTS < <(
    cargo test -p heimq --test integration -- --list --ignored 2>/dev/null \
        | grep ': test$' \
        | sed 's/: test$//' \
        | grep '^test_rdkafka'
)

PASS=0
FAIL=0
SKIP=0
FAILED_NAMES=()

for TEST in "${TESTS[@]}"; do
    # Filter support: if a filter arg was passed after --, skip non-matching tests
    if [[ $# -gt 0 && "${TEST}" != *"${1}"* ]]; then
        SKIP=$(( SKIP + 1 ))
        continue
    fi

    printf "  %-70s" "${TEST}"
    if cargo test -p heimq --test integration -- \
            --ignored --exact "${TEST}" \
            2>/dev/null 1>/dev/null; then
        echo "ok"
        PASS=$(( PASS + 1 ))
    else
        echo "FAILED"
        FAIL=$(( FAIL + 1 ))
        FAILED_NAMES+=("${TEST}")
    fi
done

echo ""
echo "==> Results: ${PASS} passed, ${FAIL} failed, ${SKIP} skipped"

if [[ ${FAIL} -gt 0 ]]; then
    echo ""
    echo "Failed tests:"
    for NAME in "${FAILED_NAMES[@]}"; do
        echo "  - ${NAME}"
    done
    exit 1
fi

echo "==> PASS"
