<bead-review>
  <bead id="heimq-caf7b41c" iter=1>
    <title>Refresh PRD and test-plan paths after the workspace split</title>
    <description>
Resolve AR13-04. Replace stale pre-split references such as tests/parity/, tests/contract.rs, src/protocol/mod.rs, and scripts/bench locations with current repository paths under tests/conformance/, crates/heimq/tests/, crates/heimq/src/, and scripts/bench/. Audit PRD, TP-001, API-001, TRAIT-001, WIRE-001, and IP-001 for material path references. Do not rewrite historical alignment reports. In scope: live governing artifacts only. Out of scope: code behavior.
    </description>
    <acceptance>
1. Every repository path changed by this bead exists. 2. rg -n "tests/parity/|tests/contract.rs|src/protocol/mod.rs" crates/heimq/docs/helix/01-frame crates/heimq/docs/helix/02-design/contracts crates/heimq/docs/helix/03-test/test-plan crates/heimq/docs/helix/04-build returns no stale live reference. 3. ddx doc stale reports all documents up to date after review hashes are refreshed through supported DDx tooling.
    </acceptance>
    <labels>helix, area:specs, area:testing, kind:docs, action:evolve</labels>
  </bead>

  <changed-files>
    <file>crates/heimq/docs/helix/01-frame/concerns.md</file>
    <file>crates/heimq/docs/helix/01-frame/features/FEAT-001-wire-protocol-compatibility.md</file>
    <file>crates/heimq/docs/helix/01-frame/features/FEAT-002-core-kafka-semantics.md</file>
    <file>crates/heimq/docs/helix/01-frame/features/FEAT-003-differential-parity-testing.md</file>
    <file>crates/heimq/docs/helix/01-frame/features/FEAT-004-benchmark-conformance.md</file>
    <file>crates/heimq/docs/helix/01-frame/features/FEAT-005-ecosystem-integrations.md</file>
    <file>crates/heimq/docs/helix/01-frame/features/FEAT-006-flexible-version-protocol.md</file>
    <file>crates/heimq/docs/helix/01-frame/features/FEAT-007-durable-offset-backend.md</file>
    <file>crates/heimq/docs/helix/01-frame/features/FEAT-008-engine-embedding-contract.md</file>
    <file>crates/heimq/docs/helix/01-frame/prd.md</file>
    <file>crates/heimq/docs/helix/01-frame/user-stories/US-001-standard-client-connects.md</file>
    <file>crates/heimq/docs/helix/01-frame/user-stories/US-002-consumer-group-rebalance.md</file>
    <file>crates/heimq/docs/helix/01-frame/user-stories/US-003-idempotent-producer.md</file>
    <file>crates/heimq/docs/helix/01-frame/user-stories/US-004-transactional-commit-abort.md</file>
    <file>crates/heimq/docs/helix/01-frame/user-stories/US-005-differential-parity-harness.md</file>
    <file>crates/heimq/docs/helix/01-frame/user-stories/US-006-kafka-perf-test.md</file>
    <file>crates/heimq/docs/helix/01-frame/user-stories/US-007-openmessaging-benchmark.md</file>
    <file>crates/heimq/docs/helix/01-frame/user-stories/US-008-kafka-connect.md</file>
    <file>crates/heimq/docs/helix/01-frame/user-stories/US-009-flink.md</file>
    <file>crates/heimq/docs/helix/01-frame/user-stories/US-010-debezium.md</file>
    <file>crates/heimq/docs/helix/01-frame/user-stories/US-011-schema-registry.md</file>
    <file>crates/heimq/docs/helix/01-frame/user-stories/US-012-multi-language-clients.md</file>
    <file>crates/heimq/docs/helix/01-frame/user-stories/US-013-modern-client-no-downgrade.md</file>
    <file>crates/heimq/docs/helix/01-frame/user-stories/US-014-ksqldb.md</file>
    <file>crates/heimq/docs/helix/01-frame/user-stories/US-015-durable-offsets-postgres.md</file>
    <file>crates/heimq/docs/helix/01-frame/user-stories/US-016-backend-trait-conformance.md</file>
    <file>crates/heimq/docs/helix/01-frame/user-stories/US-017-request-level-handler-delegation.md</file>
    <file>crates/heimq/docs/helix/02-design/DESIGN.md</file>
    <file>crates/heimq/docs/helix/02-design/contracts/API-001-kafka-protocol.md</file>
    <file>crates/heimq/docs/helix/02-design/contracts/CODEC-001-flexible.md</file>
    <file>crates/heimq/docs/helix/02-design/contracts/TRAIT-001-backend-traits.md</file>
    <file>crates/heimq/docs/helix/02-design/contracts/WIRE-001-wire-scaffolding.md</file>
    <file>crates/heimq/docs/helix/02-design/solution-designs/SD-003-differential-parity-testing.md</file>
    <file>crates/heimq/docs/helix/03-test/test-plan/test-plan.md</file>
    <file>crates/heimq/docs/helix/04-build/implementation-plan.md</file>
  </changed-files>

  <governing>
    <ref id="TP-001" path="crates/heimq/docs/helix/03-test/test-plan/test-plan.md" title="Test Plan">
      <content>
<untrusted-data>
---
ddx:
  id: TP-001
  type: test-plan
  activity: test
  status: draft
  depends_on:
    - helix.prd
    - FEAT-001
    - FEAT-002
    - FEAT-003
    - FEAT-004
    - FEAT-005
    - FEAT-006
  review:
    self_hash: 0ab4a39b378230423b0e06e506a3948adef3e14c246517906f88c8d33d1e14be
    deps:
      FEAT-001: 7133c264bc364ec4535c1d6b6187a90c9ba66d1fa3df30731ade260c2e092479
      FEAT-002: 164350929c7bbc09a589f3cd1a80b685e88cce1054445fe5373aec566464636f
      FEAT-003: d134a5740222f1b59ecda81b318c7b28de9968bba9f640bb6280310c1e906233
      FEAT-004: 15d88aa986503176f2e6b5020d55ec106bc1f7d6c05c7a1ea322d2bb9003f91e
      FEAT-005: c8b17ef2e79181581f03413860ea3e074364440af280321171b639668cb3aebc
      FEAT-006: e59d3b8965ebd35b4bbe9c5302f4218432ad83ec27691989dfd4c345ac2ae004
      helix.prd: 236574e8f31d3847bb8269d538fe07d0a47376aa7d7e75c30dca783e479ad4ab
    reviewed_at: "2026-06-22T21:30:26Z"
---

# Test Plan

**Project**: heimq
**Version**: 0.2.0
**Date**: 2026-06-11
**Status**: Draft
**Author**: Codex (freyr)

## Executive Summary

heimq targets Kafka protocol compatibility on a single node with no durability guarantees. The test strategy prioritizes protocol contract compliance and behavioral parity with Kafka/Redpanda for the subset of APIs in scope. Property-based tests are the starting layer to validate invariants in storage and protocol framing, followed by contract tests per API and integration tests with real Kafka clients.

## Testing Strategy

### Scope and Objectives

**Traceability Source**: PRD (`helix.prd`), FEAT-001–FEAT-006, and the per-API
version matrix in `API-001-kafka-protocol.md` (cited per phase under
"Spec traceability" in the Implementation Roadmap).

**Testing Goals**:
- Validate Kafka protocol behavior for all supported APIs and versions.
- Prove basic correctness of storage/offset invariants under varied inputs.
- Prevent regressions by aligning behavior against Kafka and Redpanda baselines.

**In scope (per PRD, FEAT-002)**:
- Idempotent producers and Kafka transactions (single-coordinator EOS).
  Loss of in-memory producer-id / transaction state on restart is acceptable
  per PRD non-goal #1, but functional correctness while the broker is
  running is required.

**Out of Scope (for now)**:
- Distributed system semantics (controller, replication, leader epochs).
- Security/authentication/authorization (SASL, ACLs).
- Persistence of transaction / producer-id state across restart.

### Test Levels

| Level | Purpose | Coverage Target | Priority |
| --- | --- | --- | --- |
| Contract Tests | Kafka API compliance | 100% of supported APIs | P0 |
| Integration Tests | End-to-end client behavior | Critical paths | P0 |
| Property Tests | Storage/protocol invariants | Core invariants | P0 |
| Unit Tests | Local logic | 80% | P1 |
| Baseline Parity | Kafka/Redpanda diffs | No behavioral regressions | P0 |

### Framework Selection

| Test Type | Framework | Justification |
| --- | --- | --- |
| Contract | rdkafka + custom request harness | Exercises real client behavior |
| Integration | rdkafka, kafka crate | Modern + legacy protocol coverage |
| Property | proptest | Systematic invariant testing |
| Baseline Parity | docker + redpanda/kafka | Known-good behavior comparisons |
| External Client Oracle | franz-go (pure Go, no librdkafka) | Independent implementation as "who watches the watcher" check |

## Test Organization

### Directory Structure (current + target)

```
heimq/
├── tests/
│   ├── contract.rs
│   ├── integration.rs
│   ├── compat.rs              (external client oracle — franz-go)
│   ├── compat/
│   │   └── franz_go/          (pure-Go Kafka client oracle program)
│   └── fixtures/ (planned)
└── src/
    └── ... (unit + property tests in modules)
```

### Naming Conventions

- Contract tests: `tests/contract.rs` (current), `tests/contract/{api}.rs` (target)
- Integration tests: `tests/integration.rs`
- Property/unit tests: module-level `#[cfg(test)]`

### Test Data Strategy

- **Fixtures**: Topic names, partition counts, payloads in `tests/fixtures/`.
- **Property Data**: proptest generators for record batches, offsets, and sizes.
- **External Services**: dockerized Kafka and Redpanda for parity runs.

## Coverage Requirements

### Coverage Targets

| Metric | Target | Minimum | Enforcement |
| --- | --- | --- | --- |
| Contract Coverage | 100% of supported APIs | 90% | CI gate |
| Integration Coverage | Produce/Fetch/Groups | Critical paths | CI gate |
| Property Coverage | Storage + codec invariants | Core set | CI gate |
| Line/Region Coverage | 100% | 100% | CI gate |

**Current status**: 100% line + region coverage via `cargo llvm-cov --workspace --all-features`.

### Critical Paths (P0)

1. Produce -> Fetch roundtrip (single and multi-partition)
2. Metadata discovery and auto-create behavior
3. Consumer group join/sync/heartbeat lifecycle, including rebalance on join and leave
4. Offset commit + fetch for group consumption (manual and auto-commit)
5. Single-group delivery integrity across a multi-partition topic: no gaps, no duplicate ownership

## Baseline Alignment

- **Differential parity harness (FEAT-003)**: A harness in `tests/parity/`
  drives identical client workloads against heimq and Redpanda, normalizes
  non-determinism, and asserts zero behavioral diffs for in-scope APIs
  (produce/fetch and consumer groups as gating workloads at FEAT-003
  acceptance; idempotent producers and transactions are added once
  FEAT-002 is accepted — gated on FEAT-002 acceptance).
  This is the gating mechanism for protocol-touching changes.
- **Redpanda smoke**: `scripts/compatibility-test.sh` continues to run
  weekly and on protocol changes.
- **Kafka parity**: Add a Kafka docker target and compare responses for
  the same workloads.
- **Golden traces**: Capture request/response fixtures from Kafka/Redpanda
  and assert equivalence.

## Execution Notes

- rdkafka-based integration tests are ignored in default `cargo test` due to observed segfaults.
- Run them explicitly via `cargo test --test integration -- --ignored` or `scripts/compatibility-test.sh`.

## Implementation Roadmap

### Phase 1: Property-Based Foundation (P0)
- [x] Storage invariants: segment read, partition offsets, log start/high watermark.
- [x] Codec invariants: request/response framing and correlation id integrity. (`protocol/codec.rs`: `test_decode_request_header`, `test_encode_response_body`, framing tests)

### Phase 2: Contract Tests for Supported APIs (P0)
- [x] ApiVersions: version ranges match `SUPPORTED_APIS`.
- [x] Metadata: topic discovery + error behavior.
- [x] Produce + Fetch: offsets, max_bytes, empty record batches.
- [x] ListOffsets: earliest/latest/timestamp semantics.
- [x] CreateTopics/DeleteTopics: idempotency and errors.
- [x] Consumer Groups: FindCoordinator, Join/Sync/Heartbeat/Leave, OffsetCommit/Fetch (`tests/contract.rs`).

### Phase 3: Integration and Regression (P0)
- [x] Expand integration test coverage to include consumer group offsets and group lifecycle (`tests/integration.rs`).
- [ ] Add legacy protocol edge cases via `kafka` crate.

### Phase 4: Baseline Parity — Parked — superseded by Phase 8 (differential parity harness); see docs/helix/parking-lot.md

