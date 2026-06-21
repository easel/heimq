#!/usr/bin/env bash
# Integration target 7: Debezium PostgreSQL CDC connector.
# Starts PostgreSQL + Debezium Connect backed by heimq.
# Inserts a row into PostgreSQL and verifies the CDC event appears in heimq.
#
# Requirements: Docker, heimq running on $BOOTSTRAP (default localhost:9094).

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=common.sh
source "$SCRIPT_DIR/common.sh"

PG_PORT=5432
DEBEZIUM_PORT=8083
PG_CID="eco-pg-${RUN_ID}"
DBZ_CID="eco-dbz-${RUN_ID}"
DBZ_NET="eco-dbz-net-${RUN_ID}"

cleanup() {
    docker rm -f "$DBZ_CID" "$PG_CID" 2>/dev/null || true
    docker network rm "$DBZ_NET" 2>/dev/null || true
}
trap cleanup EXIT

echo "==> [7/8] Debezium PostgreSQL CDC"

# Create a dedicated bridge network so Debezium can reach PostgreSQL by container name
docker network create "$DBZ_NET" >/dev/null

# Start PostgreSQL with logical replication enabled
docker run -d --name "$PG_CID" --network "$DBZ_NET" \
    -e POSTGRES_USER=debezium \
    -e POSTGRES_PASSWORD=debezium \
    -e POSTGRES_DB=inventory \
    postgres:16-alpine \
    postgres -c wal_level=logical >/dev/null

# Get PG bridge IP
PG_IP=$(docker inspect "$PG_CID" --format "{{(index .NetworkSettings.Networks \"${DBZ_NET}\").IPAddress}}" 2>/dev/null)
echo "  PostgreSQL container IP: $PG_IP"

# Wait for PostgreSQL
DEADLINE=$((SECONDS + 30))
while ! docker exec "$PG_CID" pg_isready -U debezium >/dev/null 2>&1; do
    [ $SECONDS -ge $DEADLINE ] && { echo "FAIL: PostgreSQL did not start" >&2; exit 1; }
    sleep 1
done
echo "  PostgreSQL ready"

# Create test table and publication
docker exec "$PG_CID" psql -U debezium -d inventory -c "
CREATE TABLE IF NOT EXISTS orders (
    id SERIAL PRIMARY KEY,
    item TEXT NOT NULL,
    quantity INT NOT NULL
);" >/dev/null
docker exec "$PG_CID" psql -U debezium -d inventory -c "
CREATE PUBLICATION dbz_pub FOR TABLE orders;" 2>/dev/null || true

# Start Debezium Connect on the same bridge network (so it can reach PG by IP)
# Also needs to reach heimq (DOCKER_BOOTSTRAP) which is on the OrbStack VM
docker run -d --name "$DBZ_CID" --network "$DBZ_NET" \
    -e BOOTSTRAP_SERVERS="$DOCKER_BOOTSTRAP" \
    -e GROUP_ID="eco-dbz-${RUN_ID}" \
    -e CONFIG_STORAGE_TOPIC="dbz-configs-${RUN_ID}" \
    -e OFFSET_STORAGE_TOPIC="dbz-offsets-${RUN_ID}" \
    -e STATUS_STORAGE_TOPIC="dbz-status-${RUN_ID}" \
    -e CONFIG_STORAGE_REPLICATION_FACTOR=1 \
    -e OFFSET_STORAGE_REPLICATION_FACTOR=1 \
    -e STATUS_STORAGE_REPLICATION_FACTOR=1 \
    -e OFFSET_STORAGE_PARTITIONS=1 \
    quay.io/debezium/connect:3.0 >/dev/null

# Get Debezium container's IP on the bridge network
DBZ_IP=$(docker inspect "$DBZ_CID" --format "{{(index .NetworkSettings.Networks \"${DBZ_NET}\").IPAddress}}" 2>/dev/null)
DBZ_URL="http://${DBZ_IP}:${DEBEZIUM_PORT}"
echo "  Debezium Connect container IP: $DBZ_IP"

