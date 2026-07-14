---
ddx:
  id: helix.prd
  depends_on:
    - helix.product-vision
  review:
    self_hash: 96f0479e307f2c240e8f47b69fff510164d0b9eda132abb22cc4a860932984fe
    deps:
      helix.product-vision: 8fda503ba36c48175e42be03782c96af196de39d6e87b6edab88d853ee1857ca
    reviewed_at: "2026-07-14T05:12:26Z"
---
# Product Requirements Document

## Summary

heimq has a dual identity: it is (a) an embeddable Kafka broker **engine** —
the `heimq-wire`/`heimq-broker` crate family providing wire scaffolding and
broker semantics over pluggable storage and node-coordination traits — and (b)
a single-binary in-memory **distribution** (the `heimq` CLI) for tests, local
development, and CI.

As a **distribution**: standard Kafka producers and consumers connect unchanged
and observe the same behavior they would against Kafka or Redpanda for the
in-scope semantic surface: consumer groups, transactional groups (EOS), and
idempotent producers. heimq is permitted to lose data on restart and to offer
limited retention; it is not a durable, distributed broker.

As an **engine**: downstream projects (fjord, pqueue, niflheim) embed the
engine crates and implement backend traits to get a Kafka wire surface with
capability gating and per-trait conformance suites. Durability is a backend
property; the engine makes no durability assumption. The engine is
multi-node-capable via the pluggable `ClusterView` coordination trait.

Success is measured by differential parity against Redpanda, by passing
standard Kafka benchmarks, by working with common Kafka-speaking ecosystem
tools, and by per-trait conformance suites passing on all consumer-shaped
backends.

## Problem and Goals

### Problem

Teams testing or developing services against Kafka either spin up
Kafka/Redpanda (heavy, slow cold start, JVM dependency for Kafka) or use
broker mocks that drift from real protocol behavior. There is no
single-binary, in-memory broker that speaks the Kafka wire protocol
faithfully enough that tests passing against it imply tests will pass against
production Kafka — including the semantics services actually rely on
(consumer groups, transactions, idempotent producers).

Separately, three sibling projects (fjord, pqueue, niflheim) each need a
Kafka-compatible protocol front-end over their own storage and coordination
planes; without a shared engine, each re-implements wire scaffolding and
broker semantics that then drift independently.

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
| Differential parity with Redpanda for in-scope APIs | 100% of in-scope APIs pass diff tests | Differential test harness in `tests/conformance/` running same workload against heimq and Redpanda |
| Standard Kafka benchmark conformance | `kafka-producer-perf-test` and `kafka-consumer-perf-test` complete with no errors at documented load; OpenMessaging Benchmark runs to completion | Bench harness scripts in `scripts/bench/` |
| Ecosystem integration coverage | ≥1 tested integration each for: Kafka Connect, Flink, ksqlDB, Debezium, a Schema Registry client, librdkafka in ≥3 languages | Integration test suite in `tests/ecosystem/` (planned) |
| Wire-protocol contract coverage | 100% of in-scope APIs covered by contract tests | `contract.rs` under `crates/heimq/tests/` + per-API contract files |
| Startup and footprint (vision differentiator) | Cold start — process exec to first successful client produce/consume round-trip — < 1 second on a CI runner; distributed as a single self-contained binary with no JVM or external services required | `scripts/startup-roundtrip-budget.sh` in `.github/workflows/conformance.yml`; execution evidence recorded in `.ddx/executions/20260714T055432-be1997d6/cold-start-budget.md` |

### Non-Goals

The following are explicit non-goals — heimq does not aim to do these:

1. **Durability across restarts.** heimq may lose all in-memory state on
   process restart. Persistent backends (e.g., Postgres for offsets) may
   exist as opt-in slices but are not the default and are not required for
   correctness of in-scope features.
2. **Long retention.** Retention is bounded by time, size, and a hard memory
   cap; multi-day or multi-TB retention is out of scope. The in-memory log
   enforces, using the **standard Kafka topic tunables** (byte- and time-based,
   **never record-count-based** — records have unknown size, so a count limit
   cannot bound memory):
   - **`retention.ms`** (time, default 7 days): a background sweeper drops record
     batches older than the TTL. Always on.
   - **`retention.bytes`** (per-partition size) and a global **`max_memory_bytes`**
     cap (0 = unlimited): bound total in-memory bytes.
   - **Backpressure**: at the memory cap, heimq first drops expired data, then
     rejects produce with the retriable `KAFKA_STORAGE_ERROR` (56) rather than
     evicting un-expired data or growing without bound — preserving the
     `retention.ms` contract under memory pressure.

   Per-topic `retention.ms` / `retention.bytes` are tunable via AlterConfigs
   (API 33) and IncrementalAlterConfigs (API 44); `max_memory_bytes` is a
   broker-level cap. A consumer that falls behind reclaimed data receives
   `OFFSET_OUT_OF_RANGE`, exactly as in Kafka.
