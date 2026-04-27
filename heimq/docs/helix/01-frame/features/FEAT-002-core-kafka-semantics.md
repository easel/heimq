---
dun:
  id: FEAT-002
  depends_on:
    - helix.prd
    - FEAT-001
    - FEAT-006
---
# Feature Specification: FEAT-002 — Core Kafka Semantics (Groups, Transactions, Idempotency)

**Feature ID**: FEAT-002
**Status**: Specified
**Priority**: P0
**Owner**: heimq core

## Overview

heimq correctly implements the Kafka semantic surface that production
services depend on: consumer groups, idempotent producers, and transactional
producers (EOS). This addresses PRD goal 2 (the semantic surface that
production services rely on works correctly).

## Problem Statement

- **Current situation**: Consumer groups are implemented (per `API-001`).
  Idempotent producers and transactions are explicitly excluded as `R3` in
  the current contract.
- **Pain points**: Services using `enable.idempotence=true` (often the
  rdkafka default) and services using transactional sinks (Flink, Kafka
  Streams, Debezium) cannot run against heimq today.
- **Desired outcome**: An idempotent or transactional Kafka client runs
  unchanged against heimq and observes the same semantics it would against
  Kafka/Redpanda for single-coordinator deployments.

## Requirements

### Functional Requirements

#### Consumer groups

1. JoinGroup / SyncGroup / Heartbeat / LeaveGroup / FindCoordinator behave
   per Kafka spec for a single coordinator.
2. OffsetCommit / OffsetFetch persist (in-memory by default) and return
   committed offsets to subsequent group members.
3. A group of N members reading a partitioned topic sees no record gaps
   and no duplicate ownership across rebalance events (join, leave,
   session-timeout expiry).
4. Members resume from committed offsets after restart.

#### Idempotent producers

5. `InitProducerId` returns a producerId / epoch.
6. The broker tracks `(producerId, epoch, partition)` sequence numbers.
7. Duplicate sequence numbers are de-duplicated or return
   `DUPLICATE_SEQUENCE_NUMBER` per Kafka spec; out-of-order returns
   `OUT_OF_ORDER_SEQUENCE_NUMBER`.
8. A producer with `enable.idempotence=true` running unchanged against
   heimq sees the same observable behavior as against Redpanda for a
   produce-with-retry workload.

#### Transactions (EOS)

9. `InitProducerId` with a `transactional.id` returns a fenced producerId
   and bumps the epoch on re-init.
10. `AddPartitionsToTxn`, `AddOffsetsToTxn`, `EndTxn`, `WriteTxnMarkers`,
    `TxnOffsetCommit` operate on a single-coordinator transaction state
    machine consistent with Kafka.
11. `read_committed` consumers do not observe records from open or aborted
    transactions; `read_uncommitted` consumers do.
12. A transactional producer + read_committed consumer running unchanged
    against heimq sees the same observable behavior as against Redpanda for
    commit and abort flows.

### Non-Functional Requirements

- **Reliability**: All P0 acceptance scenarios in the PRD for groups,
  idempotency, and transactions pass against heimq and against Redpanda.
- **Determinism**: Differential parity reports zero behavioral diffs for
  these flows.
- **Compatibility**: Loss of in-memory state on restart is acceptable; the
  client re-initializes (re-issues `InitProducerId`, rejoins group) and
  recovers per Kafka spec for a fresh broker.

## User Stories

- [US-002 — Consumer group rebalance correctness](../user-stories/US-002-consumer-group-rebalance.md)
- [US-003 — Idempotent producer dedup](../user-stories/US-003-idempotent-producer.md)
- [US-004 — Transactional commit and abort](../user-stories/US-004-transactional-commit-abort.md)

## Edge Cases and Error Handling

- **Producer fenced**: a stale epoch using a transactional id returns
  `INVALID_PRODUCER_EPOCH`.
- **Transaction timeout**: `transaction.timeout.ms` enforced; expired
  transactions are aborted.
- **Sequence wrap**: handled per Kafka spec.
- **Group rebalance during transaction**: consumer commits via
  `TxnOffsetCommit` and observes the standard Kafka semantics.
- **Restart-during-transaction / restart-with-active-producer-id**:
  in-memory state loss is acceptable. heimq presents as a fresh broker;
  clients receive `UNKNOWN_PRODUCER_ID` and re-initialize via
  `InitProducerId` per Kafka spec. This is the same behavior real Kafka
  exhibits when its log is lost (fresh broker / disk wipe), so no
  heimq-specific client handling is needed — modern librdkafka and the
  Java client recover transparently.

## Success Metrics

- All FEAT-002 acceptance test sketches in the PRD pass.
- Differential parity (FEAT-003) reports zero diffs for groups,
  idempotency, and transactions in the gating workload.

## Constraints and Assumptions

- Single transaction coordinator (no replicated transaction log).
- Loss-on-restart is acceptable; fresh-broker semantics are observed by
  clients on reconnect.

## Dependencies

- **Other features**: FEAT-001 (wire protocol), FEAT-006 (flexible-version
  codec — modern transactional APIs are flexible-only:
  InitProducerId v2+, EndTxn v3+, AddPartitionsToTxn v3+,
  TxnOffsetCommit v3+), FEAT-003 (parity tests).
- **PRD requirements**: P0 #2, #3, #4.

## Out of Scope

- Multi-coordinator transaction log replication.
- Persistence of producer-id / transaction state across restart (acceptable
  per PRD non-goal #1).
