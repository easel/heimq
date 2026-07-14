---
ddx:
  id: FEAT-002
  depends_on:
    - helix.prd
    - FEAT-001
    - FEAT-006
  review:
    self_hash: 164350929c7bbc09a589f3cd1a80b685e88cce1054445fe5373aec566464636f
    deps:
      FEAT-001: 7133c264bc364ec4535c1d6b6187a90c9ba66d1fa3df30731ade260c2e092479
      FEAT-006: e59d3b8965ebd35b4bbe9c5302f4218432ad83ec27691989dfd4c345ac2ae004
      helix.prd: 96f0479e307f2c240e8f47b69fff510164d0b9eda132abb22cc4a860932984fe
    reviewed_at: "2026-07-14T05:12:26Z"
---
# Feature Specification: FEAT-002 — Core Kafka Semantics (Groups, Transactions, Idempotency)

**Feature ID**: FEAT-002
**Status**: Specified
**Priority**: P0
**Owner**: heimq core
**Covered PRD Subsystem(s)**: Core semantics (FEAT-002)
**Covered PRD Requirements**: FR-5 (consumer groups), FR-6 (idempotent producers), FR-7 (transactional groups / EOS) — PRD P0 #2, #3, #4
**Cross-Subsystem Rationale**: None — single subsystem.

## Overview

heimq correctly implements the Kafka semantic surface that production
services depend on: consumer groups, idempotent producers, and transactional
producers (EOS). This addresses PRD goal 2 (the semantic surface that
production services rely on works correctly).

## Ideal Future State

An idempotent or transactional Kafka client, and consumer groups of any
membership, run unchanged against heimq and observe the same semantics
they would against Kafka/Redpanda for single-coordinator deployments: a
group of N members sees no gaps and no duplicate ownership across
rebalances and resumes from committed offsets; retried produces are
de-duplicated; `read_committed` consumers observe only committed records.

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

## Functional Areas

| Area | User question or job | Feature responsibility |
|------|----------------------|------------------------|
| Consumer groups | Read a partitioned topic as a group with no gaps or duplicate ownership; resume from committed offsets | Group lifecycle (join / sync / heartbeat / leave / coordinator) and offset commit/fetch per Kafka spec for a single coordinator |
| Idempotent producers | Produce with retries without introducing duplicates | producerId/epoch issuance and `(producerId, epoch, partition)` sequence tracking with standard duplicate / out-of-order error semantics |
| Transactions (EOS) | Commit or abort writes atomically; consume only committed records under `read_committed` | Single-coordinator transaction state machine, transaction markers, transactional offset commits, isolation-level visibility |

## Requirements

### Functional Requirements by Area

#### Consumer groups

- **CG-01** — JoinGroup / SyncGroup / Heartbeat / LeaveGroup / FindCoordinator behave
  per Kafka spec for a single coordinator.
- **CG-02** — OffsetCommit / OffsetFetch persist (in-memory by default) and return
  committed offsets to subsequent group members.
- **CG-03** — A group of N members reading a partitioned topic sees no record gaps
  and no duplicate ownership across rebalance events (join, leave,
  session-timeout expiry).
- **CG-04** — Members resume from committed offsets after restart.

#### Idempotent producers

- **IP-01** — `InitProducerId` returns a producerId / epoch.
- **IP-02** — The broker tracks `(producerId, epoch, partition)` sequence numbers.
- **IP-03** — Duplicate sequence numbers are de-duplicated or return
  `DUPLICATE_SEQUENCE_NUMBER` per Kafka spec; out-of-order returns
  `OUT_OF_ORDER_SEQUENCE_NUMBER`.
- **IP-04** — A producer with `enable.idempotence=true` running unchanged against
  heimq sees the same observable behavior as against Redpanda for a
  produce-with-retry workload.

#### Transactions (EOS)

- **TX-01** — `InitProducerId` with a `transactional.id` returns a fenced producerId
  and bumps the epoch on re-init.
- **TX-02** — `AddPartitionsToTxn`, `AddOffsetsToTxn`, `EndTxn`, `WriteTxnMarkers`,
  `TxnOffsetCommit` operate on a single-coordinator transaction state
  machine consistent with Kafka.
- **TX-03** — `read_committed` consumers do not observe records from open or aborted
  transactions; `read_uncommitted` consumers do.
- **TX-04** — A transactional producer + read_committed consumer running unchanged
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
