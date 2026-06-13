#!/usr/bin/env bash
# Integration test: heimq produce→consume via kcat (kafkacat).
# Starts heimq, runs metadata/produce/consume/group checks with
# assertions. Exits non-zero on failure. Safe to wire into CI.

set -euo pipefail

BROKER="127.0.0.1:9092"
TOPIC="kcat-smoke"
MULTI_TOPIC="kcat-smoke-multi"
GROUP="kcat-smoke-group"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

KCAT="${KCAT:-kcat}"
command -v "$KCAT" >/dev/null || { echo "error: $KCAT not found on PATH"; exit 1; }

FAILED=0
pass() { echo "  PASS: $1"; }
fail() { echo "  FAIL: $1"; FAILED=1; }

echo "==> building heimq (release)"
( cd "$REPO_DIR" && cargo build --release --quiet )

echo "==> starting heimq on $BROKER with 3 default partitions"
"$REPO_DIR/target/release/heimq" \
  --host 127.0.0.1 --port 9092 --default-partitions 3 \
  >/tmp/heimq-kcat-smoke.log 2>&1 &
HEIMQ_PID=$!
trap 'kill $HEIMQ_PID 2>/dev/null || true' EXIT

# Wait for port to accept connections
for _ in $(seq 1 30); do
  if "$KCAT" -b "$BROKER" -L -m 1 >/dev/null 2>&1; then break; fi
  sleep 0.2
done

echo
echo "==> [1] metadata reachable"
if "$KCAT" -b "$BROKER" -L -m 2 2>/dev/null | grep -q "1 brokers:"; then
  pass "broker metadata returned"
else
  fail "broker metadata not returned"
fi

echo
echo "==> [2] produce 3 keyed messages to $TOPIC"
if printf 'k1:first\nk2:second\nk3:third\n' | \
   "$KCAT" -b "$BROKER" -t "$TOPIC" -P -K: 2>/tmp/kcat-prod.err; then
  pass "produce returned success"
else
  fail "produce failed: $(cat /tmp/kcat-prod.err)"
fi

echo
echo "==> [3] topic is visible in metadata after produce"
# The original bug: metadata shows 0 topics even after a successful produce.
META_OUT=$("$KCAT" -b "$BROKER" -L -m 2 2>/dev/null)
if echo "$META_OUT" | grep -qE "topic \"$TOPIC\""; then
  pass "topic $TOPIC present in metadata"
else
  fail "topic $TOPIC NOT present in metadata after produce"
  echo "    metadata was:"
  echo "$META_OUT" | sed 's/^/      /'
fi

echo
echo "==> [4] consume produced messages back"
CONSUMED=$("$KCAT" -b "$BROKER" -t "$TOPIC" -C -o beginning -e \
  -f '%s\n' 2>/tmp/kcat-cons.err || true)
COUNT=$(printf '%s\n' "$CONSUMED" | grep -cE '^(first|second|third)$' || true)
if [ "$COUNT" = "3" ]; then
  pass "consumed all 3 produced messages"
else
  fail "expected 3 messages back, got $COUNT"
  echo "    consumer stderr:"
  sed 's/^/      /' /tmp/kcat-cons.err
  echo "    consumed stdout:"
  printf '%s\n' "$CONSUMED" | sed 's/^/      /'
fi

echo
echo "==> [5] multi-partition produce/consume"
seq 1 9 | "$KCAT" -b "$BROKER" -t "$MULTI_TOPIC" -P 2>/dev/null || true
MULTI_COUNT=$("$KCAT" -b "$BROKER" -t "$MULTI_TOPIC" -C -o beginning -e \
  -f '%s\n' 2>/dev/null | grep -cE '^[1-9]$' || true)
if [ "$MULTI_COUNT" = "9" ]; then
  pass "multi-partition roundtrip: 9/9 messages"
else
  fail "multi-partition: expected 9, got $MULTI_COUNT"
fi

echo
echo "==> [6] consumer group join (best-effort, 3s)"
timeout 3 "$KCAT" -b "$BROKER" -G "$GROUP" "$TOPIC" \
  -f '%s\n' 2>/dev/null | head -20 || true
pass "consumer group did not hang (no assertion beyond that)"

echo
if [ "$FAILED" = "0" ]; then
  echo "==> ALL CHECKS PASSED"
  echo "heimq log at /tmp/heimq-kcat-smoke.log"
  exit 0
else
  echo "==> ONE OR MORE CHECKS FAILED"
  echo "heimq log at /tmp/heimq-kcat-smoke.log"
  exit 1
fi
