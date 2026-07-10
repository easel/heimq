#!/usr/bin/env bash
# Run the heimq integration suite. Requires only Docker.
#
#   ./run.sh                       # whole suite
#   ./run.sh -k test_legacy        # pytest -k filter
set -euo pipefail
cd "$(dirname "$0")"

cleanup() { docker compose down -v --remove-orphans >/dev/null 2>&1 || true; }
trap cleanup EXIT

if [ $# -gt 0 ]; then
  docker compose run --build --rm pytest "$@"
else
  docker compose up --build --abort-on-container-exit --exit-code-from pytest pytest
fi