Parked (unchecked since the 2026-04-26 alignment review). This phase
proposed two items: (1) adding a Kafka docker target to
`scripts/compatibility-test.sh`, and (2) capturing Redpanda/Kafka golden
request/response fixtures and asserting equivalence for parity
regression tests. Both are functionally superseded by Phase 8's
differential parity harness, which compares live behavior against
Redpanda. See the parking-lot entry for the revisit trigger.

### Phase 5: rdkafka Producer + Consumer E2E Gaps (P0/P1)

Scope: rdkafka-driven end-to-end tests only. Contract/property/unit
layers are covered by Phases 1–2. Prioritization follows external
review (codex) after auditing `tests/integration.rs`.

Spec traceability: exercises API-001 APIs Produce (0), Fetch (1),
ListOffsets (2), Metadata (3), OffsetCommit (8), OffsetFetch (9),
FindCoordinator (10), JoinGroup (11), Heartbeat (12), LeaveGroup
(13), SyncGroup (14).

All Phase 5 items are landed under epic heimq-700e47b2; tests live in
`heimq/tests/integration.rs` (run with `cargo test --test integration --
--ignored`).

#### P0 — Consumer group correctness
- [x] Single-group delivery integrity on a multi-partition topic: no gaps, no duplicate ownership across members (supersedes any "exactly-once" framing — not a Kafka guarantee). [heimq-f17da917]
- [x] Resume from committed offset after consumer restart in the same group. [heimq-65cdaf14]
- [x] Rebalance on member **leave** (graceful LeaveGroup + session-timeout expiry). [heimq-f1e42e32]
- [x] Rebalance on member **join** (second member added to an active group). [`test_rdkafka_consumer_rebalance_on_new_member`]
- [x] Multiple independent consumer groups reading the same topic with independent committed offsets. [heimq-990ad721]

#### P0 — Offset behavior
- [x] `auto.offset.reset=earliest` vs `latest` on a fresh group with pre-existing messages. [heimq-a77fc237]
- [x] Manual commit + committed-offset fetch path, distinct from auto-commit. [heimq-6b53731e]
- [x] `enable.auto.commit=true` with explicit `auto.commit.interval.ms` covers the auto-commit code path. [heimq-1597cf54]

#### P0 — Multi-partition integrity and order
- [x] Multi-partition produce→consume roundtrip: every produced message is consumed exactly once across the group, across partitions. [heimq-c93b7b4a]
- [x] Per-partition ordering: consumed offsets monotonic per partition under concurrent produce. [heimq-6f69c0d3]

#### P0 — Partition selection
- [x] Keyed-message partitioner determinism: same key → same partition on a multi-partition topic. [heimq-9d72b499]
- [x] Explicit partition targeting via `rdkafka` producer API (bypass hash partitioner). [heimq-885505ce]

#### P1 — Fetch wait behavior
- [x] Long-poll fetch: `fetch.min.bytes` + `fetch.max.wait.ms` blocks until data arrives or timeout. [heimq-4ab3d5d7]
- [x] Empty-topic consume timeout: consumer on an empty topic returns cleanly on poll timeout. [heimq-979c6bea]

#### P1 — Error paths and broker config
- [x] Produce to nonexistent topic with `auto.create.topics.enable=false`. [heimq-57261856]
- [x] Oversized message vs `message.max.bytes`. [heimq-cd905f86]
- [x] `auto.create.topics.enable` on/off behavior end-to-end. [heimq-6c6709a1]

#### P1 — Throughput and concurrency
- [x] Concurrent rdkafka producers to the same topic. [heimq-1e0a2d52]
- [x] Produce-while-consume soak: N thousand messages, assert no loss, no duplicates, monotonic per-partition offsets. [heimq-bbab6a2e]

#### P2 — Client-surface coverage (lower broker signal)
- [x] Record headers round-trip (produce → consume). [heimq-977d7b59]
- [x] Compression codecs: gzip, snappy, lz4, zstd (distinct broker decode paths). [heimq-76674efa]
- [x] Seek to end, seek to arbitrary offset. [heimq-cd222bbb]
- [x] Pause / resume on assigned partitions. [heimq-0c5afc66]

### Phase 6: Backend Matrix (P0)

Status: landed — driver `scripts/stress-matrix.sh` exists and is wired
into `.github/workflows/stress-matrix.yml`.

Scope: run the existing `scripts/kcat-stress.sh` workload against each
sensible combination of pluggable storage backends (log / offsets /
groups), to assert that observable broker behavior is consistent
regardless of which backend is wired in.

Driver: `scripts/stress-matrix.sh` loops over the configured
combinations, exporting `HEIMQ_STORAGE_LOG`, `HEIMQ_STORAGE_OFFSETS`,
and `HEIMQ_STORAGE_GROUPS` per iteration and invoking
`kcat-stress.sh` at moderate volume (default 5 topics x 10k msgs).

Initial combinations:

| Row          | log    | offsets  | groups | Notes |
| ---          | ---    | ---      | ---    | ---   |
| memory-only  | memory | memory   | memory | default, no external deps |
| pg-offsets   | memory | postgres | memory | durability slice for committed offsets |

Skip behavior: rows that require Postgres are skipped (with a clear
`SKIPPED` line, not `FAIL`) when `POSTGRES_URL` is unset, so the
matrix stays green in environments without a Postgres instance.

CI: an optional `stress-matrix` job (see
`.github/workflows/stress-matrix.yml`) brings up a Postgres service
container, exports `POSTGRES_URL`, and runs `stress-matrix.sh`. The
job is `continue-on-error` so it does not block the main test gate.

#### Dropped from earlier draft
- `acks=0/1/all` — on a single-node in-memory broker, `acks=1` and `all` collapse to the same path; `acks=0` is not meaningfully observable from rdkafka. Not worth a dedicated test.
- Batching / `linger.ms` / `batch.size` — client-side behavior; broker decode is already exercised by large-batch contract and soak tests.

### Phase 7-pre: Flexible-version codec (P0, FEAT-006)

Spec traceability: FEAT-006, PRD P0 #1 (modern wire-protocol versions).
This phase is a prerequisite for Phase 7 (transactions: modern
transactional APIs are flexible-only) and Phase 10 (ecosystem tools
that default to flexible negotiation).

- [x] `is_flexible(api_key, api_version)` boundary table: off-by-one on each entry returns the correct legacy/flexible classification. (`test_is_flexible_boundary`)
- [x] `decode_request` legacy header path remains green; flexible header path parses `client_id` (legacy nullable string) and consumes tagged-fields trailer. (`test_decode_request_flexible_header`)
- [x] `encode_response` flexible header path emits empty tagged-fields trailer after `correlation_id`. (`test_encode_response_flexible_header`)
- [x] `encode_response` ApiVersions v3 special case — no tagged-fields trailer in response header. (`test_encode_response_apiversions_no_flexible_header`)
- [x] Router roundtrip at one flexible API version: `correlation_id` preserved; response decodes correctly via kafka-protocol crate. (rdkafka integration tests exercise flexible versions end-to-end; `send_request` verifies correlation_id on every contract test)

> Note: Codec primitive correctness (varints, compact strings, tagged fields) is delegated to the kafka-protocol crate per ADR-003.

- [x] Flexible request header v2 + flexible response header v1 wired into the router for flexible APIs. (implemented in `protocol/codec.rs` `decode_request`/`encode_response`)
- [x] Each in-scope API gains its flexible-version handler path; legacy paths retained. (FEAT-006 FR-03 — all handlers use `kafka-protocol` decode which handles both)
- [x] `SUPPORTED_APIS` updated to advertise current Kafka per-API maxima for the in-scope surface. (`protocol/mod.rs` SUPPORTED_APIS; `contract_api_versions_matches_supported_range`)
- [x] ApiVersions v3 (flexible request body, including ignored client_software_name / client_software_version) implemented. (handler ignores version; framing handled by codec; `prop_compact_string_roundtrip` and v3 proptest)
- [ ] Differential parity (Phase 8) reports zero diffs at flexible versions for in-scope APIs.

### Phase 7: Idempotent Producer + Transactions (P0, FEAT-002)

Spec traceability: FEAT-002, PRD P0 #3 (idempotent producers) and #4
(transactions). Adds the API keys currently marked `Planned (FEAT-002)` in
`API-001-kafka-protocol.md`: InitProducerId (22), AddPartitionsToTxn (24),
AddOffsetsToTxn (25), EndTxn (26), WriteTxnMarkers (27), TxnOffsetCommit
(28), DescribeProducers (61), DescribeTransactions (65), ListTransactions
(66).

#### P0 — Idempotent producer
- [x] `enable.idempotence=true` retried batch is de-duped (no duplicate visible to consumer). (`contract_duplicate_sequence_is_deduplicated`; parity `duplicate_sequence`)
- [x] Out-of-order sequence returns `OUT_OF_ORDER_SEQUENCE_NUMBER`. (`contract_out_of_order_sequence_returns_error`)
- [x] Duplicate sequence is de-duped per Kafka semantics. (`contract_duplicate_sequence_is_deduplicated`; parity `duplicate_sequence`)
- [x] `InitProducerId` returns producerId/epoch; epoch bumps on re-init. (`contract_init_producer_id_with_transactional_id`)

#### P0 — Transactions
- [x] Committed transaction is visible to `read_committed` consumers. (`contract_transactional_produce_commit_fetch`)
- [x] Aborted transaction is invisible to `read_committed` consumers — server populates `aborted_transactions` list. (`contract_aborted_transaction_invisible_to_read_committed`)
- [x] `read_uncommitted` consumer observes both committed and aborted records; `aborted_transactions` list is empty. (`contract_read_uncommitted_observes_aborted_records`)
- [x] Stale producer epoch returns `INVALID_PRODUCER_EPOCH`. (`contract_stale_epoch_returns_invalid_producer_epoch`)
- [ ] `transaction.timeout.ms` is enforced; expired transactions are aborted.
- [x] `TxnOffsetCommit` participates in transaction lifecycle for EOS consumer. (`contract_txn_offset_commit_basic`)

### Phase 8: Differential Parity Harness (P0, FEAT-003)

Spec traceability: FEAT-003, PRD P0 #5. Lives under `tests/parity/`.

- [x] Harness scaffolding: same client workload against heimq and Redpanda; structured diff output.
- [x] Normalization rules for broker ids, host timestamps, monotonic ids.
- [x] Workloads cover produce/fetch and consumer groups as gating workloads at FEAT-003 acceptance.
- [x] Workloads add idempotent producers and transactions once FEAT-002 is accepted.
- [ ] CI gating job: parity harness runs on protocol-touching changes; zero-diff is the success condition.
- [x] Known-divergence registry (e.g., loss-on-restart) with PRD references.

### Phase 8b: External Client Oracle (P0)

A second independent client library (franz-go, pure Go, no librdkafka) runs the same
produce/consume/consumer-group workload against heimq. This provides "who watches the
watcher" coverage — a pass here means two unrelated implementations agree on the Kafka
wire protocol. Lives in `tests/compat/`.

- [x] franz-go oracle: CreateTopics → Produce → ConsumerGroup Fetch → OffsetCommit
- [x] Exercises API versions independent of rdkafka: JoinGroup v9, SyncGroup v5, OffsetFetch v9, ListOffsets v8, Fetch v12
- [x] FindCoordinator v4 bug fix: populate `coordinators` array (v4+ schema) not just legacy fields
- [ ] Add multi-record-batch workload to franz-go oracle (headers, compression codecs)
- [ ] Add Java kafka-clients oracle (reference implementation)

### Phase 9: Standard Kafka Benchmark Conformance (P0, FEAT-004)

Spec traceability: FEAT-004, PRD P0 #6. Lives under `scripts/bench/`.

- [ ] `kafka-producer-perf-test` runs against heimq with documented load profile; exits 0, no error lines.
- [ ] `kafka-consumer-perf-test` runs against heimq with documented load profile; exits 0, no error lines.
- [ ] Idempotent profile (`enable.idempotence=true`) and transactional profile (`transactional.id`) each complete cleanly.
- [ ] OpenMessaging Benchmark Kafka driver runs at least one documented workload to completion.
- [ ] Bench profiles, expected exit codes, and acceptable warnings checked in.

### Phase 10: Ecosystem Integrations (P0, FEAT-005)

Spec traceability: FEAT-005, PRD P0 #7. Lives under `tests/ecosystem/`.

| Integration | Driver script | Status |
| --- | --- | --- |
| Kafka Connect (source + sink) | `tests/ecosystem/kafka-connect/run.sh` | [ ] Pending |
| Apache Flink (Kafka source + sink) | `tests/ecosystem/flink/run.sh` | [ ] Pending |
| ksqlDB | `tests/ecosystem/ksqldb/run.sh` | [ ] Pending |
| Debezium (one connector) | `tests/ecosystem/debezium/run.sh` | [ ] Pending |
| Schema Registry round-trip | `tests/ecosystem/schema-registry/run.sh` | [ ] Pending |
| `franz-go` | `tests/compat/franz_go/` (oracle, `cargo test --test compat`) | [x] Done — Phase 8b |
| `confluent-kafka-go` | `tests/ecosystem/clients/go/run.sh` | [ ] Pending |
| `confluent-kafka` (Python) | `tests/ecosystem/clients/python/run.sh` | [ ] Pending |
| `node-rdkafka` | `tests/ecosystem/clients/node/run.sh` | [ ] Pending |

