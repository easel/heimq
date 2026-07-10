#!/usr/bin/env bash
# Run the independent client oracles against a containerized heimq.
#
#   ./run.sh                    # every oracle
#   ./run.sh franz_go kcat      # named oracles
#
# Each oracle runs against a fresh heimq: they create topics with per-run names
# but share a broker otherwise, and a stale consumer group would mask a failure.
set -euo pipefail
cd "$(dirname "$0")"

oracles=("$@")
if [ ${#oracles[@]} -eq 0 ]; then
  oracles=(franz_go sarama kafkajs kcat)
fi

cleanup() { docker compose down -v --remove-orphans >/dev/null 2>&1 || true; }
trap cleanup EXIT

status=0
for oracle in "${oracles[@]}"; do
  echo "═══ compat oracle: ${oracle} ═══"
  cleanup
  if docker compose up --build --abort-on-container-exit \
      --exit-code-from "$oracle" "$oracle"; then
    echo "[PASS] ${oracle}"
  else
    echo "[FAIL] ${oracle}"
    status=1
  fi
done

exit "$status"
