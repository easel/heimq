---
ddx:
  id: US-001
  depends_on:
    - FEAT-001
  review:
    self_hash: 26bbd1249d94abf0000b005c67fc86eb59936c64075631102ba7cf7dea75e643
    deps:
      FEAT-001: 7133c264bc364ec4535c1d6b6187a90c9ba66d1fa3df30731ade260c2e092479
    reviewed_at: "2026-07-14T05:12:26Z"
---
# US-001 — Standard Kafka client connects without code changes

**Feature**: FEAT-001 — Kafka Wire-Protocol Compatibility
**Feature Requirements**: FR-01, FR-02
**PRD Requirements**: FR-1
**Priority**: P0
**Status**: Specified

## Story

**As a** developer with an existing Kafka producer or consumer,
**I want** to point my client at heimq instead of Kafka/Redpanda by changing
only the bootstrap address,
**So that** I can run my service against heimq without forking my client code
or test setup.

## Context

This is the keystone story: every other client-facing capability assumes it
holds. The Backend Developer / Test Author persona (PRD) today must run
Kafka/Redpanda or accept mock brokers that drift from real protocol behavior.
It exercises FEAT-001 FR-01 (ApiVersions advertises the supported surface)
and FR-02 (in-scope APIs answer with semantically equivalent responses),
tracing to PRD FR-1 / P0 #1.

## Walkthrough

1. Developer starts a heimq broker and changes only `bootstrap.servers` in an
   existing rdkafka producer/consumer setup.
2. The client connects; heimq answers the version-negotiation and metadata
   probes the client issues, with no client-side conditional code path.
3. Developer runs the producer; records land on the heimq topic.
4. Developer runs the consumer; it reads back the produced records.
5. Developer swaps the bootstrap address to Redpanda; the same test passes
   identically (verified via the FEAT-003 parity harness).

## Acceptance Criteria

- [ ] **US-001-AC1** — Given a running heimq broker, when an rdkafka
  producer configured only with `bootstrap.servers=<heimq>` produces
  records to a heimq topic, then the records are produced successfully.
- [ ] **US-001-AC2** — Given records produced per US-001-AC1, when an
  rdkafka consumer with the same bootstrap-address-only change reads the
  topic, then it reads those records.
- [ ] **US-001-AC3** — Given the same client code used against
  Kafka/Redpanda, when it runs against heimq, then no client-side
  conditional code path is required.
- [ ] **US-001-AC4** — Given the same test, when it is swapped to a
  Redpanda bootstrap address, then it passes identically (verified via
  FEAT-003 differential parity).

## Edge Cases

From FEAT-001 § Edge Cases and Error Handling:

- **Unknown API key**: return `UNSUPPORTED_VERSION` per Kafka spec.
- **Flexible-version request**: flexible-version negotiation is owned by
  FEAT-006; until FEAT-006 is delivered, flexible requests are rejected with
  the standard error rather than silently downgraded.

## Test Scenarios

- **US-001-AC1** → integration: rdkafka producer with bootstrap-only config produces to a heimq topic.
- **US-001-AC2** → integration: rdkafka consumer with bootstrap-only config reads the records back.
- **US-001-AC3** → integration: same client code runs against heimq with no conditional code path.
- **US-001-AC4** → parity: identical test against a Redpanda bootstrap address passes identically (FEAT-003 harness).

## Dependencies

- **Stories**: None — this is the keystone story other client-facing stories build on.
- **Feature Spec**: FEAT-001
- **Feature Requirements**: FR-01, FR-02
- **PRD Requirements**: FR-1
- **External**: Redpanda container (for the AC4 parity check via FEAT-003); rdkafka client.

## Out of Scope

Per FEAT-001 § Out of Scope and PRD Non-Goals:

- Flexible-version protocol encoding — delivered by FEAT-006 (US-013).
- Security (SASL, ACLs, delegation tokens).
- Multi-broker / replication / KRaft.
