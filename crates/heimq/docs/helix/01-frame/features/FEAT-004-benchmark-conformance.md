---
ddx:
  id: FEAT-004
  depends_on:
    - helix.prd
    - FEAT-001
    - FEAT-002
    - FEAT-006
  review:
    self_hash: f7d84558ef81d4501ca858b2fa8c0bb6c3bc6e864b2d49a32920178fcfc1fa8e
    deps:
      FEAT-001: 7133c264bc364ec4535c1d6b6187a90c9ba66d1fa3df30731ade260c2e092479
      FEAT-002: 164350929c7bbc09a589f3cd1a80b685e88cce1054445fe5373aec566464636f
      FEAT-006: e59d3b8965ebd35b4bbe9c5302f4218432ad83ec27691989dfd4c345ac2ae004
      helix.prd: debc0a32007f0c42db51e82f47848c7b988c3f67f6f97171069170492f9b5b95
    reviewed_at: "2026-07-14T06:48:37Z"
---
# Feature Specification: FEAT-004 — Standard Kafka Benchmark Conformance

**Feature ID**: FEAT-004
**Status**: Specified
**Priority**: P0
**Owner**: heimq core
**Covered PRD Subsystem(s)**: Benchmark conformance (FEAT-004)
**Covered PRD Requirements**: FR-9 (standard Kafka benchmark conformance) — PRD P0 #6
**Cross-Subsystem Rationale**: None — single subsystem.

## Overview

heimq runs the standard, commonly-used Kafka benchmarks to completion
without protocol or client errors. This is a conformance test, not a
performance target — it verifies that the API surface and client
state machines exercised by these tools are correct.

## Ideal Future State

`kafka-producer-perf-test`, `kafka-consumer-perf-test`, and the
OpenMessaging Benchmark Kafka driver all run to completion against heimq
at documented, checked-in load profiles with zero protocol/client errors,
exercising the protocol corners (admin RPCs, metadata refresh,
large-batch produce, idempotent/transactional flows under load) that unit
tests miss.

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

- **FR-01** — A bench harness (`scripts/bench/`) invokes
  `kafka-producer-perf-test` against heimq with a documented load
  profile (record size, total records, throughput cap) and asserts
  exit 0 with no error lines.
- **FR-02** — Same for `kafka-consumer-perf-test`.
- **FR-03** — A driver runs the **latest released** OpenMessaging Benchmark Kafka
  driver against heimq for at least one documented workload (e.g., 1KB
  records, N partitions, M producers, K consumers, capped duration)
  and asserts it completes without protocol/client errors. Version
  pin: upstream tag `jms` at commit
  `c0e51b8b86a3b0ff50b935152d6e600602a7f0a0`; bumps are tracked as
  ordinary maintenance.
- **FR-04** — The bench harness is runnable locally and in CI; it is not on the
  default test path but is gated on protocol-touching changes.
- **FR-05** — Idempotent and transactional bench profiles are included (e.g.,
  `kafka-producer-perf-test --producer-props enable.idempotence=true
  transactional.id=...`) and complete without errors.

### Non-Functional Requirements

- **Reliability**: The executable reliability rule is a zero-failure
  invariant over the current GitHub Actions workflow run. The FEAT-004
  evidence window is the `bench-smoke` workflow's `feat-004-bench-smoke`
  target plus the `bench-omb` workflow's `feat-004-bench-omb` target:
  one required attempt per target, zero allowed failures, emitted and
  enforced by `scripts/ci/reliability-gate.sh`. Historical pass-rate
  percentages are not claimed until a separate rolling measurement
  store exists.
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
- Current-workflow reliability evidence for FEAT-004 shows
  `reliability_rule=zero_failures`, `required_attempts=1`,
  `allowed_failures=0`, and passing `feat-004-bench-smoke` and
  `feat-004-bench-omb` targets.

## Constraints and Assumptions

- We are not asserting throughput or latency targets — only conformance
  (the tool runs and exits cleanly).
- Standard Kafka tooling is available in the bench environment.

## Dependencies

- **Other features**: FEAT-001 (wire protocol), FEAT-002 (idempotent /
  transactional bench profiles), FEAT-006 (flexible-version protocol —
  `kafka-producer-perf-test` / `kafka-consumer-perf-test` / OMB ship the
  modern Java client, which default-negotiates flexible versions; see
  PRD P0 #1).
- **External services**: Apache Kafka tooling distribution (for
  `kafka-*-perf-test`); OpenMessaging Benchmark repository.
- **PRD requirements**: P0 #6.

## Out of Scope

- Throughput / latency targets relative to Kafka or Redpanda.
- Benchmarks that target out-of-scope APIs (e.g., share-group
  benchmarks).
