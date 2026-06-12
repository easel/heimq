---
ddx:
  id: US-014
  depends_on:
    - FEAT-005
---
# US-014 — ksqlDB runs against heimq

**Feature**: FEAT-005 — Ecosystem Integrations
**Feature Requirements**: KS-01, INT-01
**PRD Requirements**: FR-10
**Priority**: P0
**Status**: Specified

## Story

**As a** developer using ksqlDB against Kafka,
**I want** a ksqlDB instance configured against heimq to execute a
`CREATE STREAM` and a simple `SELECT` query over events produced to a
heimq topic,
**So that** ksqlDB-based stream processing works in heimq-backed test
environments.

## Context

ksqlDB is one of the six required ecosystem integration targets (PRD
FR-10 / P0 #7) and, like the other JVM tools, exercises broker surface that
contract tests cannot anticipate (FEAT-005 § Problem Statement). FEAT-005
KS-01 requires that a ksqlDB instance configured against heimq executes a
`CREATE STREAM` and a simple `SELECT` query, producing expected results.
This story exercises FEAT-005 KS-01 and INT-01.

## Walkthrough

1. Developer invokes the ksqlDB integration script; it brings up ksqlDB
   configured against heimq.
2. The developer executes a `CREATE STREAM` statement over a heimq topic;
   the stream is created.
3. Events are produced to the underlying heimq topic.
4. A simple `SELECT` query runs over the stream and produces the expected
   results.
5. The script tears down and exits 0.

## Acceptance Criteria

- [ ] **US-014-AC1** — Given a ksqlDB instance configured against heimq,
  when a `CREATE STREAM` statement is executed over a heimq topic, then
  the stream is created successfully.
- [ ] **US-014-AC2** — Given events produced to the underlying heimq
  topic, when a simple `SELECT` query runs over the stream, then it
  produces the expected results.
- [ ] **US-014-AC3** — Given the integration, when its script is invoked,
  then a single script brings up ksqlDB, runs the workload, tears down,
  and exits 0.

## Edge Cases

From FEAT-005 § Edge Cases and Error Handling:

- **Tool requires APIs heimq does not implement**: parking-lot the required
  API; either implement a minimal stub or document the non-support and
  exclude that tool from the matrix with a written rationale.
- **Tool assumes durability**: document as a deliberate divergence per PRD
  non-goal #1.
- **Tool depends on Confluent-only APIs**: scope decision recorded in the
  integration's README and the parking lot.

## Test Scenarios

- **US-014-AC1** → ecosystem: `CREATE STREAM` over a heimq topic succeeds.
- **US-014-AC2** → ecosystem: simple `SELECT` over the stream returns the expected results for events produced to the heimq topic.
- **US-014-AC3** → ecosystem: single script brings up ksqlDB, runs the workload, tears down, exits 0.

## Dependencies

- **Stories**: US-001 (client connectivity), US-002 (ksqlDB consumes via consumer groups).
- **Feature Spec**: FEAT-005
- **Feature Requirements**: KS-01, INT-01
- **PRD Requirements**: FR-10
- **External**: ksqlDB runtime / Docker image; JVM-capable test environment; FEAT-001 wire protocol, FEAT-002 core semantics, FEAT-006 flexible-version protocol (ksqlDB negotiates modern flexible-only API versions; per FEAT-005 dependencies).

## Out of Scope

Per FEAT-005 § Out of Scope and PRD Non-Goals:

- Performance comparisons against Kafka/Redpanda for this integration.
- ksqlDB features depending on out-of-scope APIs (e.g., multi-broker
  reassignment).
- Durability of ksqlDB streams/state across heimq restart (PRD non-goal #1).
