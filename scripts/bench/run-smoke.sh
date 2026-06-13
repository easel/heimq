#!/usr/bin/env bash
# Smoke benchmark: producer-perf-test + consumer-perf-test against heimq.
# Requirements: Kafka CLI tools on PATH, heimq running on localhost:9094.
# Exit code: 0 on success, non-zero on any protocol/client error line.
#
# Usage: BOOTSTRAP=localhost:9094 ./scripts/bench/run-smoke.sh

set -euo pipefail

BOOTSTRAP="${BOOTSTRAP:-localhost:9094}"
RECORDS=10000
RECORD_SIZE=128
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

run_check() {
    local out
    out="$("$@" 2>&1)"
    echo "$out"
    if echo "$out" | grep -qiE "^.*\[ERROR\]|Exception|FATAL|^FAIL"; then
        echo "FAIL: error lines detected in output of: $*" >&2
        return 1
    fi
}

# ── 1. Non-idempotent smoke ───────────────────────────────────────────────────
TOPIC="bench-smoke-$$"
cleanup1() { kafka-topics.sh --bootstrap-server "$BOOTSTRAP" --delete --topic "$TOPIC" 2>/dev/null || true; }
trap cleanup1 EXIT

echo "==> [1/3] Non-idempotent producer + consumer"
kafka-topics.sh --bootstrap-server "$BOOTSTRAP" --create --topic "$TOPIC" \
    --partitions 1 --replication-factor 1

run_check kafka-producer-perf-test.sh \
    --producer-props bootstrap.servers="$BOOTSTRAP" acks=1 enable.idempotence=false \
    --topic "$TOPIC" \
    --num-records "$RECORDS" \
    --record-size "$RECORD_SIZE" \
    --throughput -1

run_check kafka-consumer-perf-test.sh \
    --bootstrap-server "$BOOTSTRAP" \
    --consumer.config "$SCRIPT_DIR/profiles/consumer-smoke.properties" \
    --topic "$TOPIC" \
    --messages "$RECORDS" \
    --group "bench-consumer-$$"

cleanup1; trap - EXIT

# ── 2. Idempotent producer ────────────────────────────────────────────────────
TOPIC_IDEM="bench-idempotent-$$"
cleanup2() { kafka-topics.sh --bootstrap-server "$BOOTSTRAP" --delete --topic "$TOPIC_IDEM" 2>/dev/null || true; }
trap cleanup2 EXIT

echo "==> [2/3] Idempotent producer (enable.idempotence=true)"
kafka-topics.sh --bootstrap-server "$BOOTSTRAP" --create --topic "$TOPIC_IDEM" \
    --partitions 1 --replication-factor 1

run_check kafka-producer-perf-test.sh \
    --producer.config "$SCRIPT_DIR/profiles/producer-idempotent.properties" \
    --topic "$TOPIC_IDEM" \
    --num-records "$RECORDS" \
    --record-size "$RECORD_SIZE" \
    --throughput -1 \
    --producer-props bootstrap.servers="$BOOTSTRAP"

cleanup2; trap - EXIT

# ── 3. Transactional producer ─────────────────────────────────────────────────
TOPIC_TXN="bench-txn-$$"
cleanup3() { kafka-topics.sh --bootstrap-server "$BOOTSTRAP" --delete --topic "$TOPIC_TXN" 2>/dev/null || true; }
trap cleanup3 EXIT

echo "==> [3/3] Transactional producer (transactional.id)"
kafka-topics.sh --bootstrap-server "$BOOTSTRAP" --create --topic "$TOPIC_TXN" \
    --partitions 1 --replication-factor 1

run_check kafka-producer-perf-test.sh \
    --producer.config "$SCRIPT_DIR/profiles/producer-transactional.properties" \
    --topic "$TOPIC_TXN" \
    --num-records "$RECORDS" \
    --record-size "$RECORD_SIZE" \
    --throughput -1 \
    --transaction-duration-ms 1000 \
    --producer-props bootstrap.servers="$BOOTSTRAP"

cleanup3; trap - EXIT

echo "==> PASS: all three bench profiles completed"
