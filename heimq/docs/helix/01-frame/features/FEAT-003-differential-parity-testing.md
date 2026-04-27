---
dun:
  id: FEAT-003
  depends_on:
    - helix.prd
    - FEAT-001
    - FEAT-002
---
# Feature Specification: FEAT-003 — Differential Parity Testing vs Redpanda/Kafka

**Feature ID**: FEAT-003
**Status**: Specified
**Priority**: P0
**Owner**: heimq core

## Overview

A test harness drives the same client-level workload against heimq and
against a real Redpanda (or Kafka) instance and asserts that observable
behavior is equivalent for in-scope APIs. This is how heimq's PRD claim
of "behaves identically to Kafka/Redpanda" is verified rather than
asserted.

## Problem Statement

- **Current situation**: `scripts/compatibility-test.sh` runs against
  Redpanda; per-test parity is ad-hoc and not enforced as a diff. Test
  plan Phase 4 ("Baseline Parity") is in flight but incomplete.
- **Pain points**: Divergences from Redpanda may pass contract tests yet
  fail in production tools (Kafka Connect, Debezium, etc.) because the
  contract test only asserts what we thought to assert.
- **Desired outcome**: A reproducible diff harness reports zero
  behavioral diffs between heimq and Redpanda for in-scope APIs at a
  gating workload, and CI fails on regressions.

## Requirements

### Functional Requirements

1. A harness runs the same client workload (driver script + identical
   client config) against heimq and against a Redpanda container.
2. The harness records observable client outputs: records consumed
   (key, value, headers, partition, offset), error codes returned, group
   state observed via client API, transactional outcomes (committed vs
   aborted record visibility under each isolation level).
3. The harness normalizes non-determinism (broker ids, host-specific
   timestamps, monotonic ids) before diffing.
4. The harness reports zero diffs as success and prints a structured
   diff on failure (target field, heimq value, redpanda value, request
   correlation_id).
5. Workloads cover produce/fetch and consumer groups as gating workloads
   at FEAT-003 acceptance; idempotent producers and transactions are added
   once FEAT-002 is accepted (see US-005 acceptance criteria and
   HARNESS-001-parity.md § Expansion path).
6. The harness is runnable locally (developer machine with Docker) and in
   CI.

### Non-Functional Requirements

- **Determinism**: Diff harness must be deterministic — runs of the same
  workload produce the same diff (after normalization).
- **Performance**: Harness completes its gating workload in under 10
  minutes on CI hardware.
- **Reliability**: Less than 1% flake rate on the gating workload.

## User Stories

- [US-005 — Run the same workload against heimq and Redpanda and diff](../user-stories/US-005-differential-parity-harness.md)

## Edge Cases and Error Handling

- **Redpanda container fails to start**: harness exits with a clear,
  non-flake error; CI marks the run as `error` not `fail`.
- **Workload non-determinism slips through**: harness logs the
  normalization rules used so divergences can be triaged.
- **Intentional diffs (e.g., loss-on-restart)**: encoded as known
  exemptions in the harness with a reference to the PRD non-goal.

## Success Metrics

- Zero unmatched diffs at the gating workload across in-scope APIs.
- Differential harness gates CI for protocol-touching changes.

## Constraints and Assumptions

- Redpanda is an acceptable proxy for Kafka behavior for the in-scope APIs.
- Docker-in-Docker (or privileged docker) is available in CI.

## Dependencies

- **Other features**: FEAT-001 and FEAT-002 (defines the in-scope APIs).
- **External services**: Redpanda container; optionally Kafka container.
- **PRD requirements**: P0 #5.

## Out of Scope

- Performance / throughput diffs (handled by FEAT-004).
- Diffs for out-of-scope APIs (security, share groups, admin
  reassignment, etc.).
