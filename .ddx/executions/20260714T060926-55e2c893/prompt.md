<bead-review>
  <bead id="heimq-cdd87b6a" iter=1>
    <title>Enforce the cold-start first-roundtrip budget</title>
    <description>
Resolve AR13-06. Add a bounded executable check for the PRD cold-start metric: process exec through the first successful standard-client produce/consume roundtrip must complete in under one second on a CI runner. Define warm-cache and retry policy so the budget is reproducible, record elapsed time as evidence, and wire a non-flaky gating job or test. In scope: focused script/test, CI workflow, PRD and TP-001 evidence reference. Out of scope: broad performance optimization unless the measured budget fails.
    </description>
    <acceptance>
1. A repository command starts the release or debug broker, completes a real Kafka-client produce/consume roundtrip, prints elapsed milliseconds, and exits non-zero at 1000 ms or above. 2. The command has a bounded timeout and deterministic cleanup. 3. The check runs in CI and its focused local command passes three consecutive times.
    </acceptance>
    <labels>helix, area:testing, area:infra, kind:test, ac-quality:needs-refinement</labels>
  </bead>

  <changed-files>
    <file>.ddx/executions/20260714T055432-be1997d6/cold-start-budget.md</file>
    <file>.github/workflows/conformance.yml</file>
    <file>crates/heimq/docs/helix/01-frame/prd.md</file>
    <file>crates/heimq/docs/helix/03-test/test-plan/test-plan.md</file>
    <file>scripts/startup-roundtrip-budget.sh</file>
  </changed-files>

  <governing>
    <ref id="helix.prd" path="crates/heimq/docs/helix/01-frame/prd.md" title="Product Requirements Document">
      <content>
<untrusted-data>
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
| Startup and footprint (vision differentiator) | Cold start — process exec to first successful client produce/consume round-trip — < 1 second on a CI runner; distributed as a single self-contained binary with no JVM or external services required | Startup-timing check in CI (planned); release artifact inspection |

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
</untrusted-data>
      </content>
    </ref>
  </governing>

  <diff rev="954540038566163b39868d7e5c650c36c923477d">
<untrusted-data>
diff --git a/crates/heimq/docs/helix/01-frame/prd.md b/crates/heimq/docs/helix/01-frame/prd.md
index 5bfbdd4..1e4745e 100644
--- a/crates/heimq/docs/helix/01-frame/prd.md
+++ b/crates/heimq/docs/helix/01-frame/prd.md
@@ -73,7 +73,7 @@ broker semantics that then drift independently.
 | Standard Kafka benchmark conformance | `kafka-producer-perf-test` and `kafka-consumer-perf-test` complete with no errors at documented load; OpenMessaging Benchmark runs to completion | Bench harness scripts in `scripts/bench/` |
 | Ecosystem integration coverage | ≥1 tested integration each for: Kafka Connect, Flink, ksqlDB, Debezium, a Schema Registry client, librdkafka in ≥3 languages | Integration test suite in `tests/ecosystem/` (planned) |
 | Wire-protocol contract coverage | 100% of in-scope APIs covered by contract tests | `contract.rs` under `crates/heimq/tests/` + per-API contract files |
-| Startup and footprint (vision differentiator) | Cold start — process exec to first successful client produce/consume round-trip — < 1 second on a CI runner; distributed as a single self-contained binary with no JVM or external services required | Startup-timing check in CI (planned); release artifact inspection |
+| Startup and footprint (vision differentiator) | Cold start — process exec to first successful client produce/consume round-trip — < 1 second on a CI runner; distributed as a single self-contained binary with no JVM or external services required | `scripts/startup-roundtrip-budget.sh` in `.github/workflows/conformance.yml`; execution evidence recorded in `.ddx/executions/20260714T055432-be1997d6/cold-start-budget.md` |
 
 ### Non-Goals
 
diff --git a/.ddx/executions/20260714T055432-be1997d6/cold-start-budget.md b/.ddx/executions/20260714T055432-be1997d6/cold-start-budget.md
new file mode 100644
index 0000000..3723bad
--- /dev/null
+++ b/.ddx/executions/20260714T055432-be1997d6/cold-start-budget.md
@@ -0,0 +1,27 @@
+# Cold-start budget evidence
+
+Command:
+
+```bash
+source /tmp/heimq-startup-venv-312/bin/activate && ./scripts/startup-roundtrip-budget.sh
+```
+
+Validation notes:
+
+- Broker profile: `release`
+- Warm-cache policy: one release build before timing, then three fresh broker processes
+- Client: `confluent-kafka` 2.6.1 on Python 3.12
+
+Observed results:
+
+- attempt 1: `elapsed_ms=91.303`
+- attempt 2: `elapsed_ms=85.827`
+- attempt 3: `elapsed_ms=86.325`
+
+Threshold:
+
+- fail at `>= 1000 ms`
+
+Outcome:
+
+- pass
diff --git a/.github/workflows/conformance.yml b/.github/workflows/conformance.yml
index 4c005f1..5bcf325 100644
--- a/.github/workflows/conformance.yml
+++ b/.github/workflows/conformance.yml
@@ -1,8 +1,9 @@
 name: conformance
 
