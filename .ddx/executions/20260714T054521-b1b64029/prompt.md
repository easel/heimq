<bead-review>
  <bead id="heimq-3bfa949e" iter=1>
    <title>Cite ecosystem acceptance coverage</title>
    <description>
Resolve the ecosystem portion of AR13-05. Map every criterion in US-008, US-009, US-010, US-011, US-012, and US-014 to the eight executable ecosystem scripts and their CI steps. Add exact @covers comments only where scripts assert the outcome; strengthen script assertions for any uncovered criterion. In scope: tests/ecosystem/ and .github/workflows/ecosystem.yml. Out of scope: broker protocol changes.
    </description>
    <acceptance>
1. The HELIX alignment checker reports no uncited AC for US-008 through US-012 and US-014. 2. bash -n tests/ecosystem/*.sh passes. 3. Each cited AC has a concrete success assertion or non-zero failure path, not only a setup command.
    </acceptance>
    <labels>helix, area:testing, area:infra, kind:traceability, ac-quality:needs-refinement</labels>
  </bead>

  <changed-files>
    <file>.github/workflows/ecosystem.yml</file>
    <file>tests/ecosystem/01-librdkafka-python.sh</file>
    <file>tests/ecosystem/02-librdkafka-go.sh</file>
    <file>tests/ecosystem/03-librdkafka-node.sh</file>
    <file>tests/ecosystem/04-schema-registry.sh</file>
    <file>tests/ecosystem/05-kafka-connect.sh</file>
    <file>tests/ecosystem/06-ksqldb.sh</file>
    <file>tests/ecosystem/07-debezium.sh</file>
    <file>tests/ecosystem/08-flink.sh</file>
  </changed-files>

  <governing>
    <ref id="FEAT-005" path="crates/heimq/docs/helix/01-frame/features/FEAT-005-ecosystem-integrations.md" title="Feature Specification: FEAT-005 — Ecosystem Integrations">
      <content>
<untrusted-data>
---
ddx:
  id: FEAT-005
  depends_on:
    - helix.prd
    - FEAT-001
    - FEAT-002
    - FEAT-006
  review:
    self_hash: c8b17ef2e79181581f03413860ea3e074364440af280321171b639668cb3aebc
    deps:
      FEAT-001: 7133c264bc364ec4535c1d6b6187a90c9ba66d1fa3df30731ade260c2e092479
      FEAT-002: 164350929c7bbc09a589f3cd1a80b685e88cce1054445fe5373aec566464636f
      FEAT-006: e59d3b8965ebd35b4bbe9c5302f4218432ad83ec27691989dfd4c345ac2ae004
      helix.prd: 96f0479e307f2c240e8f47b69fff510164d0b9eda132abb22cc4a860932984fe
    reviewed_at: "2026-07-14T05:12:26Z"
---
# Feature Specification: FEAT-005 — Ecosystem Integrations

**Feature ID**: FEAT-005
**Status**: Specified
**Priority**: P0
**Owner**: heimq core
**Covered PRD Subsystem(s)**: Ecosystem integrations (FEAT-005)
**Covered PRD Requirements**: FR-10 (ecosystem integration coverage) — PRD P0 #7
**Cross-Subsystem Rationale**: None — single subsystem.

## Overview

Common Kafka-speaking systems run successfully against heimq, with at
least one tested integration each for Kafka Connect, Apache Flink,
ksqlDB, Debezium, a Schema Registry client, and librdkafka-based clients
in at least three languages. This is the practical proof that heimq is
"a Kafka the ecosystem can talk to."

## Ideal Future State

A user points a Kafka Connect connector, a Flink job, a ksqlDB query, a
Debezium connector, a Schema Registry serializer, or a librdkafka binding
in Go, Python, or Node.js at heimq, and the tool's primary use case works
— each demonstrated by a canonical, reproducible example that runs via a
single script per target.

## Problem Statement

- **Current situation**: heimq is tested against rdkafka in Rust only.
  The wider Kafka ecosystem (Connect, Flink, ksqlDB, Debezium, Schema
  Registry, multi-language librdkafka clients) is untested.
- **Pain points**: Wire-protocol contract tests cannot anticipate the
  full surface these tools exercise (admin metadata refresh patterns,
  consumer-group session timing, transactional sinks, header
  conventions). Without integration tests, users discover
  incompatibilities only when they try a real tool.
- **Desired outcome**: A canonical example for each target tool runs
  successfully against heimq and demonstrates the tool's primary use
  case (e.g., a Debezium connector emits CDC events into a heimq topic
  that an rdkafka consumer reads).

## Functional Areas

| Area | User question or job | Feature responsibility |
|------|----------------------|------------------------|
| Kafka Connect | Run source and sink connectors against heimq | A representative source task and sink task each complete end-to-end |
| Apache Flink | Run a Kafka source + sink job against heimq | Job reads from one heimq topic and writes to another, exactly-once where supported |
| ksqlDB | Run stream queries over heimq topics | `CREATE STREAM` and a simple `SELECT` produce expected results |
| Debezium | Emit CDC events into heimq | One connector emits the expected CDC envelope, readable by an rdkafka consumer |
| Schema Registry | Round-trip schema-serialized records via heimq | Confluent serializer publishes/resolves schemas; producer/consumer round-trip works |
| librdkafka clients | Use the canonical librdkafka binding in ≥3 languages | Integration tests pass for Go, Python, and Node.js bindings |

## Requirements

### Functional Requirements by Area

- **KC-01** — **Kafka Connect**: a representative source connector and a
  representative sink connector run against heimq and complete one
  end-to-end task each (e.g., file source → topic; topic → file sink).
- **FL-01** — **Apache Flink**: a Flink job using the Kafka source and Kafka sink
  connectors reads from one heimq topic and writes to another, with
  exactly-once configuration where supported.
- **KS-01** — **ksqlDB**: a ksqlDB instance configured against heimq executes a
  `CREATE STREAM` and a simple `SELECT` query, producing expected
  results.
- **DB-01** — **Debezium**: at least one Debezium connector (e.g., the embedded /
  PostgreSQL connector) emits CDC events into a heimq topic, and an
  rdkafka consumer reads the expected envelope.
- **SR-01** — **Schema Registry**: a client publishes and resolves schemas via a
  Confluent Schema Registry instance, while a producer / consumer using
  the Confluent Avro/Protobuf serializer round-trips records via heimq.
- **LC-01** — **librdkafka clients**: at least three languages run integration
  tests against heimq using the canonical librdkafka binding for that
  language. Recommended: Go (`confluent-kafka-go`), Python
  (`confluent-kafka`), Node.js (`node-rdkafka`).
- **INT-01** — Each integration target's test exits 0 and is reproducible via a
  single script per target.

### Non-Functional Requirements

- **Reliability**: Each integration test pass rate ≥ 99% on its gating
  profile.
- **Reproducibility**: Pinned tool versions per integration; one script
  per integration that brings up dependencies, runs the test, tears
  down.

## User Stories

- [US-008 — Kafka Connect against heimq](../user-stories/US-008-kafka-connect.md)
- [US-009 — Flink Kafka source/sink against heimq](../user-stories/US-009-flink.md)
- [US-014 — ksqlDB against heimq](../user-stories/US-014-ksqldb.md)
- [US-010 — Debezium emits CDC into heimq](../user-stories/US-010-debezium.md)
- [US-011 — Schema Registry round-trip against heimq](../user-stories/US-011-schema-registry.md)
- [US-012 — Multi-language librdkafka clients against heimq](../user-stories/US-012-multi-language-clients.md)

## Edge Cases and Error Handling

- **Tool requires APIs heimq does not implement**: parking-lot the
  required API; either implement a minimal stub or document the
  non-support and exclude that tool from the matrix with a written
  rationale.
- **Tool assumes durability**: document as a deliberate divergence per
  PRD non-goal #1.
- **Tool depends on Confluent-only APIs**: scope decision recorded in
  the integration's README and the parking lot.

## Success Metrics

- Each of the six integration targets has at least one passing test in
  CI.
- The integration matrix is documented in the test plan with each
  target's status (green / yellow / parked).

## Constraints and Assumptions

- Integration test environment can run JVM-based tools (Connect, Flink,
  ksqlDB) and language runtimes for librdkafka bindings.
- Schema Registry target is the Confluent Schema Registry API (PRD
  resolved decision).

## Dependencies

- **Other features**: FEAT-001 (wire protocol), FEAT-002 (transactions
  / idempotency for Flink EOS, Debezium semantics), FEAT-006
  (flexible-version protocol — ecosystem tools negotiate modern
  flexible-only API versions; see PRD P0 #1).
- **External services**: Each tool's runtime / Docker image.
- **PRD requirements**: P0 #7.

## Out of Scope

- Performance comparisons against Kafka/Redpanda for these integrations.
- Tools that depend on out-of-scope APIs (e.g., share-group consumers,
  multi-broker reassignment).
</untrusted-data>
      </content>
    </ref>
  </governing>

  <diff rev="1e85f9a1f11d84b150df3037adc6d450ea2bed63">
<untrusted-data>
diff --git a/.github/workflows/ecosystem.yml b/.github/workflows/ecosystem.yml
index bc6ef26..e281158 100644
--- a/.github/workflows/ecosystem.yml
+++ b/.github/workflows/ecosystem.yml
@@ -31,12 +31,18 @@ jobs:
           timeout 15 bash -c 'until nc -z localhost 9094; do sleep 0.2; done'
 
       - name: Test Python confluent-kafka
+        # @covers US-012-AC2
+        # @covers US-012-AC4
         run: BOOTSTRAP=localhost:9094 bash tests/ecosystem/01-librdkafka-python.sh
 
       - name: Test Go confluent-kafka-go
+        # @covers US-012-AC1
+        # @covers US-012-AC4
         run: BOOTSTRAP=localhost:9094 bash tests/ecosystem/02-librdkafka-go.sh
 
       - name: Test Node node-rdkafka
+        # @covers US-012-AC3
+        # @covers US-012-AC4
         run: BOOTSTRAP=localhost:9094 bash tests/ecosystem/03-librdkafka-node.sh
 
       - name: Stop heimq
@@ -62,12 +68,23 @@ jobs:
           timeout 15 bash -c 'until nc -z localhost 9094; do sleep 0.2; done'
 
       - name: Test Schema Registry
+        # @covers US-011-AC1
+        # @covers US-011-AC2
+        # @covers US-011-AC3
+        # @covers US-011-AC4
         run: BOOTSTRAP=localhost:9094 bash tests/ecosystem/04-schema-registry.sh
 
       - name: Test Kafka Connect (source + sink)
+        # @covers US-008-AC1
+        # @covers US-008-AC2
+        # @covers US-008-AC3
+        # @covers US-008-AC4
         run: BOOTSTRAP=localhost:9094 bash tests/ecosystem/05-kafka-connect.sh
 
       - name: Test ksqlDB
+        # @covers US-014-AC1
+        # @covers US-014-AC2
+        # @covers US-014-AC3
         run: BOOTSTRAP=localhost:9094 bash tests/ecosystem/06-ksqldb.sh
 
       - name: Stop heimq
@@ -93,9 +110,15 @@ jobs:
           timeout 15 bash -c 'until nc -z localhost 9094; do sleep 0.2; done'
 
       - name: Test Debezium PostgreSQL CDC
+        # @covers US-010-AC1
+        # @covers US-010-AC2
+        # @covers US-010-AC3
         run: BOOTSTRAP=localhost:9094 bash tests/ecosystem/07-debezium.sh
 
       - name: Test Apache Flink (Kafka source + sink, EOS)
+        # @covers US-009-AC1
+        # @covers US-009-AC2
+        # @covers US-009-AC3
         run: BOOTSTRAP=localhost:9094 bash tests/ecosystem/08-flink.sh
 
       - name: Stop heimq
diff --git a/tests/ecosystem/01-librdkafka-python.sh b/tests/ecosystem/01-librdkafka-python.sh
index 67f4360..de24fdd 100755
--- a/tests/ecosystem/01-librdkafka-python.sh
+++ b/tests/ecosystem/01-librdkafka-python.sh
@@ -44,7 +44,8 @@ for i in range(n):
     p.produce(topic, f"msg-{i}".encode())
 p.flush(timeout=10)
 
-# Consume
+# @covers US-012-AC2
+# Consume and require every produced message to come back through confluent-kafka.
 c = Consumer({
     "bootstrap.servers": bootstrap,
     "group.id": "eco-python-test",
@@ -75,4 +76,5 @@ PYEOF
 '
 
 eco_pass "librdkafka Python: produce+consume via confluent-kafka"
+# @covers US-012-AC4
 eco_summary
diff --git a/tests/ecosystem/02-librdkafka-go.sh b/tests/ecosystem/02-librdkafka-go.sh
index 303f2a2..cb36509 100755
--- a/tests/ecosystem/02-librdkafka-go.sh
+++ b/tests/ecosystem/02-librdkafka-go.sh
@@ -79,7 +79,8 @@ func main() {
 	p.Flush(10000)
 	p.Close()
 
-	// Consume
+	// @covers US-012-AC1
+	// Consume and require every produced message to come back through confluent-kafka-go.
 	c, err := kafka.NewConsumer(&kafka.ConfigMap{
 		"bootstrap.servers":  bootstrap,
 		"group.id":           "eco-go-test",
@@ -118,4 +119,5 @@ go run main.go
 '
 
 eco_pass "librdkafka Go: produce+consume via confluent-kafka-go"
+# @covers US-012-AC4
 eco_summary
diff --git a/tests/ecosystem/03-librdkafka-node.sh b/tests/ecosystem/03-librdkafka-node.sh
index 599e57d..088a524 100755
--- a/tests/ecosystem/03-librdkafka-node.sh
+++ b/tests/ecosystem/03-librdkafka-node.sh
@@ -50,7 +50,8 @@ async function main() {
         producer.on("event.error", reject);
     });
 
-    // Consume
+    // @covers US-012-AC3
+    // Consume and require every produced message to come back through node-rdkafka.
     await new Promise((resolve, reject) => {
         const consumer = new Kafka.KafkaConsumer({
             "metadata.broker.list": bootstrap,
@@ -86,4 +87,5 @@ node test.js
 '
 
 eco_pass "librdkafka Node: produce+consume via node-rdkafka"
+# @covers US-012-AC4
 eco_summary
diff --git a/tests/ecosystem/04-schema-registry.sh b/tests/ecosystem/04-schema-registry.sh
index 1974517..37ff45a 100755
--- a/tests/ecosystem/04-schema-registry.sh
+++ b/tests/ecosystem/04-schema-registry.sh
@@ -51,7 +51,8 @@ RESPONSE=$(curl -sf -X POST "${SR_URL}/subjects/${TOPIC}-value/versions" \
 echo "  schema registered: $RESPONSE"
 SCHEMA_ID=$(echo "$RESPONSE" | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
 
-# Retrieve schema back and verify
+# @covers US-011-AC1
+# Retrieve schema back and verify that registration resolved to a concrete schema id.
 RETRIEVED=$(curl -sf "${SR_URL}/subjects/${TOPIC}-value/versions/latest")
 echo "  schema retrieved: $RETRIEVED"
 
@@ -62,5 +63,73 @@ if [ -z "$RETRIEVED_SCHEMA" ]; then
     exit 1
 fi
 
+# @covers US-011-AC2
+# @covers US-011-AC3
+# Produce and consume through Confluent's registry-aware Avro serializer/deserializer.
+docker run --rm \
+    -e BOOTSTRAP="$DOCKER_BOOTSTRAP" \
+    -e SCHEMA_REGISTRY_URL="$SR_URL" \
+    -e TOPIC="$TOPIC" \
+    python:3.11-slim bash -c '
+pip install --quiet "confluent-kafka[avro]"
+python3 - << '"'"'PYEOF'"'"'
+import os, sys, time
+from confluent_kafka import KafkaError
+from confluent_kafka.admin import AdminClient, NewTopic
+from confluent_kafka.avro import AvroConsumer, AvroProducer, loads
+
+bootstrap = os.environ["BOOTSTRAP"]
+registry_url = os.environ["SCHEMA_REGISTRY_URL"]
+topic = os.environ["TOPIC"]
+expected = {"value": "schema-registry-ok"}
+schema = loads("""{"type":"record","name":"Msg","fields":[{"name":"value","type":"string"}]}""")
+
+admin = AdminClient({"bootstrap.servers": bootstrap})
+fs = admin.create_topics([NewTopic(topic, num_partitions=1, replication_factor=1)])
+for f in fs.values():
+    try:
+        f.result()
+    except Exception:
+        pass
+
+producer = AvroProducer(
+    {"bootstrap.servers": bootstrap, "schema.registry.url": registry_url},
+    default_value_schema=schema,
+)
+producer.produce(topic=topic, value=expected)
+producer.flush(10)
+
+consumer = AvroConsumer({
+    "bootstrap.servers": bootstrap,
+    "schema.registry.url": registry_url,
+    "group.id": "eco-sr-verify",
+    "auto.offset.reset": "earliest",
+    "enable.auto.commit": "false",
+})
+consumer.subscribe([topic])
+deadline = time.time() + 20
+while time.time() < deadline:
+    msg = consumer.poll(1.0)
+    if msg is None:
+        continue
+    if msg.error():
+        if msg.error().code() != KafkaError._PARTITION_EOF:
+            print(f"consumer error: {msg.error()}", file=sys.stderr)
+            sys.exit(1)
+        continue
+    if msg.value() == expected:
+        print("registry serializer/deserializer round-trip ok")
+        consumer.close()
+        sys.exit(0)
+    print(f"FAIL: decoded {msg.value()!r}, expected {expected!r}", file=sys.stderr)
+    consumer.close()
+    sys.exit(1)
+consumer.close()
+print("FAIL: no registry-serialized record within 20s", file=sys.stderr)
+sys.exit(1)
+PYEOF
+'
+
 eco_pass "Schema Registry: registered and retrieved Avro schema (id=$SCHEMA_ID)"
+# @covers US-011-AC4
 eco_summary
diff --git a/tests/ecosystem/05-kafka-connect.sh b/tests/ecosystem/05-kafka-connect.sh
index 2767a9a..c2a65fc 100755
--- a/tests/ecosystem/05-kafka-connect.sh
+++ b/tests/ecosystem/05-kafka-connect.sh
@@ -110,7 +110,8 @@ SOURCE_RESP=$(post_connector_config "source connector" "{
 }")
 echo "  source connector: $SOURCE_RESP"
 
-# Wait for source connector to enter RUNNING state
+# @covers US-008-AC1
+# Wait for the source connector to enter RUNNING state after writing source data.
 DEADLINE=$((SECONDS + 30))
 STATUS=""
 while [ $SECONDS -lt $DEADLINE ]; do
@@ -138,7 +139,8 @@ SINK_RESP=$(post_connector_config "sink connector" "{
 }")
 echo "  sink connector: $SINK_RESP"
 
-# Wait for sink to produce output (at least 10 lines)
+# @covers US-008-AC2
+# Wait for the sink connector to consume the source records from heimq.
 DEADLINE=$((SECONDS + 30))
 SINK_LINES=0
 while [ $SECONDS -lt $DEADLINE ]; do
@@ -154,4 +156,6 @@ if [ "$SINK_LINES" -lt 10 ]; then
 fi
 
 eco_pass "Kafka Connect: source+sink via heimq ($SINK_LINES lines)"
+# @covers US-008-AC3
+# @covers US-008-AC4
 eco_summary
diff --git a/tests/ecosystem/06-ksqldb.sh b/tests/ecosystem/06-ksqldb.sh
index d197d8a..1dd753b 100755
--- a/tests/ecosystem/06-ksqldb.sh
+++ b/tests/ecosystem/06-ksqldb.sh
@@ -67,14 +67,29 @@ if ! wait_for_http "${KSQL_URL}/info" 90; then
 fi
 echo "  ksqlDB is up"
 
-# Create a stream over the topic
+# @covers US-014-AC1
+# Create a stream over the topic and require the REST call to succeed.
 STREAM_NAME="ECO_STREAM_${RUN_ID}"
 CREATE_RESP=$(curl -sf -X POST "${KSQL_URL}/ksql" \
     -H "Content-Type: application/vnd.ksql.v1+json" \
     -d "{\"ksql\": \"CREATE STREAM ${STREAM_NAME} (val VARCHAR) WITH (kafka_topic='${TOPIC}', value_format='KAFKA', partitions=1);\", \"streamsProperties\": {}}")
 echo "  CREATE STREAM: $(echo "$CREATE_RESP" | head -c 200)"
 
-# Verify ksqlDB server status
+# @covers US-014-AC2
+# Run a SELECT over the stream and require one of the seeded values to appear.
+QUERY_RESP=$(curl -sf -X POST "${KSQL_URL}/query-stream" \
+    -H "Content-Type: application/vnd.ksqlapi.delimited.v1" \
+    -d "{\"sql\": \"SELECT VAL FROM ${STREAM_NAME} EMIT CHANGES LIMIT 10;\", \"properties\": {\"auto.offset.reset\": \"earliest\"}}" \
+    | tee /tmp/heimq-ksql-query-"${RUN_ID}".log)
+echo "  SELECT result: $(echo "$QUERY_RESP" | grep -E "event-[0-9]" | head -1 || true)"
+
+if ! echo "$QUERY_RESP" | grep -q "event-"; then
+    echo "FAIL: SELECT did not return seeded event values" >&2
+    cat /tmp/heimq-ksql-query-"${RUN_ID}".log >&2
+    exit 1
+fi
+
+# Verify ksqlDB server status.
 SERVER_INFO=$(curl -sf "${KSQL_URL}/info")
 STATUS=$(echo "$SERVER_INFO" | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d.get("KsqlServerInfo", {}).get("serverStatus", ""))' 2>/dev/null || echo "unknown")
 echo "  ksqlDB serverStatus=$STATUS"
@@ -84,4 +99,5 @@ if [ "$STATUS" = "RUNNING" ]; then
 else
     eco_fail "ksqlDB: serverStatus=$STATUS (expected RUNNING)"
 fi
+# @covers US-014-AC3
 eco_summary
diff --git a/tests/ecosystem/07-debezium.sh b/tests/ecosystem/07-debezium.sh
index 0bc94df..e149870 100755
--- a/tests/ecosystem/07-debezium.sh
+++ b/tests/ecosystem/07-debezium.sh
@@ -147,7 +147,9 @@ echo "  connector RUNNING; inserting test row..."
 docker exec "$PG_CID" psql -U debezium -d inventory -c \
     "INSERT INTO orders (item, quantity) VALUES ('widget', 42);" >/dev/null
 
-# Consume the CDC event from heimq using Python
+# @covers US-010-AC1
+# @covers US-010-AC2
+# Consume the CDC event from heimq and require the inserted row to be present.
 CDC_TOPIC="eco-dbz-${RUN_ID}.public.orders"
 RECEIVED=$(docker run --rm \
     -e BOOTSTRAP="$DOCKER_BOOTSTRAP" \
@@ -173,7 +175,12 @@ while time.time() < deadline:
         if msg.error().code() != KafkaError._PARTITION_EOF:
             print(\"consumer error:\", msg.error(), file=sys.stderr)
         continue
-    print(\"received CDC event:\", msg.value()[:80] if msg.value() else \"(tombstone)\")
+    value = msg.value() or b\"\"
+    if b\"widget\" not in value or b\"42\" not in value:
+        print(\"FAIL: CDC event did not contain inserted row:\", value[:200], file=sys.stderr)
+        c.close()
+        sys.exit(1)
+    print(\"received CDC event:\", value[:80])
     c.close()
     sys.exit(0)
 c.close()
@@ -188,4 +195,5 @@ else
     eco_pass "Debezium CDC: captured PostgreSQL row change via heimq"
     echo "  $RECEIVED"
 fi
+# @covers US-010-AC3
 eco_summary
diff --git a/tests/ecosystem/08-flink.sh b/tests/ecosystem/08-flink.sh
index b63d277..22c48f4 100755
--- a/tests/ecosystem/08-flink.sh
+++ b/tests/ecosystem/08-flink.sh
@@ -93,18 +93,62 @@ if [ "$SLOTS" -lt 1 ]; then
 fi
 echo "  TaskManager registered ($SLOTS slots available)"
 
-# Run Flink SQL: copy SRC → SINK with exactly-once semantics
-# Execute SQL inside the JobManager container (which has sql-client.sh)
-SQL_JOB="CREATE TABLE src (\`value\` BYTES) WITH ('connector' = 'kafka', 'topic' = '${SRC_TOPIC}', 'properties.bootstrap.servers' = '${DOCKER_BOOTSTRAP}', 'properties.group.id' = 'eco-flink-src', 'scan.startup.mode' = 'earliest-offset', 'value.format' = 'raw');
+# @covers US-009-AC1
+# @covers US-009-AC2
+# Run Flink SQL: copy SRC to SINK with exactly-once semantics and checkpoints enabled.
+SQL_JOB="SET 'execution.checkpointing.interval' = '1s';
+SET 'execution.checkpointing.mode' = 'EXACTLY_ONCE';
+CREATE TABLE src (\`value\` BYTES) WITH ('connector' = 'kafka', 'topic' = '${SRC_TOPIC}', 'properties.bootstrap.servers' = '${DOCKER_BOOTSTRAP}', 'properties.group.id' = 'eco-flink-src', 'scan.startup.mode' = 'earliest-offset', 'value.format' = 'raw');
 CREATE TABLE sink (\`value\` BYTES) WITH ('connector' = 'kafka', 'topic' = '${SINK_TOPIC}', 'properties.bootstrap.servers' = '${DOCKER_BOOTSTRAP}', 'properties.transaction.timeout.ms' = '30000', 'sink.delivery-guarantee' = 'exactly-once', 'sink.transactional-id-prefix' = 'eco-flink-eos', 'value.format' = 'raw');
-INSERT INTO sink SELECT \`value\` FROM src LIMIT 20;"
+INSERT INTO sink SELECT \`value\` FROM src;"
 
-JOB_OUTPUT=$(docker exec "$JM_CID" bash -c "
+docker exec "$JM_CID" bash -c "
 printf '%s\n' '${SQL_JOB//\'/\'\\\'\'}' > /tmp/job.sql
-timeout 60 /opt/flink/bin/sql-client.sh -f /tmp/job.sql 2>&1 || true
-" 2>&1)
+nohup /opt/flink/bin/sql-client.sh -f /tmp/job.sql >/tmp/job.out 2>&1 &
+" 2>&1
 
-echo "  Flink SQL output: $(echo "$JOB_OUTPUT" | grep -v "^$" | tail -5)"
+DEADLINE=$((SECONDS + 60))
+JOB_ID=""
+while [ $SECONDS -lt $DEADLINE ]; do
+    JOB_ID=$(curl -sf "${FLINK_URL}/jobs/overview" 2>/dev/null \
+        | python3 -c 'import json,sys; jobs=json.load(sys.stdin).get("jobs", []); print(next((j["jid"] for j in jobs if j.get("state") == "RUNNING"), ""))' 2>/dev/null || true)
+    [ -n "$JOB_ID" ] && break
+    if docker exec "$JM_CID" grep -qiE "exception|error" /tmp/job.out 2>/dev/null; then
+        echo "FAIL: Flink SQL job failed to start" >&2
+        docker exec "$JM_CID" tail -80 /tmp/job.out >&2 || true
+        exit 1
+    fi
+    sleep 2
+done
+
+if [ -z "$JOB_ID" ]; then
+    echo "FAIL: no running Flink SQL job appeared within 60s" >&2
+    docker exec "$JM_CID" tail -80 /tmp/job.out >&2 || true
+    exit 1
+fi
+echo "  Flink SQL job running: $JOB_ID"
+
+DEADLINE=$((SECONDS + 90))
+COMPLETED_CHECKPOINTS=0
+while [ $SECONDS -lt $DEADLINE ]; do
+    COMPLETED_CHECKPOINTS=$(curl -sf "${FLINK_URL}/jobs/${JOB_ID}/checkpoints" 2>/dev/null \
+        | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d.get("counts", {}).get("completed", 0))' 2>/dev/null || echo 0)
+    [ "$COMPLETED_CHECKPOINTS" -ge 1 ] && break
+    if docker exec "$JM_CID" grep -qiE "exception|error" /tmp/job.out 2>/dev/null; then
+        echo "FAIL: Flink SQL job failed before completing a checkpoint" >&2
+        docker exec "$JM_CID" tail -80 /tmp/job.out >&2 || true
+        exit 1
+    fi
+    sleep 2
+done
+
+if [ "$COMPLETED_CHECKPOINTS" -lt 1 ]; then
+    echo "FAIL: Flink job completed $COMPLETED_CHECKPOINTS checkpoints (expected >= 1)" >&2
+    curl -sf "${FLINK_URL}/jobs/${JOB_ID}/checkpoints" >&2 || true
+    docker exec "$JM_CID" tail -80 /tmp/job.out >&2 || true
+    exit 1
+fi
+echo "  completed checkpoints: $COMPLETED_CHECKPOINTS"
 
 # Verify data arrived in sink topic
 SINK_COUNT=$(docker run --rm \
@@ -143,9 +187,9 @@ echo "  sink topic received $SINK_COUNT messages"
 if [ "$SINK_COUNT" -ge 20 ]; then
     eco_pass "Flink: Kafka source → sink (EOS) via heimq ($SINK_COUNT messages)"
 else
-    # Partial pass: Flink cluster started successfully, SQL job submitted.
-    # If Kafka SQL connector JAR is missing, the job won't run but the cluster is verified.
-    echo "  NOTE: Flink cluster verified. Kafka connector JAR may need to be on classpath for full EOS test."
-    eco_pass "Flink: JobManager + TaskManager cluster started against heimq ($SINK_COUNT messages in sink)"
+    echo "FAIL: Flink sink received $SINK_COUNT messages (expected >= 20)" >&2
+    docker exec "$JM_CID" tail -80 /tmp/job.out >&2 || true
+    exit 1
 fi
+# @covers US-009-AC3
 eco_summary
</untrusted-data>
  </diff>

  <strictness-mode mode="strict">strict — each AC must be anchored to a named Test* function or a diff-touched symbol; file-only evidence is insufficient.</strictness-mode>

  <instructions>
You are reviewing a bead implementation against its acceptance criteria.

## AC-Check Ratification

When an &lt;ac-check&gt; section is present, ratify the mechanical results rather
than re-verifying them independently from the diff:

- result="pass": confirm the evidence is credible. Override to fail only if
  the evidence is fabricated — include judgment_override_reason and a diff
  citation (file:line) in the per_ac evidence string.
- result="fail": mechanically verified failure. Grade as fail and BLOCK unless
  the commit message contains an explicit AC-Waive trailer for this AC.
- result="needs_judgment": adjudicate from the diff. If you cannot determine
  pass/fail without additional bead context from the operator, use
  REQUEST_CLARIFICATION for that AC item.
- result="error": treat as needs_judgment.

Overriding a mechanical grade (pass→fail or fail→pass) requires an explicit
judgment_override_reason note and a concrete diff citation in the evidence.

## Strictness Mode

The &lt;strictness-mode&gt; tag specifies per-bead evidence requirements:

- strict (kind:fix, kind:feat): each AC must be anchored to a named Test*
  function or a diff-touched symbol; file-only evidence is insufficient.
- behavior-light (kind:refactor, kind:chore): build green plus file/symbol
  evidence suffices; test-name match required only when an AC explicitly
  names a Test* function.
- mechanical (kind:doc, kind:mechanical): file presence, renames, or symbol
  evidence only; no test-name or runtime evidence required.

## Verdicts

For each acceptance-criteria (AC) item, decide whether it is implemented
correctly, then assign one overall verdict:

- APPROVE — every AC item is fully and correctly implemented.
- REQUEST_CHANGES — some AC items are partial or have fixable minor issues.
- BLOCK — at least one AC item is not implemented or incorrectly implemented;
  or the diff is insufficient to evaluate.
- REQUEST_CLARIFICATION — you cannot adjudicate one or more needs_judgment AC
  items without operator clarification. Use this ONLY when the item is
  ambiguous even given the full diff. This verdict does NOT block the queue;
  it routes to the operator lane for input.

## Required output format (schema_version: 1)

Respond with EXACTLY one JSON object as your final response, fenced as a single ```json … ``` code block. Do not include any prose outside the fenced block. The JSON must match this schema:

```json
{
  "schema_version": 1,
  "verdict": "APPROVE",
  "summary": "≤300 char human-readable verdict justification",
  "per_ac": [
    { "number": 1, "item": "acceptance criterion text", "grade": "pass", "evidence": "file:line or test evidence" }
  ],
  "findings": [
    { "severity": "info", "summary": "what is wrong or notable", "location": "path/to/file.go:42" }
  ]
}
```

Rules:
- "verdict" must be exactly one of "APPROVE", "REQUEST_CHANGES", "BLOCK", "REQUEST_CLARIFICATION".
- "severity" must be exactly one of "info", "warn", "block".
- Output the JSON object inside ONE fenced ```json … ``` block. No additional prose, no extra fences, no markdown headings.
- Do not echo this template back. Do not write the verdict value anywhere except as the JSON value of the verdict field.
  </instructions>
</bead-review>
