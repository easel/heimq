---
ddx:
  id: US-012
  depends_on:
    - FEAT-005
  review:
    self_hash: 7ca108079732515c953e1be73600ba2dc1ac5cbd02c849b216e9147951166fc3
    deps:
      FEAT-005: c8b17ef2e79181581f03413860ea3e074364440af280321171b639668cb3aebc
    reviewed_at: "2026-06-22T21:30:26Z"
---
# US-012 — Multi-language librdkafka clients against heimq

**Feature**: FEAT-005 — Ecosystem Integrations
**Feature Requirements**: LC-01, INT-01
**PRD Requirements**: FR-10
**Priority**: P0
**Status**: Specified

## Story

**As a** developer using a non-Rust Kafka client,
**I want** canonical librdkafka-based clients in multiple languages to run
against heimq,
**So that** polyglot test environments and services can use heimq.

## Context

heimq is tested against rdkafka in Rust only; polyglot teams need the
canonical librdkafka binding in their language to work too (FEAT-005
§ Problem Statement). FEAT-005 LC-01 requires integration tests in at least
three languages — recommended Go (`confluent-kafka-go`), Python
(`confluent-kafka`), and Node.js (`node-rdkafka`). This story exercises
FEAT-005 LC-01 and INT-01, tracing to PRD FR-10 / P0 #7.

## Walkthrough

1. Developer invokes a language's integration script (one per language).
2. The Go test using `confluent-kafka-go` produces to and consumes from
   heimq successfully.
3. The Python test using `confluent-kafka` does the same.
4. The Node.js test using `node-rdkafka` does the same.
5. Each script exits 0.

## Acceptance Criteria

- [ ] **US-012-AC1** — Given an integration test in Go using
  `confluent-kafka-go`, when it runs against heimq, then it produces and
  consumes successfully.
- [ ] **US-012-AC2** — Given an integration test in Python using
  `confluent-kafka`, when it runs against heimq, then it produces and
  consumes successfully.
- [ ] **US-012-AC3** — Given an integration test in Node.js using
  `node-rdkafka`, when it runs against heimq, then it produces and
  consumes successfully.
- [ ] **US-012-AC4** — Given each language's test, when it is invoked via
  its single per-language script, then each exits 0.

## Edge Cases

From FEAT-005 § Edge Cases and Error Handling:

- **Tool requires APIs heimq does not implement**: parking-lot the required
  API; either implement a minimal stub or document the non-support and
  exclude that tool from the matrix with a written rationale.
- **Tool assumes durability**: document as a deliberate divergence per PRD
  non-goal #1.

## Test Scenarios

- **US-012-AC1** → ecosystem: `confluent-kafka-go` test produces and consumes against heimq.
- **US-012-AC2** → ecosystem: `confluent-kafka` (Python) test produces and consumes against heimq.
- **US-012-AC3** → ecosystem: `node-rdkafka` test produces and consumes against heimq.
- **US-012-AC4** → ecosystem: each per-language script exits 0.

## Dependencies

- **Stories**: US-001 (client connectivity).
- **Feature Spec**: FEAT-005
- **Feature Requirements**: LC-01, INT-01
- **PRD Requirements**: FR-10
- **External**: Go, Python, and Node.js runtimes with their canonical librdkafka bindings; FEAT-001 wire protocol, FEAT-002 core semantics, FEAT-006 flexible-version protocol (modern librdkafka default-negotiates flexible versions; per FEAT-005 dependencies).

## Out of Scope

Per FEAT-005 § Out of Scope and PRD Non-Goals:

- Performance comparisons against Kafka/Redpanda for these clients.
- Non-librdkafka client libraries (e.g., pure-language reimplementations) —
  LC-01 targets the canonical librdkafka binding per language.
- Languages beyond the three required by LC-01.
