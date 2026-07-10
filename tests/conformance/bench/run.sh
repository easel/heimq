#!/usr/bin/env bash
# Throughput benchmark. Requires only Docker.
#
#   ./run.sh                 # heimq alone (was: bench_baseline.rs)
#   ./run.sh compare         # heimq vs kafka vs redpanda (was: parity/bench.rs)
#   BENCH_RECORDS=10000 ./run.sh
set -euo pipefail
cd "$(dirname "$0")"

cleanup() { docker compose --profile compare down -v --remove-orphans >/dev/null 2>&1 || true; }
trap cleanup EXIT

if [ "${1:-}" = "compare" ]; then
  export BENCH_TARGETS="heimq=heimq:9092,kafka=kafka:9092,redpanda=redpanda:9092"
  docker compose --profile compare up -d kafka redpanda
fi

docker compose up --build --abort-on-container-exit --exit-code-from bench bench
