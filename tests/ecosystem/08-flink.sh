#!/usr/bin/env bash
# Integration target 8: Apache Flink Kafka source + sink (EOS).
# Starts Flink (JobManager + TaskManager) and runs a Flink SQL job that
# reads from a heimq topic and writes to another via exactly-once semantics.
#
# Requirements: Docker, heimq running on $BOOTSTRAP (default localhost:9094).

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=common.sh
source "$SCRIPT_DIR/common.sh"

FLINK_PORT=8081
JM_CID="eco-flink-jm-${RUN_ID}"
TM_CID="eco-flink-tm-${RUN_ID}"
FLINK_NET="eco-flink-net-${RUN_ID}"
SRC_TOPIC="eco-flink-src-${RUN_ID}"
SINK_TOPIC="eco-flink-sink-${RUN_ID}"

cleanup() {
    docker rm -f "$JM_CID" "$TM_CID" 2>/dev/null || true
    docker network rm "$FLINK_NET" 2>/dev/null || true
}
trap cleanup EXIT

echo "==> [8/8] Apache Flink Kafka source + sink (EOS)"

# Seed source topic
docker run --rm \
    -e BOOTSTRAP="$DOCKER_BOOTSTRAP" \
    -e SRC="$SRC_TOPIC" \
    -e SINK="$SINK_TOPIC" \
    python:3.11-slim bash -c '
pip install --quiet confluent-kafka
python3 -c "
import os, time
from confluent_kafka import Producer
from confluent_kafka.admin import AdminClient, NewTopic

