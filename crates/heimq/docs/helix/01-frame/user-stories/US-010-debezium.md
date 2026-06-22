---
ddx:
  id: US-010
  depends_on:
    - FEAT-005
  review:
    self_hash: d4c990816e9d3ffd0ee45dd5da9b0a5a2fb80724c546527945a0eba02f9c7f02
    deps:
      FEAT-005: c8b17ef2e79181581f03413860ea3e074364440af280321171b639668cb3aebc
    reviewed_at: "2026-06-22T21:30:26Z"
---
# US-010 — Debezium emits CDC into heimq

**Feature**: FEAT-005 — Ecosystem Integrations
**Feature Requirements**: DB-01, INT-01
**PRD Requirements**: FR-10
**Priority**: P0
**Status**: Specified

## Story

**As a** Debezium user,
**I want** a Debezium connector to emit CDC events into a heimq topic,
**So that** CDC pipelines work against heimq for tests / local
development.

## Context

Debezium is the PRD's canonical ecosystem acceptance sketch: a configured
connector emits CDC events to a heimq topic and an rdkafka consumer reads
the expected envelope (PRD § Acceptance Test Sketches). Without an
integration test, Debezium users discover incompatibilities only when they
try the real tool (FEAT-005 § Problem Statement). This story exercises
FEAT-005 DB-01 and INT-01, tracing to PRD FR-10 / P0 #7.

## Walkthrough

1. Debezium user invokes the integration script; it brings up a Debezium
   connector (e.g., the PostgreSQL connector or Debezium Server) configured
   against heimq.
2. The script applies a sample database change set.
3. The connector produces CDC envelopes into the configured heimq topic.
4. An rdkafka consumer reads the topic and observes the expected CDC
   envelope.
5. The script tears down and exits 0.

## Acceptance Criteria

- [ ] **US-010-AC1** — Given a Debezium connector (e.g., the PostgreSQL
  connector or Debezium Server) configured against heimq, when a sample
  database change set is applied, then the connector produces CDC
  envelopes.
- [ ] **US-010-AC2** — Given CDC envelopes emitted into the configured
  heimq topic, when an rdkafka consumer reads that topic, then it reads
  the expected CDC envelope.
- [ ] **US-010-AC3** — Given the integration, when its script is invoked,
  then a single script brings up the connector, runs the change set,
  tears down, and exits 0.

## Edge Cases

From FEAT-005 § Edge Cases and Error Handling:

- **Tool requires APIs heimq does not implement**: parking-lot the required
  API; either implement a minimal stub or document the non-support and
  exclude that tool from the matrix with a written rationale.
- **Tool assumes durability**: document as a deliberate divergence per PRD
  non-goal #1.

## Test Scenarios

- **US-010-AC1** → ecosystem: Debezium connector against heimq produces CDC envelopes from a sample change set.
- **US-010-AC2** → ecosystem: rdkafka consumer reads the expected CDC envelope from the heimq topic.
- **US-010-AC3** → ecosystem: single script brings up the connector, runs the change set, tears down, exits 0.

## Dependencies

- **Stories**: US-001 (client connectivity).
- **Feature Spec**: FEAT-005
- **Feature Requirements**: DB-01, INT-01
- **PRD Requirements**: FR-10
- **External**: Debezium connector runtime / Docker image and a source database (e.g., PostgreSQL); FEAT-001 wire protocol, FEAT-002 (Debezium semantics), FEAT-006 flexible-version protocol (per FEAT-005 dependencies).

## Out of Scope

Per FEAT-005 § Out of Scope and PRD Non-Goals:

- Performance comparisons against Kafka/Redpanda for this integration.
- More than one representative Debezium connector (PRD P0 #7 requires one).
- Durability of emitted CDC topics across heimq restart (PRD non-goal #1).
