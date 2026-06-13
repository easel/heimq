#!/usr/bin/env bash
# Kafka perf-test bench harness for heimq.
# Runs kafka-producer-perf-test and kafka-consumer-perf-test via Docker
# against a locally-launched heimq instance (no JVM required on the host).
#
# Usage:
#   ./kafka-perf-test.sh              # run both producer and consumer
#   ./kafka-perf-test.sh --producer   # producer only
#   ./kafka-perf-test.sh --consumer   # consumer only (requires prior producer run)
#   ./kafka-perf-test.sh --profile <path>  # override profile (default: profiles/producer-legacy.conf)
#
# Environment overrides:
#   HEIMQ_BINARY   path to heimq binary (default: build from source)
#   HEIMQ_PORT     broker port (default: 19092, chosen to avoid conflicts)
#   KAFKA_IMAGE    Docker image with kafka tools (default: confluentinc/cp-kafka:7.6.1)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

KAFKA_IMAGE="${KAFKA_IMAGE:-confluentinc/cp-kafka:7.6.1}"
HEIMQ_PORT="${HEIMQ_PORT:-19092}"
BROKER="127.0.0.1:${HEIMQ_PORT}"
LOG_DIR="${TMPDIR:-/tmp}/heimq-bench-$$"

FAILED=0
pass() { echo "  PASS: $1"; }
fail() { echo "  FAIL: $1"; FAILED=1; }

# Parse args
RUN_PRODUCER=1
RUN_CONSUMER=1
PROFILE_OVERRIDE=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --producer) RUN_CONSUMER=0; shift ;;
    --consumer) RUN_PRODUCER=0; shift ;;
    --profile)  PROFILE_OVERRIDE="$2"; shift 2 ;;
    *) echo "unknown arg: $1"; exit 1 ;;
  esac
done

mkdir -p "$LOG_DIR"

# ── docker check ────────────────────────────────────────────────────────────

command -v docker >/dev/null || { echo "error: docker not found on PATH"; exit 1; }

# ── build or locate heimq ───────────────────────────────────────────────────

if [[ -n "${HEIMQ_BINARY:-}" ]]; then
  HEIMQ="$HEIMQ_BINARY"
else
  echo "==> building heimq (release)"
  ( cd "$REPO_DIR" && cargo build --release --quiet )
  HEIMQ="$REPO_DIR/target/release/heimq"
fi
[[ -x "$HEIMQ" ]] || { echo "error: heimq binary not found or not executable: $HEIMQ"; exit 1; }

# ── start heimq ─────────────────────────────────────────────────────────────

# shellcheck disable=SC2329  # invoked via trap EXIT below
cleanup() {
  echo "==> stopping heimq (pid ${HEIMQ_PID:-<unset>})"
  kill "${HEIMQ_PID:-}" 2>/dev/null || true
  wait "${HEIMQ_PID:-}" 2>/dev/null || true
  if [[ "$FAILED" = "0" ]]; then
    rm -rf "$LOG_DIR"
  else
    echo "    logs preserved at $LOG_DIR"
  fi
}

echo "==> starting heimq on $BROKER"
# --data-dir is forward-defensive: heimq currently uses memory:// storage for all
# backends and ignores data_dir at runtime. If a disk-backed backend is added
# later, this flag ensures bench runs stay isolated per invocation.
"$HEIMQ" --host 127.0.0.1 --port "$HEIMQ_PORT" --default-partitions 3 \
  --data-dir "$LOG_DIR/data" \
  >"$LOG_DIR/heimq.log" 2>&1 &
HEIMQ_PID=$!
trap cleanup EXIT

# Wait for heimq to accept connections (up to 15s)
echo "==> waiting for heimq to be ready"
for _ in $(seq 1 75); do
  if docker run --rm --network host "$KAFKA_IMAGE" \
       kafka-topics --bootstrap-server "$BROKER" --list >/dev/null 2>&1; then
    break
  fi
  sleep 0.2
done
# Final check
if ! docker run --rm --network host "$KAFKA_IMAGE" \
     kafka-topics --bootstrap-server "$BROKER" --list >/dev/null 2>&1; then
  echo "error: heimq not reachable on $BROKER after 15s"
  cat "$LOG_DIR/heimq.log"
  exit 1
fi
echo "  heimq is ready"

# ── helper: run kafka tool via docker, capture output, check for errors ─────

run_kafka_tool() {
  local label="$1"; shift
  local out_log="$LOG_DIR/${label}.out"
  local err_log="$LOG_DIR/${label}.err"

  echo "  running: $*"
  # --network host so the container can reach 127.0.0.1:$HEIMQ_PORT
  if ! docker run --rm --network host "$KAFKA_IMAGE" "$@" \
       >"$out_log" 2>"$err_log"; then
    fail "$label: tool exited non-zero"
    echo "    stdout: $(tail -5 "$out_log")"
    echo "    stderr: $(tail -5 "$err_log")"
    return 1
  fi

  # Check for error lines in combined output
  local error_lines
  error_lines=$(grep -iE '^\s*(error|exception|WARN.*error)' "$out_log" "$err_log" 2>/dev/null || true)
  if [[ -n "$error_lines" ]]; then
    fail "$label: error lines found in output"
    echo "$error_lines" | head -10 | sed 's/^/    /'
    return 1
  fi

  pass "$label: exit 0, no error lines"
}

# ── producer perf-test ───────────────────────────────────────────────────────

if [[ "$RUN_PRODUCER" = "1" ]]; then
  PROD_CONF="${PROFILE_OVERRIDE:-$SCRIPT_DIR/profiles/producer-legacy.conf}"
  # shellcheck source=/dev/null
  source "$PROD_CONF"

  echo
  echo "==> [producer] profile: $PROD_CONF"
  echo "    records=$RECORDS record-size=$RECORD_SIZE throughput=$THROUGHPUT acks=$ACKS topic=$TOPIC"

  run_kafka_tool "producer-perf-test" \
    kafka-producer-perf-test \
      --topic "$TOPIC" \
      --num-records "$RECORDS" \
      --record-size "$RECORD_SIZE" \
      --throughput "$THROUGHPUT" \
      --producer-props \
        bootstrap.servers="$BROKER" \
        acks="$ACKS" || true
fi

# ── consumer perf-test ───────────────────────────────────────────────────────

if [[ "$RUN_CONSUMER" = "1" ]]; then
  CONS_CONF="${PROFILE_OVERRIDE:-$SCRIPT_DIR/profiles/consumer-legacy.conf}"
  # shellcheck source=/dev/null
  source "$CONS_CONF"

  echo
  echo "==> [consumer] profile: $CONS_CONF"
  echo "    records=$RECORDS topic=$TOPIC group=$GROUP_ID threads=$THREADS"

  run_kafka_tool "consumer-perf-test" \
    kafka-consumer-perf-test \
      --topic "$TOPIC" \
      --messages "$RECORDS" \
      --group "$GROUP_ID" \
      --threads "$THREADS" \
      --fetch-size "$FETCH_SIZE" \
      --bootstrap-server "$BROKER" || true
fi

# ── summary ─────────────────────────────────────────────────────────────────

echo
if [[ "$FAILED" = "0" ]]; then
  echo "==> ALL CHECKS PASSED"
  exit 0
else
  echo "==> ONE OR MORE CHECKS FAILED"
  exit 1
fi
