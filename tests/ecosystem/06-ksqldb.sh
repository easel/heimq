#!/usr/bin/env bash
# Integration target 6: ksqlDB stream processing.
# Starts ksqlDB backed by heimq, creates a stream over a seeded topic,
# and verifies the server is running.
#
# Requirements: Docker, heimq running on $BOOTSTRAP (default localhost:9094).

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=common.sh
source "$SCRIPT_DIR/common.sh"

KSQL_PORT=8088
KSQL_CID="eco-ksql-${RUN_ID}"
TOPIC="eco-ksql-${RUN_ID}"

cleanup() {
    docker rm -f "$KSQL_CID" 2>/dev/null || true
}
trap cleanup EXIT

echo "==> [6/8] ksqlDB"

# Seed the topic before starting ksqlDB so the stream has data
docker run --rm \
    -e BOOTSTRAP="$DOCKER_BOOTSTRAP" \
    -e TOPIC="$TOPIC" \
    python:3.11-slim bash -c '
pip install --quiet confluent-kafka
python3 -c "
import os, time
from confluent_kafka import Producer
from confluent_kafka.admin import AdminClient, NewTopic

b = os.environ[\"BOOTSTRAP\"]
t = os.environ[\"TOPIC\"]
admin = AdminClient({\"bootstrap.servers\": b})
admin.create_topics([NewTopic(t, num_partitions=1, replication_factor=1)])
time.sleep(1)
p = Producer({\"bootstrap.servers\": b})
for i in range(10):
    p.produce(t, key=str(i).encode(), value=\"event-{}\".format(i).encode())
p.flush(10)
print(\"seeded 10 messages to\", t)
"
'

# Start ksqlDB (bridge network; access via container IP)
docker run -d --name "$KSQL_CID" \
    -e KSQL_BOOTSTRAP_SERVERS="$DOCKER_BOOTSTRAP" \
    -e KSQL_LISTENERS="http://0.0.0.0:${KSQL_PORT}" \
    -e KSQL_KSQL_SERVICE_ID="eco-ksql-${RUN_ID}" \
    -e KSQL_KSQL_LOGGING_PROCESSING_STREAM_AUTO_CREATE=true \
    -e KSQL_KSQL_LOGGING_PROCESSING_TOPIC_AUTO_CREATE=true \
    confluentinc/ksqldb-server:0.29.0 >/dev/null

# Get container bridge IP
KSQL_IP=$(docker inspect "$KSQL_CID" --format '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' 2>/dev/null)
KSQL_URL="http://${KSQL_IP}:${KSQL_PORT}"
echo "  ksqlDB container IP: $KSQL_IP"

echo "  waiting for ksqlDB REST API (up to 90s)..."
if ! wait_for_http "${KSQL_URL}/info" 90; then
    echo "FAIL: ksqlDB did not start within 90s" >&2
    docker logs "$KSQL_CID" 2>&1 | tail -20 >&2
    exit 1
fi
echo "  ksqlDB is up"

# Create a stream over the topic
STREAM_NAME="ECO_STREAM_${RUN_ID}"
CREATE_RESP=$(curl -sf -X POST "${KSQL_URL}/ksql" \
    -H "Content-Type: application/vnd.ksql.v1+json" \
    -d "{\"ksql\": \"CREATE STREAM ${STREAM_NAME} (val VARCHAR) WITH (kafka_topic='${TOPIC}', value_format='KAFKA', partitions=1);\", \"streamsProperties\": {}}")
echo "  CREATE STREAM: $(echo "$CREATE_RESP" | head -c 200)"

# Verify ksqlDB server status
SERVER_INFO=$(curl -sf "${KSQL_URL}/info")
STATUS=$(echo "$SERVER_INFO" | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d.get("KsqlServerInfo", {}).get("serverStatus", ""))' 2>/dev/null || echo "unknown")
echo "  ksqlDB serverStatus=$STATUS"

if [ "$STATUS" = "RUNNING" ]; then
    eco_pass "ksqlDB: stream created and server running (serverStatus=RUNNING)"
else
    eco_fail "ksqlDB: serverStatus=$STATUS (expected RUNNING)"
fi
eco_summary