Each integration: a single script brings up the tool, runs its primary
use case against heimq, asserts exit 0, tears down. Tools that depend on
out-of-scope APIs are parking-lotted with a written rationale rather
than silently emulated.

## Test Infrastructure

### Environment Requirements

**Local Development**:
- Rust toolchain with `cargo`, `llvm-cov`, and `clang` available.
- Docker (for Kafka/Redpanda parity runs).

**CI/CD Pipeline**:
- Linux runner with Rust toolchain.
- Docker-in-Docker or privileged docker for parity jobs.

### Tools and Dependencies

| Tool | Version | Purpose |
| --- | --- | --- |
| cargo | stable | Build + run tests |
| cargo-llvm-cov | latest | Coverage reporting |
| proptest | 1.x | Property-based tests |
| rdkafka | latest | Kafka client integration |
| docker | latest | Kafka/Redpanda parity |

### CI/CD Integration

```
test:
  stage: test
  script:
    - cargo test --workspace --all-features
    - cargo llvm-cov --workspace --all-features --fail-under-lines 100 --fail-under-regions 100
```

## Risk Assessment

| Risk | Impact | Probability | Mitigation |
| --- | --- | --- | --- |
| Flaky client integration tests | High | Medium | Isolate ports, deterministic timeouts |
| Missing protocol edge cases | High | High | Expand contract tests per API |
| Parity drift from Kafka | High | Medium | Regular Kafka/Redpanda baseline runs |

## Success Metrics

- 100% contract tests for supported APIs passing.
- Property tests cover core storage and protocol invariants.
- Differential parity harness reports zero behavioral diffs vs Redpanda for in-scope APIs at the gating workload (FEAT-003).
- All P0 idempotent-producer and transaction acceptance scenarios pass against heimq and against Redpanda (FEAT-002).
- Standard Kafka benchmarks (`kafka-producer-perf-test`, `kafka-consumer-perf-test`, OpenMessaging Benchmark) complete without protocol/client errors (FEAT-004).
- Each ecosystem integration target has at least one passing test in CI (FEAT-005).

## Acceptance Criteria Layer Allocation

Machine-checkable via `grep -r '@covers' crates/heimq/tests/`. Legend: **C** = contract, **I** = integration, **P** = parity, **E** = ecosystem/manual, **NYI** = not-yet-implemented (capability-gated or future story).

| AC ID | Layer | Exercising Test(s) | Status |
|---|---|---|---|
| US-001-AC1 | C, I | `contract_api_versions_matches_supported_range`, `contract_produce_fetch_roundtrip`, `test_rdkafka_simple_produce`, `test_rdkafka_produce_consume_roundtrip` | Automated |
| US-001-AC2 | C, I | `contract_produce_fetch_roundtrip`, `test_rdkafka_produce_consume_roundtrip` | Automated |
| US-001-AC3 | I | `test_rdkafka_simple_produce`, `test_rdkafka_produce_consume_roundtrip` | Automated |
| US-001-AC4 | P | `parity::workloads::produce_fetch::ProduceFetchRoundtrip` | Automated |
| US-002-AC1 | C, I | `contract_find_coordinator_returns_self`, `contract_join_group_new_member`, `contract_sync_group_leader`, `contract_heartbeat_active_member`, `contract_consumer_group_lifecycle`, `test_rdkafka_consumer_group_join`, `test_rdkafka_consumer_group_lifecycle` | Automated |
| US-002-AC2 | C, I | `contract_leave_group_success`, `contract_consumer_group_lifecycle`, `test_rdkafka_group_rebalance_on_graceful_leave` | Automated |
| US-002-AC3 | C, I | `contract_join_group_new_member`, `test_rdkafka_consumer_group_join`, `test_rdkafka_consumer_rebalance_on_new_member` | Automated |
| US-002-AC4 | C, I | `contract_offset_commit_and_fetch`, `test_rdkafka_consumer_group_offset_commit`, `test_rdkafka_consumer_group_manual_offset_fetch`, `test_rdkafka_group_resume_from_committed` | Automated |
| US-002-AC5 | P | `parity::workloads::consumer_group::ConsumerGroupLifecycle` | Automated |
| US-003-AC1 | — | — | NYI: capability-gated off (FEAT-002) |
| US-003-AC2 | — | — | NYI: capability-gated off |
| US-003-AC3 | — | — | NYI: capability-gated off |
| US-003-AC4 | — | — | NYI: capability-gated off |
| US-003-AC5 | — | — | NYI: capability-gated off |
| US-004-AC1 | — | — | NYI: capability-gated off (FEAT-002) |
| US-004-AC2 | — | — | NYI: capability-gated off |
| US-004-AC3 | — | — | NYI: capability-gated off |
| US-004-AC4 | — | — | NYI: capability-gated off |
| US-004-AC5 | — | — | NYI: capability-gated off |
| US-004-AC6 | — | — | NYI: capability-gated off |
| US-004-AC7 | — | — | NYI: capability-gated off |
| US-004-AC8 | — | — | NYI: capability-gated off |
| US-004-AC9 | — | — | NYI: capability-gated off |
| US-004-AC10 | — | — | NYI: capability-gated off |
| US-004-AC11 | — | — | NYI: capability-gated off |
| US-005-AC1 | P | `tests/parity/main.rs` (harness entry point) | Automated |
| US-005-AC2 | P | `tests/parity/diff.rs` | Automated |
| US-005-AC3 | P | `tests/parity/normalize.rs` | Automated |
| US-005-AC4 | P | `parity::workloads::produce_fetch`, `parity::workloads::consumer_group` | Automated |
| US-005-AC5 | P | `parity::workloads::produce_fetch`, `parity::workloads::consumer_group` | Automated |
| US-005-AC6 | P | `tests/parity/exemptions.rs` | Automated |
| US-005-AC7 | — | — | NYI: txn workloads gated off (FEAT-002) |
| US-006-AC1 | E | `scripts/bench/run-smoke.sh` + CI (`bench-smoke.yml`) | Automated (CI) |
| US-006-AC2 | E | `scripts/bench/run-smoke.sh` + CI (`bench-smoke.yml`) | Automated (CI) |
| US-006-AC3 | — | — | NYI: idempotent/transactional bench profiles pending FEAT-002 |
| US-006-AC4 | E | `scripts/bench/profiles/producer-smoke.properties`, `scripts/bench/profiles/consumer-smoke.properties` | Manual (files checked in) |
| US-007-AC1 | — | — | NYI: OMB integration not yet implemented |
| US-007-AC2 | — | — | NYI: OMB YAML config not yet created |
| US-007-AC3 | — | — | NYI: OMB CI job not yet added |
| US-008-AC1 | — | — | NYI: Kafka Connect ecosystem test |
| US-008-AC2 | — | — | NYI: Kafka Connect ecosystem test |
| US-008-AC3 | — | — | NYI: Kafka Connect ecosystem test |
| US-008-AC4 | — | — | NYI: Kafka Connect ecosystem test |
| US-009-AC1 | — | — | NYI: Flink ecosystem test |
| US-009-AC2 | — | — | NYI: Flink ecosystem test |
| US-009-AC3 | — | — | NYI: Flink ecosystem test |
| US-010-AC1 | — | — | NYI: Debezium CDC ecosystem test |
| US-010-AC2 | — | — | NYI: Debezium CDC ecosystem test |
| US-010-AC3 | — | — | NYI: Debezium CDC ecosystem test |
| US-011-AC1 | — | — | NYI: Schema Registry ecosystem test |
| US-011-AC2 | — | — | NYI: Schema Registry ecosystem test |
| US-011-AC3 | — | — | NYI: Schema Registry ecosystem test |
| US-011-AC4 | — | — | NYI: Schema Registry ecosystem test |
| US-012-AC1 | — | — | NYI: multi-language Go client test |
| US-012-AC2 | — | — | NYI: multi-language Python client test |
| US-012-AC3 | — | — | NYI: multi-language Node.js client test |
| US-012-AC4 | — | — | NYI: multi-language per-language scripts |
| US-013-AC1 | I | `test_rdkafka_simple_produce` (default-config rdkafka) | Automated |
| US-013-AC2 | C | `contract_api_versions_matches_supported_range` | Automated |
| US-013-AC3 | C | `contract_produce_fetch_roundtrip`, `contract_fetch_respects_max_bytes` | Automated |
| US-013-AC4 | P | `parity::workloads::produce_fetch` (rdkafka uses flexible versions) | Automated |
| US-014-AC1 | — | — | NYI: ksqlDB ecosystem test |
| US-014-AC2 | — | — | NYI: ksqlDB ecosystem test |
| US-014-AC3 | — | — | NYI: ksqlDB ecosystem test |
- Flexible-version codec implemented for in-scope APIs; `SUPPORTED_APIS` advertises target maxima; differential parity harness reports zero diffs at flexible versions for in-scope APIs (FEAT-006).
</untrusted-data>
      </content>
    </ref>
  </governing>

  <diff rev="e62d40730562b3a343bc1e8377db5ff84f94619f">
<untrusted-data>
diff --git a/crates/heimq/docs/helix/03-test/test-plan/test-plan.md b/crates/heimq/docs/helix/03-test/test-plan/test-plan.md
index 9a99534..4b5d5cb 100644
--- a/crates/heimq/docs/helix/03-test/test-plan/test-plan.md
+++ b/crates/heimq/docs/helix/03-test/test-plan/test-plan.md
@@ -13,16 +13,16 @@ ddx:
     - FEAT-005
     - FEAT-006
   review:
-    self_hash: 0ab4a39b378230423b0e06e506a3948adef3e14c246517906f88c8d33d1e14be
+    self_hash: 85693e54a11e4843d420d965d5fb01311581237c65579516411cc5516279d4d1
     deps:
       FEAT-001: 7133c264bc364ec4535c1d6b6187a90c9ba66d1fa3df30731ade260c2e092479
       FEAT-002: 164350929c7bbc09a589f3cd1a80b685e88cce1054445fe5373aec566464636f
       FEAT-003: d134a5740222f1b59ecda81b318c7b28de9968bba9f640bb6280310c1e906233
-      FEAT-004: 15d88aa986503176f2e6b5020d55ec106bc1f7d6c05c7a1ea322d2bb9003f91e
+      FEAT-004: 84531399bf0bf4d4ff962e6f516c45506d67340944a8ba924e6010b4a39b64a6
       FEAT-005: c8b17ef2e79181581f03413860ea3e074364440af280321171b639668cb3aebc
       FEAT-006: e59d3b8965ebd35b4bbe9c5302f4218432ad83ec27691989dfd4c345ac2ae004
-      helix.prd: 236574e8f31d3847bb8269d538fe07d0a47376aa7d7e75c30dca783e479ad4ab
-    reviewed_at: "2026-06-22T21:30:26Z"
+      helix.prd: 96f0479e307f2c240e8f47b69fff510164d0b9eda132abb22cc4a860932984fe
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 
 # Test Plan
@@ -100,8 +100,8 @@ heimq/
 
 ### Naming Conventions
 
-- Contract tests: `tests/contract.rs` (current), `tests/contract/{api}.rs` (target)
-- Integration tests: `tests/integration.rs`
+- Contract tests: `contract.rs` under `crates/heimq/tests/` (current); per-API files are a future target when introduced.
+- Integration tests: `tests/conformance/integration/`
 - Property/unit tests: module-level `#[cfg(test)]`
 
 ### Test Data Strategy
@@ -133,7 +133,7 @@ heimq/
 
 ## Baseline Alignment
 
