---
ddx:
  id: US-013
  depends_on:
    - FEAT-006
  review:
    self_hash: 7ed2a25a6a8ddb99b7be414e6625c161ee01f3388ad13ec5981cb2a645db74fc
    deps:
      FEAT-006: e59d3b8965ebd35b4bbe9c5302f4218432ad83ec27691989dfd4c345ac2ae004
    reviewed_at: "2026-07-14T06:48:37Z"
---
# US-013 — Modern Kafka client connects without forcing legacy versions

**Feature**: FEAT-006 — Flexible-Version Kafka Protocol Support
**Feature Requirements**: FR-01, FR-02, FR-03, FR-05
**PRD Requirements**: FR-4
**Priority**: P0
**Status**: Specified

## Story

**As a** developer using a current-release Kafka client (librdkafka
≥ ~1.6, Java client ≥ 2.4) with default configuration,
**I want** my client to negotiate flexible-version APIs against heimq,
**So that** I can use modern features (transactions, modern Metadata /
Fetch / ApiVersions) without setting `api.version.request=false`,
pinning a legacy version, or otherwise diverging from how I'd
configure the client against Kafka or Redpanda.

## Context

Modern Kafka clients negotiate the highest version both sides advertise, and
many modern features (transactions v3+, Metadata v9+, Fetch v12+, ApiVersions
v3) only exist at flexible versions — the downgrade path is fragile or absent
(FEAT-006 § Problem Statement). Without this, FEAT-002 transactions and
FEAT-005 ecosystem tools cannot interoperate with heimq. This story exercises
FEAT-006 FR-01, FR-02, FR-03, and FR-05, tracing to PRD FR-4 (the
flexible-version portion of P0 #1).

## Walkthrough

1. Developer points a current-release rdkafka producer and consumer with
   default configuration (no `api.version.request=false`, no version pin) at
   heimq.
2. The client exchanges ApiVersions v3 (flexible request and response);
   heimq advertises current Kafka per-API maxima for the in-scope surface.
3. The client negotiates flexible-only API versions (Metadata v9+,
   Fetch v12+, Produce v9+, InitProducerId v2+, EndTxn v3+) and the workload
   succeeds.
4. The producer/consumer complete a produce/fetch round-trip without being
   forced onto a legacy downgrade path.
5. Differential parity vs Redpanda (FEAT-003) at flexible versions reports
   zero diffs for in-scope APIs.

## Acceptance Criteria

- [ ] **US-013-AC1** — Given an rdkafka producer and consumer with
  default configuration, when they connect to heimq, then they complete a
  produce/fetch round-trip.
- [ ] **US-013-AC2** — Given a client that negotiates modern versions,
  when it exchanges ApiVersions v3, then heimq serves it (flexible
  request and response).
- [ ] **US-013-AC3** — Given a workload exercising flexible-only API
  versions (Metadata v9+, Fetch v12+, Produce v9+, InitProducerId v2+,
  EndTxn v3+), when it runs against heimq, then it succeeds.
- [ ] **US-013-AC4** — Given differential parity vs Redpanda (FEAT-003),
  when it runs at flexible versions for in-scope APIs, then it reports
  zero diffs.

## Edge Cases

From FEAT-006 § Edge Cases and Error Handling:

- **Unknown tagged fields**: ignored on decode, preserved or dropped per
  Kafka spec for that response.
- **Truncated tagged-fields block**: standard codec error.
- **Mixed legacy + flexible workload**: each request is decoded per its
  api_key + api_version; legacy handlers continue to serve legacy versions.
- **Compact-string null sentinel**: encoded as varint 0 (length+1 = 0 meaning
  null), distinct from empty string (varint 1).

## Test Scenarios

- **US-013-AC1** → integration: default-configuration rdkafka producer/consumer complete a produce/fetch round-trip.
- **US-013-AC2** → contract: ApiVersions v3 exchange served with flexible request and response (codec round-trips also covered by property tests per FEAT-006 NFRs).
- **US-013-AC3** → contract: flexible-only versions (Metadata v9+, Fetch v12+, Produce v9+, InitProducerId v2+, EndTxn v3+) succeed.
- **US-013-AC4** → parity: FEAT-003 harness at flexible versions reports zero diffs for in-scope APIs.

## Dependencies

- **Stories**: US-001 (the legacy-layer keystone this story extends to modern versions); US-005 (parity harness for AC4).
- **Feature Spec**: FEAT-006
- **Feature Requirements**: FR-01, FR-02, FR-03, FR-05
- **PRD Requirements**: FR-4
- **External**: FEAT-001 (foundation; per FEAT-006 dependencies); flexible codec primitives from the `kafka-protocol` Rust crate or equivalent (selection is a Design-phase ADR); modern rdkafka / Java clients; Redpanda container for AC4 (FEAT-003).

## Out of Scope

Per FEAT-006 § Out of Scope:

- KIP-482 tagged-field semantic interpretation beyond pass-through.
- Header-versioning quirks for APIs not in the in-scope surface.
- Compression-format upgrades (already covered by Phase 5 client surface).
