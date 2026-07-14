---
ddx:
  id: US-011
  depends_on:
    - FEAT-005
  review:
    self_hash: ab6e0ae60d62bfd32fbae3aa0f4caeaaf105ee38e81d63c3ddeb9811f1ccfb08
    deps:
      FEAT-005: 36cf0d0af7c2a412b9a7fce2edf8a917dcf8c7518190e444acc7813b1ff563b8
    reviewed_at: "2026-07-14T06:48:37Z"
---
# US-011 — Schema Registry round-trip against heimq

**Feature**: FEAT-005 — Ecosystem Integrations
**Feature Requirements**: SR-01, INT-01
**PRD Requirements**: FR-10
**Priority**: P0
**Status**: Specified

## Story

**As a** developer using a Schema Registry,
**I want** a producer with an Avro/Protobuf serializer to publish records
to heimq and a consumer with the matching deserializer to consume them,
**So that** schema-aware pipelines work against heimq for tests / local
development.

## Context

Schema-aware pipelines pair a registry with the broker; the Confluent Schema
Registry API is the resolved target (ADR-005, FEAT-005 § Constraints). A
developer using a registry needs proof that the Confluent Avro/Protobuf
serializer publishes and resolves schemas while records round-trip via heimq
(FEAT-005 SR-01). This story exercises FEAT-005 SR-01 and INT-01, tracing to
PRD FR-10 / P0 #7.

## Walkthrough

1. Developer invokes the integration script; it brings up a Confluent Schema
   Registry instance alongside heimq.
2. A schema is registered against the registry; registration succeeds.
3. A producer using the registry serializer writes records; they land in a
   heimq topic.
4. A consumer using the registry deserializer reads the topic and decodes
   the records correctly.
5. The script tears down and exits 0.

## Acceptance Criteria

- [ ] **US-011-AC1** — Given a Confluent Schema Registry instance, when a
  schema is registered against it, then registration succeeds.
- [ ] **US-011-AC2** — Given the registered schema, when a producer with
  the registry serializer writes records, then the records land in a
  heimq topic.
- [ ] **US-011-AC3** — Given records written per US-011-AC2, when a
  consumer with the registry deserializer reads them, then it decodes
  those records correctly.
- [ ] **US-011-AC4** — Given the integration, when its script is invoked,
  then a single script brings up the registry, runs the round-trip, tears
  down, and exits 0.

## Edge Cases

From FEAT-005 § Edge Cases and Error Handling:

- **Tool requires APIs heimq does not implement**: parking-lot the required
  API; either implement a minimal stub or document the non-support and
  exclude that tool from the matrix with a written rationale.
- **Tool depends on Confluent-only APIs**: scope decision recorded in the
  integration's README and the parking lot.

## Test Scenarios

- **US-011-AC1** → ecosystem: schema registration against a Confluent Schema Registry instance succeeds.
- **US-011-AC2** → ecosystem: producer with the registry serializer lands records in a heimq topic.
- **US-011-AC3** → ecosystem: consumer with the registry deserializer decodes the records correctly.
- **US-011-AC4** → ecosystem: single script brings up the registry, runs the round-trip, tears down, exits 0.

## Dependencies

- **Stories**: US-001 (client connectivity).
- **Feature Spec**: FEAT-005
- **Feature Requirements**: SR-01, INT-01
- **PRD Requirements**: FR-10
- **External**: Confluent Schema Registry instance / Docker image (target per ADR-005); Confluent Avro/Protobuf serializers; FEAT-001 wire protocol, FEAT-002 core semantics, FEAT-006 flexible-version protocol (Confluent serializers default to flexible-version APIs; per FEAT-005 dependencies).

## Out of Scope

Per FEAT-005 § Out of Scope and PRD Non-Goals:

- Performance comparisons against Kafka/Redpanda for this integration.
- Registry APIs other than the Confluent Schema Registry API (ADR-005).
- Durability of registry-serialized topics across heimq restart (PRD
  non-goal #1).
