#!/usr/bin/env bash
# Integration target 3: Node.js node-rdkafka (librdkafka native binding).
# Produces 10 messages and consumes them back via heimq.
#
# Requirements: Docker, heimq running on $BOOTSTRAP (default localhost:9094).

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=common.sh
source "$SCRIPT_DIR/common.sh"

TOPIC="eco-node-${RUN_ID}"

echo "==> [3/8] librdkafka Node (node-rdkafka)"

docker run --rm --network host \
    -e BOOTSTRAP="$DOCKER_BOOTSTRAP" \
    -e TOPIC="$TOPIC" \
    node:20-bullseye \
    bash -c '
set -e
mkdir -p /tmp/nodetest && cd /tmp/nodetest
npm init -y --quiet
npm install --save node-rdkafka --quiet 2>&1 | tail -5

cat > test.js << '"'"'JSEOF'"'"'
const Kafka = require("node-rdkafka");

const bootstrap = process.env.BOOTSTRAP;
const topic = process.env.TOPIC;
const N = 10;

async function main() {
    // Produce
    await new Promise((resolve, reject) => {
        const producer = new Kafka.Producer({
            "metadata.broker.list": bootstrap,
            "dr_cb": true,
        });
        producer.connect();
        producer.on("ready", () => {
            for (let i = 0; i < N; i++) {
                producer.produce(topic, 0, Buffer.from(`msg-${i}`), null);
            }
            producer.flush(10000, (err) => {
                producer.disconnect();
                if (err) reject(err); else resolve();
            });
        });
        producer.on("event.error", reject);
    });

    // @covers US-012-AC3
    // Consume and require every produced message to come back through node-rdkafka.
    await new Promise((resolve, reject) => {
        const consumer = new Kafka.KafkaConsumer({
            "metadata.broker.list": bootstrap,
            "group.id": "eco-node-test",
            "auto.offset.reset": "earliest",
            "enable.auto.commit": "false",
        }, {});
        consumer.connect();
        let received = 0;
        consumer.on("ready", () => {
            consumer.assign([{ topic, partition: 0, offset: 0 }]);
            consumer.consume();
        });
        consumer.on("data", (msg) => {
            received++;
            if (received >= N) {
                consumer.disconnect(() => resolve(received));
            }
        });
        consumer.on("event.error", reject);
        setTimeout(() => {
            if (received < N) reject(new Error(`timeout: received ${received}/${N}`));
        }, 15000);
    }).then((received) => {
        console.log(`produced ${N}, consumed ${received}`);
    });
}

main().catch((e) => { console.error("FAIL:", e.message); process.exit(1); });
JSEOF

node test.js
'

eco_pass "librdkafka Node: produce+consume via node-rdkafka"
# @covers US-012-AC4
eco_summary