-# The Kafka-facing test suite. Everything here runs in containers and needs
-# nothing on the runner but Docker -- no Rust toolchain, no librdkafka, no Go,
-# Node or kcat. A missing runtime is a build failure, never a skipped test.
+# The Kafka-facing test suite. The parity and compat jobs run in containers
+# and need only Docker; the runner-based startup gate uses the Rust toolchain
+# plus a Python Kafka client wheel. A missing runtime is a build failure,
+# never a skipped test.
 #
 #   parity      heimq vs apache/kafka + redpanda, two independent clients
 #   compat      independent client oracles against heimq alone
@@ -57,3 +58,16 @@ jobs:
       - uses: actions/checkout@v4
       - name: Run integration suite
         run: ./tests/conformance/integration/run.sh
+
+  # Cold-start budget gate: a fresh broker process must complete a real
+  # produce/consume roundtrip under 1s, three times in a row.
+  cold-start:
+    runs-on: ubuntu-latest
+    steps:
+      - uses: actions/checkout@v4
+      - uses: dtolnay/rust-toolchain@stable
+      - uses: Swatinem/rust-cache@v2
+      - name: Install Python Kafka client
+        run: python3 -m pip install --user --no-cache-dir confluent-kafka==2.6.1
+      - name: Run cold-start budget gate
+        run: ./scripts/startup-roundtrip-budget.sh
diff --git a/crates/heimq/docs/helix/03-test/test-plan/test-plan.md b/crates/heimq/docs/helix/03-test/test-plan/test-plan.md
index 4b5d5cb..082785c 100644
--- a/crates/heimq/docs/helix/03-test/test-plan/test-plan.md
+++ b/crates/heimq/docs/helix/03-test/test-plan/test-plan.md
@@ -346,6 +346,15 @@ Spec traceability: FEAT-004, PRD P0 #6. Lives under `scripts/bench/`.
 - [ ] OpenMessaging Benchmark Kafka driver runs at least one documented workload to completion.
 - [ ] Bench profiles, expected exit codes, and acceptable warnings checked in.
 
+### Phase 9b: Cold-start budget gate (P0, PRD startup metric)
+
+Spec traceability: PRD Startup and footprint success metric. Lives under `scripts/` and `.github/workflows/`.
+
+- [x] `scripts/startup-roundtrip-budget.sh` starts a fresh broker process, performs a real librdkafka produce/consume roundtrip, prints elapsed milliseconds, and exits non-zero at 1000 ms or above.
+- [x] The command uses a bounded readiness/client timeout and deterministic cleanup for each attempt.
+- [x] CI runs the command three consecutive times on fresh broker processes.
+- [x] Execution evidence: `.ddx/executions/20260714T055432-be1997d6/cold-start-budget.md`.
+
 ### Phase 10: Ecosystem Integrations (P0, FEAT-005)
 
 Spec traceability: FEAT-005, PRD P0 #7. Lives under `tests/ecosystem/`.