echo "  waiting for Debezium Connect REST API (up to 120s)..."
if ! wait_for_http "${DBZ_URL}/connectors" 120; then
    echo "FAIL: Debezium Connect did not start within 120s" >&2
    docker logs "$DBZ_CID" 2>&1 | tail -30 >&2
    exit 1
fi
echo "  Debezium Connect is up"

# Deploy PostgreSQL CDC connector (use PG_IP since they're on the same bridge)
SLOT_NAME="eco_slot_$(echo "${RUN_ID}" | tr -dc '0-9' | head -c 8)"
CONNECTOR_CFG="{
  \"name\": \"eco-pg-cdc-${RUN_ID}\",
  \"config\": {
    \"connector.class\": \"io.debezium.connector.postgresql.PostgresConnector\",
    \"database.hostname\": \"${PG_IP}\",
    \"database.port\": \"${PG_PORT}\",
    \"database.user\": \"debezium\",
    \"database.password\": \"debezium\",
    \"database.dbname\": \"inventory\",
    \"topic.prefix\": \"eco-dbz-${RUN_ID}\",
    \"table.include.list\": \"public.orders\",
    \"plugin.name\": \"pgoutput\",
    \"slot.name\": \"${SLOT_NAME}\"
  }
}"

CONN_RESP=$(curl -sf -X POST "${DBZ_URL}/connectors" \
    -H "Content-Type: application/json" \
    -d "$CONNECTOR_CFG")
echo "  connector created: $(echo "$CONN_RESP" | head -c 200)"

# Wait for connector to be RUNNING
DEADLINE=$((SECONDS + 30))
STATUS=""
while [ $SECONDS -lt $DEADLINE ]; do
    STATUS=$(curl -sf "${DBZ_URL}/connectors/eco-pg-cdc-${RUN_ID}/status" 2>/dev/null \
        | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d["connector"]["state"])' 2>/dev/null || echo "UNKNOWN")
    [ "$STATUS" = "RUNNING" ] && break
    sleep 2
done

if [ "$STATUS" != "RUNNING" ]; then
    echo "FAIL: Debezium connector state=$STATUS (expected RUNNING)" >&2
    curl -sf "${DBZ_URL}/connectors/eco-pg-cdc-${RUN_ID}/status" >&2 || true
    exit 1
fi
echo "  connector RUNNING; inserting test row..."

# Insert a row to trigger CDC
docker exec "$PG_CID" psql -U debezium -d inventory -c \
    "INSERT INTO orders (item, quantity) VALUES ('widget', 42);" >/dev/null

# Consume the CDC event from heimq using Python
CDC_TOPIC="eco-dbz-${RUN_ID}.public.orders"
RECEIVED=$(docker run --rm \
    -e BOOTSTRAP="$DOCKER_BOOTSTRAP" \
    -e TOPIC="$CDC_TOPIC" \
    python:3.11-slim bash -c '
pip install --quiet confluent-kafka
python3 -c "
import os, time, sys
from confluent_kafka import Consumer, KafkaError

c = Consumer({
    \"bootstrap.servers\": os.environ[\"BOOTSTRAP\"],
    \"group.id\": \"eco-dbz-verify\",
    \"auto.offset.reset\": \"earliest\",
    \"enable.auto.commit\": \"false\",
})
c.subscribe([os.environ[\"TOPIC\"]])
deadline = time.time() + 20
while time.time() < deadline:
    msg = c.poll(1.0)
    if msg is None: continue
    if msg.error():
        if msg.error().code() != KafkaError._PARTITION_EOF:
            print(\"consumer error:\", msg.error(), file=sys.stderr)
        continue
    print(\"received CDC event:\", msg.value()[:80] if msg.value() else \"(tombstone)\")
    c.close()
    sys.exit(0)
c.close()
print(\"FAIL: no CDC event within 20s\", file=sys.stderr)
sys.exit(1)
"
' 2>&1)

if echo "$RECEIVED" | grep -q "FAIL"; then
    eco_fail "Debezium: $RECEIVED"
else
    eco_pass "Debezium CDC: captured PostgreSQL row change via heimq"
    echo "  $RECEIVED"
fi
eco_summary
