---
ddx:
  id: US-003
  depends_on:
    - FEAT-002
  review:
    self_hash: 07f27eaae7593ae43b1298811d182edf0b53e89c0b629712104ecbfc5aecdaa3
    deps:
      FEAT-002: 164350929c7bbc09a589f3cd1a80b685e88cce1054445fe5373aec566464636f
    reviewed_at: "2026-06-22T21:30:26Z"
---
# US-003 — Idempotent producer dedup

**Feature**: FEAT-002 — Core Kafka Semantics (Groups, Transactions, Idempotency)
**Feature Requirements**: IP-01, IP-02, IP-03, IP-04
**PRD Requirements**: FR-6
**Priority**: P0
**Status**: Specified

## Story

**As a** developer using `enable.idempotence=true` on my Kafka producer,
**I want** heimq to dedup retried batches via producer-id + sequence number,
**So that** retries under network blips do not produce duplicate records.

## Context

`enable.idempotence=true` is often the rdkafka default, so services using it
cannot run against heimq until producer-id issuance and sequence tracking
work (FEAT-002 § Problem Statement). This story exercises FEAT-002 IP-01
through IP-04, tracing to PRD FR-6 / P0 #3: duplicates from retries are
collapsed and sequence gaps return the standard error codes.

## Walkthrough

1. Developer runs a producer with `enable.idempotence=true` against heimq;
   the client calls `InitProducerId` and receives a producerId / epoch.
2. The producer sends batches; heimq tracks `(producerId, epoch, partition)`
   sequence numbers per Kafka spec.
3. A network blip causes the producer to retry an already-accepted batch with
   the same sequence; heimq collapses the retry (or returns
   `DUPLICATE_SEQUENCE_NUMBER` per Kafka semantics) — consumers see no
   duplicate record.
4. A batch arrives with an out-of-order sequence; heimq returns
   `OUT_OF_ORDER_SEQUENCE_NUMBER`.
5. The same produce-with-retry workload runs against Redpanda via FEAT-003
   and shows the same delivery profile.

## Acceptance Criteria

- [ ] **US-003-AC1** — Given an idempotent producer, when it calls
  `InitProducerId`, then the broker returns a producerId / epoch.
- [ ] **US-003-AC2** — Given an idempotent producer session, when batches
  are produced, then the broker tracks `(producerId, epoch, partition)`
  sequence numbers per Kafka spec.
- [ ] **US-003-AC3** — Given a batch already accepted by the broker, when
  the producer retries it with the same sequence, then the retry is
  collapsed (no duplicate record visible to consumers) or returns
  `DUPLICATE_SEQUENCE_NUMBER` per Kafka semantics.
- [ ] **US-003-AC4** — Given an idempotent producer session, when a batch
  arrives with an out-of-order sequence, then the broker returns
  `OUT_OF_ORDER_SEQUENCE_NUMBER`.
- [ ] **US-003-AC5** — Given an identical workload run against Redpanda
  (FEAT-003), when delivery profiles are compared, then heimq shows the
  same delivery profile.

## Edge Cases

From FEAT-002 § Edge Cases and Error Handling:

- **Sequence wrap**: handled per Kafka spec.
- **Restart with active producer id**: in-memory state loss is acceptable;
  heimq presents as a fresh broker, clients receive `UNKNOWN_PRODUCER_ID`
  and re-initialize via `InitProducerId` per Kafka spec (see ADR-006).

## Test Scenarios

- **US-003-AC1** → contract: `InitProducerId` returns a producerId / epoch.
- **US-003-AC2** → property: `(producerId, epoch, partition)` sequence tracking invariants across generated batch sequences.
- **US-003-AC3** → integration: retry of an accepted batch is collapsed or returns `DUPLICATE_SEQUENCE_NUMBER`; no duplicate visible to consumers.
- **US-003-AC4** → contract: out-of-order sequence returns `OUT_OF_ORDER_SEQUENCE_NUMBER`.
- **US-003-AC5** → parity: produce-with-retry workload diffed against Redpanda (FEAT-003 harness).

## Dependencies

- **Stories**: US-001 (standard client connectivity).
- **Feature Spec**: FEAT-002
- **Feature Requirements**: IP-01, IP-02, IP-03, IP-04
- **PRD Requirements**: FR-6
- **External**: FEAT-001 wire protocol and FEAT-006 flexible-version codec (modern `InitProducerId` v2+ is flexible-only; per FEAT-002 dependencies); Redpanda container for the AC5 parity check (FEAT-003).

## Out of Scope

Per FEAT-002 § Out of Scope and PRD Non-Goals:

- Persistence of producer-id state across restart (PRD non-goal #1; ADR-006).
- Transactional semantics (`transactional.id`, EOS) — US-004.
