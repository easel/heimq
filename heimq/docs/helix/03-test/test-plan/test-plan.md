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

**Out of Scope (for now)**:
- Distributed system semantics (controller, replication, leader epochs).
- Security/authentication/authorization (SASL, ACLs).
- Transactions and idempotent producer guarantees.

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

1. Produce -> Fetch roundtrip (single partition)
2. Metadata discovery and auto-create behavior
3. Consumer group join/sync/heartbeat lifecycle
4. Offset commit + fetch for group consumption

## Baseline Alignment

- **Redpanda parity**: Run `scripts/compatibility-test.sh` weekly and on protocol changes.
- **Kafka parity**: Add a Kafka docker target and compare responses for the same workloads.
- **Golden traces**: Capture request/response fixtures from Kafka/Redpanda and assert equivalence.

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
- [ ] Consumer Groups: FindCoordinator, Join/Sync/Heartbeat/Leave, OffsetCommit/Fetch (unit tests done; contract parity pending).

### Phase 3: Integration and Regression (P0)
- [ ] Expand integration test coverage to include consumer group offsets and group lifecycle.
- [ ] Add legacy protocol edge cases via `kafka` crate.

### Phase 4: Baseline Parity (P0)
- [ ] Add Kafka docker target to `scripts/compatibility-test.sh`.
- [ ] Add Redpanda/Kafka golden request/response fixtures for parity regression tests.

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
- Parity diffs against Kafka/Redpanda are tracked and intentional.
