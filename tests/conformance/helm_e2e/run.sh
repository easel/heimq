#!/usr/bin/env bash
# Helm fixed-memory evidence harness against a local heimq. Requires only Docker.
#
#   ./run.sh              # every case, each on a fresh broker
#   ./run.sh scenario-d-backpressure
#
# Mirrors scripts/helm-memory-e2e.sh: every case gets a brand-new broker with
# its own broker-level retention.ms. They cannot share one -- scenario D fills
# memory with retention.ms-protected records that would still be held when the
# next scenario starts.
set -euo pipefail
cd "$(dirname "$0")"
HOST_UID="$(id -u)"; HOST_GID="$(id -g)"
export HOST_UID HOST_GID
mkdir -p ../../../target/helm-memory-e2e

# case-name : scenario : broker_retention_ms : a_case
CASES=(
  "scenario-a-1-topic:A:604800000:A-1-topic"
  "scenario-a-10-topics:A:604800000:A-10-topics"
  "scenario-a-100-topics:A:604800000:A-100-topics"
  "scenario-a2-throughput:A2:604800000:"
  "scenario-b-plateau:B:604800000:"
  "scenario-c-retention-ms:C:604800000:"
  "scenario-d-backpressure:D:1000:"
  "scenario-e-retention-bytes:E:604800000:"
)

cleanup() { docker compose down -v --remove-orphans >/dev/null 2>&1 || true; }
trap cleanup EXIT

status=0
for entry in "${CASES[@]}"; do
  IFS=: read -r name scenario retention a_case <<<"$entry"
  [ $# -gt 0 ] && [ "$1" != "$name" ] && continue

  echo "==> ${name}"
  cleanup
  export HEIMQ_RETENTION_MS="$retention"
  export HEIMQ_E2E_SCENARIO="$scenario"
  export HEIMQ_E2E_A_CASE="$a_case"
  if ! docker compose up --build --abort-on-container-exit --exit-code-from e2e e2e; then
    echo "[FAIL] ${name}"
    status=1
  fi
done

exit "$status"
