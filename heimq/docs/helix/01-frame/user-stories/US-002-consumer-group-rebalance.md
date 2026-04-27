# US-002 — Consumer group rebalance correctness

**Feature**: FEAT-002
**Priority**: P0

## Story

As a service owner running a consumer-group-based application,
I want heimq to handle group join, leave, and session-timeout rebalances
correctly,
so that my service observes no record gaps or duplicate ownership during
membership changes.

## Acceptance Criteria

- A group of N members reading a partitioned topic delivers every
  produced record exactly once across the group.
- On member leave (graceful and session-timeout), remaining members
  collectively own all partitions.
- On member join, partitions are re-divided per the configured
  partition assignor without dropping records.
- After member restart, the resumed member starts from the committed
  offset.
- Identical workload run against Redpanda (FEAT-003) shows the same
  delivery profile.
