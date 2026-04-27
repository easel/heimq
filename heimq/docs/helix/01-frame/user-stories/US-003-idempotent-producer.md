# US-003 — Idempotent producer dedup

**Feature**: FEAT-002
**Priority**: P0

## Story

As a developer using `enable.idempotence=true` on my Kafka producer,
I want heimq to dedup retried batches via producer-id + sequence number,
so that retries under network blips do not produce duplicate records.

## Acceptance Criteria

- `InitProducerId` returns a producerId / epoch.
- The broker tracks `(producerId, epoch, partition)` sequence numbers
  per Kafka spec.
- A retried batch with the same sequence is collapsed (no duplicate
  record visible to consumers) or returns
  `DUPLICATE_SEQUENCE_NUMBER` per Kafka semantics.
- Out-of-order sequence returns `OUT_OF_ORDER_SEQUENCE_NUMBER`.
- Identical workload run against Redpanda (FEAT-003) shows the same
  delivery profile.