diff --git a/scripts/startup-roundtrip-budget.sh b/scripts/startup-roundtrip-budget.sh
new file mode 100755
index 0000000..67a3d43
--- /dev/null
+++ b/scripts/startup-roundtrip-budget.sh
@@ -0,0 +1,278 @@
+#!/usr/bin/env bash
+# Measure the cold-start budget from broker exec to first successful
+# produce/consume roundtrip.
+#
+# Warm-cache policy:
+# - build the broker binary once before timing
+# - each measured attempt uses a fresh temp data dir and a fresh broker process
+# - the client library is expected to be preinstalled by the caller/CI job
+#
+# Retry policy:
+# - no automatic retries inside an attempt
+# - the script performs three consecutive measured attempts by default
+# - if a broker readiness deadline or client deadline expires, the script fails
+#
+# Usage:
+#   ./scripts/startup-roundtrip-budget.sh
+#
+# Environment:
+#   HEIMQ_STARTUP_BUDGET_ATTEMPTS=3
+#   HEIMQ_STARTUP_BUDGET_MS=1000
+#   HEIMQ_STARTUP_BUDGET_PROFILE=release|debug
+#   HEIMQ_STARTUP_BUDGET_READY_TIMEOUT_SECONDS=20
+#   HEIMQ_STARTUP_BUDGET_CLIENT_TIMEOUT_SECONDS=10
+#   HEIMQ_STARTUP_BUDGET_RUST_LOG=error
+
+set -euo pipefail
+
+ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
+PROFILE="${HEIMQ_STARTUP_BUDGET_PROFILE:-release}"
+ATTEMPTS="${HEIMQ_STARTUP_BUDGET_ATTEMPTS:-3}"
+BUDGET_MS="${HEIMQ_STARTUP_BUDGET_MS:-1000}"
+READY_TIMEOUT_SECONDS="${HEIMQ_STARTUP_BUDGET_READY_TIMEOUT_SECONDS:-20}"
+CLIENT_TIMEOUT_SECONDS="${HEIMQ_STARTUP_BUDGET_CLIENT_TIMEOUT_SECONDS:-10}"
+BROKER_HOST="${HEIMQ_STARTUP_BUDGET_HOST:-127.0.0.1}"
+BROKER_RUST_LOG="${HEIMQ_STARTUP_BUDGET_RUST_LOG:-error}"
+
+BROKER_PID=""
+BROKER_DATA_DIR=""
+BROKER_LOG=""
+
+require_tool() {
+  if ! command -v "$1" >/dev/null 2>&1; then
+    echo "missing required tool: $1" >&2
+    exit 2
+  fi
+}
+
+cleanup_current() {
+  if [[ -n "${BROKER_PID}" ]]; then
+    kill "${BROKER_PID}" >/dev/null 2>&1 || true
+    for _ in $(seq 1 50); do
+      if ! kill -0 "${BROKER_PID}" >/dev/null 2>&1; then
+        break
+      fi
+      sleep 0.1
+    done
+    if kill -0 "${BROKER_PID}" >/dev/null 2>&1; then
+      kill -9 "${BROKER_PID}" >/dev/null 2>&1 || true
+    fi
+    wait "${BROKER_PID}" >/dev/null 2>&1 || true
+    BROKER_PID=""
+  fi
+  if [[ -n "${BROKER_DATA_DIR}" ]]; then
+    rm -rf "${BROKER_DATA_DIR}"
+    BROKER_DATA_DIR=""
+  fi
+  BROKER_LOG=""
+}
+
+trap cleanup_current EXIT INT TERM
+
+pick_free_port() {
+  python3 - <<'PY'
+import socket
+
+sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
+sock.bind(("127.0.0.1", 0))
+try:
+    print(sock.getsockname()[1])
+finally:
+    sock.close()
+PY
+}
+
+wait_for_broker() {
+  local pid="$1"
+  local bootstrap="$2"
+  local log_file="$3"
+
+python3 - "$pid" "$bootstrap" "$log_file" "$READY_TIMEOUT_SECONDS" <<'PY'
+import pathlib
+import sys
+import time
+
+from confluent_kafka.admin import AdminClient
+
+pid = int(sys.argv[1])
+bootstrap = sys.argv[2]
+log_file = pathlib.Path(sys.argv[3])
+deadline = time.monotonic() + float(sys.argv[4])
+last_error = None
+
+while time.monotonic() < deadline:
+    try:
+        import os
+
+        os.kill(pid, 0)
+    except OSError:
+        raise SystemExit(
+            "broker exited before readiness:\n"
+            f"{log_file.read_text(encoding='utf-8', errors='replace')}"
+        )
+
+    try:
+        AdminClient({"bootstrap.servers": bootstrap}).list_topics(timeout=1)
+        sys.exit(0)
+    except Exception as exc:  # noqa: BLE001 - readiness probe retries by design
+        last_error = exc
+        time.sleep(0.05)
+
+raise SystemExit(
+    "broker was not ready before the deadline:\n"
+    f"{last_error}\n"
+    f"{log_file.read_text(encoding='utf-8', errors='replace')}"
+)
+PY
+}
+
+run_roundtrip() {
+  local start_ns="$1"
+  local bootstrap="$2"
+  local attempt="$3"
+  local budget_ms="$4"
+
+  python3 - "$start_ns" "$bootstrap" "$attempt" "$budget_ms" "$CLIENT_TIMEOUT_SECONDS" <<'PY'
+import sys
+import time
+import uuid
+
+from confluent_kafka import Consumer, KafkaError, Producer, TopicPartition
+
+start_ns = int(sys.argv[1])
+bootstrap = sys.argv[2]
+attempt = int(sys.argv[3])
+budget_ms = float(sys.argv[4])
+client_timeout = float(sys.argv[5])
+
+topic = f"cold-start-{attempt}-{uuid.uuid4().hex[:10]}"
+group = f"cold-start-{attempt}-{uuid.uuid4().hex[:10]}"
+
+producer = Producer(
+    {
+        "bootstrap.servers": bootstrap,
+        "message.timeout.ms": 5000,
+    }
+)
+delivery = []
+
+def on_delivery(err, msg):
+    delivery.append((err, msg))
+
+producer.produce(topic, value=b"cold-start-roundtrip", key=b"cold-start", on_delivery=on_delivery)
+producer.flush(10)
+
+if not delivery:
+    raise SystemExit("no delivery report from producer")
+
+err, msg = delivery[0]
+if err is not None:
+    raise SystemExit(f"produce failed: {err}")
+
+consumer = Consumer(
+    {
+        "bootstrap.servers": bootstrap,
+        "group.id": group,
+        "auto.offset.reset": "earliest",
+        "enable.auto.commit": False,
+        "session.timeout.ms": 6000,
+    }
+)
+consumer.assign([TopicPartition(topic, 0, 0)])
+try:
+    deadline = time.monotonic() + client_timeout
+    received = None
+    while time.monotonic() < deadline:
+        record = consumer.poll(0.1)
+        if record is None:
+            continue
+        if record.error():
+            if record.error().code() == KafkaError._PARTITION_EOF:
+                continue
+            raise SystemExit(f"consume failed: {record.error()}")
+        received = record
+        break
+
+    if received is None:
+        raise SystemExit("timed out waiting for the consumed record")
+
+    elapsed_ms = (time.monotonic_ns() - start_ns) / 1_000_000
+    print(f"elapsed_ms={elapsed_ms:.3f}")
+
+    if elapsed_ms >= budget_ms:
+        raise SystemExit(
+            f"cold-start budget exceeded: {elapsed_ms:.3f}ms >= {budget_ms:.0f}ms"
+        )
+finally:
+    consumer.close()
+PY
+}
+
+main() {
+  require_tool cargo
+  require_tool python3
+
+  python3 - <<'PY'
+import importlib
+try:
+    importlib.import_module("confluent_kafka")
+except ModuleNotFoundError as exc:
+    raise SystemExit(
+        "missing Python dependency: confluent-kafka\n"
+        "Install it with: python3 -m pip install --user confluent-kafka==2.6.1"
+    ) from exc
+PY
+
+  case "${PROFILE}" in
+    release)
+      cargo build --locked --release -p heimq --bin heimq
+      BROKER_BIN="${ROOT_DIR}/target/release/heimq"
+      ;;
+    debug)
+      cargo build --locked -p heimq --bin heimq
+      BROKER_BIN="${ROOT_DIR}/target/debug/heimq"
+      ;;
+    *)
+      echo "unsupported HEIMQ_STARTUP_BUDGET_PROFILE=${PROFILE}" >&2
+      exit 2
+      ;;
+  esac
+
+  if [[ ! -x "${BROKER_BIN}" ]]; then
+    echo "missing broker binary: ${BROKER_BIN}" >&2
+    exit 2
+  fi
+
+  for attempt in $(seq 1 "${ATTEMPTS}"); do
+    cleanup_current
+    BROKER_DATA_DIR="$(mktemp -d "${ROOT_DIR}/target/startup-budget.XXXXXX")"
+    BROKER_LOG="${BROKER_DATA_DIR}/heimq.log"
+    local_port="$(pick_free_port)"
+    start_ns="$(python3 - <<'PY'
+import time
+print(time.monotonic_ns())
+PY
+)"
+
+    HEIMQ_HOST="${BROKER_HOST}" \
+    HEIMQ_PORT="${local_port}" \
+    HEIMQ_ADVERTISED_HOST="${BROKER_HOST}" \
+    HEIMQ_MEMORY_ONLY=true \
+    HEIMQ_DEFAULT_PARTITIONS=1 \
+    HEIMQ_AUTO_CREATE_TOPICS=true \
+    HEIMQ_DATA_DIR="${BROKER_DATA_DIR}" \
+    RUST_LOG="${BROKER_RUST_LOG}" \
+    "${BROKER_BIN}" >"${BROKER_LOG}" 2>&1 &
+    BROKER_PID=$!
+
+    wait_for_broker "${BROKER_PID}" "${BROKER_HOST}:${local_port}" "${BROKER_LOG}"
+    run_roundtrip "${start_ns}" "${BROKER_HOST}:${local_port}" "${attempt}" "${BUDGET_MS}"
+
+    echo "[attempt ${attempt}/${ATTEMPTS}] PASS"
+  done
+
+  cleanup_current
+  echo "PASS: cold-start budget satisfied for ${ATTEMPTS} consecutive attempts"
+}
+
+main "$@"
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