b = os.environ[\"BOOTSTRAP\"]
admin = AdminClient({\"bootstrap.servers\": b})
for t in [os.environ[\"SRC\"], os.environ[\"SINK\"]]:
    admin.create_topics([NewTopic(t, num_partitions=1, replication_factor=1)])
time.sleep(1)
p = Producer({\"bootstrap.servers\": b})
for i in range(20):
    p.produce(os.environ[\"SRC\"], key=str(i).encode(), value=\"flink-msg-{}\".format(i).encode())
p.flush(10)
print(\"seeded 20 messages\")
"
'

# Create Docker network for JM↔TM communication
docker network create "$FLINK_NET" >/dev/null

# Start Flink JobManager
docker run -d --name "$JM_CID" --network "$FLINK_NET" \
    -e FLINK_PROPERTIES="$(printf 'jobmanager.rpc.address: %s\nrest.port: %s\nrest.bind-address: 0.0.0.0' "$JM_CID" "$FLINK_PORT")" \
    apache/flink:1.19-java17 \
    jobmanager >/dev/null

JM_IP=$(docker inspect "$JM_CID" --format "{{(index .NetworkSettings.Networks \"${FLINK_NET}\").IPAddress}}" 2>/dev/null)
FLINK_URL="http://${JM_IP}:${FLINK_PORT}"
echo "  Flink JobManager IP: $JM_IP"

echo "  waiting for Flink JobManager REST API (up to 60s)..."
if ! wait_for_http "${FLINK_URL}/overview" 60; then
    echo "FAIL: Flink JobManager did not start" >&2
    docker logs "$JM_CID" 2>&1 | tail -20 >&2
    exit 1
fi
echo "  Flink JobManager is up"

# Start TaskManager
docker run -d --name "$TM_CID" --network "$FLINK_NET" \
    -e FLINK_PROPERTIES="$(printf 'jobmanager.rpc.address: %s\ntaskmanager.numberOfTaskSlots: 2' "$JM_CID")" \
    apache/flink:1.19-java17 \
    taskmanager >/dev/null

# Wait for TaskManager slots
DEADLINE=$((SECONDS + 30))
SLOTS=0
while [ $SECONDS -lt $DEADLINE ]; do
    SLOTS=$(curl -sf "${FLINK_URL}/overview" 2>/dev/null \
        | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d.get("slots-available", 0))' 2>/dev/null || echo 0)
    [ "$SLOTS" -ge 1 ] && break
    sleep 2
done

if [ "$SLOTS" -lt 1 ]; then
    echo "FAIL: No TaskManager slots available after 30s" >&2
    exit 1
fi
echo "  TaskManager registered ($SLOTS slots available)"

# @covers US-009-AC1
# @covers US-009-AC2
# Run Flink SQL: copy SRC to SINK with exactly-once semantics and checkpoints enabled.
SQL_JOB="SET 'execution.checkpointing.interval' = '1s';
SET 'execution.checkpointing.mode' = 'EXACTLY_ONCE';
CREATE TABLE src (\`value\` BYTES) WITH ('connector' = 'kafka', 'topic' = '${SRC_TOPIC}', 'properties.bootstrap.servers' = '${DOCKER_BOOTSTRAP}', 'properties.group.id' = 'eco-flink-src', 'scan.startup.mode' = 'earliest-offset', 'value.format' = 'raw');
CREATE TABLE sink (\`value\` BYTES) WITH ('connector' = 'kafka', 'topic' = '${SINK_TOPIC}', 'properties.bootstrap.servers' = '${DOCKER_BOOTSTRAP}', 'properties.transaction.timeout.ms' = '30000', 'sink.delivery-guarantee' = 'exactly-once', 'sink.transactional-id-prefix' = 'eco-flink-eos', 'value.format' = 'raw');
INSERT INTO sink SELECT \`value\` FROM src;"

docker exec "$JM_CID" bash -c "
printf '%s\n' '${SQL_JOB//\'/\'\\\'\'}' > /tmp/job.sql
nohup /opt/flink/bin/sql-client.sh -f /tmp/job.sql >/tmp/job.out 2>&1 &
" 2>&1

DEADLINE=$((SECONDS + 60))
JOB_ID=""
while [ $SECONDS -lt $DEADLINE ]; do
    JOB_ID=$(curl -sf "${FLINK_URL}/jobs/overview" 2>/dev/null \
        | python3 -c 'import json,sys; jobs=json.load(sys.stdin).get("jobs", []); print(next((j["jid"] for j in jobs if j.get("state") == "RUNNING"), ""))' 2>/dev/null || true)
    [ -n "$JOB_ID" ] && break
    if docker exec "$JM_CID" grep -qiE "exception|error" /tmp/job.out 2>/dev/null; then
        echo "FAIL: Flink SQL job failed to start" >&2
        docker exec "$JM_CID" tail -80 /tmp/job.out >&2 || true
        exit 1
    fi
    sleep 2
done

if [ -z "$JOB_ID" ]; then
    echo "FAIL: no running Flink SQL job appeared within 60s" >&2
    docker exec "$JM_CID" tail -80 /tmp/job.out >&2 || true
    exit 1
fi
echo "  Flink SQL job running: $JOB_ID"

DEADLINE=$((SECONDS + 90))
COMPLETED_CHECKPOINTS=0
while [ $SECONDS -lt $DEADLINE ]; do
    COMPLETED_CHECKPOINTS=$(curl -sf "${FLINK_URL}/jobs/${JOB_ID}/checkpoints" 2>/dev/null \
        | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d.get("counts", {}).get("completed", 0))' 2>/dev/null || echo 0)
    [ "$COMPLETED_CHECKPOINTS" -ge 1 ] && break
    if docker exec "$JM_CID" grep -qiE "exception|error" /tmp/job.out 2>/dev/null; then
        echo "FAIL: Flink SQL job failed before completing a checkpoint" >&2
        docker exec "$JM_CID" tail -80 /tmp/job.out >&2 || true
        exit 1
    fi
    sleep 2
done

if [ "$COMPLETED_CHECKPOINTS" -lt 1 ]; then
    echo "FAIL: Flink job completed $COMPLETED_CHECKPOINTS checkpoints (expected >= 1)" >&2
    curl -sf "${FLINK_URL}/jobs/${JOB_ID}/checkpoints" >&2 || true
    docker exec "$JM_CID" tail -80 /tmp/job.out >&2 || true
    exit 1
fi
echo "  completed checkpoints: $COMPLETED_CHECKPOINTS"

# Verify data arrived in sink topic
SINK_COUNT=$(docker run --rm \
    -e BOOTSTRAP="$DOCKER_BOOTSTRAP" \
    -e TOPIC="$SINK_TOPIC" \
    python:3.11-slim bash -c '
pip install --quiet confluent-kafka
python3 -c "
import os, time, sys
from confluent_kafka import Consumer, KafkaError

c = Consumer({
    \"bootstrap.servers\": os.environ[\"BOOTSTRAP\"],
    \"group.id\": \"eco-flink-verify\",
    \"auto.offset.reset\": \"earliest\",
    \"enable.auto.commit\": \"false\",
})
c.subscribe([os.environ[\"TOPIC\"]])
received = 0
deadline = time.time() + 30
while time.time() < deadline:
    msg = c.poll(1.0)
    if msg is None: continue
    if msg.error():
        if msg.error().code() != KafkaError._PARTITION_EOF: continue
        if received > 0: break
        continue
    received += 1
c.close()
print(received)
"
' 2>/dev/null || echo 0)

echo "  sink topic received $SINK_COUNT messages"

if [ "$SINK_COUNT" -ge 20 ]; then
    eco_pass "Flink: Kafka source → sink (EOS) via heimq ($SINK_COUNT messages)"
else
    echo "FAIL: Flink sink received $SINK_COUNT messages (expected >= 20)" >&2
    docker exec "$JM_CID" tail -80 /tmp/job.out >&2 || true
    exit 1
fi
# @covers US-009-AC3
eco_summary
