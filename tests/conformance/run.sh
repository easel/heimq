#!/usr/bin/env bash
# Run the conformance suite. Requires only Docker.
#
#   ./run.sh            # python runner
#   ./run.sh python     # explicit
set -euo pipefail

runner="${1:-python}"
cd "$(dirname "$0")"

cleanup() { docker compose down -v --remove-orphans >/dev/null 2>&1 || true; }
trap cleanup EXIT

docker compose up --build --abort-on-container-exit --exit-code-from "$runner"
