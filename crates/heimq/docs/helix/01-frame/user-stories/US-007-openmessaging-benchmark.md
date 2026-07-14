---
ddx:
  id: US-007
  depends_on:
    - FEAT-004
  review:
    self_hash: d5b7edf3b50f6b1189c2f6178df7b842b9371c5a46f8dda9de91d96aba28bb37
    deps:
      FEAT-004: f7d84558ef81d4501ca858b2fa8c0bb6c3bc6e864b2d49a32920178fcfc1fa8e
    reviewed_at: "2026-07-14T06:48:37Z"
---
# US-007 — Run OpenMessaging Benchmark against heimq

**Feature**: FEAT-004 — Standard Kafka Benchmark Conformance
**Feature Requirements**: FR-03, FR-04
**PRD Requirements**: FR-9
**Priority**: P0
**Status**: Specified

## Story

**As a** heimq maintainer,
**I want** the OpenMessaging Benchmark Kafka driver to run against heimq
to completion on a documented workload,
**So that** heimq is exercised by an industry-standard, multi-tool
benchmark beyond Apache Kafka's own perf-test scripts.

## Context

The OpenMessaging Benchmark Kafka driver exercises multi-producer /
multi-consumer load patterns beyond Apache Kafka's own perf-test scripts;
without it, divergences only surface in production tools (FEAT-004
§ Problem Statement). The target is the latest released driver, pinned, with
version bumps as ordinary maintenance (ADR-004). This story exercises
FEAT-004 FR-03 and FR-04, tracing to PRD FR-9 / P0 #6.

## Walkthrough

1. Maintainer invokes the OpenMessaging Benchmark harness against a running
   heimq broker, locally (with Docker) or in CI.
2. The pinned, latest-released OMB Kafka driver runs the documented workload
   (e.g., 1KB records, N partitions, M producers, K consumers, capped
   duration).
3. The benchmark runs to completion with no protocol or client errors.
4. Maintainer inspects the harness: the workload definition (YAML) is checked
   in under `scripts/bench/openmessaging/`.

## Acceptance Criteria

- [ ] **US-007-AC1** — Given at least one documented workload (e.g., 1KB
  records, N partitions, M producers, K consumers, capped duration), when
  the OpenMessaging Benchmark Kafka driver runs against heimq, then it
  completes with no protocol or client errors.
- [ ] **US-007-AC2** — Given the benchmark harness, when its workload
  definition is inspected, then the workload definition (YAML) is checked
  in under `scripts/bench/openmessaging/`.
- [ ] **US-007-AC3** — Given a local machine with Docker or a CI
  environment, when the benchmark is invoked, then it runs in both.

## Edge Cases

From FEAT-004 § Edge Cases and Error Handling:

- **Tool requires admin APIs heimq does not implement**: capture the required
  API in a parking-lot item and either implement a minimal stub or document
  the non-support with a workaround flag if the tool exposes one.
- **Tool assumes durability**: document as a deliberate divergence per PRD
  non-goal #1.

## Test Scenarios

- **US-007-AC1** → bench: OMB Kafka driver runs the documented workload to completion with no protocol/client errors.
- **US-007-AC2** → bench: workload definition (YAML) is checked in under `scripts/bench/openmessaging/`.
- **US-007-AC3** → bench: harness invokable locally (Docker) and in CI.

## Dependencies

- **Stories**: US-001 (client connectivity); US-006 (shared bench harness under `scripts/bench/`).
- **Feature Spec**: FEAT-004
- **Feature Requirements**: FR-03, FR-04
- **PRD Requirements**: FR-9
- **External**: OpenMessaging Benchmark repository (latest released driver, pinned per ADR-004); Docker; FEAT-001 wire protocol, FEAT-002, FEAT-006 flexible-version protocol (OMB ships the modern Java client; per FEAT-004 dependencies).

## Out of Scope

Per FEAT-004 § Out of Scope and PRD Non-Goals:

- Throughput / latency targets relative to Kafka or Redpanda — conformance
  only (PRD non-goal #6).
- `kafka-producer-perf-test` / `kafka-consumer-perf-test` — US-006.
- Benchmarks that target out-of-scope APIs (e.g., share-group benchmarks).
