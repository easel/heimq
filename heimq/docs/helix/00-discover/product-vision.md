# Product Vision

## Mission Statement

heimq is a single-node, Kafka-wire-compatible broker that lets developers and
test harnesses stand in for Kafka or Redpanda without changing client code,
prioritizing fast startup and protocol fidelity over durability and scale.

## Positioning

For developers and CI systems that need a Kafka endpoint for tests, local
development, or ephemeral workloads, heimq is a Kafka-compatible broker that
boots fast and speaks the wire protocol unchanged.
Unlike running Kafka or Redpanda, heimq trades durability and multi-broker
features for a small, embeddable footprint with the same client surface.

## Vision

A producer or consumer written for Apache Kafka — using any standard client
library — connects to heimq and behaves indistinguishably from one connected
to Kafka or Redpanda for the supported semantic surface (consumer groups,
transactions, idempotent producers). Developers stop maintaining mock brokers
and conditional client code paths for tests; they point at heimq.

**North Star**: Any Kafka client that runs against Redpanda runs against
heimq with the same observable behavior for the in-scope semantic surface,
verified by differential tests and standard Kafka benchmarks.

## User Experience

A developer writes a service that produces to a Kafka topic with idempotent
producers and reads via a consumer group with transactional commits. They
launch heimq instead of `docker compose up redpanda`, point their client at
`localhost:9092`, and the service runs. Their CI pipeline runs the same tests
against heimq for speed and against Redpanda for parity; both pass. When they
add a Debezium connector, Flink job, or Schema Registry client, those work
too.

## Target Market

| Attribute | Description |
|-----------|-------------|
| Who | Backend developers, platform/SRE teams, CI maintainers building or testing Kafka-using services |
| Pain | Real Kafka/Redpanda is heavy for tests; mocks drift from real broker behavior |
| Current Solution | Dockerized Kafka/Redpanda, ad-hoc broker mocks, embedded Kafka |
| Why They Switch | Faster startup, smaller footprint, no JVM, identical wire-protocol semantics for tested surface |

## Key Value Propositions

| Value Proposition | Customer Benefit |
|-------------------|------------------|
| Standard Kafka clients work unchanged | No special client code or test-only adapters |
| Differential parity with Redpanda | Confidence that passing tests against heimq imply correct production behavior |
| Single-binary, in-memory operation | Fast cold start; trivially embeddable in CI |
| Coverage of consumer groups, transactions, idempotent producers | Works for the semantics most production services rely on |

## Success Definition

| Metric | Target |
|--------|--------|
| Primary KPI | 100% of in-scope Kafka APIs pass differential tests against Redpanda |
| Supporting | Standard Kafka perf-test tools (kafka-producer-perf-test, kafka-consumer-perf-test) and the OpenMessaging Benchmark complete successfully against heimq |
| Supporting | Tested ecosystem integrations: Kafka Connect, Flink, ksqlDB, Debezium, Schema Registry clients, librdkafka clients in ≥3 languages |

## Why Now

Kafka-compatible brokers (Redpanda, WarpStream, AutoMQ) have proven the
demand for protocol-compatible alternatives, but none target the
ephemeral/test-harness niche with a single-binary, in-memory profile. The
Kafka protocol surface is stable enough to target without chasing a moving
spec.
