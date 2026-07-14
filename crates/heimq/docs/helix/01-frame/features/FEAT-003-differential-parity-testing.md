---
ddx:
  id: FEAT-003
  depends_on:
    - helix.prd
    - FEAT-001
    - FEAT-002
  review:
    self_hash: 2d80d13a1295f71bfa6d8dd0ee2097b4835f242dd4aca487a3479860706410ce
    deps:
      FEAT-001: 7133c264bc364ec4535c1d6b6187a90c9ba66d1fa3df30731ade260c2e092479
      FEAT-002: 164350929c7bbc09a589f3cd1a80b685e88cce1054445fe5373aec566464636f
      helix.prd: debc0a32007f0c42db51e82f47848c7b988c3f67f6f97171069170492f9b5b95
    reviewed_at: "2026-07-14T06:48:37Z"
---
# Feature Specification: FEAT-003 — Differential Parity Testing vs Redpanda/Kafka

**Feature ID**: FEAT-003
**Status**: Specified
**Priority**: P0
**Owner**: heimq core
**Covered PRD Subsystem(s)**: Differential parity (FEAT-003)
**Covered PRD Requirements**: FR-8 (differential parity testing) — PRD P0 #5
**Cross-Subsystem Rationale**: None — single subsystem.

## Overview

A test harness drives the same client-level workload against heimq and
against a real Redpanda (or Kafka) instance and asserts that observable
behavior is equivalent for in-scope APIs. This is how heimq's PRD claim
of "behaves identically to Kafka/Redpanda" is verified rather than
asserted.

## Ideal Future State

Every protocol-touching change is gated by a reproducible diff harness:
the same client-level workload runs against heimq and a Redpanda
container, observable outputs are normalized and diffed, zero behavioral
diffs is the passing condition, and CI fails on regressions — so parity
with Redpanda is observable rather than asserted.

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

- **FR-01** — A harness runs the same client workload (driver script + identical
  client config) against heimq and against a Redpanda container.
- **FR-02** — The harness records observable client outputs: records consumed
  (key, value, headers, partition, offset), error codes returned, group
  state observed via client API, transactional outcomes (committed vs
  aborted record visibility under each isolation level).
- **FR-03** — The harness normalizes non-determinism (broker ids, host-specific
  timestamps, monotonic ids) before diffing.
- **FR-04** — The harness reports zero diffs as success and prints a structured
  diff on failure (target field, heimq value, redpanda value, request
  correlation_id).
- **FR-05** — Workloads cover produce/fetch and consumer groups as gating workloads
  at FEAT-003 acceptance; idempotent producers and transactions are added
  once FEAT-002 is accepted (see US-005 acceptance criteria and
  solution-designs/SD-003-differential-parity-testing.md § Expansion path).
- **FR-06** — The harness is runnable locally (developer machine with Docker) and in
  CI.

### Non-Functional Requirements

- **Determinism**: Diff harness must be deterministic — runs of the same
  workload produce the same diff (after normalization).
- **Performance**: Harness completes its gating workload in under 10
  minutes on CI hardware.
- **Reliability**: The executable reliability rule is a zero-failure
  invariant over the current GitHub Actions workflow run. The FEAT-003
  evidence window is the `conformance` workflow's `parity` job: one
  required parity-harness attempt, zero allowed failures, emitted and
  enforced by `scripts/ci/reliability-gate.sh feat-003-parity -- ./tests/conformance/run.sh`.
  Historical flake-rate percentages are not claimed until a separate
  rolling measurement store exists.

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
- Current-workflow reliability evidence for FEAT-003 shows
  `reliability_rule=zero_failures`, `required_attempts=1`,
  `allowed_failures=0`, and a passing `feat-003-parity` target.

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
