#!/usr/bin/env bash
# Integration target 4: Confluent Schema Registry round-trip.
# Starts Schema Registry pointing at heimq, registers an Avro schema,
# and verifies retrieval.
#
# Requirements: Docker, heimq running on $BOOTSTRAP (default localhost:9094).

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=common.sh
source "$SCRIPT_DIR/common.sh"

SR_PORT=8081
SR_CID="eco-sr-${RUN_ID}"
TOPIC="eco-sr-${RUN_ID}"

cleanup() {
    docker rm -f "$SR_CID" 2>/dev/null || true
}
trap cleanup EXIT

echo "==> [4/8] Confluent Schema Registry"

# Start Schema Registry (bridge network; access via container IP)
docker run -d --name "$SR_CID" \
    -e SCHEMA_REGISTRY_HOST_NAME=localhost \
    -e SCHEMA_REGISTRY_LISTENERS="http://0.0.0.0:${SR_PORT}" \
    -e SCHEMA_REGISTRY_KAFKASTORE_BOOTSTRAP_SERVERS="$DOCKER_BOOTSTRAP" \
    -e SCHEMA_REGISTRY_KAFKASTORE_TOPIC=_schemas \
    confluentinc/cp-schema-registry:7.6.0 >/dev/null

# Get container's bridge IP so we can call its REST API
SR_IP=$(docker inspect "$SR_CID" --format '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' 2>/dev/null)
SR_URL="http://${SR_IP}:${SR_PORT}"
echo "  Schema Registry container IP: $SR_IP"

echo "  waiting for Schema Registry REST API..."
if ! wait_for_http "${SR_URL}/subjects" 90; then
    echo "FAIL: Schema Registry did not start" >&2
    docker logs "$SR_CID" 2>&1 | tail -30 >&2
    exit 1
fi

# Register an Avro schema
SCHEMA='{"type":"record","name":"Msg","fields":[{"name":"value","type":"string"}]}'
SCHEMA_JSON=$(python3 -c "import json,sys; print(json.dumps(sys.argv[1]))" "$SCHEMA")
RESPONSE=$(curl -sf -X POST "${SR_URL}/subjects/${TOPIC}-value/versions" \
    -H "Content-Type: application/vnd.schemaregistry.v1+json" \
    -d "{\"schema\": ${SCHEMA_JSON}}")

echo "  schema registered: $RESPONSE"
SCHEMA_ID=$(echo "$RESPONSE" | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')

# Retrieve schema back and verify
RETRIEVED=$(curl -sf "${SR_URL}/subjects/${TOPIC}-value/versions/latest")
echo "  schema retrieved: $RETRIEVED"

RETRIEVED_SCHEMA=$(echo "$RETRIEVED" | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d["schema"])' 2>/dev/null)
if [ -z "$RETRIEVED_SCHEMA" ]; then
    eco_fail "Schema Registry: could not retrieve registered schema"
    eco_summary
    exit 1
fi

eco_pass "Schema Registry: registered and retrieved Avro schema (id=$SCHEMA_ID)"
eco_summary
