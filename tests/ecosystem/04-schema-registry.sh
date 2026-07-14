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

# @covers US-011-AC1
# Retrieve schema back and verify that registration resolved to a concrete schema id.
RETRIEVED=$(curl -sf "${SR_URL}/subjects/${TOPIC}-value/versions/latest")
echo "  schema retrieved: $RETRIEVED"

RETRIEVED_SCHEMA=$(echo "$RETRIEVED" | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d["schema"])' 2>/dev/null)
if [ -z "$RETRIEVED_SCHEMA" ]; then
    eco_fail "Schema Registry: could not retrieve registered schema"
    eco_summary
    exit 1
fi

# @covers US-011-AC2
# @covers US-011-AC3
# Produce and consume through Confluent's registry-aware Avro serializer/deserializer.
docker run --rm \
    -e BOOTSTRAP="$DOCKER_BOOTSTRAP" \
    -e SCHEMA_REGISTRY_URL="$SR_URL" \
    -e TOPIC="$TOPIC" \
    python:3.11-slim bash -c '
pip install --quiet "confluent-kafka[avro]"
python3 - << '"'"'PYEOF'"'"'
import os, sys, time
from confluent_kafka import KafkaError
from confluent_kafka.admin import AdminClient, NewTopic
from confluent_kafka.avro import AvroConsumer, AvroProducer, loads

bootstrap = os.environ["BOOTSTRAP"]
registry_url = os.environ["SCHEMA_REGISTRY_URL"]
topic = os.environ["TOPIC"]
expected = {"value": "schema-registry-ok"}
schema = loads("""{"type":"record","name":"Msg","fields":[{"name":"value","type":"string"}]}""")

admin = AdminClient({"bootstrap.servers": bootstrap})
fs = admin.create_topics([NewTopic(topic, num_partitions=1, replication_factor=1)])
for f in fs.values():
    try:
        f.result()
    except Exception:
        pass

producer = AvroProducer(
    {"bootstrap.servers": bootstrap, "schema.registry.url": registry_url},
    default_value_schema=schema,
)
producer.produce(topic=topic, value=expected)
producer.flush(10)

consumer = AvroConsumer({
    "bootstrap.servers": bootstrap,
    "schema.registry.url": registry_url,
    "group.id": "eco-sr-verify",
    "auto.offset.reset": "earliest",
    "enable.auto.commit": "false",
})
consumer.subscribe([topic])
deadline = time.time() + 20
while time.time() < deadline:
    msg = consumer.poll(1.0)
    if msg is None:
        continue
    if msg.error():
        if msg.error().code() != KafkaError._PARTITION_EOF:
            print(f"consumer error: {msg.error()}", file=sys.stderr)
            sys.exit(1)
        continue
    if msg.value() == expected:
        print("registry serializer/deserializer round-trip ok")
        consumer.close()
        sys.exit(0)
    print(f"FAIL: decoded {msg.value()!r}, expected {expected!r}", file=sys.stderr)
    consumer.close()
    sys.exit(1)
consumer.close()
print("FAIL: no registry-serialized record within 20s", file=sys.stderr)
sys.exit(1)
PYEOF
'

eco_pass "Schema Registry: registered and retrieved Avro schema (id=$SCHEMA_ID)"
# @covers US-011-AC4
eco_summary
