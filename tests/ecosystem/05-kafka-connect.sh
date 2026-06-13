#!/usr/bin/env bash
# Integration target 5: Kafka Connect source + sink.
# Starts Kafka Connect backed by heimq, deploys FileStream source and sink
# connectors, verifies data flows through heimq.
#
# Requirements: Docker, heimq running on $BOOTSTRAP (default localhost:9094).

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=common.sh
source "$SCRIPT_DIR/common.sh"

CONNECT_PORT=8083
CONNECT_CID="eco-connect-${RUN_ID}"
TOPIC="eco-connect-${RUN_ID}"

cleanup() {
    docker rm -f "$CONNECT_CID" 2>/dev/null || true
}
trap cleanup EXIT

echo "==> [5/8] Kafka Connect (source + sink)"

# Start Kafka Connect in distributed mode backed by heimq
docker run -d --name "$CONNECT_CID" \
    -e CONNECT_BOOTSTRAP_SERVERS="$DOCKER_BOOTSTRAP" \
    -e CONNECT_REST_PORT="$CONNECT_PORT" \
    -e CONNECT_GROUP_ID="eco-connect-${RUN_ID}" \
    -e CONNECT_CONFIG_STORAGE_TOPIC="connect-configs-${RUN_ID}" \
    -e CONNECT_OFFSET_STORAGE_TOPIC="connect-offsets-${RUN_ID}" \
    -e CONNECT_STATUS_STORAGE_TOPIC="connect-status-${RUN_ID}" \
    -e CONNECT_CONFIG_STORAGE_REPLICATION_FACTOR=1 \
    -e CONNECT_OFFSET_STORAGE_REPLICATION_FACTOR=1 \
    -e CONNECT_STATUS_STORAGE_REPLICATION_FACTOR=1 \
    -e CONNECT_OFFSET_STORAGE_PARTITIONS=1 \
    -e CONNECT_KEY_CONVERTER=org.apache.kafka.connect.storage.StringConverter \
    -e CONNECT_VALUE_CONVERTER=org.apache.kafka.connect.storage.StringConverter \
    -e CONNECT_INTERNAL_KEY_CONVERTER=org.apache.kafka.connect.json.JsonConverter \
    -e CONNECT_INTERNAL_VALUE_CONVERTER=org.apache.kafka.connect.json.JsonConverter \
    -e CONNECT_REST_ADVERTISED_HOST_NAME=localhost \
    confluentinc/cp-kafka-connect:7.6.0 >/dev/null

# Get container bridge IP
CONNECT_IP=$(docker inspect "$CONNECT_CID" --format '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' 2>/dev/null)
CONNECT_URL="http://${CONNECT_IP}:${CONNECT_PORT}"
echo "  Kafka Connect container IP: $CONNECT_IP"

echo "  waiting for Kafka Connect REST API (up to 120s)..."
if ! wait_for_http "${CONNECT_URL}/connectors" 120; then
    echo "FAIL: Kafka Connect did not start within 120s" >&2
    docker logs "$CONNECT_CID" 2>&1 | tail -30 >&2
    exit 1
fi
echo "  Kafka Connect is up; deploying FileStream source connector..."

# Write source data inside the Connect container
docker exec "$CONNECT_CID" bash -c "
    mkdir -p /tmp/connect-data
    for i in \$(seq 1 10); do echo \"line-\$i\"; done > /tmp/connect-data/source.txt
"

# Deploy FileStreamSourceConnector
SOURCE_RESP=$(curl -sf -X POST "${CONNECT_URL}/connectors" \
    -H "Content-Type: application/json" \
    -d "{
  \"name\": \"eco-source-${RUN_ID}\",
  \"config\": {
    \"connector.class\": \"FileStreamSource\",
    \"file\": \"/tmp/connect-data/source.txt\",
    \"topic\": \"${TOPIC}\"
  }
}")
echo "  source connector: $SOURCE_RESP"

# Wait for source connector to enter RUNNING state
DEADLINE=$((SECONDS + 30))
STATUS=""
while [ $SECONDS -lt $DEADLINE ]; do
    STATUS=$(curl -sf "${CONNECT_URL}/connectors/eco-source-${RUN_ID}/status" 2>/dev/null \
        | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d["connector"]["state"])' 2>/dev/null || echo "UNKNOWN")
    [ "$STATUS" = "RUNNING" ] && break
    sleep 2
done

if [ "$STATUS" != "RUNNING" ]; then
    echo "FAIL: source connector state=$STATUS (expected RUNNING)" >&2
    curl -sf "${CONNECT_URL}/connectors/eco-source-${RUN_ID}/status" >&2 || true
    exit 1
fi
echo "  source connector RUNNING"

# Deploy FileStreamSinkConnector
SINK_RESP=$(curl -sf -X POST "${CONNECT_URL}/connectors" \
    -H "Content-Type: application/json" \
    -d "{
  \"name\": \"eco-sink-${RUN_ID}\",
  \"config\": {
    \"connector.class\": \"FileStreamSink\",
    \"file\": \"/tmp/connect-data/sink.txt\",
    \"topics\": \"${TOPIC}\"
  }
}")
echo "  sink connector: $SINK_RESP"

# Wait for sink to produce output (at least 10 lines)
DEADLINE=$((SECONDS + 30))
SINK_LINES=0
while [ $SECONDS -lt $DEADLINE ]; do
    SINK_LINES=$(docker exec "$CONNECT_CID" bash -c "wc -l < /tmp/connect-data/sink.txt 2>/dev/null || echo 0")
    [ "$SINK_LINES" -ge 10 ] && break
    sleep 2
done

if [ "$SINK_LINES" -lt 10 ]; then
    echo "FAIL: sink received $SINK_LINES lines (expected >= 10)" >&2
    docker exec "$CONNECT_CID" cat /tmp/connect-data/sink.txt 2>/dev/null >&2 || true
    exit 1
fi

eco_pass "Kafka Connect: source+sink via heimq ($SINK_LINES lines)"
eco_summary
