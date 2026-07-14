#!/usr/bin/env bash
# Measure the cold-start budget from broker exec to first successful
# produce/consume roundtrip.
#
# Warm-cache policy:
# - build the broker binary once before timing
# - each measured attempt uses a fresh temp data dir and a fresh broker process
# - the client library is expected to be preinstalled by the caller/CI job
#
# Retry policy:
# - no automatic retries inside an attempt
# - the script performs three consecutive measured attempts by default
# - if a broker readiness deadline or client deadline expires, the script fails
#
# Usage:
#   ./scripts/startup-roundtrip-budget.sh
#
# Environment:
#   HEIMQ_STARTUP_BUDGET_ATTEMPTS=3
#   HEIMQ_STARTUP_BUDGET_MS=1000
#   HEIMQ_STARTUP_BUDGET_PROFILE=release|debug
#   HEIMQ_STARTUP_BUDGET_READY_TIMEOUT_SECONDS=20
#   HEIMQ_STARTUP_BUDGET_CLIENT_TIMEOUT_SECONDS=10
#   HEIMQ_STARTUP_BUDGET_RUST_LOG=error

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROFILE="${HEIMQ_STARTUP_BUDGET_PROFILE:-release}"
ATTEMPTS="${HEIMQ_STARTUP_BUDGET_ATTEMPTS:-3}"
BUDGET_MS="${HEIMQ_STARTUP_BUDGET_MS:-1000}"
READY_TIMEOUT_SECONDS="${HEIMQ_STARTUP_BUDGET_READY_TIMEOUT_SECONDS:-20}"
CLIENT_TIMEOUT_SECONDS="${HEIMQ_STARTUP_BUDGET_CLIENT_TIMEOUT_SECONDS:-10}"
BROKER_HOST="${HEIMQ_STARTUP_BUDGET_HOST:-127.0.0.1}"
BROKER_RUST_LOG="${HEIMQ_STARTUP_BUDGET_RUST_LOG:-error}"

BROKER_PID=""
BROKER_DATA_DIR=""
BROKER_LOG=""

require_tool() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required tool: $1" >&2
    exit 2
  fi
}

cleanup_current() {
  if [[ -n "${BROKER_PID}" ]]; then
    kill "${BROKER_PID}" >/dev/null 2>&1 || true
    for _ in $(seq 1 50); do
      if ! kill -0 "${BROKER_PID}" >/dev/null 2>&1; then
        break
      fi
      sleep 0.1
    done
    if kill -0 "${BROKER_PID}" >/dev/null 2>&1; then
      kill -9 "${BROKER_PID}" >/dev/null 2>&1 || true
    fi
    wait "${BROKER_PID}" >/dev/null 2>&1 || true
    BROKER_PID=""
  fi
  if [[ -n "${BROKER_DATA_DIR}" ]]; then
    rm -rf "${BROKER_DATA_DIR}"
    BROKER_DATA_DIR=""
  fi
  BROKER_LOG=""
}

trap cleanup_current EXIT INT TERM

pick_free_port() {
  python3 - <<'PY'
import socket

sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.bind(("127.0.0.1", 0))
try:
    print(sock.getsockname()[1])
finally:
    sock.close()
PY
}

wait_for_broker() {
  local pid="$1"
  local bootstrap="$2"
  local log_file="$3"

python3 - "$pid" "$bootstrap" "$log_file" "$READY_TIMEOUT_SECONDS" <<'PY'
import pathlib
import sys
import time

from confluent_kafka.admin import AdminClient

pid = int(sys.argv[1])
bootstrap = sys.argv[2]
log_file = pathlib.Path(sys.argv[3])
deadline = time.monotonic() + float(sys.argv[4])
last_error = None

while time.monotonic() < deadline:
    try:
        import os

        os.kill(pid, 0)
    except OSError:
        raise SystemExit(
            "broker exited before readiness:\n"
            f"{log_file.read_text(encoding='utf-8', errors='replace')}"
        )

    try:
        AdminClient({"bootstrap.servers": bootstrap}).list_topics(timeout=1)
        sys.exit(0)
    except Exception as exc:  # noqa: BLE001 - readiness probe retries by design
        last_error = exc
        time.sleep(0.05)

raise SystemExit(
    "broker was not ready before the deadline:\n"
    f"{last_error}\n"
    f"{log_file.read_text(encoding='utf-8', errors='replace')}"
)
PY
}

