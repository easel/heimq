# US-004 — Transactional commit and abort

**Feature**: FEAT-002
**Priority**: P0

## Story

As a developer using Kafka transactions (`transactional.id`,
`beginTransaction` / `commitTransaction` / `abortTransaction`),
I want heimq to honor Kafka transaction semantics for a single
coordinator,
so that a `read_committed` consumer never observes records from open or
aborted transactions.

## Acceptance Criteria

- A transactional producer can call `InitProducerId` with a
  `transactional.id`, get a fenced producerId / epoch, and have stale
  epochs rejected with `INVALID_PRODUCER_EPOCH`.
- `AddPartitionsToTxn`, `AddOffsetsToTxn`, `EndTxn`, `WriteTxnMarkers`,
  `TxnOffsetCommit` operate per Kafka spec for a single coordinator.
- A committed transaction is visible to `read_committed` consumers.
- An aborted transaction is invisible to `read_committed` consumers
  (records are filtered out via control batches / LSO).
- A `read_uncommitted` consumer sees both committed and aborted
  records.
- Identical workload against Redpanda (FEAT-003) shows the same
  visibility profile.
