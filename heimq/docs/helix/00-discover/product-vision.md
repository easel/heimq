---
ddx:
  id: helix.product-vision
---
# Product Vision

## Mission Statement

heimq has a dual identity. As an **engine**, it is an embeddable Kafka broker
runtime — wire scaffolding and broker semantics over pluggable storage and
node-coordination traits — that downstream projects (fjord, pqueue, niflheim)
embed to get a conformant Kafka surface without writing their own wire layer.
As a **distribution**, it is the single-binary `heimq` CLI: an in-memory,
single-node, Kafka-wire-compatible broker that lets developers and test
harnesses stand in for Kafka or Redpanda without changing client code,
prioritizing fast startup and protocol fidelity over durability and scale.

## Positioning

For developers and CI systems that need a Kafka endpoint for tests, local
development, or ephemeral workloads, the **heimq distribution** is a
Kafka-compatible broker that boots fast and speaks the wire protocol unchanged.
Unlike running Kafka or Redpanda, heimq trades durability and multi-broker
features for a small, embeddable footprint with the same client surface.

For projects building Kafka-speaking systems (object-store brokers, WAL-backed
ingest paths, producer front-ends), the **heimq engine** — the
`heimq-wire`/`heimq-broker` crate family — is the shared wire and semantics
layer they embed rather than build themselves. Unlike writing a custom Kafka
wire implementation, embedding the engine provides a conformance-tested, trait-
driven foundation where durability and coordination are backend properties, not
hardcoded assumptions.

## Vision

A producer or consumer written for Apache Kafka — using any standard client
library — connects to heimq and behaves indistinguishably from one connected
to Kafka or Redpanda for the supported semantic surface (consumer groups,
transactions, idempotent producers). Developers stop maintaining mock brokers
and conditional client code paths for tests; they point at heimq.

Concurrently, projects building Kafka-speaking systems embed the heimq engine
crates and get conformance for free: the same trait families that back the
in-memory distribution are the seam their backends implement. Conformance
proves the traits — the in-memory reference broker passing standard differential
and benchmark suites, together with per-trait conformance suites passing on
consumer-shaped fixture backends, is the proof mechanism that the engine's
interfaces are correct. The engine is multi-node-capable via the pluggable
`ClusterView` coordination trait; the distribution stays single-node.

**North Star**: Any Kafka client that runs against Redpanda runs against
heimq with the same observable behavior for the in-scope semantic surface,
verified by differential tests and standard Kafka benchmarks; and any project
embedding the heimq engine crates can certify its backends by running the
per-trait conformance suites.

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
| Who (engine) | Systems engineers building Kafka-speaking infrastructure (object-store brokers, WAL-backed ingest, producer front-ends) that need a Kafka wire + semantics layer |
| Pain (engine) | Writing and maintaining a correct Kafka wire implementation is a large, undifferentiated cost; conformance is hard to verify |
| Current Solution (engine) | Roll their own wire layer or fork an existing broker |
| Why They Switch (engine) | Stable trait contracts, capability gating, per-trait conformance suites — get a certified Kafka surface without owning the wire layer |

## Key Value Propositions

| Value Proposition | Customer Benefit |
|-------------------|------------------|
| Standard Kafka clients work unchanged | No special client code or test-only adapters |
| Differential parity with Redpanda | Confidence that passing tests against heimq imply correct production behavior |
| Single-binary, in-memory operation | Fast cold start; trivially embeddable in CI |
| Coverage of consumer groups, transactions, idempotent producers | Works for the semantics most production services rely on |
| Embeddable engine with stable trait contracts | Build a Kafka-speaking system by implementing traits, not a wire protocol |
| Per-trait conformance suites | Backend authors get a structured, runnable certification path; no manual protocol inspection |
| Capability-gated advertisement | A backend that doesn't implement a trait family causes those APIs to not be advertised — safe partial implementations out of the box |
| Durability is a backend property (engine) | The distribution trades durability for simplicity; engine consumers choose their own durability model |

## Success Definition

| Metric | Target |
|--------|--------|
| Primary KPI | 100% of in-scope Kafka APIs pass differential tests against Redpanda |
| Supporting | Standard Kafka perf-test tools (kafka-producer-perf-test, kafka-consumer-perf-test) and the OpenMessaging Benchmark complete successfully against heimq |
| Supporting | Tested ecosystem integrations: Kafka Connect, Flink, ksqlDB, Debezium, Schema Registry clients, librdkafka clients in ≥3 languages |
| Supporting | Per-trait conformance suites (TopicLog, OffsetStore, GroupCoordinatorBackend, ClusterView) pass green on the in-memory reference backends and on all consumer-shaped fixture backends |
| Supporting | Three consumer projects (fjord, pqueue, niflheim) embed the engine crates and pass their per-trait suites |

## Why Now

Kafka-compatible brokers (Redpanda, WarpStream, AutoMQ) have proven the
demand for protocol-compatible alternatives, but none target the
ephemeral/test-harness niche with a single-binary, in-memory profile. The
Kafka protocol surface is stable enough to target without chasing a moving
spec.

Simultaneously, a cluster of Rust-native data systems (fjord, pqueue,
niflheim) need a Kafka wire surface but cannot justify writing and maintaining
a full Kafka wire implementation in-house. The same protocol stability that
makes the distribution viable makes a shared engine crate family viable: the
trait families are stable, the conformance story is clear, and three concrete
consumer needs satisfy the rule-of-three before the first trait is cut.
