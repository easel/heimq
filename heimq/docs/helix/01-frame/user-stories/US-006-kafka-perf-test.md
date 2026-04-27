# US-006 — Run kafka-producer-perf-test and kafka-consumer-perf-test against heimq

**Feature**: FEAT-004
**Priority**: P0

## Story

As a heimq maintainer,
I want the standard Apache Kafka `kafka-producer-perf-test` and
`kafka-consumer-perf-test` tools to run against heimq to completion,
so that I have continuous evidence the broker handles standard
benchmark traffic patterns without protocol or client errors.

## Acceptance Criteria

- `kafka-producer-perf-test` completes against heimq with exit 0 and
  no error lines on a documented load profile.
- `kafka-consumer-perf-test` completes against heimq with exit 0 and
  no error lines on a documented load profile.
- An idempotent profile (`enable.idempotence=true`) and a
  transactional profile (`transactional.id`) each complete cleanly.
- Profiles, expected exit codes, and acceptable warnings are checked
  in under `scripts/bench/`.
