---
dun:
  id: FEAT-005
  depends_on:
    - helix.prd
    - FEAT-001
    - FEAT-002
    - FEAT-006
---
# Feature Specification: FEAT-005 — Ecosystem Integrations

**Feature ID**: FEAT-005
**Status**: Specified
**Priority**: P0
**Owner**: heimq core

## Overview

Common Kafka-speaking systems run successfully against heimq, with at
least one tested integration each for Kafka Connect, Apache Flink,
ksqlDB, Debezium, a Schema Registry client, and librdkafka-based clients
in at least three languages. This is the practical proof that heimq is
"a Kafka the ecosystem can talk to."

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

## Requirements

### Functional Requirements

1. **Kafka Connect**: a representative source connector and a
   representative sink connector run against heimq and complete one
   end-to-end task each (e.g., file source → topic; topic → file sink).
2. **Apache Flink**: a Flink job using the Kafka source and Kafka sink
   connectors reads from one heimq topic and writes to another, with
   exactly-once configuration where supported.
3. **ksqlDB**: a ksqlDB instance configured against heimq executes a
   `CREATE STREAM` and a simple `SELECT` query, producing expected
   results.
4. **Debezium**: at least one Debezium connector (e.g., the embedded /
   PostgreSQL connector) emits CDC events into a heimq topic, and an
   rdkafka consumer reads the expected envelope.
5. **Schema Registry**: a client publishes and resolves schemas via a
   Confluent Schema Registry instance, while a producer / consumer using
   the Confluent Avro/Protobuf serializer round-trips records via heimq.
6. **librdkafka clients**: at least three languages run integration
   tests against heimq using the canonical librdkafka binding for that
   language. Recommended: Go (`confluent-kafka-go`), Python
   (`confluent-kafka`), Node.js (`node-rdkafka`).
7. Each integration target's test exits 0 and is reproducible via a
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
  / idempotency for Flink EOS, Debezium semantics).
- **External services**: Each tool's runtime / Docker image.
- **PRD requirements**: P0 #7.

## Out of Scope

- Performance comparisons against Kafka/Redpanda for these integrations.
- Tools that depend on out-of-scope APIs (e.g., share-group consumers,
  multi-broker reassignment).
