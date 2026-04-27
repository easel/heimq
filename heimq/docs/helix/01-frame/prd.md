---
dun:
  id: helix.prd
---
# Product Requirements Document

## Summary

heimq is a single-node, Kafka-wire-compatible broker for tests, local
development, and ephemeral workloads. Standard Kafka producers and consumers
connect unchanged and observe the same behavior they would against Kafka or
Redpanda for the in-scope semantic surface: consumer groups, transactional
groups (EOS), and idempotent producers. heimq is permitted to lose data on
restart and to offer limited retention; it is not a durable, distributed
broker. Success is measured by differential parity against Redpanda, by
passing standard Kafka benchmarks, and by working with common Kafka-speaking
ecosystem tools.

## Problem and Goals

### Problem

Teams testing or developing services against Kafka either spin up
Kafka/Redpanda (heavy, slow cold start, JVM dependency for Kafka) or use
broker mocks that drift from real protocol behavior. There is no
single-binary, in-memory broker that speaks the Kafka wire protocol
faithfully enough that tests passing against it imply tests will pass against
production Kafka — including the semantics services actually rely on
(consumer groups, transactions, idempotent producers).

### Goals

1. Standard Kafka clients connect to heimq and exchange data identically to
   how they would against Apache Kafka or Redpanda, with no client code
   changes.
2. The semantic surface that production services depend on — consumer
   groups, transactional groups (EOS), idempotent producers — works
   correctly.
3. Behavior is verified against Redpanda (and/or Kafka) via differential
   tests, and against standard Kafka benchmarks and ecosystem tools, so
   parity is observable rather than asserted.

### Success Metrics

| Metric | Target | Measurement Method |
|--------|--------|--------------------|
| Differential parity with Redpanda for in-scope APIs | 100% of in-scope APIs pass diff tests | Differential test harness in `tests/parity/` running same workload against heimq and Redpanda |
| Standard Kafka benchmark conformance | `kafka-producer-perf-test` and `kafka-consumer-perf-test` complete with no errors at documented load; OpenMessaging Benchmark runs to completion | Bench harness scripts in `scripts/bench/` |
| Ecosystem integration coverage | ≥1 tested integration each for: Kafka Connect, Flink, ksqlDB, Debezium, a Schema Registry client, librdkafka in ≥3 languages | Integration test suite in `tests/ecosystem/` |
| Wire-protocol contract coverage | 100% of in-scope APIs covered by contract tests | `tests/contract.rs` + per-API contract files |

### Non-Goals

The following are explicit non-goals — heimq does not aim to do these:

1. **Durability across restarts.** heimq may lose all in-memory state on
   process restart. Persistent backends (e.g., Postgres for offsets) may
   exist as opt-in slices but are not the default and are not required for
   correctness of in-scope features.
2. **Long retention.** Retention is bounded by available memory and any
   configured caps. Multi-day or multi-TB retention is out of scope.
3. **Multi-broker / replication / KRaft / controller responsibilities.**
   heimq is single-node by design.
4. **Security / SASL / ACLs / delegation tokens.** Out of scope for the
   current PRD.
5. **Share groups, telemetry APIs, admin reassignment APIs.** Out of scope.
6. **Performance at production-Kafka scale.** heimq must *complete* standard
   benchmarks correctly; it does not aim to match Kafka/Redpanda throughput.

Deferred items tracked in `docs/helix/parking-lot.md`.

## Users and Scope

### Primary Persona: Backend Developer / Test Author

**Role**: Engineer writing or testing a service that produces to or consumes
from Kafka.
**Goals**: Run end-to-end tests against a real Kafka wire surface without
spinning up Kafka/Redpanda; iterate locally on a laptop.
**Pain Points**: Container startup latency for Kafka/Redpanda; mock brokers
drift from real semantics; transactions and idempotent producers usually
unsupported in lightweight alternatives.

### Secondary Persona: Platform / CI Maintainer

**Role**: Owns CI infrastructure and shared dev tooling.
**Goals**: Reduce CI minutes and infra cost for Kafka-using test suites;
keep parity with production broker behavior.
**Pain Points**: Kafka images are large; flaky cluster bootstraps; debugging
mock-vs-real divergence.

## Requirements

### Must Have (P0)

1. **Wire-protocol compatibility, including modern (flexible) versions.**
   A standard Kafka producer and a standard Kafka consumer connect to heimq
   with no client code or configuration changes beyond the bootstrap
   address, and exchange data identically to how they would against Kafka
   or Redpanda for the in-scope APIs. heimq must negotiate **current**
   Kafka API versions, not a legacy subset — modern librdkafka, Java
   client, Kafka Connect, Flink, Debezium, ksqlDB, and the Confluent
   serializers default to flexible-version APIs. Without flexible-version
   support, FEAT-002 (transactions: InitProducerId v2+, EndTxn v3+,
   AddPartitionsToTxn v3+, TxnOffsetCommit v3+) and FEAT-005 (ecosystem
   tools) cannot pass. See FEAT-006.
