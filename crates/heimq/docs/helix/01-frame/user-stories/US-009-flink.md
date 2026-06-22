---
ddx:
  id: US-009
  depends_on:
    - FEAT-005
  review:
    self_hash: 3fbf5f7cc7e25c696f69855dbf196df0260d48c851d4e4622ea2e76147c30bf3
    deps:
      FEAT-005: c8b17ef2e79181581f03413860ea3e074364440af280321171b639668cb3aebc
    reviewed_at: "2026-06-22T21:30:26Z"
---
# US-009 — Flink Kafka source/sink against heimq

**Feature**: FEAT-005 — Ecosystem Integrations
**Feature Requirements**: FL-01, INT-01
**PRD Requirements**: FR-10
**Priority**: P0
**Status**: Specified

## Story

**As a** Flink user,
**I want** a Flink job using the Kafka source and Kafka sink connectors to
read from one heimq topic and write to another,
**So that** Flink pipelines (including EOS sinks where applicable) work
against heimq.

## Context

Flink's Kafka source and sink exercise consumer-group session timing and —
for EOS sinks — transactional broker behavior that contract tests cannot
anticipate (FEAT-005 § Problem Statement). A Flink user needs proof that a
job reads from one heimq topic and writes to another, exactly-once where
supported. This story exercises FEAT-005 FL-01 and INT-01, tracing to PRD
FR-10 / P0 #7.

## Walkthrough

1. Flink user invokes the integration script; it brings up Flink pointed at
   heimq.
2. A Flink job using the Kafka source connector reads records from a heimq
   topic and, via the Kafka sink connector, writes results to another heimq
   topic.
3. With an EOS-configured sink (depends on FEAT-002 transactional support), a
   checkpoint cycle runs and completes without errors.
4. The script runs the job to a checkpoint, tears down, and exits 0.

## Acceptance Criteria

- [ ] **US-009-AC1** — Given a Flink job using the Kafka source and Kafka
  sink connectors, when it runs against heimq, then it reads from a heimq
  topic and writes to another heimq topic.
- [ ] **US-009-AC2** — Given an EOS-configured Flink sink (depends on
  FEAT-002 transactional support), when a checkpoint cycle runs, then it
  completes without errors.
- [ ] **US-009-AC3** — Given the integration, when its script is invoked,
  then a single script brings up Flink, runs the job to a checkpoint,
  tears down, and exits 0.

## Edge Cases

From FEAT-005 § Edge Cases and Error Handling:

- **Tool requires APIs heimq does not implement**: parking-lot the required
  API; either implement a minimal stub or document the non-support and
  exclude that tool from the matrix with a written rationale.
- **Tool assumes durability**: document as a deliberate divergence per PRD
  non-goal #1.

## Test Scenarios

- **US-009-AC1** → ecosystem: Flink job reads from one heimq topic and writes to another via the Kafka source/sink connectors.
- **US-009-AC2** → ecosystem: EOS-configured sink completes a checkpoint cycle without errors.
- **US-009-AC3** → ecosystem: single script brings up Flink, runs the job to a checkpoint, tears down, exits 0.

## Dependencies

- **Stories**: US-001 (client connectivity), US-004 (transactional support for the EOS sink in AC2).
- **Feature Spec**: FEAT-005
- **Feature Requirements**: FL-01, INT-01
- **PRD Requirements**: FR-10
- **External**: Apache Flink runtime / Docker image; JVM-capable test environment; FEAT-001 wire protocol, FEAT-002 (transactions for Flink EOS), FEAT-006 flexible-version protocol (per FEAT-005 dependencies).

## Out of Scope

Per FEAT-005 § Out of Scope and PRD Non-Goals:

- Performance comparisons against Kafka/Redpanda for this integration.
- Flink deployments depending on out-of-scope APIs (e.g., multi-broker
  reassignment).
- Durability of checkpointed state across heimq restart (PRD non-goal #1).
