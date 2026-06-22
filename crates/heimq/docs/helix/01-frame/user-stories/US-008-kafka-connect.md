---
ddx:
  id: US-008
  depends_on:
    - FEAT-005
  review:
    self_hash: fd681a21fd3e5c0d0383d82fec67c1571f3efc2c572df672ddc8c95b86d69061
    deps:
      FEAT-005: c8b17ef2e79181581f03413860ea3e074364440af280321171b639668cb3aebc
    reviewed_at: "2026-06-22T21:30:26Z"
---
# US-008 — Kafka Connect runs against heimq

**Feature**: FEAT-005 — Ecosystem Integrations
**Feature Requirements**: KC-01, INT-01
**PRD Requirements**: FR-10
**Priority**: P0
**Status**: Specified

## Story

**As a** developer using Kafka Connect,
**I want** a representative source connector and a representative sink
connector to run against heimq,
**So that** Connect-based pipelines work in heimq-backed test environments.

## Context

Kafka Connect exercises broker surface that wire-protocol contract tests
cannot anticipate — admin metadata refresh patterns, consumer-group session
timing, header conventions (FEAT-005 § Problem Statement).
A developer using Connect needs proof that a representative source and sink
task complete end-to-end against heimq. This story exercises FEAT-005 KC-01
and INT-01, tracing to PRD FR-10 / P0 #7.

## Walkthrough

1. Developer invokes the Kafka Connect integration script; it brings up
   Connect pointed at heimq.
2. A representative source connector (e.g., `FileStreamSource`) runs and
   writes records to a heimq topic.
3. A representative sink connector (e.g., `FileStreamSink`) runs and consumes
   records from a heimq topic.
4. The script tears everything down and exits 0 with no error lines.

## Acceptance Criteria


- [ ] **US-008-AC1** — Given a representative source connector (e.g.,
  `FileStreamSource`), when it runs against heimq, then it writes records
  to a topic.
- [ ] **US-008-AC2** — Given a representative sink connector (e.g.,
  `FileStreamSink`), when it runs against heimq, then it consumes records
  from a topic.
- [ ] **US-008-AC3** — Given each integration, when its test is invoked,
  then a single script brings up Connect, runs the test, and tears down.
- [ ] **US-008-AC4** — Given the integration scripts, when the tests run,
  then they exit 0 with no error lines.

## Edge Cases

From FEAT-005 § Edge Cases and Error Handling:

- **Tool requires APIs heimq does not implement**: parking-lot the required
  API; either implement a minimal stub or document the non-support and
  exclude that tool from the matrix with a written rationale.
- **Tool assumes durability**: document as a deliberate divergence per PRD
  non-goal #1.

## Test Scenarios

- **US-008-AC1** → ecosystem: representative source connector writes records to a heimq topic.
- **US-008-AC2** → ecosystem: representative sink connector consumes records from a heimq topic.
- **US-008-AC3** → ecosystem: single script brings up Connect, runs the test, tears down.
- **US-008-AC4** → ecosystem: integration scripts exit 0 with no error lines.

## Dependencies

- **Stories**: US-001 (client connectivity), US-002 (Connect uses consumer groups).
- **Feature Spec**: FEAT-005
- **Feature Requirements**: KC-01, INT-01
- **PRD Requirements**: FR-10
- **External**: Kafka Connect runtime / Docker image; JVM-capable test environment; FEAT-001 wire protocol, FEAT-002 core semantics, FEAT-006 flexible-version protocol (ecosystem tools negotiate modern flexible-only API versions; per FEAT-005 dependencies).

## Out of Scope

Per FEAT-005 § Out of Scope and PRD Non-Goals:

- Performance comparisons against Kafka/Redpanda for this integration.
- Connectors that depend on out-of-scope APIs (e.g., share-group consumers,
  multi-broker reassignment).
- Durability of Connect's internal topics across heimq restart (PRD
  non-goal #1).
