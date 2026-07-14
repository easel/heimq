---
ddx:
  id: US-006
  depends_on:
    - FEAT-004
  review:
    self_hash: ea037831114a863a64e1b8462648e7258daa055320a3a3f2356281e065e7ad5a
    deps:
      FEAT-004: f7d84558ef81d4501ca858b2fa8c0bb6c3bc6e864b2d49a32920178fcfc1fa8e
    reviewed_at: "2026-07-14T06:48:37Z"
---
# US-006 — Run kafka-producer-perf-test and kafka-consumer-perf-test against heimq

**Feature**: FEAT-004 — Standard Kafka Benchmark Conformance
**Feature Requirements**: FR-01, FR-02, FR-04, FR-05
**PRD Requirements**: FR-9
**Priority**: P0
**Status**: Specified

## Story

**As a** heimq maintainer,
**I want** the standard Apache Kafka `kafka-producer-perf-test` and
`kafka-consumer-perf-test` tools to run against heimq to completion,
**So that** I have continuous evidence the broker handles standard
benchmark traffic patterns without protocol or client errors.

## Context

Standard benchmarks exercise protocol corners that unit tests miss — admin
RPCs, repeated metadata refresh, large-batch produce, idempotent and
transactional flows under load (FEAT-004 § Problem Statement). This is a
conformance check, not a performance target. The story exercises FEAT-004
FR-01, FR-02, FR-04, and FR-05, tracing to PRD FR-9 / P0 #6.

## Walkthrough

1. Maintainer invokes the bench harness under `scripts/bench/` against a
   running heimq broker.
2. `kafka-producer-perf-test` runs with its documented load profile (record
   size, total records, throughput cap) and completes with exit 0 and no
   error lines.
3. `kafka-consumer-perf-test` runs with its documented load profile and
   completes the same way.
4. The idempotent (`enable.idempotence=true`) and transactional
   (`transactional.id`) profiles each run and complete cleanly.
5. Maintainer inspects the harness: each profile, its expected exit code, and
   its acceptable-warnings list are checked in under `scripts/bench/`.

## Acceptance Criteria

- [ ] **US-006-AC1** — Given a documented load profile, when
  `kafka-producer-perf-test` runs against heimq, then it completes with
  exit 0 and no error lines.
- [ ] **US-006-AC2** — Given a documented load profile, when
  `kafka-consumer-perf-test` runs against heimq, then it completes with
  exit 0 and no error lines.
- [ ] **US-006-AC3** — Given an idempotent profile
  (`enable.idempotence=true`) and a transactional profile
  (`transactional.id`), when each runs against heimq, then each completes
  cleanly.
- [ ] **US-006-AC4** — Given the bench harness, when its profiles are
  inspected, then profiles, expected exit codes, and acceptable warnings
  are checked in under `scripts/bench/`.

## Edge Cases

From FEAT-004 § Edge Cases and Error Handling:

- **Tool requires admin APIs heimq does not implement**: capture the required
  API in a parking-lot item and either implement a minimal stub or document
  the non-support with a workaround flag if the tool exposes one.
- **Tool assumes durability**: document as a deliberate divergence per PRD
  non-goal #1.

## Test Scenarios

- **US-006-AC1** → bench: `kafka-producer-perf-test` at the documented profile exits 0 with no error lines.
- **US-006-AC2** → bench: `kafka-consumer-perf-test` at the documented profile exits 0 with no error lines.
- **US-006-AC3** → bench: idempotent and transactional profiles each complete cleanly.
- **US-006-AC4** → bench: profiles, expected exit codes, and acceptable warnings are checked in under `scripts/bench/`.

## Dependencies

- **Stories**: US-001 (client connectivity); US-003 and US-004 (idempotent / transactional profiles in AC3).
- **Feature Spec**: FEAT-004
- **Feature Requirements**: FR-01, FR-02, FR-04, FR-05
- **PRD Requirements**: FR-9
- **External**: Apache Kafka tooling distribution (for `kafka-*-perf-test`); FEAT-001 wire protocol, FEAT-002 (idempotent / transactional bench profiles), FEAT-006 flexible-version protocol (the perf-test tools ship the modern Java client; per FEAT-004 dependencies).

## Out of Scope

Per FEAT-004 § Out of Scope and PRD Non-Goals:

- Throughput / latency targets relative to Kafka or Redpanda — conformance
  only (PRD non-goal #6).
- The OpenMessaging Benchmark driver — US-007.
- Benchmarks that target out-of-scope APIs (e.g., share-group benchmarks).