3. **Multi-broker / replication / KRaft / controller responsibilities.**
   heimq is single-node by design.
4. **SASL / ACLs / delegation tokens.** SASL PLAIN/TLS exist as a gated
   `heimq-wire` capability for embedding consumers (normative surface in
   WIRE-001 at `docs/helix/02-design/contracts/WIRE-001-wire-scaffolding.md`);
   they are OFF in the heimq distribution and not a distribution feature.
   ACLs and delegation tokens remain fully out of scope.
5. **Share groups, telemetry APIs, admin reassignment APIs.** Out of scope.
6. **Production-Kafka durability/scale.** heimq must *complete* standard
   benchmarks correctly; durability, replication, and multi-broker scale are out
   of scope. Note: on the in-memory produce/fetch hot path heimq is CPU-bound and
   *exceeds* both Apache Kafka and Redpanda on a like-for-like single-node
   librdkafka benchmark (2.8–4.8× produce, 3.6–5.2× consume; see
   `crates/heimq/benches/BASELINE.md`) — throughput parity is not a non-goal, it
   is a demonstrated property of the in-memory design.

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

### Tertiary Persona: Broker Builder

**Role**: Engineer embedding `heimq-wire`/`heimq-broker` into a larger system
(object-store broker, WAL-backed ingest path, producer front-end, or similar).
**Goals**: Obtain a conformant Kafka wire surface and broker semantics without
writing or owning a Kafka wire implementation; certify their backend
implementations using the per-trait conformance suites.
**Pain Points**: Kafka wire protocol is complex to implement correctly;
conformance is hard to verify without an established test surface; capability
gating (advertising only APIs the backend can serve) requires framework
support.
**Constraint**: Broker Builders depend on the engine crates (`heimq-wire`,
`heimq-broker`, `heimq-testkit`); they never depend on the `heimq` bin crate.

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
8. **Engine trait surface with per-trait conformance suites.** The engine
   exposes stable trait families — `TopicLog`/`LogBackend`/`PartitionLog`,
   `OffsetStore`, `GroupCoordinatorBackend`, `ClusterView` — each accompanied
   by a per-trait conformance suite in `heimq-testkit`. A backend that does
   not implement a trait family causes the corresponding Kafka APIs to not be
   advertised (capability gating). The normative trait surface is defined in
   TRAIT-001 (`docs/helix/02-design/contracts/TRAIT-001-backend-traits.md`);
   this PRD states the capability requirement, not the signatures.

### Should Have (P1)

1. Optional durable backends for individual subsystems (e.g., Postgres
   offsets) so committed offsets can survive a restart even though message
   logs do not. (See FEAT-007.)
2. Per-partition compression codecs (gzip, snappy, lz4, zstd) round-trip
   correctly through standard clients (already covered in Phase 5 client-
   surface coverage).

### Nice to Have (P2)

1. Headers / record metadata round-trip parity at byte level vs Redpanda.
2. Configurable artificial latency / drop injection for chaos-style tests.

## Functional Requirements

### Subsystem: Wire protocol (FEAT-001 + FEAT-006)

- **FR-1** — All in-scope API keys / versions advertised via `ApiVersions` are decoded
  and answered with byte-equivalent semantics to Kafka/Redpanda for the
  same request.
- **FR-2** — Unsupported APIs are advertised correctly and return the standard error
  for unsupported version where applicable.
- **FR-3** — Capability gating in `compute_supported_apis` continues to filter
  advertised APIs to what the configured backends can serve.
- **FR-4** — **Flexible versions** (compact strings, unsigned varints, tagged
  fields) are decoded and encoded for every API that has a flexible
  variant in current Kafka. heimq advertises versions through current
  Kafka per-API maxima, not a frozen legacy subset. (FEAT-006.)

### Subsystem: Core semantics (FEAT-002)

- **FR-5** — Consumer-group lifecycle (join, sync, heartbeat, rebalance on join/leave,
  session timeout) matches Kafka semantics for single-coordinator
  deployments.
- **FR-6** — Idempotent producer: `InitProducerId` returns a producer id; the broker
  tracks `(producerId, epoch, partition)` sequence numbers; duplicate
  sequence retries are de-duplicated; out-of-order returns
  `OUT_OF_ORDER_SEQUENCE_NUMBER`.
- **FR-7** — Transactions: `AddPartitionsToTxn` / `AddOffsetsToTxn` / `EndTxn` /
  `WriteTxnMarkers` / `TxnOffsetCommit` operate on a single-coordinator
  transaction state machine; `read_committed` consumers do not observe
  uncommitted records or aborted records.

### Subsystem: Differential parity (FEAT-003)

- **FR-8** — A parity harness drives the same client-level workload against heimq and
  a Redpanda container, compares observable outputs (records consumed,
  offsets committed, error codes returned, group state observed via
  client API), and reports diffs.

### Subsystem: Benchmark conformance (FEAT-004)