run_roundtrip() {
  local start_ns="$1"
  local bootstrap="$2"
  local attempt="$3"
  local budget_ms="$4"

  python3 - "$start_ns" "$bootstrap" "$attempt" "$budget_ms" "$CLIENT_TIMEOUT_SECONDS" <<'PY'
import sys
import time
import uuid

from confluent_kafka import Consumer, KafkaError, Producer, TopicPartition

start_ns = int(sys.argv[1])
bootstrap = sys.argv[2]
attempt = int(sys.argv[3])
budget_ms = float(sys.argv[4])
client_timeout = float(sys.argv[5])

topic = f"cold-start-{attempt}-{uuid.uuid4().hex[:10]}"
group = f"cold-start-{attempt}-{uuid.uuid4().hex[:10]}"

producer = Producer(
    {
        "bootstrap.servers": bootstrap,
        "message.timeout.ms": 5000,
    }
)
delivery = []

def on_delivery(err, msg):
    delivery.append((err, msg))

producer.produce(topic, value=b"cold-start-roundtrip", key=b"cold-start", on_delivery=on_delivery)
producer.flush(10)

if not delivery:
    raise SystemExit("no delivery report from producer")

err, msg = delivery[0]
if err is not None:
    raise SystemExit(f"produce failed: {err}")

consumer = Consumer(
    {
        "bootstrap.servers": bootstrap,
        "group.id": group,
        "auto.offset.reset": "earliest",
        "enable.auto.commit": False,
        "session.timeout.ms": 6000,
    }
)
consumer.assign([TopicPartition(topic, 0, 0)])
try:
    deadline = time.monotonic() + client_timeout
    received = None
    while time.monotonic() < deadline:
        record = consumer.poll(0.1)
        if record is None:
            continue
        if record.error():
            if record.error().code() == KafkaError._PARTITION_EOF:
                continue
            raise SystemExit(f"consume failed: {record.error()}")
        received = record
        break

    if received is None:
        raise SystemExit("timed out waiting for the consumed record")

    elapsed_ms = (time.monotonic_ns() - start_ns) / 1_000_000
    print(f"elapsed_ms={elapsed_ms:.3f}")

    if elapsed_ms >= budget_ms:
        raise SystemExit(
            f"cold-start budget exceeded: {elapsed_ms:.3f}ms >= {budget_ms:.0f}ms"
        )
finally:
    consumer.close()
PY
}

main() {
  require_tool cargo
  require_tool python3

  python3 - <<'PY'
import importlib
try:
    importlib.import_module("confluent_kafka")
except ModuleNotFoundError as exc:
    raise SystemExit(
        "missing Python dependency: confluent-kafka\n"
        "Install it with: python3 -m pip install --user confluent-kafka==2.6.1"
    ) from exc
PY

  case "${PROFILE}" in
    release)
      cargo build --locked --release -p heimq --bin heimq
      BROKER_BIN="${ROOT_DIR}/target/release/heimq"
      ;;
    debug)
      cargo build --locked -p heimq --bin heimq
      BROKER_BIN="${ROOT_DIR}/target/debug/heimq"
      ;;
    *)
      echo "unsupported HEIMQ_STARTUP_BUDGET_PROFILE=${PROFILE}" >&2
      exit 2
      ;;
  esac

  if [[ ! -x "${BROKER_BIN}" ]]; then
    echo "missing broker binary: ${BROKER_BIN}" >&2
    exit 2
  fi

  for attempt in $(seq 1 "${ATTEMPTS}"); do
    cleanup_current
    BROKER_DATA_DIR="$(mktemp -d "${ROOT_DIR}/target/startup-budget.XXXXXX")"
    BROKER_LOG="${BROKER_DATA_DIR}/heimq.log"
    local_port="$(pick_free_port)"
    start_ns="$(python3 - <<'PY'
import time
print(time.monotonic_ns())
PY
)"

    HEIMQ_HOST="${BROKER_HOST}" \
    HEIMQ_PORT="${local_port}" \
    HEIMQ_ADVERTISED_HOST="${BROKER_HOST}" \
    HEIMQ_MEMORY_ONLY=true \
    HEIMQ_DEFAULT_PARTITIONS=1 \
    HEIMQ_AUTO_CREATE_TOPICS=true \
    HEIMQ_DATA_DIR="${BROKER_DATA_DIR}" \
    RUST_LOG="${BROKER_RUST_LOG}" \
    "${BROKER_BIN}" >"${BROKER_LOG}" 2>&1 &
    BROKER_PID=$!

    wait_for_broker "${BROKER_PID}" "${BROKER_HOST}:${local_port}" "${BROKER_LOG}"
    run_roundtrip "${start_ns}" "${BROKER_HOST}:${local_port}" "${attempt}" "${BUDGET_MS}"

    echo "[attempt ${attempt}/${ATTEMPTS}] PASS"
  done

  cleanup_current
  echo "PASS: cold-start budget satisfied for ${ATTEMPTS} consecutive attempts"
}

main "$@"
