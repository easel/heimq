# US-007 — Run OpenMessaging Benchmark against heimq

**Feature**: FEAT-004
**Priority**: P0

## Story

As a heimq maintainer,
I want the OpenMessaging Benchmark Kafka driver to run against heimq
to completion on a documented workload,
so that heimq is exercised by an industry-standard, multi-tool
benchmark beyond Apache Kafka's own perf-test scripts.

## Acceptance Criteria

- The OpenMessaging Benchmark Kafka driver completes against heimq
  for at least one documented workload (e.g., 1KB records, N
  partitions, M producers, K consumers, capped duration) with no
  protocol or client errors.
- The workload definition (YAML) is checked in under
  `scripts/bench/openmessaging/`.
- The benchmark is runnable locally with Docker and in CI.
