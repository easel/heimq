---
dun:
  id: FEAT-004
  depends_on:
    - helix.prd
    - FEAT-001
---
# Feature Specification: FEAT-004 — Standard Kafka Benchmark Conformance

**Feature ID**: FEAT-004
**Status**: Specified
**Priority**: P0
**Owner**: heimq core

## Overview

heimq runs the standard, commonly-used Kafka benchmarks to completion
without protocol or client errors. This is a conformance test, not a
performance target — it verifies that the API surface and client
state machines exercised by these tools are correct.

## Problem Statement

- **Current situation**: heimq has no benchmark harness; no standard
  Kafka benchmark has been run end-to-end against it.
- **Pain points**: Standard benchmarks exercise protocol corners that
  unit tests miss (admin RPCs, repeated metadata refresh, large-batch
  produce, idempotent / transactional flows under load). Without them,
  divergences only surface in production tools.
- **Desired outcome**: `kafka-producer-perf-test`,
  `kafka-consumer-perf-test`, and the OpenMessaging Benchmark Kafka
  driver all complete against heimq at a documented load profile,
  reporting zero protocol/client errors.

## Requirements

### Functional Requirements

1. A bench harness (`scripts/bench/`) invokes
   `kafka-producer-perf-test` against heimq with a documented load
   profile (record size, total records, throughput cap) and asserts
   exit 0 with no error lines.
2. Same for `kafka-consumer-perf-test`.
3. A driver runs the **latest released** OpenMessaging Benchmark Kafka
   driver against heimq for at least one documented workload (e.g., 1KB
   records, N partitions, M producers, K consumers, capped duration)
   and asserts it completes without protocol/client errors. Version
   bumps are tracked as ordinary maintenance.
4. The bench harness is runnable locally and in CI; it is not on the
   default test path but is gated on protocol-touching changes.
5. Idempotent and transactional bench profiles are included (e.g.,
   `kafka-producer-perf-test --producer-props enable.idempotence=true
   transactional.id=...`) and complete without errors.

### Non-Functional Requirements

- **Reliability**: Bench harness pass rate ≥ 99% on the gating workload.
- **Reproducibility**: Each profile is a checked-in script with pinned
  client / tool versions.
- **Performance**: Bench harness wall-clock budget ≤ 30 min on CI
  hardware (this is a budget, not a throughput target — heimq is not
  competing on throughput).

## User Stories

- [US-006 — Run kafka-producer-perf-test against heimq](../user-stories/US-006-kafka-perf-test.md)
- [US-007 — Run OpenMessaging Benchmark against heimq](../user-stories/US-007-openmessaging-benchmark.md)

## Edge Cases and Error Handling

- **Tool requires admin APIs heimq does not implement**: capture the
  required API in a parking-lot item and either implement a minimal
  stub or document the non-support with a workaround flag if the tool
  exposes one.
- **Tool assumes durability**: document as a deliberate divergence
  per PRD non-goal #1.

## Success Metrics

- All listed standard benchmarks complete with zero protocol/client
  errors at their gating profile.
- Each benchmark profile is checked in with its expected exit code and
  acceptable warnings list.

## Constraints and Assumptions

- We are not asserting throughput or latency targets — only conformance
  (the tool runs and exits cleanly).
- Standard Kafka tooling is available in the bench environment.

## Dependencies

- **Other features**: FEAT-001 (wire protocol), FEAT-002 (idempotent /
  transactional bench profiles).
- **External services**: Apache Kafka tooling distribution (for
  `kafka-*-perf-test`); OpenMessaging Benchmark repository.
- **PRD requirements**: P0 #6.

## Out of Scope

- Throughput / latency targets relative to Kafka or Redpanda.
- Benchmarks that target out-of-scope APIs (e.g., share-group
  benchmarks).