- **FR-9** — A benchmark harness invokes `kafka-producer-perf-test`,
  `kafka-consumer-perf-test`, and the OpenMessaging Benchmark Kafka driver
  against heimq with documented load profiles and asserts they complete
  without protocol/client errors.

### Subsystem: Ecosystem integrations (FEAT-005)

- **FR-10** — Each integration target has a runnable test/example that points the tool
  at heimq and exercises its primary use case (e.g., a Debezium connector
  emits CDC events into a heimq topic; a Flink job reads from a heimq topic
  and writes to a sink; a Schema Registry client publishes and resolves
  schemas).

### Subsystem: Durable offset backend (FEAT-007)

- **FR-11** — An opt-in Postgres-backed offset store, selected via
  `HEIMQ_STORAGE_OFFSETS=postgres://…` (or `--storage-offsets`), persists
  committed consumer-group offsets so they survive a broker restart even
  though message logs do not. The in-memory offset store remains the
  default, and correctness of in-scope features must not depend on the
  durable backend (PRD non-goal #1). Priority: P1 (P1 #1).

### Subsystem: Engine trait surface (TRAIT-001)

- **FR-12** — The engine exposes stable trait families (`TopicLog`/`LogBackend`/`PartitionLog`,
  `OffsetStore`, `GroupCoordinatorBackend`, `ClusterView`) with per-trait
  conformance suites in `heimq-testkit`. A backend that does not implement a
  trait family causes the corresponding Kafka APIs to not be advertised
  (capability-gated advertisement). The normative trait signatures and
  conformance obligations are defined in TRAIT-001
  (`docs/helix/02-design/contracts/TRAIT-001-backend-traits.md`).

- **FR-13** — Wire handler contract: handlers are async, produce handlers
  support deferred acks, every request carries a per-request principal context
  threaded through to trait calls, and backpressure is mapped to standard
  Kafka errors (e.g., `KAFKA_STORAGE_ERROR` for ingestion backpressure).
  The normative handler/registry surface — including typed-vs-raw body policy,
  response-encoding ownership, cancellation, and SASL/TLS capability gating —
  is defined in WIRE-001
  (`docs/helix/02-design/contracts/WIRE-001-wire-scaffolding.md`).

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
- **Key Libraries**: `kafka-protocol` (codec — single workspace pin per ADR-007), `tokio` (runtime), `proptest` (property tests), `rdkafka` (integration tests).
- **Data/Storage**: In-memory by default; pluggable backends include Postgres for offsets (`HEIMQ_STORAGE_OFFSETS=postgres`).
- **APIs**: Kafka wire protocol per `https://kafka.apache.org/protocol/`; per-API version matrix in `docs/helix/02-design/contracts/API-001-kafka-protocol.md`.
- **Platform Targets**: Linux x86_64 / arm64, macOS arm64. Single-binary distribution.

### Workspace Structure

| Crate | Contents | Consumers |
|-------|----------|-----------|
| `heimq-wire` | Framing, codec, flexible headers, connection loop, SASL/TLS hooks (gated), error-frame policy, handler registry | niflheim, pqueue, heimq-broker |
| `heimq-broker` | Handlers, group/idempotence/transaction semantics, capability gating, trait families (`LogBackend`/`TopicLog`/`PartitionLog`, `OffsetStore`, `GroupCoordinatorBackend`, `ClusterView`), in-memory reference backends | fjord, niflheim (produce path), heimq bin |
| `heimq-testkit` | Per-trait conformance suites, contract-test pattern, differential parity harness, expected-divergence annotations | All consumers |
| `heimq` (bin) | CLI, config, backend dispatch (memory/postgres), packaging — the distribution | End users |

Consumers (fjord, pqueue, niflheim) embed engine crates; they never depend on the `heimq` bin crate.

`kafka-protocol` is pinned at a single version for the entire workspace (ADR-007); consumers that embed engine crates use the same pin.

Resolved product decisions are recorded as ADRs:

- [ADR-004](../02-design/adr/ADR-004-openmessaging-benchmark-version.md) — OpenMessaging Benchmark: target the latest released driver, pin it; version bumps are ordinary maintenance.
- [ADR-005](../02-design/adr/ADR-005-schema-registry-target.md) — Schema Registry: the Confluent Schema Registry API is the target.
- [ADR-006](../02-design/adr/ADR-006-no-state-across-restart.md) — Producer / transaction state is not retained across restart; clients receive `UNKNOWN_PRODUCER_ID` and re-initialize per Kafka spec.
- [ADR-007](../02-design/adr/ADR-007-workspace-split-engine-crates.md) — Workspace split and crate boundaries; `kafka-protocol` single-pin policy; niflheim/fjord/pqueue consumption model.

## Constraints, Assumptions, Dependencies

### Constraints

- **Technical**: The heimq distribution is single-node only; no
  replication. The engine is multi-node-capable through the `ClusterView`
  coordination trait (TRAIT-001) — multi-node topology is a consumer
  responsibility (e.g. fjord), never a distribution feature. Transactions and
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
