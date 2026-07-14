---
ddx:
  id: US-005
  depends_on:
    - FEAT-003
  review:
    self_hash: 168616bd29809066f578398a7c34ba64f23005e237d58eac1381dde83a4ef2fd
    deps:
      FEAT-003: d134a5740222f1b59ecda81b318c7b28de9968bba9f640bb6280310c1e906233
    reviewed_at: "2026-07-14T05:12:26Z"
---
# US-005 — Run the same workload against heimq and Redpanda and diff

**Feature**: FEAT-003 — Differential Parity Testing vs Redpanda/Kafka
**Feature Requirements**: FR-01, FR-02, FR-03, FR-04, FR-05, FR-06
**PRD Requirements**: FR-8
**Priority**: P0
**Status**: Specified

## Story

**As a** heimq maintainer,
**I want** a harness that runs identical Kafka client workloads against heimq
and Redpanda and diffs observable behavior,
**So that** protocol-touching changes cannot silently regress parity.

## Context

heimq's PRD claim of "behaves identically to Kafka/Redpanda" must be verified
rather than asserted; today per-test parity is ad-hoc and divergences can pass
contract tests yet fail in real tools (FEAT-003 § Problem Statement). The
maintainer needs a reproducible diff harness whose zero-diff result gates
protocol-touching changes. This story exercises FEAT-003 FR-01 through FR-06,
tracing to PRD FR-8 / P0 #5.

## Walkthrough

1. Maintainer invokes a single command on a machine with Docker (locally or
   in CI).
2. The harness runs the gating workload (produce/fetch and consumer groups)
   with identical client config against heimq and a Redpanda container.
3. The harness records observable client outputs, normalizes non-determinism
   (broker ids, host-specific timestamps, monotonic ids), and diffs the two
   runs.
4. On success it reports zero diffs; on failure it prints a structured diff;
   known intentional diffs (e.g., loss-on-restart) are encoded as exemptions
   with a PRD reference.

## Acceptance Criteria

- [ ] **US-005-AC1** — Given heimq and Redpanda are available, when a
  single command runs the gating workload against both brokers, then it
  reports a structured diff.
- [ ] **US-005-AC2** — Given observable outputs from both brokers, when
  the harness diffs them, then broker ids, host-specific timestamps, and
  monotonic ids are normalized before diffing.
- [ ] **US-005-AC3** — Given the gating workload, when the harness
  completes, then zero diffs is the success condition.
- [ ] **US-005-AC4** — Given the gating workloads, when the harness runs,
  then produce/fetch and consumer groups are covered.
- [ ] **US-005-AC5** — Given a local checkout or a CI environment, when
  the harness is invoked, then it runs in both.
- [ ] **US-005-AC6** — Given a known intentional diff (e.g.,
  loss-on-restart), when the harness encounters it, then it is encoded as
  an exemption with a PRD reference.
- [ ] **US-005-AC7** — (activates when FEAT-002 lands) Given FEAT-002 is
  accepted, when the harness runs, then idempotent producer and
  transaction workloads are included (see
  ../../02-design/solution-designs/SD-003-differential-parity-testing.md
  § Expansion path).

## Edge Cases

From FEAT-003 § Edge Cases and Error Handling:

- **Redpanda container fails to start**: harness exits with a clear,
  non-flake error; CI marks the run as `error` not `fail`.
- **Workload non-determinism slips through**: harness logs the normalization
  rules used so divergences can be triaged.
- **Intentional diffs (e.g., loss-on-restart)**: encoded as known exemptions
  in the harness with a reference to the PRD non-goal.

## Test Scenarios

- **US-005-AC1** → parity: single command runs the gating workload against both brokers and reports a structured diff.
- **US-005-AC2** → parity: broker ids, host-specific timestamps, monotonic ids normalized before diffing.
- **US-005-AC3** → parity: zero diffs is the success condition on the gating workload.
- **US-005-AC4** → parity: produce/fetch and consumer-group workloads covered.
- **US-005-AC5** → integration: harness invokable from a local checkout and from CI.
- **US-005-AC6** → parity: known intentional diff is matched by an exemption carrying a PRD reference.
- **US-005-AC7** → parity (activates when FEAT-002 lands): idempotent/transactional workloads added once FEAT-002 is accepted.

## Dependencies

- **Stories**: US-001, US-002 (the gating workload covers produce/fetch and consumer groups); US-003, US-004 (workload expansion once FEAT-002 is accepted).
- **Feature Spec**: FEAT-003
- **Feature Requirements**: FR-01, FR-02, FR-03, FR-04, FR-05, FR-06
- **PRD Requirements**: FR-8
- **External**: Docker; Redpanda container (optionally a Kafka container); FEAT-001 and FEAT-002 define the in-scope APIs; ../../02-design/solution-designs/SD-003-differential-parity-testing.md § Expansion path.

## Out of Scope

Per FEAT-003 § Out of Scope:

- Performance / throughput diffs (handled by FEAT-004).
- Diffs for out-of-scope APIs (security, share groups, admin reassignment,
  etc.).
