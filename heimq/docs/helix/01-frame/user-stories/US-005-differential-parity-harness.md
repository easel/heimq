# US-005 — Run the same workload against heimq and Redpanda and diff

**Feature**: FEAT-003
**Priority**: P0

## Story

As a heimq maintainer,
I want a harness that runs identical Kafka client workloads against heimq
and Redpanda and diffs observable behavior,
so that protocol-touching changes cannot silently regress parity.

## Acceptance Criteria

- A single command runs the gating workload against both brokers and
  reports a structured diff.
- The harness normalizes broker ids, host-specific timestamps, and
  monotonic ids before diffing.
- Zero diffs on the gating workload is the success condition.
- The harness covers produce/fetch and consumer groups as gating
  workloads; idempotent producers and transactions are included once
  FEAT-002 is accepted (see HARNESS-001-parity.md § Expansion path).
- The harness is runnable locally and in CI.
- Known intentional diffs (e.g., loss-on-restart) are encoded as
  exemptions with PRD references.
