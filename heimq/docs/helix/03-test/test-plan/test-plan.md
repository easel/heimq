# Test Plan

**Project**: heimq
**Version**: 0.1.0
**Date**: 2026-01-31
**Status**: Draft
**Author**: Codex (freyr)

## Executive Summary

heimq targets Kafka protocol compatibility on a single node with no durability guarantees. The test strategy prioritizes protocol contract compliance and behavioral parity with Kafka/Redpanda for the subset of APIs in scope. Property-based tests are the starting layer to validate invariants in storage and protocol framing, followed by contract tests per API and integration tests with real Kafka clients.

## Testing Strategy

### Scope and Objectives

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

## Test Organization

### Directory Structure (current + target)

```
heimq/
├── tests/
│   ├── contract.rs
│   ├── integration.rs
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
  (produce/fetch, consumer groups, idempotent producers, transactions).
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
- [ ] Codec invariants: request/response framing and correlation id integrity.

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

### Phase 4: Baseline Parity (P0)
- [ ] Add Kafka docker target to `scripts/compatibility-test.sh`.
- [ ] Add Redpanda/Kafka golden request/response fixtures for parity regression tests.

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

- [ ] Codec primitives: compact string, compact bytes, compact array, unsigned varint, signed (zigzag) varint, tagged-fields block — round-trip property tests.
- [ ] Flexible request header v2 + flexible response header v1 wired into the router for flexible APIs.
- [ ] Each in-scope API gains its flexible-version handler path; legacy paths retained.
- [ ] `SUPPORTED_APIS` updated to advertise current Kafka per-API maxima for the in-scope surface.
- [ ] ApiVersions v3 (flexible request body, including ignored client_software_name / client_software_version) implemented.
- [ ] Differential parity (Phase 8) reports zero diffs at flexible versions for in-scope APIs.

### Phase 7: Idempotent Producer + Transactions (P0, FEAT-002)

Spec traceability: FEAT-002, PRD P0 #3 (idempotent producers) and #4
(transactions). Adds the API keys currently marked `Planned (FEAT-002)` in
`API-001-kafka-protocol.md`: InitProducerId (22), AddPartitionsToTxn (24),
AddOffsetsToTxn (25), EndTxn (26), WriteTxnMarkers (27), TxnOffsetCommit
(28), DescribeProducers (61), DescribeTransactions (65), ListTransactions
(66).

#### P0 — Idempotent producer
- [ ] `enable.idempotence=true` retried batch is de-duped (no duplicate visible to consumer).
- [ ] Out-of-order sequence returns `OUT_OF_ORDER_SEQUENCE_NUMBER`.
- [ ] Duplicate sequence returns `DUPLICATE_SEQUENCE_NUMBER` (or is silently de-duped per Kafka semantics).
- [ ] `InitProducerId` returns producerId/epoch; epoch bumps on re-init.

#### P0 — Transactions
- [ ] Committed transaction is visible to `read_committed` consumers.
- [ ] Aborted transaction is invisible to `read_committed` consumers.
- [ ] `read_uncommitted` consumer observes both.
- [ ] Stale producer epoch returns `INVALID_PRODUCER_EPOCH`.
- [ ] `transaction.timeout.ms` is enforced; expired transactions are aborted.
- [ ] `TxnOffsetCommit` participates in transaction lifecycle for EOS consumer.

### Phase 8: Differential Parity Harness (P0, FEAT-003)

Spec traceability: FEAT-003, PRD P0 #5. Lives under `tests/parity/`.

- [ ] Harness scaffolding: same client workload against heimq and Redpanda; structured diff output.
- [ ] Normalization rules for broker ids, host timestamps, monotonic ids.
- [ ] Workloads cover produce/fetch, consumer groups, idempotent producers, transactions.
- [ ] CI gating job: parity harness runs on protocol-touching changes; zero-diff is the success condition.
- [ ] Known-divergence registry (e.g., loss-on-restart) with PRD references.

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
