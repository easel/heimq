# US-001 — Standard Kafka client connects without code changes

**Feature**: FEAT-001
**Priority**: P0

## Story

As a developer with an existing Kafka producer or consumer,
I want to point my client at heimq instead of Kafka/Redpanda by changing
only the bootstrap address,
so that I can run my service against heimq without forking my client code
or test setup.

## Acceptance Criteria

- An rdkafka producer configured only with `bootstrap.servers=<heimq>`
  successfully produces records to a heimq topic.
- An rdkafka consumer with the same change reads those records.
- No client-side conditional code path is required.
- The same test, swapped to a Redpanda bootstrap address, passes
  identically (verified via FEAT-003 differential parity).

## Notes

This is the keystone story: every other client-facing capability assumes
this one holds.