-- **Differential parity harness (FEAT-003)**: A harness in `tests/parity/`
+- **Differential parity harness (FEAT-003)**: A harness in `tests/conformance/`
   drives identical client workloads against heimq and Redpanda, normalizes
   non-determinism, and asserts zero behavioral diffs for in-scope APIs
   (produce/fetch and consumer groups as gating workloads at FEAT-003
@@ -164,10 +164,10 @@ heimq/
 - [x] Produce + Fetch: offsets, max_bytes, empty record batches.
 - [x] ListOffsets: earliest/latest/timestamp semantics.
 - [x] CreateTopics/DeleteTopics: idempotency and errors.
-- [x] Consumer Groups: FindCoordinator, Join/Sync/Heartbeat/Leave, OffsetCommit/Fetch (`tests/contract.rs`).
+- [x] Consumer Groups: FindCoordinator, Join/Sync/Heartbeat/Leave, OffsetCommit/Fetch (`contract.rs` under `crates/heimq/tests/`).
 
 ### Phase 3: Integration and Regression (P0)
-- [x] Expand integration test coverage to include consumer group offsets and group lifecycle (`tests/integration.rs`).
+- [x] Expand integration test coverage to include consumer group offsets and group lifecycle (`tests/conformance/integration/`).
 - [ ] Add legacy protocol edge cases via `kafka` crate.
 
 ### Phase 4: Baseline Parity — Parked — superseded by Phase 8 (differential parity harness); see docs/helix/parking-lot.md
@@ -184,7 +184,7 @@ Redpanda. See the parking-lot entry for the revisit trigger.
 
 Scope: rdkafka-driven end-to-end tests only. Contract/property/unit
 layers are covered by Phases 1–2. Prioritization follows external
-review (codex) after auditing `tests/integration.rs`.
+review (codex) after auditing `tests/conformance/integration/`.
 
 Spec traceability: exercises API-001 APIs Produce (0), Fetch (1),
 ListOffsets (2), Metadata (3), OffsetCommit (8), OffsetFetch (9),
@@ -192,8 +192,7 @@ FindCoordinator (10), JoinGroup (11), Heartbeat (12), LeaveGroup
 (13), SyncGroup (14).
 
 All Phase 5 items are landed under epic heimq-700e47b2; tests live in
-`heimq/tests/integration.rs` (run with `cargo test --test integration --
---ignored`).
+`tests/conformance/integration/`.
 
 #### P0 — Consumer group correctness
 - [x] Single-group delivery integrity on a multi-partition topic: no gaps, no duplicate ownership across members (supersedes any "exactly-once" framing — not a Kafka guarantee). [heimq-f17da917]
@@ -315,7 +314,7 @@ AddOffsetsToTxn (25), EndTxn (26), WriteTxnMarkers (27), TxnOffsetCommit
 
 ### Phase 8: Differential Parity Harness (P0, FEAT-003)
 
-Spec traceability: FEAT-003, PRD P0 #5. Lives under `tests/parity/`.
+Spec traceability: FEAT-003, PRD P0 #5. Lives under `tests/conformance/`.
 
 - [x] Harness scaffolding: same client workload against heimq and Redpanda; structured diff output.
 - [x] Normalization rules for broker ids, host timestamps, monotonic ids.
@@ -448,12 +447,12 @@ Machine-checkable via `grep -r '@covers' crates/heimq/tests/`. Legend: **C** = c
 | US-004-AC9 | — | — | NYI: capability-gated off |
 | US-004-AC10 | — | — | NYI: capability-gated off |
 | US-004-AC11 | — | — | NYI: capability-gated off |
-| US-005-AC1 | P | `tests/parity/main.rs` (harness entry point) | Automated |
-| US-005-AC2 | P | `tests/parity/diff.rs` | Automated |
-| US-005-AC3 | P | `tests/parity/normalize.rs` | Automated |
+| US-005-AC1 | P | `tests/conformance/go/main.go` (harness entry point) | Automated |
+| US-005-AC2 | P | `tests/conformance/go/observation.go` | Automated |
+| US-005-AC3 | P | `tests/conformance/go/raw.go` | Automated |
 | US-005-AC4 | P | `parity::workloads::produce_fetch`, `parity::workloads::consumer_group` | Automated |
 | US-005-AC5 | P | `parity::workloads::produce_fetch`, `parity::workloads::consumer_group` | Automated |
-| US-005-AC6 | P | `tests/parity/exemptions.rs` | Automated |
+| US-005-AC6 | P | `tests/conformance/exemptions.toml` | Automated |
 | US-005-AC7 | — | — | NYI: txn workloads gated off (FEAT-002) |
 | US-006-AC1 | E | `scripts/bench/run-smoke.sh` + CI (`bench-smoke.yml`) | Automated (CI) |
 | US-006-AC2 | E | `scripts/bench/run-smoke.sh` + CI (`bench-smoke.yml`) | Automated (CI) |
diff --git a/crates/heimq/docs/helix/01-frame/concerns.md b/crates/heimq/docs/helix/01-frame/concerns.md
index ff98d6b..8eac701 100644
--- a/crates/heimq/docs/helix/01-frame/concerns.md
+++ b/crates/heimq/docs/helix/01-frame/concerns.md
@@ -7,9 +7,9 @@ ddx:
   review:
     self_hash: a0172ef88e0fe7e93e4c789e8f33b2d1a33cde54b7c032516a20c0dbaeca4049
     deps:
-      helix.prd: 236574e8f31d3847bb8269d538fe07d0a47376aa7d7e75c30dca783e479ad4ab
+      helix.prd: 96f0479e307f2c240e8f47b69fff510164d0b9eda132abb22cc4a860932984fe
       helix.product-vision: 8fda503ba36c48175e42be03782c96af196de39d6e87b6edab88d853ee1857ca
-    reviewed_at: "2026-06-22T21:30:26Z"
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 
 # Project Concerns
diff --git a/crates/heimq/docs/helix/01-frame/features/FEAT-001-wire-protocol-compatibility.md b/crates/heimq/docs/helix/01-frame/features/FEAT-001-wire-protocol-compatibility.md
index d0a8ee1..2e6aa95 100644
--- a/crates/heimq/docs/helix/01-frame/features/FEAT-001-wire-protocol-compatibility.md
+++ b/crates/heimq/docs/helix/01-frame/features/FEAT-001-wire-protocol-compatibility.md
@@ -6,8 +6,8 @@ ddx:
   review:
     self_hash: 7133c264bc364ec4535c1d6b6187a90c9ba66d1fa3df30731ade260c2e092479
     deps:
-      helix.prd: 236574e8f31d3847bb8269d538fe07d0a47376aa7d7e75c30dca783e479ad4ab
-    reviewed_at: "2026-06-22T21:30:26Z"
+      helix.prd: 96f0479e307f2c240e8f47b69fff510164d0b9eda132abb22cc4a860932984fe
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 # Feature Specification: FEAT-001 — Kafka Wire-Protocol Compatibility
 
diff --git a/crates/heimq/docs/helix/01-frame/features/FEAT-002-core-kafka-semantics.md b/crates/heimq/docs/helix/01-frame/features/FEAT-002-core-kafka-semantics.md
index a97048e..d1161e4 100644
--- a/crates/heimq/docs/helix/01-frame/features/FEAT-002-core-kafka-semantics.md
+++ b/crates/heimq/docs/helix/01-frame/features/FEAT-002-core-kafka-semantics.md
@@ -10,8 +10,8 @@ ddx:
     deps:
       FEAT-001: 7133c264bc364ec4535c1d6b6187a90c9ba66d1fa3df30731ade260c2e092479
       FEAT-006: e59d3b8965ebd35b4bbe9c5302f4218432ad83ec27691989dfd4c345ac2ae004
-      helix.prd: 236574e8f31d3847bb8269d538fe07d0a47376aa7d7e75c30dca783e479ad4ab
-    reviewed_at: "2026-06-22T21:30:26Z"
+      helix.prd: 96f0479e307f2c240e8f47b69fff510164d0b9eda132abb22cc4a860932984fe
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 # Feature Specification: FEAT-002 — Core Kafka Semantics (Groups, Transactions, Idempotency)
 
diff --git a/crates/heimq/docs/helix/01-frame/features/FEAT-003-differential-parity-testing.md b/crates/heimq/docs/helix/01-frame/features/FEAT-003-differential-parity-testing.md
index 924e326..78dc4d8 100644
--- a/crates/heimq/docs/helix/01-frame/features/FEAT-003-differential-parity-testing.md
+++ b/crates/heimq/docs/helix/01-frame/features/FEAT-003-differential-parity-testing.md
@@ -10,8 +10,8 @@ ddx:
     deps:
       FEAT-001: 7133c264bc364ec4535c1d6b6187a90c9ba66d1fa3df30731ade260c2e092479
       FEAT-002: 164350929c7bbc09a589f3cd1a80b685e88cce1054445fe5373aec566464636f
-      helix.prd: 236574e8f31d3847bb8269d538fe07d0a47376aa7d7e75c30dca783e479ad4ab
-    reviewed_at: "2026-06-22T21:30:26Z"
+      helix.prd: 96f0479e307f2c240e8f47b69fff510164d0b9eda132abb22cc4a860932984fe
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 # Feature Specification: FEAT-003 — Differential Parity Testing vs Redpanda/Kafka
 
diff --git a/crates/heimq/docs/helix/01-frame/features/FEAT-004-benchmark-conformance.md b/crates/heimq/docs/helix/01-frame/features/FEAT-004-benchmark-conformance.md
index 391b9bc..539d67f 100644
--- a/crates/heimq/docs/helix/01-frame/features/FEAT-004-benchmark-conformance.md
+++ b/crates/heimq/docs/helix/01-frame/features/FEAT-004-benchmark-conformance.md
@@ -7,13 +7,13 @@ ddx:
     - FEAT-002
     - FEAT-006
   review:
-    self_hash: 15d88aa986503176f2e6b5020d55ec106bc1f7d6c05c7a1ea322d2bb9003f91e
+    self_hash: 84531399bf0bf4d4ff962e6f516c45506d67340944a8ba924e6010b4a39b64a6
     deps:
       FEAT-001: 7133c264bc364ec4535c1d6b6187a90c9ba66d1fa3df30731ade260c2e092479
       FEAT-002: 164350929c7bbc09a589f3cd1a80b685e88cce1054445fe5373aec566464636f
       FEAT-006: e59d3b8965ebd35b4bbe9c5302f4218432ad83ec27691989dfd4c345ac2ae004
-      helix.prd: 236574e8f31d3847bb8269d538fe07d0a47376aa7d7e75c30dca783e479ad4ab
-    reviewed_at: "2026-06-22T21:30:26Z"
+      helix.prd: 96f0479e307f2c240e8f47b69fff510164d0b9eda132abb22cc4a860932984fe
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 # Feature Specification: FEAT-004 — Standard Kafka Benchmark Conformance
 
diff --git a/crates/heimq/docs/helix/01-frame/features/FEAT-005-ecosystem-integrations.md b/crates/heimq/docs/helix/01-frame/features/FEAT-005-ecosystem-integrations.md
index 3aebe98..bbf175b 100644
--- a/crates/heimq/docs/helix/01-frame/features/FEAT-005-ecosystem-integrations.md
+++ b/crates/heimq/docs/helix/01-frame/features/FEAT-005-ecosystem-integrations.md
@@ -12,8 +12,8 @@ ddx:
       FEAT-001: 7133c264bc364ec4535c1d6b6187a90c9ba66d1fa3df30731ade260c2e092479
       FEAT-002: 164350929c7bbc09a589f3cd1a80b685e88cce1054445fe5373aec566464636f
       FEAT-006: e59d3b8965ebd35b4bbe9c5302f4218432ad83ec27691989dfd4c345ac2ae004
-      helix.prd: 236574e8f31d3847bb8269d538fe07d0a47376aa7d7e75c30dca783e479ad4ab
-    reviewed_at: "2026-06-22T21:30:26Z"
+      helix.prd: 96f0479e307f2c240e8f47b69fff510164d0b9eda132abb22cc4a860932984fe
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 # Feature Specification: FEAT-005 — Ecosystem Integrations
 
diff --git a/crates/heimq/docs/helix/01-frame/features/FEAT-006-flexible-version-protocol.md b/crates/heimq/docs/helix/01-frame/features/FEAT-006-flexible-version-protocol.md
index aa0d191..aa95e4a 100644
--- a/crates/heimq/docs/helix/01-frame/features/FEAT-006-flexible-version-protocol.md
+++ b/crates/heimq/docs/helix/01-frame/features/FEAT-006-flexible-version-protocol.md
@@ -8,8 +8,8 @@ ddx:
     self_hash: e59d3b8965ebd35b4bbe9c5302f4218432ad83ec27691989dfd4c345ac2ae004
     deps:
       FEAT-001: 7133c264bc364ec4535c1d6b6187a90c9ba66d1fa3df30731ade260c2e092479
-      helix.prd: 236574e8f31d3847bb8269d538fe07d0a47376aa7d7e75c30dca783e479ad4ab
-    reviewed_at: "2026-06-22T21:30:26Z"
+      helix.prd: 96f0479e307f2c240e8f47b69fff510164d0b9eda132abb22cc4a860932984fe
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 # Feature Specification: FEAT-006 — Flexible-Version Kafka Protocol Support
 
diff --git a/crates/heimq/docs/helix/01-frame/features/FEAT-007-durable-offset-backend.md b/crates/heimq/docs/helix/01-frame/features/FEAT-007-durable-offset-backend.md
index ddd5ae4..a06de55 100644
--- a/crates/heimq/docs/helix/01-frame/features/FEAT-007-durable-offset-backend.md
+++ b/crates/heimq/docs/helix/01-frame/features/FEAT-007-durable-offset-backend.md
@@ -6,8 +6,8 @@ ddx:
   review:
     self_hash: e947608aa0633732bde029cdf025cb8875286da1fed7bf9480548917157390ad
     deps:
-      helix.prd: 236574e8f31d3847bb8269d538fe07d0a47376aa7d7e75c30dca783e479ad4ab
-    reviewed_at: "2026-06-22T21:30:26Z"
+      helix.prd: 96f0479e307f2c240e8f47b69fff510164d0b9eda132abb22cc4a860932984fe
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 # Feature Specification: FEAT-007 — Durable Offset Backend (Postgres)
 
diff --git a/crates/heimq/docs/helix/01-frame/features/FEAT-008-engine-embedding-contract.md b/crates/heimq/docs/helix/01-frame/features/FEAT-008-engine-embedding-contract.md
index fa6b467..5129389 100644
--- a/crates/heimq/docs/helix/01-frame/features/FEAT-008-engine-embedding-contract.md
+++ b/crates/heimq/docs/helix/01-frame/features/FEAT-008-engine-embedding-contract.md
@@ -14,6 +14,13 @@ ddx:
       to: WIRE-001
     - kind: resolves
       to: AR-2026-07-13-repo
+  review:
+    self_hash: 9c4681949c5ee4823eb76b3b78411e72752795175efa03ac41f2c30d38d8f7a4
+    deps:
+      TRAIT-001: 0f7508ff4e3da65c8f84f465ea2b2c7db82797016f5333240148cdcaa7e7cc23
+      WIRE-001: de364438fe7c52ed13c6debb0f5a76ac7f4e7ee50ad3e3a27852c7dbe601eaa9
+      helix.prd: 96f0479e307f2c240e8f47b69fff510164d0b9eda132abb22cc4a860932984fe
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 # Feature Specification: FEAT-008 — Engine Embedding Contract
 
diff --git a/crates/heimq/docs/helix/01-frame/prd.md b/crates/heimq/docs/helix/01-frame/prd.md
index c0e5f3a..5bfbdd4 100644
--- a/crates/heimq/docs/helix/01-frame/prd.md
+++ b/crates/heimq/docs/helix/01-frame/prd.md
@@ -4,10 +4,10 @@ ddx:
   depends_on:
     - helix.product-vision
   review:
-    self_hash: 236574e8f31d3847bb8269d538fe07d0a47376aa7d7e75c30dca783e479ad4ab
+    self_hash: 96f0479e307f2c240e8f47b69fff510164d0b9eda132abb22cc4a860932984fe
     deps:
       helix.product-vision: 8fda503ba36c48175e42be03782c96af196de39d6e87b6edab88d853ee1857ca
-    reviewed_at: "2026-06-22T21:30:26Z"
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 # Product Requirements Document
 
@@ -69,10 +69,10 @@ broker semantics that then drift independently.
 
 | Metric | Target | Measurement Method |
 |--------|--------|--------------------|
-| Differential parity with Redpanda for in-scope APIs | 100% of in-scope APIs pass diff tests | Differential test harness in `tests/parity/` running same workload against heimq and Redpanda |
+| Differential parity with Redpanda for in-scope APIs | 100% of in-scope APIs pass diff tests | Differential test harness in `tests/conformance/` running same workload against heimq and Redpanda |
 | Standard Kafka benchmark conformance | `kafka-producer-perf-test` and `kafka-consumer-perf-test` complete with no errors at documented load; OpenMessaging Benchmark runs to completion | Bench harness scripts in `scripts/bench/` |
 | Ecosystem integration coverage | ≥1 tested integration each for: Kafka Connect, Flink, ksqlDB, Debezium, a Schema Registry client, librdkafka in ≥3 languages | Integration test suite in `tests/ecosystem/` (planned) |
-| Wire-protocol contract coverage | 100% of in-scope APIs covered by contract tests | `tests/contract.rs` + per-API contract files |
+| Wire-protocol contract coverage | 100% of in-scope APIs covered by contract tests | `contract.rs` under `crates/heimq/tests/` + per-API contract files |
 | Startup and footprint (vision differentiator) | Cold start — process exec to first successful client produce/consume round-trip — < 1 second on a CI runner; distributed as a single self-contained binary with no JVM or external services required | Startup-timing check in CI (planned); release artifact inspection |
 
 ### Non-Goals
diff --git a/crates/heimq/docs/helix/01-frame/user-stories/US-001-standard-client-connects.md b/crates/heimq/docs/helix/01-frame/user-stories/US-001-standard-client-connects.md
index c682111..c93e831 100644
--- a/crates/heimq/docs/helix/01-frame/user-stories/US-001-standard-client-connects.md
+++ b/crates/heimq/docs/helix/01-frame/user-stories/US-001-standard-client-connects.md
@@ -7,7 +7,7 @@ ddx:
     self_hash: 26bbd1249d94abf0000b005c67fc86eb59936c64075631102ba7cf7dea75e643
     deps:
       FEAT-001: 7133c264bc364ec4535c1d6b6187a90c9ba66d1fa3df30731ade260c2e092479
-    reviewed_at: "2026-06-22T21:30:26Z"
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 # US-001 — Standard Kafka client connects without code changes
 
diff --git a/crates/heimq/docs/helix/01-frame/user-stories/US-002-consumer-group-rebalance.md b/crates/heimq/docs/helix/01-frame/user-stories/US-002-consumer-group-rebalance.md
index b035e60..43c0bbe 100644
--- a/crates/heimq/docs/helix/01-frame/user-stories/US-002-consumer-group-rebalance.md
+++ b/crates/heimq/docs/helix/01-frame/user-stories/US-002-consumer-group-rebalance.md
@@ -7,7 +7,7 @@ ddx:
     self_hash: bd708ab171bb664271aa9da8cd6d24c41fe241191b1a98607544e77c75d8967a
     deps:
       FEAT-002: 164350929c7bbc09a589f3cd1a80b685e88cce1054445fe5373aec566464636f
-    reviewed_at: "2026-06-22T21:30:26Z"
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 # US-002 — Consumer group rebalance correctness
 
diff --git a/crates/heimq/docs/helix/01-frame/user-stories/US-003-idempotent-producer.md b/crates/heimq/docs/helix/01-frame/user-stories/US-003-idempotent-producer.md
index 620f7af..490f205 100644
--- a/crates/heimq/docs/helix/01-frame/user-stories/US-003-idempotent-producer.md
+++ b/crates/heimq/docs/helix/01-frame/user-stories/US-003-idempotent-producer.md
@@ -7,7 +7,7 @@ ddx:
     self_hash: 07f27eaae7593ae43b1298811d182edf0b53e89c0b629712104ecbfc5aecdaa3
     deps:
       FEAT-002: 164350929c7bbc09a589f3cd1a80b685e88cce1054445fe5373aec566464636f
-    reviewed_at: "2026-06-22T21:30:26Z"
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 # US-003 — Idempotent producer dedup
 
diff --git a/crates/heimq/docs/helix/01-frame/user-stories/US-004-transactional-commit-abort.md b/crates/heimq/docs/helix/01-frame/user-stories/US-004-transactional-commit-abort.md
index 456ef0f..e244c43 100644
--- a/crates/heimq/docs/helix/01-frame/user-stories/US-004-transactional-commit-abort.md
+++ b/crates/heimq/docs/helix/01-frame/user-stories/US-004-transactional-commit-abort.md
@@ -7,7 +7,7 @@ ddx:
     self_hash: 347fdc2f017d655b6dd9e06ea4657baaba492f3269a0894e23f3cb8883bfde16
     deps:
       FEAT-002: 164350929c7bbc09a589f3cd1a80b685e88cce1054445fe5373aec566464636f
-    reviewed_at: "2026-06-22T21:30:26Z"
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 # US-004 — Transactional commit and abort
 
diff --git a/crates/heimq/docs/helix/01-frame/user-stories/US-005-differential-parity-harness.md b/crates/heimq/docs/helix/01-frame/user-stories/US-005-differential-parity-harness.md
index 114b603..a84789b 100644
--- a/crates/heimq/docs/helix/01-frame/user-stories/US-005-differential-parity-harness.md
+++ b/crates/heimq/docs/helix/01-frame/user-stories/US-005-differential-parity-harness.md
@@ -7,7 +7,7 @@ ddx:
     self_hash: 168616bd29809066f578398a7c34ba64f23005e237d58eac1381dde83a4ef2fd
     deps:
       FEAT-003: d134a5740222f1b59ecda81b318c7b28de9968bba9f640bb6280310c1e906233
-    reviewed_at: "2026-06-22T21:30:26Z"
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 # US-005 — Run the same workload against heimq and Redpanda and diff
 
diff --git a/crates/heimq/docs/helix/01-frame/user-stories/US-006-kafka-perf-test.md b/crates/heimq/docs/helix/01-frame/user-stories/US-006-kafka-perf-test.md
index 8a5a2c7..4518d32 100644
--- a/crates/heimq/docs/helix/01-frame/user-stories/US-006-kafka-perf-test.md
+++ b/crates/heimq/docs/helix/01-frame/user-stories/US-006-kafka-perf-test.md
@@ -6,8 +6,8 @@ ddx:
   review:
     self_hash: ea037831114a863a64e1b8462648e7258daa055320a3a3f2356281e065e7ad5a
     deps:
-      FEAT-004: 15d88aa986503176f2e6b5020d55ec106bc1f7d6c05c7a1ea322d2bb9003f91e
-    reviewed_at: "2026-06-22T21:30:26Z"
+      FEAT-004: 84531399bf0bf4d4ff962e6f516c45506d67340944a8ba924e6010b4a39b64a6
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 # US-006 — Run kafka-producer-perf-test and kafka-consumer-perf-test against heimq
 
diff --git a/crates/heimq/docs/helix/01-frame/user-stories/US-007-openmessaging-benchmark.md b/crates/heimq/docs/helix/01-frame/user-stories/US-007-openmessaging-benchmark.md
index c9fac82..1df4982 100644
--- a/crates/heimq/docs/helix/01-frame/user-stories/US-007-openmessaging-benchmark.md
+++ b/crates/heimq/docs/helix/01-frame/user-stories/US-007-openmessaging-benchmark.md
@@ -6,8 +6,8 @@ ddx:
   review:
     self_hash: d5b7edf3b50f6b1189c2f6178df7b842b9371c5a46f8dda9de91d96aba28bb37
     deps:
-      FEAT-004: 15d88aa986503176f2e6b5020d55ec106bc1f7d6c05c7a1ea322d2bb9003f91e
-    reviewed_at: "2026-06-22T21:30:26Z"
+      FEAT-004: 84531399bf0bf4d4ff962e6f516c45506d67340944a8ba924e6010b4a39b64a6
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 # US-007 — Run OpenMessaging Benchmark against heimq
 
diff --git a/crates/heimq/docs/helix/01-frame/user-stories/US-008-kafka-connect.md b/crates/heimq/docs/helix/01-frame/user-stories/US-008-kafka-connect.md
index 47af9f4..0cf7bbd 100644
--- a/crates/heimq/docs/helix/01-frame/user-stories/US-008-kafka-connect.md
+++ b/crates/heimq/docs/helix/01-frame/user-stories/US-008-kafka-connect.md
@@ -7,7 +7,7 @@ ddx:
     self_hash: fd681a21fd3e5c0d0383d82fec67c1571f3efc2c572df672ddc8c95b86d69061
     deps:
       FEAT-005: c8b17ef2e79181581f03413860ea3e074364440af280321171b639668cb3aebc
-    reviewed_at: "2026-06-22T21:30:26Z"
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 # US-008 — Kafka Connect runs against heimq
 
diff --git a/crates/heimq/docs/helix/01-frame/user-stories/US-009-flink.md b/crates/heimq/docs/helix/01-frame/user-stories/US-009-flink.md
index 34c23fb..6243663 100644
--- a/crates/heimq/docs/helix/01-frame/user-stories/US-009-flink.md
+++ b/crates/heimq/docs/helix/01-frame/user-stories/US-009-flink.md
@@ -7,7 +7,7 @@ ddx:
     self_hash: 3fbf5f7cc7e25c696f69855dbf196df0260d48c851d4e4622ea2e76147c30bf3
     deps:
       FEAT-005: c8b17ef2e79181581f03413860ea3e074364440af280321171b639668cb3aebc
-    reviewed_at: "2026-06-22T21:30:26Z"
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 # US-009 — Flink Kafka source/sink against heimq
 
diff --git a/crates/heimq/docs/helix/01-frame/user-stories/US-010-debezium.md b/crates/heimq/docs/helix/01-frame/user-stories/US-010-debezium.md
index 327da4b..4b2b1bf 100644
--- a/crates/heimq/docs/helix/01-frame/user-stories/US-010-debezium.md
+++ b/crates/heimq/docs/helix/01-frame/user-stories/US-010-debezium.md
@@ -7,7 +7,7 @@ ddx:
     self_hash: d4c990816e9d3ffd0ee45dd5da9b0a5a2fb80724c546527945a0eba02f9c7f02
     deps:
       FEAT-005: c8b17ef2e79181581f03413860ea3e074364440af280321171b639668cb3aebc
-    reviewed_at: "2026-06-22T21:30:26Z"
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 # US-010 — Debezium emits CDC into heimq
 
diff --git a/crates/heimq/docs/helix/01-frame/user-stories/US-011-schema-registry.md b/crates/heimq/docs/helix/01-frame/user-stories/US-011-schema-registry.md
index 2963b0a..69e444c 100644
--- a/crates/heimq/docs/helix/01-frame/user-stories/US-011-schema-registry.md
+++ b/crates/heimq/docs/helix/01-frame/user-stories/US-011-schema-registry.md
@@ -7,7 +7,7 @@ ddx:
     self_hash: ab6e0ae60d62bfd32fbae3aa0f4caeaaf105ee38e81d63c3ddeb9811f1ccfb08
     deps:
       FEAT-005: c8b17ef2e79181581f03413860ea3e074364440af280321171b639668cb3aebc
-    reviewed_at: "2026-06-22T21:30:26Z"
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 # US-011 — Schema Registry round-trip against heimq
 
diff --git a/crates/heimq/docs/helix/01-frame/user-stories/US-012-multi-language-clients.md b/crates/heimq/docs/helix/01-frame/user-stories/US-012-multi-language-clients.md
index 6515da9..5f52b7f 100644
--- a/crates/heimq/docs/helix/01-frame/user-stories/US-012-multi-language-clients.md
+++ b/crates/heimq/docs/helix/01-frame/user-stories/US-012-multi-language-clients.md
@@ -7,7 +7,7 @@ ddx:
     self_hash: 7ca108079732515c953e1be73600ba2dc1ac5cbd02c849b216e9147951166fc3
     deps:
       FEAT-005: c8b17ef2e79181581f03413860ea3e074364440af280321171b639668cb3aebc
-    reviewed_at: "2026-06-22T21:30:26Z"
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 # US-012 — Multi-language librdkafka clients against heimq
 
diff --git a/crates/heimq/docs/helix/01-frame/user-stories/US-013-modern-client-no-downgrade.md b/crates/heimq/docs/helix/01-frame/user-stories/US-013-modern-client-no-downgrade.md
index b029abc..ec2be10 100644
--- a/crates/heimq/docs/helix/01-frame/user-stories/US-013-modern-client-no-downgrade.md
+++ b/crates/heimq/docs/helix/01-frame/user-stories/US-013-modern-client-no-downgrade.md
@@ -7,7 +7,7 @@ ddx:
     self_hash: 7ed2a25a6a8ddb99b7be414e6625c161ee01f3388ad13ec5981cb2a645db74fc
     deps:
       FEAT-006: e59d3b8965ebd35b4bbe9c5302f4218432ad83ec27691989dfd4c345ac2ae004
-    reviewed_at: "2026-06-22T21:30:26Z"
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 # US-013 — Modern Kafka client connects without forcing legacy versions
 
diff --git a/crates/heimq/docs/helix/01-frame/user-stories/US-014-ksqldb.md b/crates/heimq/docs/helix/01-frame/user-stories/US-014-ksqldb.md
index 758fe6c..9f99d49 100644
--- a/crates/heimq/docs/helix/01-frame/user-stories/US-014-ksqldb.md
+++ b/crates/heimq/docs/helix/01-frame/user-stories/US-014-ksqldb.md
@@ -7,7 +7,7 @@ ddx:
     self_hash: 93cd8d9e3a0539a6584a9dade7bdde41c77f8160282d0b34ab52bebb18f1256d
     deps:
       FEAT-005: c8b17ef2e79181581f03413860ea3e074364440af280321171b639668cb3aebc
-    reviewed_at: "2026-06-22T21:30:26Z"
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 # US-014 — ksqlDB runs against heimq
 
diff --git a/crates/heimq/docs/helix/01-frame/user-stories/US-015-durable-offsets-postgres.md b/crates/heimq/docs/helix/01-frame/user-stories/US-015-durable-offsets-postgres.md
index 3547c84..64bc098 100644
--- a/crates/heimq/docs/helix/01-frame/user-stories/US-015-durable-offsets-postgres.md
+++ b/crates/heimq/docs/helix/01-frame/user-stories/US-015-durable-offsets-postgres.md
@@ -7,7 +7,7 @@ ddx:
     self_hash: 2e5fa6adc59b1e69ed0b8be84483707fe98b10907be6beefc9c39b7b62d14174
     deps:
       FEAT-007: e947608aa0633732bde029cdf025cb8875286da1fed7bf9480548917157390ad
-    reviewed_at: "2026-06-22T21:30:26Z"
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 # US-015 — Durable offsets via Postgres
 
diff --git a/crates/heimq/docs/helix/01-frame/user-stories/US-016-backend-trait-conformance.md b/crates/heimq/docs/helix/01-frame/user-stories/US-016-backend-trait-conformance.md
index f1e893a..f5b8395 100644
--- a/crates/heimq/docs/helix/01-frame/user-stories/US-016-backend-trait-conformance.md
+++ b/crates/heimq/docs/helix/01-frame/user-stories/US-016-backend-trait-conformance.md
@@ -10,6 +10,11 @@ ddx:
       to: TRAIT-001
     - kind: covers
       to: FR-12
+  review:
+    self_hash: dbc937ade93ef6f92e5ee3a5f95ee4a23a7383ca0e4c94bd69e92099ed1d0b92
+    deps:
+      FEAT-008: 9c4681949c5ee4823eb76b3b78411e72752795175efa03ac41f2c30d38d8f7a4
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 # US-016 — Backend trait conformance for embedders
 
diff --git a/crates/heimq/docs/helix/01-frame/user-stories/US-017-request-level-handler-delegation.md b/crates/heimq/docs/helix/01-frame/user-stories/US-017-request-level-handler-delegation.md
index 257267f..e81c8f1 100644
--- a/crates/heimq/docs/helix/01-frame/user-stories/US-017-request-level-handler-delegation.md
+++ b/crates/heimq/docs/helix/01-frame/user-stories/US-017-request-level-handler-delegation.md
@@ -12,6 +12,11 @@ ddx:
       to: TRAIT-001
     - kind: covers
       to: FR-13
+  review:
+    self_hash: 9a8ed6ad154999b4271a14704f58b6d0ba07c1ed61970fc54e42dbce77c07940
+    deps:
+      FEAT-008: 9c4681949c5ee4823eb76b3b78411e72752795175efa03ac41f2c30d38d8f7a4
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 # US-017 — Request-level handler delegation for embedders
 
diff --git a/crates/heimq/docs/helix/02-design/DESIGN.md b/crates/heimq/docs/helix/02-design/DESIGN.md
index 6ecf319..e7e7c99 100644
--- a/crates/heimq/docs/helix/02-design/DESIGN.md
+++ b/crates/heimq/docs/helix/02-design/DESIGN.md
@@ -7,9 +7,9 @@ ddx:
   review:
     self_hash: 307e061e00f8bdb809b6af005acf7156d7d5647f9f6b8ec20d67aca623f0b523
     deps:
-      helix.prd: 236574e8f31d3847bb8269d538fe07d0a47376aa7d7e75c30dca783e479ad4ab
+      helix.prd: 96f0479e307f2c240e8f47b69fff510164d0b9eda132abb22cc4a860932984fe
       helix.product-vision: 8fda503ba36c48175e42be03782c96af196de39d6e87b6edab88d853ee1857ca
-    reviewed_at: "2026-06-22T21:30:26Z"
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 
 # DESIGN.md - heimq Microsite
diff --git a/crates/heimq/docs/helix/02-design/contracts/API-001-kafka-protocol.md b/crates/heimq/docs/helix/02-design/contracts/API-001-kafka-protocol.md
index 26f89e3..cde7202 100644
--- a/crates/heimq/docs/helix/02-design/contracts/API-001-kafka-protocol.md
+++ b/crates/heimq/docs/helix/02-design/contracts/API-001-kafka-protocol.md
@@ -11,14 +11,14 @@ ddx:
     - FEAT-006
     - ADR-003
   review:
-    self_hash: d85884155fba0fe46b242b6bdc939b612c55ea033bc2d68b6cf0c50c114d18b4
+    self_hash: 85aaecdeafe47af73d0e287b3851f021313a23a96dc492d2646f78913978d5de
     deps:
       ADR-003: c06761cf28323ddb6162d309cfcfe4db1037f915a2c05a716ad4262222193145
       FEAT-001: 7133c264bc364ec4535c1d6b6187a90c9ba66d1fa3df30731ade260c2e092479
       FEAT-002: 164350929c7bbc09a589f3cd1a80b685e88cce1054445fe5373aec566464636f
       FEAT-006: e59d3b8965ebd35b4bbe9c5302f4218432ad83ec27691989dfd4c345ac2ae004
-      helix.prd: 236574e8f31d3847bb8269d538fe07d0a47376aa7d7e75c30dca783e479ad4ab
-    reviewed_at: "2026-06-22T21:30:26Z"
+      helix.prd: 96f0479e307f2c240e8f47b69fff510164d0b9eda132abb22cc4a860932984fe
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 
 # API Contract: Kafka Wire Protocol [heimq]
@@ -62,7 +62,7 @@ ddx:
   generated encode/decode (see ADR-003); a thin dispatch shim under
   `src/protocol/codec/` may or may not be introduced during FEAT-006
   implementation.
-- The static version table is `SUPPORTED_APIS` in `src/protocol/mod.rs`;
+- The static version table is `SUPPORTED_APIS` in `mod.rs` under `crates/heimq/src/protocol/`;
   the matrix below mirrors it. Any change to advertised versions must
   update both.
 - **Per-API maxima** (pinned by FEAT-006 FR-03):
@@ -72,8 +72,8 @@ ddx:
   ApiVersions v3, CreateTopics v7, DeleteTopics v6, InitProducerId v5,
   AddPartitionsToTxn v5, AddOffsetsToTxn v4, EndTxn v4, WriteTxnMarkers v1,
   TxnOffsetCommit v4. The matrix below mirrors `SUPPORTED_APIS` in
-  `src/protocol/mod.rs`; any change to advertised versions must update both.
-- **Capability-derived advertisement**: The ApiVersions response is not a verbatim copy of `SUPPORTED_APIS`. At runtime, `compute_supported_apis` (`src/protocol/mod.rs`) intersects the static table with the per-API `CapabilityGate` against each backend's descriptor (`BackendCapabilities`, `OffsetStoreCapabilities`, `GroupCoordinatorCapabilities`). APIs whose required backend is absent (e.g. no group coordinator) are filtered out before the response is encoded, so heimq advertises only what its currently configured backends can actually serve. Gating is per-API, not a global meet — a backend that lacks compaction does not lose unrelated APIs.
+  `mod.rs` under `crates/heimq/src/protocol/`; any change to advertised versions must update both.
+- **Capability-derived advertisement**: The ApiVersions response is not a verbatim copy of `SUPPORTED_APIS`. At runtime, `compute_supported_apis` (`mod.rs` under `crates/heimq/src/protocol/`) intersects the static table with the per-API `CapabilityGate` against each backend's descriptor (`BackendCapabilities`, `OffsetStoreCapabilities`, `GroupCoordinatorCapabilities`). APIs whose required backend is absent (e.g. no group coordinator) are filtered out before the response is encoded, so heimq advertises only what its currently configured backends can actually serve. Gating is per-API, not a global meet — a backend that lacks compaction does not lose unrelated APIs.
 
 ### Support Matrix (Kafka API Keys)
 
@@ -92,31 +92,31 @@ Reason codes (Exclusions):
 
 | API Key | Name | heimq Status | Supported Versions | Tests | Notes |
 | --- | --- | --- | --- | --- | --- |
-| 0 | Produce | Supported | 0-11 | `src/handler/tests.rs`; `tests/contract.rs`; `tests/integration.rs` | In-memory append only |
-| 1 | Fetch | Supported | 0-12 | `src/handler/tests.rs`; `tests/contract.rs`; `tests/integration.rs` | In-memory read only; capped at v12 (v13+ uses topic_id UUID, name-based lookup unsupported) |
-| 2 | ListOffsets | Supported | 0-9 | `src/handler/tests.rs`; `tests/contract.rs` | Timestamp lookups simplified |
-| 3 | Metadata | Supported | 0-12 | `src/handler/tests.rs`; `tests/contract.rs`; `tests/integration.rs` | Single broker only |
-| 8 | OffsetCommit | Supported | 0-9 | `src/handler/tests.rs`; `tests/contract.rs` | In-memory offsets |
-| 9 | OffsetFetch | Supported | 0-9 | `src/handler/tests.rs`; `tests/contract.rs`; `tests/integration.rs` | In-memory offsets; v8+ uses groups response structure |
-| 10 | FindCoordinator | Supported | 0-6 | `src/handler/tests.rs` | Single coordinator (self) |
-| 11 | JoinGroup | Supported | 0-9 | `src/handler/tests.rs` | Simplified group state |
-| 12 | Heartbeat | Supported | 0-4 | `src/handler/tests.rs` | Simplified liveness |
-| 13 | LeaveGroup | Supported | 0-5 | `src/handler/tests.rs` | Member removal only |
-| 14 | SyncGroup | Supported | 0-5 | `src/handler/tests.rs` | Simplified assignment |
+| 0 | Produce | Supported | 0-11 | `crates/heimq/src/handler/tests.rs`; `contract.rs` under `crates/heimq/tests/`; `tests/conformance/integration/` | In-memory append only |
+| 1 | Fetch | Supported | 0-12 | `crates/heimq/src/handler/tests.rs`; `contract.rs` under `crates/heimq/tests/`; `tests/conformance/integration/` | In-memory read only; capped at v12 (v13+ uses topic_id UUID, name-based lookup unsupported) |
+| 2 | ListOffsets | Supported | 0-9 | `crates/heimq/src/handler/tests.rs`; `contract.rs` under `crates/heimq/tests/` | Timestamp lookups simplified |
+| 3 | Metadata | Supported | 0-12 | `crates/heimq/src/handler/tests.rs`; `contract.rs` under `crates/heimq/tests/`; `tests/conformance/integration/` | Single broker only |
+| 8 | OffsetCommit | Supported | 0-9 | `crates/heimq/src/handler/tests.rs`; `contract.rs` under `crates/heimq/tests/` | In-memory offsets |
+| 9 | OffsetFetch | Supported | 0-9 | `crates/heimq/src/handler/tests.rs`; `contract.rs` under `crates/heimq/tests/`; `tests/conformance/integration/` | In-memory offsets; v8+ uses groups response structure |
+| 10 | FindCoordinator | Supported | 0-6 | `crates/heimq/src/handler/tests.rs` | Single coordinator (self) |
+| 11 | JoinGroup | Supported | 0-9 | `crates/heimq/src/handler/tests.rs` | Simplified group state |
+| 12 | Heartbeat | Supported | 0-4 | `crates/heimq/src/handler/tests.rs` | Simplified liveness |
+| 13 | LeaveGroup | Supported | 0-5 | `crates/heimq/src/handler/tests.rs` | Member removal only |
+| 14 | SyncGroup | Supported | 0-5 | `crates/heimq/src/handler/tests.rs` | Simplified assignment |
 | 15 | DescribeGroups | Excluded (R4) | N/A | N/A | Out of scope for current single-node implementation |
 | 16 | ListGroups | Excluded (R4) | N/A | N/A | Out of scope for current single-node implementation |
 | 17 | SaslHandshake | Excluded (R2) | N/A | N/A | Out of scope for current single-node implementation |
-| 18 | ApiVersions | Supported | 0-3 | `src/handler/tests.rs`; `tests/contract.rs`; `src/protocol/router.rs` | Version negotiation only |
-| 19 | CreateTopics | Supported | 0-7 | `src/handler/tests.rs`; `tests/contract.rs` | No config validation |
-| 20 | DeleteTopics | Supported | 0-6 | `src/handler/tests.rs`; `tests/contract.rs` | Best-effort delete |
+| 18 | ApiVersions | Supported | 0-3 | `crates/heimq/src/handler/tests.rs`; `contract.rs` under `crates/heimq/tests/`; `crates/heimq/src/protocol/router.rs` | Version negotiation only |
+| 19 | CreateTopics | Supported | 0-7 | `crates/heimq/src/handler/tests.rs`; `contract.rs` under `crates/heimq/tests/` | No config validation |
+| 20 | DeleteTopics | Supported | 0-6 | `crates/heimq/src/handler/tests.rs`; `contract.rs` under `crates/heimq/tests/` | Best-effort delete |
 | 21 | DeleteRecords | Excluded (R4) | N/A | N/A | Out of scope for current single-node implementation |
-| 22 | InitProducerId | Supported (FEAT-002) | 0-5 | `tests/contract.rs` | Required for idempotent producer + transactions |
+| 22 | InitProducerId | Supported (FEAT-002) | 0-5 | `contract.rs` under `crates/heimq/tests/` | Required for idempotent producer + transactions |
 | 23 | OffsetForLeaderEpoch | Excluded (R1) | N/A | N/A | Out of scope for current single-node implementation |
-| 24 | AddPartitionsToTxn | Supported (FEAT-002) | 0-5 | `tests/contract.rs` | Single-coordinator transaction state machine |
-| 25 | AddOffsetsToTxn | Supported (FEAT-002) | 0-4 | `tests/contract.rs` | Single-coordinator transaction state machine |
-| 26 | EndTxn | Supported (FEAT-002) | 0-4 | `tests/contract.rs` | Single-coordinator transaction state machine |
-| 27 | WriteTxnMarkers | Supported (FEAT-002) | 0-1 | `tests/contract.rs` | Control batches drive read_committed visibility |
-| 28 | TxnOffsetCommit | Supported (FEAT-002) | 0-4 | `tests/contract.rs` | EOS consumer offset commits |
+| 24 | AddPartitionsToTxn | Supported (FEAT-002) | 0-5 | `contract.rs` under `crates/heimq/tests/` | Single-coordinator transaction state machine |
+| 25 | AddOffsetsToTxn | Supported (FEAT-002) | 0-4 | `contract.rs` under `crates/heimq/tests/` | Single-coordinator transaction state machine |
+| 26 | EndTxn | Supported (FEAT-002) | 0-4 | `contract.rs` under `crates/heimq/tests/` | Single-coordinator transaction state machine |
+| 27 | WriteTxnMarkers | Supported (FEAT-002) | 0-1 | `contract.rs` under `crates/heimq/tests/` | Control batches drive read_committed visibility |
+| 28 | TxnOffsetCommit | Supported (FEAT-002) | 0-4 | `contract.rs` under `crates/heimq/tests/` | EOS consumer offset commits |
 | 29 | DescribeAcls | Excluded (R2) | N/A | N/A | Out of scope for current single-node implementation |
 | 30 | CreateAcls | Excluded (R2) | N/A | N/A | Out of scope for current single-node implementation |
 | 31 | DeleteAcls | Excluded (R2) | N/A | N/A | Out of scope for current single-node implementation |
@@ -143,10 +143,10 @@ Reason codes (Exclusions):
 | 55 | DescribeQuorum | Excluded (R1) | N/A | N/A | Out of scope for current single-node implementation |
 | 57 | UpdateFeatures | Excluded (R1) | N/A | N/A | Out of scope for current single-node implementation |
 | 60 | DescribeCluster | Excluded (R4) | N/A | N/A | Out of scope for current single-node implementation |
-| 61 | DescribeProducers | Planned (FEAT-002) | TBD | Pending (`tests/contract/transactions.rs`) | Producer-id state introspection |
+| 61 | DescribeProducers | Planned (FEAT-002) | TBD | Pending per-API contract test | Producer-id state introspection |
 | 64 | UnregisterBroker | Excluded (R1) | N/A | N/A | Out of scope for current single-node implementation |
-| 65 | DescribeTransactions | Planned (FEAT-002) | TBD | Pending (`tests/contract/transactions.rs`) | Transaction-state introspection |
-| 66 | ListTransactions | Planned (FEAT-002) | TBD | Pending (`tests/contract/transactions.rs`) | Transaction-state introspection |
+| 65 | DescribeTransactions | Planned (FEAT-002) | TBD | Pending per-API contract test | Transaction-state introspection |
+| 66 | ListTransactions | Planned (FEAT-002) | TBD | Pending per-API contract test | Transaction-state introspection |
 | 68 | ConsumerGroupHeartbeat | Excluded (R5) | N/A | N/A | Out of scope for current single-node implementation |
 | 69 | ConsumerGroupDescribe | Excluded (R5) | N/A | N/A | Out of scope for current single-node implementation |
 | 71 | GetTelemetrySubscriptions | Excluded (R5) | N/A | N/A | Out of scope for current single-node implementation |
@@ -177,7 +177,7 @@ Reason codes (Exclusions):
 ## Validation Checklist
 
 ### Required Contract Tests (per supported API)
-1. **ApiVersions**: version negotiation reflects `compute_supported_apis(SUPPORTED_APIS, backend capabilities)` — i.e. the static table intersected with the per-API capability gates of the configured backends. Unit tests in `src/protocol/mod.rs` pin the intersection behaviour (memory default advertises full set; missing group coordinator drops only group APIs; missing offset store drops only offset APIs; capability flags do not leak across APIs).
+1. **ApiVersions**: version negotiation reflects `compute_supported_apis(SUPPORTED_APIS, backend capabilities)` — i.e. the static table intersected with the per-API capability gates of the configured backends. Unit tests in `mod.rs` under `crates/heimq/src/protocol/` pin the intersection behaviour (memory default advertises full set; missing group coordinator drops only group APIs; missing offset store drops only offset APIs; capability flags do not leak across APIs).
 2. **Metadata**: topic discovery, auto-create behavior, error for disabled auto-create.
 3. **Produce**: empty batch, keyed/unkeyed, large batch, partition error codes.
 4. **Fetch**: offsets, high watermark, empty responses, max_bytes enforcement.
@@ -192,7 +192,7 @@ Reason codes (Exclusions):
 
 - Versioning: per-API supported version ranges are advertised via ApiVersions
   (see Version Policy above); the static table is `SUPPORTED_APIS` in
-  `src/protocol/mod.rs`.
+  `mod.rs` under `crates/heimq/src/protocol/`.
 - Flexible versions are supported per FEAT-006; legacy versions remain
   supported and are exercised by existing contract tests.
 - All future changes must remain additive or gated by ApiVersions.
@@ -210,13 +210,13 @@ zero diffs at flexible versions vs Redpanda.
 Concrete request/response examples are not inlined here. Wire-format examples
 for each API follow the Kafka Protocol Guide (see **Source** above); per-API
 executable examples are pinned by the contract tests listed in the Support
-Matrix `Tests` column (`tests/contract.rs`, `tests/integration.rs`,
-`src/handler/tests.rs`).
+Matrix `Tests` column (`contract.rs` under `crates/heimq/tests/`, `tests/conformance/integration/`,
+`crates/heimq/src/handler/tests.rs`).
 
 ## Feature Traceability
 
 - **PRD**: `docs/helix/01-frame/prd.md` (P0 #1 wire compat, #2 groups, #3 idempotent producers, #4 transactions, #5 parity, #6 benchmarks, #7 ecosystem).
 - **Feature specs**: FEAT-001 (wire protocol), FEAT-002 (groups + transactions + idempotency), FEAT-003 (differential parity), FEAT-004 (benchmark conformance), FEAT-005 (ecosystem integrations), FEAT-006 (flexible-version protocol).
-- **Implementation**: `src/handler/*.rs`, `src/protocol/*`, `src/storage/*`; transaction coordinator and producer-id manager TBD under FEAT-002.
-- **Tests**: `tests/contract.rs`, `tests/integration.rs`, `src/handler/tests.rs`, `src/protocol/router.rs`, storage module unit/property tests; planned `tests/contract/transactions.rs`, `tests/parity/`, `tests/ecosystem/`, `scripts/bench/`.
+- **Implementation**: `crates/heimq/src/handler/*.rs`, `crates/heimq/src/protocol/*`, `crates/heimq/src/storage/*`; transaction coordinator and producer-id manager TBD under FEAT-002.
+- **Tests**: `contract.rs` under `crates/heimq/tests/`, `tests/conformance/integration/`, `crates/heimq/src/handler/tests.rs`, `crates/heimq/src/protocol/router.rs`, storage module unit/property tests; planned per-API contract files, `tests/conformance/`, `tests/ecosystem/`, `scripts/bench/`.
 - **Related Doc**: `docs/helix/03-test/test-plan/test-plan.md`.
diff --git a/crates/heimq/docs/helix/02-design/contracts/CODEC-001-flexible.md b/crates/heimq/docs/helix/02-design/contracts/CODEC-001-flexible.md
index 57a664d..008daf5 100644
--- a/crates/heimq/docs/helix/02-design/contracts/CODEC-001-flexible.md
+++ b/crates/heimq/docs/helix/02-design/contracts/CODEC-001-flexible.md
@@ -9,12 +9,12 @@ ddx:
     - API-001
     - ADR-003
   review:
-    self_hash: c6d7122c6717a77692e3af031be383978036f7c85d57813998b33c51cd81c9ff
+    self_hash: d83709237e2ca19409a81cc063bfba856b91e67d8231964561686d2ec602c919
     deps:
       ADR-003: c06761cf28323ddb6162d309cfcfe4db1037f915a2c05a716ad4262222193145
-      API-001: d85884155fba0fe46b242b6bdc939b612c55ea033bc2d68b6cf0c50c114d18b4
+      API-001: 85aaecdeafe47af73d0e287b3851f021313a23a96dc492d2646f78913978d5de
       FEAT-006: e59d3b8965ebd35b4bbe9c5302f4218432ad83ec27691989dfd4c345ac2ae004
-    reviewed_at: "2026-06-22T21:30:26Z"
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 
 # CODEC-001: Flexible-Version Codec Module Surface
@@ -82,7 +82,7 @@ version. No heimq-owned codec primitive module is required.
 
 #### `is_flexible(api_key: i16, api_version: i16) -> bool`
 
-**Location**: `src/protocol/flexible.rs` (re-exported via `src/protocol/mod.rs`)
+**Location**: `crates/heimq-handlers/src/flexible.rs` (re-exported via `mod.rs` under `crates/heimq/src/protocol/`)
 
 Returns `true` when the request/response pair for `(api_key, api_version)`
 must use flexible framing. Implemented as a lookup against the
@@ -187,7 +187,7 @@ from the `RequestHeader`.
 
 ### Router Integration
 
-**Location**: `src/protocol/router.rs`
+**Location**: `crates/heimq/src/protocol/router.rs`
 
 `Router::route` calls `decode_request`, which already returns the parsed
 `RequestHeader` (containing `api_key` and `api_version`). No change to the
@@ -296,7 +296,7 @@ The following tests verify this contract at FEAT-006 implementation time:
    end-to-end; assert correlation_id is preserved and response decodes cleanly.
 
 Test locations: `src/protocol/codec.rs` (unit tests 1–5),
-`src/protocol/router.rs` (integration test 6).
+`crates/heimq/src/protocol/router.rs` (integration test 6).
 
 ---
 
diff --git a/crates/heimq/docs/helix/02-design/contracts/TRAIT-001-backend-traits.md b/crates/heimq/docs/helix/02-design/contracts/TRAIT-001-backend-traits.md
index b7016f8..5884e68 100644
--- a/crates/heimq/docs/helix/02-design/contracts/TRAIT-001-backend-traits.md
+++ b/crates/heimq/docs/helix/02-design/contracts/TRAIT-001-backend-traits.md
@@ -10,12 +10,12 @@ ddx:
     - ADR-006
     - ADR-007
   review:
-    self_hash: 34a65294dc8ed94e0fb07b0f1c247f23a2ffacf24b42164b375320148462d3c3
+    self_hash: 0f7508ff4e3da65c8f84f465ea2b2c7db82797016f5333240148cdcaa7e7cc23
     deps:
       ADR-006: 881bf5e99cfef0f38fec536b48898ee1d3bf40b1be8bd2fbfc036f48aabfc385
-      ADR-007: 44b47ae3485b6c355c48380610ad1ae6d2cb3779c8ea5d2f0b96910993826500
-      helix.prd: 236574e8f31d3847bb8269d538fe07d0a47376aa7d7e75c30dca783e479ad4ab
-    reviewed_at: "2026-06-22T21:30:26Z"
+      ADR-007: 8d887546b3e7a83c6648b84d4c325cd3a1a1545ebabb877cedf17f8ca9195ea0
+      helix.prd: 96f0479e307f2c240e8f47b69fff510164d0b9eda132abb22cc4a860932984fe
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 
 # TRAIT-001: Backend Trait Families
@@ -44,7 +44,7 @@ principal context threading, per-family conformance suite obligations.
 Out of scope: internal backend implementations; wire framing (WIRE-001,
 CODEC-001); SASL/TLS; fjord/pqueue/niflheim product semantics.
 
-Owning system: `heimq-broker` crate (`src/storage/`, `src/consumer_group/`).
+Owning system: `heimq-broker` crate (`crates/heimq-broker/src/storage/`, `crates/heimq-broker/src/consumer_group/`).
 
 ---
 
@@ -64,7 +64,7 @@ covers only the context threading.
 
 **Capability-gated advertisement**: A backend family absent or returning
 `name == "unknown"` causes `compute_supported_apis`
-(`src/protocol/mod.rs:108`) to filter all APIs gated on that family from the
+(`mod.rs` under `crates/heimq/src/protocol/`) to filter all APIs gated on that family from the
 ApiVersions response. Gating is per-API — a missing family does not remove
 unrelated APIs.
 
@@ -80,12 +80,12 @@ on restart. Clients receive `UNKNOWN_PRODUCER_ID` and re-initialize via
 
 ### Family 1 — Log (`LogBackend` / `TopicLog` / `PartitionLog`)
 
-Provenance: `src/storage/mod.rs:64–148`
+Provenance: `crates/heimq-broker/src/storage/mod.rs:64–148`
 
 Existing baseline signatures (verbatim):
 
 ```rust
-// src/storage/mod.rs:64
+// crates/heimq-broker/src/storage/mod.rs:64
 pub trait LogBackend: Send + Sync {
     fn create_topic(&self, name: &str, num_partitions: i32) -> Result<Arc<dyn TopicLog>>;
     fn delete_topic(&self, name: &str) -> Result<()>;
@@ -113,7 +113,7 @@ pub trait LogBackend: Send + Sync {
     fn log_start_offset(&self, topic_name: &str, partition: i32) -> Result<i64>;
 }
 
-// src/storage/mod.rs:112
+// crates/heimq-broker/src/storage/mod.rs:112
 pub trait TopicLog: Send + Sync {
     fn name(&self) -> &str;
     fn num_partitions(&self) -> i32;
@@ -121,7 +121,7 @@ pub trait TopicLog: Send + Sync {
     fn config(&self) -> &TopicConfig;
 }
 
-// src/storage/mod.rs:121
+// crates/heimq-broker/src/storage/mod.rs:121
 pub trait PartitionLog: Send + Sync {
     fn id(&self) -> i32;
     fn append(&self, view: &RecordBatchView<'_>, raw_bytes: Option<&[u8]>) -> Result<(i64, i64)>;
@@ -164,10 +164,10 @@ Normative rules:
 
 ### Family 2 — `OffsetStore`
 
-Provenance: `src/storage/offset_store.rs:58–84`
+Provenance: `crates/heimq-broker/src/storage/offset_store.rs:58–84`
 
 ```rust
-// src/storage/offset_store.rs:58
+// crates/heimq-broker/src/storage/offset_store.rs:58
 pub trait OffsetStore: Send + Sync {
     // Returns Result<()> since seam heimq-ec30673c: completion semantics require
     // the impl to complete all durability work before returning and signal failure.
@@ -182,7 +182,7 @@ pub trait OffsetStore: Send + Sync {
 
 Normative rules:
 - Durability is an impl property declared in `OffsetStoreCapabilities`
-  (`src/storage/offset_store.rs:28`): `durability: Durability`,
+  (`crates/heimq-broker/src/storage/offset_store.rs:28`): `durability: Durability`,
   `survives_restart: bool`.
 - After `commit` returns, a subsequent `fetch` for the same key MUST return the
   committed value (read-your-writes within a process).
@@ -195,10 +195,10 @@ Normative rules:
 
 ### Family 3 — `GroupCoordinatorBackend`
 
-Provenance: `src/consumer_group/backend.rs:123–142`
+Provenance: `crates/heimq-broker/src/consumer_group/backend.rs:123–142`
 
 ```rust
-// src/consumer_group/backend.rs:123
+// crates/heimq-broker/src/consumer_group/backend.rs:123
 pub trait GroupCoordinatorBackend: Send + Sync {
     fn join_group(&self, req: JoinRequest) -> JoinResult;
     fn sync_group(&self, req: SyncRequest) -> SyncResult;
@@ -234,10 +234,10 @@ the current synchronous surface.
 
 Key DTOs are normative: `JoinRequest`, `JoinResult`, `JoinMember`,
 `SyncRequest`, `SyncResult`, `HeartbeatResult`, `LeaveResult`
-(verbatim `src/consumer_group/backend.rs:53–116`). Field semantics match the
+(verbatim `crates/heimq-broker/src/consumer_group/backend.rs:53–116`). Field semantics match the
 Kafka protocol.
 
-`GroupCoordinatorCapabilities` (`src/consumer_group/backend.rs:19`):
+`GroupCoordinatorCapabilities` (`crates/heimq-broker/src/consumer_group/backend.rs:19`):
 `name`, `version`, `durability`, `survives_restart`, `multi_node: bool`.
 
 ---
@@ -352,7 +352,7 @@ fn find_coordinator(&self, _g: &str) -> Result<BrokerInfo, ClusterViewError> {
 - [x] Normative fields and rules are explicit.
 - [x] Compatibility and precedence rules are explicit.
 - [x] Error handling is explicit.
-- [x] Executable tests derivable: per-family conformance suites; `compute_supported_apis` tests at `src/protocol/mod.rs:125–250`.
+- [x] Executable tests derivable: per-family conformance suites; `compute_supported_apis` tests in `mod.rs` under `crates/heimq/src/protocol/`.
 - [x] Non-normative notes cannot be mistaken for contract requirements.
 - [ ] `ClusterView` trait and DTOs finalized in Slice 1 (currently proposed).
 - [x] `RequestContext` principal/tenant/client-id threading implemented in backend traits (bead heimq-43770932).
diff --git a/crates/heimq/docs/helix/02-design/contracts/WIRE-001-wire-scaffolding.md b/crates/heimq/docs/helix/02-design/contracts/WIRE-001-wire-scaffolding.md
index a4de92b..fc7c477 100644
--- a/crates/heimq/docs/helix/02-design/contracts/WIRE-001-wire-scaffolding.md
+++ b/crates/heimq/docs/helix/02-design/contracts/WIRE-001-wire-scaffolding.md
@@ -12,10 +12,10 @@ ddx:
   review:
     self_hash: de364438fe7c52ed13c6debb0f5a76ac7f4e7ee50ad3e3a27852c7dbe601eaa9
     deps:
-      API-001: d85884155fba0fe46b242b6bdc939b612c55ea033bc2d68b6cf0c50c114d18b4
-      CODEC-001: c6d7122c6717a77692e3af031be383978036f7c85d57813998b33c51cd81c9ff
-      helix.prd: 236574e8f31d3847bb8269d538fe07d0a47376aa7d7e75c30dca783e479ad4ab
-    reviewed_at: "2026-06-22T21:30:26Z"
+      API-001: 85aaecdeafe47af73d0e287b3851f021313a23a96dc492d2646f78913978d5de
+      CODEC-001: d83709237e2ca19409a81cc063bfba856b91e67d8231964561686d2ec602c919
+      helix.prd: 96f0479e307f2c240e8f47b69fff510164d0b9eda132abb22cc4a860932984fe
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 
 # WIRE-001: Wire Scaffolding Contract
diff --git a/crates/heimq/docs/helix/02-design/solution-designs/SD-003-differential-parity-testing.md b/crates/heimq/docs/helix/02-design/solution-designs/SD-003-differential-parity-testing.md
index 056adce..d326531 100644
--- a/crates/heimq/docs/helix/02-design/solution-designs/SD-003-differential-parity-testing.md
+++ b/crates/heimq/docs/helix/02-design/solution-designs/SD-003-differential-parity-testing.md
@@ -13,8 +13,8 @@ ddx:
     deps:
       FEAT-003: d134a5740222f1b59ecda81b318c7b28de9968bba9f640bb6280310c1e906233
       US-005: 168616bd29809066f578398a7c34ba64f23005e237d58eac1381dde83a4ef2fd
-      helix.prd: 236574e8f31d3847bb8269d538fe07d0a47376aa7d7e75c30dca783e479ad4ab
-    reviewed_at: "2026-06-22T21:30:26Z"
+      helix.prd: 96f0479e307f2c240e8f47b69fff510164d0b9eda132abb22cc4a860932984fe
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 
 # Design: Differential Parity Harness [SD-003]
diff --git a/crates/heimq/docs/helix/04-build/implementation-plan.md b/crates/heimq/docs/helix/04-build/implementation-plan.md
index ba4255a..a36ae10 100644
--- a/crates/heimq/docs/helix/04-build/implementation-plan.md
+++ b/crates/heimq/docs/helix/04-build/implementation-plan.md
@@ -7,11 +7,11 @@ ddx:
     - helix.prd
     - helix.product-vision
   review:
-    self_hash: 329b4d9ab3db67b9977ed0bf2c489426542a9e510e26154558f87ba51913fee4
+    self_hash: a81de258a7ce95ebf74b07d1f46b742c0e5c24013b6a8d41ae6ca8d6ccf5be7a
     deps:
-      helix.prd: 236574e8f31d3847bb8269d538fe07d0a47376aa7d7e75c30dca783e479ad4ab
+      helix.prd: 96f0479e307f2c240e8f47b69fff510164d0b9eda132abb22cc4a860932984fe
       helix.product-vision: 8fda503ba36c48175e42be03782c96af196de39d6e87b6edab88d853ee1857ca
-    reviewed_at: "2026-06-22T21:30:26Z"
+    reviewed_at: "2026-07-14T05:12:26Z"
 ---
 # Implementation Plan: heimq Engine Restructuring Program
 
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
