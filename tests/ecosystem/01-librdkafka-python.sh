#!/usr/bin/env bash
# Integration target 1: Python confluent-kafka (librdkafka binding).
# Produces 20 messages and consumes them back via heimq.
#
# Requirements: Docker, heimq running on $BOOTSTRAP (default localhost:9094).

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=common.sh
source "$SCRIPT_DIR/common.sh"

TOPIC="eco-python-${RUN_ID}"

echo "==> [1/8] librdkafka Python (confluent-kafka)"

docker run --rm --network host \
    -e BOOTSTRAP="$DOCKER_BOOTSTRAP" \
    -e TOPIC="$TOPIC" \
    python:3.11-slim \
    bash -c '
pip install --quiet confluent-kafka

python3 - << '"'"'PYEOF'"'"'
import os, time, sys
from confluent_kafka import Producer, Consumer, KafkaError
from confluent_kafka.admin import AdminClient, NewTopic

bootstrap = os.environ["BOOTSTRAP"]
topic = os.environ["TOPIC"]
n = 20

# Create topic
admin = AdminClient({"bootstrap.servers": bootstrap})
fs = admin.create_topics([NewTopic(topic, num_partitions=1, replication_factor=1)])
for t, f in fs.items():
    try:
        f.result()
    except Exception as e:
        print(f"topic create: {e}", file=sys.stderr)

# Produce
p = Producer({"bootstrap.servers": bootstrap})
for i in range(n):
    p.produce(topic, f"msg-{i}".encode())
p.flush(timeout=10)

# @covers US-012-AC2
# Consume and require every produced message to come back through confluent-kafka.
c = Consumer({
    "bootstrap.servers": bootstrap,
    "group.id": "eco-python-test",
    "auto.offset.reset": "earliest",
    "enable.auto.commit": "false",
})
c.subscribe([topic])
received = 0
deadline = time.time() + 15
while received < n and time.time() < deadline:
    msg = c.poll(1.0)
    if msg is None:
        continue
    if msg.error():
        if msg.error().code() != KafkaError._PARTITION_EOF:
            print(f"consumer error: {msg.error()}", file=sys.stderr)
            sys.exit(1)
        continue
    received += 1
c.close()

if received < n:
    print(f"FAIL: expected {n} messages, received {received}", file=sys.stderr)
    sys.exit(1)

print(f"produced {n}, consumed {received}")
PYEOF
'

eco_pass "librdkafka Python: produce+consume via confluent-kafka"
# @covers US-012-AC4
eco_summary