2. **Consumer groups.** JoinGroup / SyncGroup / Heartbeat / LeaveGroup /
   OffsetCommit / OffsetFetch / FindCoordinator behave correctly: a group of
   N members reading a partitioned topic sees no gaps, no duplicate
   ownership, and resumes from committed offsets after a restart of any
   member.
3. **Idempotent producers.** A producer with `enable.idempotence=true` and
   the corresponding `InitProducerId` / sequence-number behavior produces
   exactly-once at the producer level — duplicates from retries are
   collapsed, sequence gaps return the standard error codes.
4. **Transactional groups (EOS).** A transactional producer using
   `transactional.id`, `InitProducerId`, `AddPartitionsToTxn`,
   `AddOffsetsToTxn`, `EndTxn`, `WriteTxnMarkers`, `TxnOffsetCommit`
   completes commit and abort transactions correctly; consumers configured
   with `isolation.level=read_committed` observe only committed records.
5. **Differential parity testing.** A test harness runs the same workload
   against heimq and against a real Redpanda (or Kafka) instance and asserts
   equivalent observable client behavior for in-scope APIs.
6. **Standard Kafka benchmark conformance.** Standard, commonly-used Kafka
   benchmarks — at minimum `kafka-producer-perf-test`,
   `kafka-consumer-perf-test`, and the OpenMessaging Benchmark Kafka driver —
   run to completion against heimq without protocol or client errors at a
   documented load profile.
7. **Ecosystem integration coverage.** Common Kafka-speaking systems are
   tested against heimq: Kafka Connect (a representative source and sink
   connector), Apache Flink (Kafka source + sink), ksqlDB, Debezium (one
   representative connector), a Schema Registry client (Confluent or
   Apicurio API), and librdkafka-based clients in at least three languages
   (e.g., Go/confluent-kafka-go, Python/confluent-kafka, Node/node-rdkafka).

### Should Have (P1)

1. Optional durable backends for individual subsystems (e.g., Postgres
   offsets) so committed offsets can survive a restart even though message
   logs do not.
2. Per-partition compression codecs (gzip, snappy, lz4, zstd) round-trip
   correctly through standard clients (already covered in Phase 5 client-
   surface coverage).

### Nice to Have (P2)

1. Headers / record metadata round-trip parity at byte level vs Redpanda.
2. Configurable artificial latency / drop injection for chaos-style tests.

## Functional Requirements

### Wire protocol (FEAT-001 + FEAT-006)

- All in-scope API keys / versions advertised via `ApiVersions` are decoded
  and answered with byte-equivalent semantics to Kafka/Redpanda for the
  same request.
- Unsupported APIs are advertised correctly and return the standard error
  for unsupported version where applicable.
- Capability gating in `compute_supported_apis` continues to filter
  advertised APIs to what the configured backends can serve.
- **Flexible versions** (compact strings, unsigned varints, tagged
  fields) are decoded and encoded for every API that has a flexible
  variant in current Kafka. heimq advertises versions through current
  Kafka per-API maxima, not a frozen legacy subset. (FEAT-006.)

### Core semantics (FEAT-002)

- Consumer-group lifecycle (join, sync, heartbeat, rebalance on join/leave,
  session timeout) matches Kafka semantics for single-coordinator
  deployments.
- Idempotent producer: `InitProducerId` returns a producer id; the broker
  tracks `(producerId, epoch, partition)` sequence numbers; duplicate
  sequence numbers return `DUPLICATE_SEQUENCE_NUMBER`; out-of-order returns
  `OUT_OF_ORDER_SEQUENCE_NUMBER`.
- Transactions: `AddPartitionsToTxn` / `AddOffsetsToTxn` / `EndTxn` /
  `WriteTxnMarkers` / `TxnOffsetCommit` operate on a single-coordinator
  transaction state machine; `read_committed` consumers do not observe
  uncommitted records or aborted records.

### Differential parity (FEAT-003)

- A parity harness drives the same client-level workload against heimq and
  a Redpanda container, compares observable outputs (records consumed,
  offsets committed, error codes returned, group state observed via
  client API), and reports diffs.

### Benchmark conformance (FEAT-004)

- A benchmark harness invokes `kafka-producer-perf-test`,
  `kafka-consumer-perf-test`, and the OpenMessaging Benchmark Kafka driver
  against heimq with documented load profiles and asserts they complete
  without protocol/client errors.

### Ecosystem integrations (FEAT-005)

- Each integration target has a runnable test/example that points the tool
  at heimq and exercises its primary use case (e.g., a Debezium connector
  emits CDC events into a heimq topic; a Flink job reads from a heimq topic
  and writes to a sink; a Schema Registry client publishes and resolves
  schemas).

## Acceptance Test Sketches

