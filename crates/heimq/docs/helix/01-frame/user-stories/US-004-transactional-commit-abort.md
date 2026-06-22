---
ddx:
  id: US-004
  depends_on:
    - FEAT-002
  review:
    self_hash: 347fdc2f017d655b6dd9e06ea4657baaba492f3269a0894e23f3cb8883bfde16
    deps:
      FEAT-002: 164350929c7bbc09a589f3cd1a80b685e88cce1054445fe5373aec566464636f
    reviewed_at: "2026-06-22T21:30:26Z"
---
# US-004 — Transactional commit and abort

**Feature**: FEAT-002 — Core Kafka Semantics (Groups, Transactions, Idempotency)
**Feature Requirements**: TX-01, TX-02, TX-03, TX-04
**PRD Requirements**: FR-7
**Priority**: P0
**Status**: Specified

## Story

**As a** developer using Kafka transactions (`transactional.id`,
`beginTransaction` / `commitTransaction` / `abortTransaction`),
**I want** heimq to honor Kafka transaction semantics for a single
coordinator,
**So that** a `read_committed` consumer never observes records from open or
aborted transactions.

## Context

Services using transactional sinks (Flink, Kafka Streams, Debezium) cannot
run against heimq while transactions are excluded from the contract
(FEAT-002 § Problem Statement). This story exercises FEAT-002 TX-01 through
TX-04, tracing to PRD FR-7 / P0 #4: a single-coordinator transaction state
machine where commit and abort flows behave per Kafka spec and
`read_committed` consumers observe only committed records.

## Walkthrough

1. Developer configures a producer with a `transactional.id`; the client
   calls `InitProducerId` and receives a fenced producerId / epoch (stale
   epochs are rejected with `INVALID_PRODUCER_EPOCH`).
2. The producer begins a transaction and drives `AddPartitionsToTxn`,
   `AddOffsetsToTxn`, `EndTxn`, `WriteTxnMarkers`, and `TxnOffsetCommit`;
   each operates per Kafka spec on the single-coordinator state machine.
3. The producer commits one transaction; a `read_committed` consumer fetches
   and sees the committed records.
4. The producer aborts another transaction; the `read_committed` consumer
   does not see those records (filtered via control batches / LSO), while a
   `read_uncommitted` consumer sees both committed and aborted records.
5. The same commit/abort workload runs against Redpanda via FEAT-003 and
   shows the same visibility profile.

## Acceptance Criteria

- [ ] **US-004-AC1** — Given a transactional producer with a
  `transactional.id`, when it calls `InitProducerId`, then it receives a
  fenced producerId / epoch.
- [ ] **US-004-AC2** — Given a single-coordinator transaction, when the
  producer drives `AddPartitionsToTxn`, then it operates per Kafka spec.
- [ ] **US-004-AC3** — Given a transaction that was committed, when a
  `read_committed` consumer fetches, then the committed records are
  visible.
- [ ] **US-004-AC4** — Given a transaction that was aborted, when a
  `read_committed` consumer fetches, then the aborted records are
  invisible (records are filtered out via control batches / LSO).
- [ ] **US-004-AC5** — Given the same topic with committed and aborted
  transactions, when a `read_uncommitted` consumer fetches, then it sees
  both committed and aborted records.
- [ ] **US-004-AC6** — Given an identical workload against Redpanda
  (FEAT-003), when visibility profiles are compared, then heimq shows the
  same visibility profile.
- [ ] **US-004-AC7** — Given a transactional producer with a
  `transactional.id` that has been fenced, when a request carries a stale
  epoch, then it is rejected with `INVALID_PRODUCER_EPOCH`.
- [ ] **US-004-AC8** — Given a single-coordinator transaction, when the
  producer drives `AddOffsetsToTxn`, then it operates per Kafka spec.
- [ ] **US-004-AC9** — Given a single-coordinator transaction, when the
  producer drives `EndTxn`, then it operates per Kafka spec.
- [ ] **US-004-AC10** — Given a single-coordinator transaction, when the
  producer drives `WriteTxnMarkers`, then it operates per Kafka spec.
- [ ] **US-004-AC11** — Given a single-coordinator transaction, when the
  producer drives `TxnOffsetCommit`, then it operates per Kafka spec.

## Edge Cases

From FEAT-002 § Edge Cases and Error Handling:

- **Producer fenced**: a stale epoch using a transactional id returns
  `INVALID_PRODUCER_EPOCH`.
- **Transaction timeout**: `transaction.timeout.ms` enforced; expired
  transactions are aborted.
- **Group rebalance during transaction**: consumer commits via
  `TxnOffsetCommit` and observes the standard Kafka semantics.
- **Restart during transaction**: in-memory state loss is acceptable; heimq
  presents as a fresh broker, clients receive `UNKNOWN_PRODUCER_ID` and
  re-initialize via `InitProducerId` per Kafka spec (see ADR-006).

## Test Scenarios

- **US-004-AC1** → contract: `InitProducerId` with `transactional.id` returns fenced producerId / epoch.
- **US-004-AC2** → contract: `AddPartitionsToTxn` per-API behavior on the single-coordinator state machine.
- **US-004-AC3** → integration: committed transaction visible to a `read_committed` consumer.
- **US-004-AC4** → integration: aborted transaction invisible to a `read_committed` consumer (control batches / LSO).
- **US-004-AC5** → integration: `read_uncommitted` consumer sees committed and aborted records.
- **US-004-AC6** → parity: commit/abort workload visibility diffed against Redpanda (FEAT-003 harness).
- **US-004-AC7** → contract: stale epoch rejected with `INVALID_PRODUCER_EPOCH`.
- **US-004-AC8** → contract: `AddOffsetsToTxn` per-API behavior on the single-coordinator state machine.
- **US-004-AC9** → contract: `EndTxn` per-API behavior on the single-coordinator state machine.
- **US-004-AC10** → contract: `WriteTxnMarkers` per-API behavior on the single-coordinator state machine.
- **US-004-AC11** → contract: `TxnOffsetCommit` per-API behavior on the single-coordinator state machine.

## Dependencies

- **Stories**: US-001 (standard client connectivity); US-003 (idempotent producer-id / sequence machinery underpins transactions).
- **Feature Spec**: FEAT-002
- **Feature Requirements**: TX-01, TX-02, TX-03, TX-04
- **PRD Requirements**: FR-7
- **External**: FEAT-001 wire protocol and FEAT-006 flexible-version codec (modern transactional APIs are flexible-only: InitProducerId v2+, EndTxn v3+, AddPartitionsToTxn v3+, TxnOffsetCommit v3+; per FEAT-002 dependencies); Redpanda container for the AC6 parity check (FEAT-003).

## Out of Scope

Per FEAT-002 § Out of Scope and PRD Non-Goals:

- Multi-coordinator transaction log replication.
- Persistence of transaction / producer-id state across restart (PRD
  non-goal #1; ADR-006).
