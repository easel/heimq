#!/usr/bin/env bash
# Run the conformance suite. Requires only Docker.
#
#   ./run.sh            # both runners, sequentially
#   ./run.sh python     # confluent-kafka (wraps librdkafka)
#   ./run.sh go         # franz-go (independent Go implementation)
#
# Each runner gets fresh brokers. They cannot share a cluster: the workloads use
# fixed topic and consumer-group names, so a second run against the same brokers
# would find committed offsets already at the end and consume nothing.
set -euo pipefail

cd "$(dirname "$0")"

# Runners write JSONL into ../../target/conformance via a bind mount; run them
# as the invoking user so the output is not root-owned.
HOST_UID="$(id -u)"; HOST_GID="$(id -g)"
export HOST_UID HOST_GID
mkdir -p ../../target/conformance

runners=("$@")
if [ ${#runners[@]} -eq 0 ]; then
  runners=(python go)
fi

cleanup() { docker compose down -v --remove-orphans >/dev/null 2>&1 || true; }
trap cleanup EXIT

status=0
for runner in "${runners[@]}"; do
  echo "═══ conformance runner: ${runner} ═══"
  cleanup
  if ! docker compose up --build --abort-on-container-exit \
      --exit-code-from "$runner" "$runner"; then
    status=1
  fi
done

exit "$status"