| Requirement | Scenario | Input | Expected Output |
|-------------|----------|-------|-----------------|
| P0 wire compat | rdkafka producer + consumer round-trip on multi-partition topic | 10k records produced across 4 partitions; consumed by group of 2 members | All records delivered; per-partition offsets monotonic; no gaps, no duplicates |
| P0 consumer groups | Member leave triggers rebalance | Group of 3 reading 6 partitions; member 2 calls LeaveGroup | Remaining 2 members own all 6 partitions; no record skipped, no duplicate ownership |
| P0 idempotent producer | Retry under simulated network blip | Producer with `enable.idempotence=true` retries a batch with same sequence | Broker accepts once; duplicate retry returns DUPLICATE_SEQUENCE_NUMBER (or is silently de-duped per Kafka semantics) |
| P0 transactions | Aborted transaction not visible to read_committed | Producer in transaction writes 100 records, calls `abortTransaction()` | `read_committed` consumer observes 0 of those records; `read_uncommitted` consumer sees them |
| P0 transactions | Committed transaction visible | Producer commits 100 records via `commitTransaction()` | `read_committed` consumer observes all 100 in order |
| P0 differential parity | Same workload against heimq and Redpanda | Identical produce/consume script against both | Diff harness reports zero behavioral diffs in records, offsets, error codes for in-scope APIs |
| P0 benchmark | `kafka-producer-perf-test` against heimq | Standard CLI invocation with documented record-size/throughput profile | Tool exits 0 with no error lines |
| P0 ecosystem | Debezium connector against heimq | Configured connector emits CDC events to heimq topic | rdkafka consumer reads expected CDC envelope from topic |

## Technical Context

- **Language/Runtime**: Rust (toolchain version pinned in `rust-toolchain.toml` if present, otherwise stable).
- **Key Libraries**: `kafka-protocol` (codec), `tokio` (runtime), `proptest` (property tests), `rdkafka` (integration tests).
- **Data/Storage**: In-memory by default; pluggable backends include Postgres for offsets (`HEIMQ_STORAGE_OFFSETS=postgres`).
- **APIs**: Kafka wire protocol per `https://kafka.apache.org/protocol/`; per-API version matrix in `docs/helix/02-design/contracts/API-001-kafka-protocol.md`.
- **Platform Targets**: Linux x86_64 / arm64, macOS arm64. Single-binary distribution.

## Constraints, Assumptions, Dependencies

### Constraints

- **Technical**: Single-node only; no replication. Transactions and
  idempotency operate against a single-coordinator state machine, not a
  replicated transaction log. Flexible-version Kafka APIs (compact
  strings, unsigned varints, tagged fields) ARE in scope per FEAT-006 —
  modern client default-negotiated versions must be supported.
- **Business**: This is a tooling/test product — not durable infrastructure.
- **Legal/Compliance**: None in current scope.

### Assumptions

- Standard Kafka clients (rdkafka, java client, sarama) implement the
  protocol per spec and do not rely on undocumented Kafka behavior.
- Redpanda is a sufficient stand-in for Kafka for differential parity for
  the in-scope APIs.
- Standard Kafka benchmarks are runnable against any
  protocol-compatible broker.

### Dependencies

- Docker (for Redpanda/Kafka in differential and ecosystem tests).
- Standard Kafka tooling (`kafka-producer-perf-test`, OpenMessaging Benchmark).
- Ecosystem tools (Kafka Connect, Flink, ksqlDB, Debezium, Schema Registry).

## Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Transactions / idempotency are large surfaces and may diverge subtly from Kafka | High | High | Differential parity tests gate every change; per-API contract tests pin sequence-number / epoch / marker behavior |
| Ecosystem tools rely on undocumented Kafka behavior | Medium | High | Capture each divergence as a parking-lot item with reproducer; do not silently emulate |
| Benchmark tools assume APIs we don't implement (e.g., admin/config APIs at startup) | Medium | Medium | Profile each benchmark's API trace; either implement minimal stubs or document non-support |
| Differential harness is flaky due to non-determinism (timestamps, broker ids) | Medium | Medium | Compare normalized outputs (canonicalize broker ids, masked timestamps); pin client config |

## Resolved Decisions

- **OpenMessaging Benchmark version**: target the latest released
  driver. The bench harness pins to that release; bumps are tracked as
  ordinary maintenance.
- **Schema Registry**: target the **Confluent Schema Registry API**.
  Apicurio's Confluent-compatibility mode may incidentally pass but is
  not a separate target.
- **Idempotent producer / transaction state across restart**: not
  retained. heimq is in-memory; on restart the broker presents as a
  fresh broker. This matches what real Kafka does when its log is lost
  (e.g., disk wipe / fresh broker): clients receive
  `UNKNOWN_PRODUCER_ID` and re-initialize via `InitProducerId`.
  Modern librdkafka and the Java client handle this transparently.
  No heimq-specific recovery behavior — Kafka spec applies.

## Open Questions

(none currently blocking)

## Success Criteria

- All P0 requirements have passing acceptance tests in the test suite.
- Differential harness reports zero behavioral diffs against Redpanda for
  in-scope APIs at the gating workload.
- Standard Kafka benchmarks complete without protocol/client errors.
- Each ecosystem integration target has at least one passing test
  exercising its primary use case.
- Test plan (`docs/helix/03-test/test-plan/test-plan.md`) reflects all of
  the above.
