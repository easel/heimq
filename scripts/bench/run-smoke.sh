#!/usr/bin/env bash
# Smoke benchmark: producer-perf-test + consumer-perf-test against heimq.
# Requirements: Kafka CLI tools on PATH, heimq running on localhost:9094.
# Exit code: 0 on success, non-zero on any protocol/client error line.
#
# Usage: BOOTSTRAP=localhost:9094 ./scripts/bench/run-smoke.sh

set -euo pipefail

BOOTSTRAP="${BOOTSTRAP:-localhost:9094}"
TOPIC="bench-smoke-$$"
RECORDS=10000
RECORD_SIZE=128

cleanup() {
    kafka-topics.sh --bootstrap-server "$BOOTSTRAP" --delete --topic "$TOPIC" 2>/dev/null || true
}
trap cleanup EXIT

echo "==> Creating topic $TOPIC"
kafka-topics.sh --bootstrap-server "$BOOTSTRAP" --create --topic "$TOPIC" \
    --partitions 1 --replication-factor 1

echo "==> Producer perf test"
producer_out=$(kafka-producer-perf-test.sh \
    --producer-props bootstrap.servers="$BOOTSTRAP" acks=1 enable.idempotence=false \
    --topic "$TOPIC" \
    --num-records "$RECORDS" \
    --record-size "$RECORD_SIZE" \
    --throughput -1 2>&1)
echo "$producer_out"
if echo "$producer_out" | grep -qiE "error|exception|failed"; then
    echo "FAIL: producer error lines detected" >&2
    exit 1
fi

echo "==> Consumer perf test"
consumer_out=$(kafka-consumer-perf-test.sh \
    --bootstrap-server "$BOOTSTRAP" \
    --consumer.config scripts/bench/profiles/consumer-smoke.properties \
    --topic "$TOPIC" \
    --messages "$RECORDS" \
    --group "bench-consumer-$$" 2>&1)
echo "$consumer_out"
if echo "$consumer_out" | grep -qiE "error|exception|failed"; then
    echo "FAIL: consumer error lines detected" >&2
    exit 1
fi

echo "==> PASS: producer + consumer smoke bench completed"
