---
ddx:
  id: US-002
  depends_on:
    - FEAT-002
  review:
    self_hash: bd708ab171bb664271aa9da8cd6d24c41fe241191b1a98607544e77c75d8967a
    deps:
      FEAT-002: 164350929c7bbc09a589f3cd1a80b685e88cce1054445fe5373aec566464636f
    reviewed_at: "2026-07-14T05:12:26Z"
---
# US-002 — Consumer group rebalance correctness

**Feature**: FEAT-002 — Core Kafka Semantics (Groups, Transactions, Idempotency)
**Feature Requirements**: CG-01, CG-02, CG-03, CG-04
**PRD Requirements**: FR-5
**Priority**: P0
**Status**: Specified

## Story

**As a** service owner running a consumer-group-based application,
**I want** heimq to handle group join, leave, and session-timeout rebalances
correctly,
**So that** my service observes no record gaps or duplicate ownership during
membership changes.

## Context

Consumer groups are part of the semantic surface production services depend
on (PRD goal 2). A service owner running a group-based application needs
group lifecycle (join / sync / heartbeat / leave / coordinator) and offset
commit/fetch to match Kafka semantics for a single coordinator, or tests
passing against heimq say nothing about production. This story exercises
FEAT-002 CG-01 through CG-04, tracing to PRD FR-5 / P0 #2.

## Walkthrough

1. Service owner starts a group of N members reading a partitioned heimq
   topic; records are produced and every record is delivered exactly once
   across the group.
2. A member leaves (gracefully or via session timeout); heimq rebalances and
   the remaining members collectively own all partitions.
3. A new member joins; partitions are re-divided per the configured partition
   assignor without dropping records.
4. A member that committed offsets restarts; it resumes from the committed
   offset.
5. The same workload runs against Redpanda via the FEAT-003 harness and shows
   the same delivery profile.

## Acceptance Criteria

- [ ] **US-002-AC1** — Given a group of N members reading a partitioned
  topic, when records are produced, then every produced record is
  delivered exactly once across the group.
- [ ] **US-002-AC2** — Given an active group, when a member leaves
  (gracefully or via session timeout), then the remaining members
  collectively own all partitions.
- [ ] **US-002-AC3** — Given an active group, when a new member joins,
  then partitions are re-divided per the configured partition assignor
  without dropping records.
- [ ] **US-002-AC4** — Given a member that committed offsets, when that
  member restarts, then the resumed member starts from the committed
  offset.
- [ ] **US-002-AC5** — Given an identical workload run against Redpanda
  (FEAT-003), when delivery profiles are compared, then heimq shows the
  same delivery profile.

## Edge Cases

From FEAT-002:

- **Session-timeout expiry**: counts as a rebalance event; the group still
  sees no record gaps and no duplicate ownership (CG-03).
- **Broker restart loses in-memory group/offset state**: acceptable per PRD
  non-goal #1; clients rejoin the group and recover per Kafka spec for a
  fresh broker (FEAT-002 § Non-Functional Requirements, Compatibility).

## Test Scenarios

- **US-002-AC1** → integration: group of N members on a partitioned topic; exactly-once delivery across the group.
- **US-002-AC2** → integration: member leave (graceful and session-timeout); remaining members own all partitions.
- **US-002-AC3** → integration: member join; assignor re-divides partitions without dropped records.
- **US-002-AC4** → integration: restart of a member that committed offsets; resume from committed offset.
- **US-002-AC5** → parity: identical group workload diffed against Redpanda (FEAT-003 harness).

## Dependencies

- **Stories**: US-001 (standard client connectivity).
- **Feature Spec**: FEAT-002
- **Feature Requirements**: CG-01, CG-02, CG-03, CG-04
- **PRD Requirements**: FR-5
- **External**: FEAT-001 wire protocol and FEAT-006 flexible-version codec (per FEAT-002 dependencies); Redpanda container for the AC5 parity check (FEAT-003).

## Out of Scope

Per FEAT-002 § Out of Scope and PRD Non-Goals:

- Offset persistence across broker restart (in-memory by default; durable
  backends are an opt-in P1 slice, not part of this story).
- Idempotent producers and transactions (US-003, US-004).
- Multi-broker / replicated coordinators — single coordinator only.
