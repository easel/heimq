---
ddx:
  id: US-016
  depends_on:
    - FEAT-008
  links:
    - kind: derived_from
      to: FEAT-008
    - kind: constrained_by
      to: TRAIT-001
    - kind: covers
      to: FR-12
  review:
    self_hash: dbc937ade93ef6f92e5ee3a5f95ee4a23a7383ca0e4c94bd69e92099ed1d0b92
    deps:
      FEAT-008: 9c4681949c5ee4823eb76b3b78411e72752795175efa03ac41f2c30d38d8f7a4
    reviewed_at: "2026-07-14T06:48:37Z"
---
# US-016 — Backend trait conformance for embedders

**Feature**: FEAT-008 — Engine Embedding Contract
**Feature Requirements**: EMB-01, EMB-02, EMB-03
**PRD Requirements**: FR-12
**Priority**: P0
**Status**: Specified

## Story

**As a** Broker Builder,
**I want** to implement the heimq backend trait families and run their
conformance suites,
**So that** my storage or coordination plane can serve only the Kafka APIs it
actually supports while remaining compatible with the engine contract.

## Context

Broker Builders embed `heimq-wire`, `heimq-broker`, and `heimq-testkit` rather
than the `heimq` bin crate. FR-12 requires stable trait families, per-trait
conformance suites, and capability-gated API advertisement; TRAIT-001 owns the
normative trait and capability rules. This story covers the backend side of
FEAT-008.

## Walkthrough

1. Broker Builder selects the trait families their backend can serve.
2. Builder implements those families against the TRAIT-001 contract.
3. Builder runs the corresponding `heimq-testkit` conformance suites.
4. Builder starts the embedded engine with those capabilities.
5. Standard clients see only APIs backed by implemented trait families in
   ApiVersions, while unrelated APIs remain available.

## Acceptance Criteria

- [ ] **US-016-AC1** — Given a backend implements a TRAIT-001 trait family, when
  its matching `heimq-testkit` conformance suite runs, then the suite verifies
  the family behavior required by FR-12 without depending on the `heimq` bin
  crate.
- [ ] **US-016-AC2** — Given a backend does not implement a trait family, when
  ApiVersions is computed, then Kafka APIs requiring that family are not
  advertised.
- [ ] **US-016-AC3** — Given a backend omits one trait family but implements
  another, when ApiVersions is computed, then only APIs gated by the missing
  family are removed and unrelated supported APIs remain advertised.
- [ ] **US-016-AC4** — Given backend trait methods serve a client request, when
  the engine delegates to them, then the request context defined by TRAIT-001 is
  available to the backend without the engine enforcing tenancy, quota, or RBAC.

## Edge Cases

- **Unknown or placeholder capability**: the family is treated as absent for
  advertisement, preventing clients from selecting unsupported APIs.
- **Conformance failure**: the backend is not considered compatible with that
  trait family until the failing behavior is fixed or explicitly marked out of
  scope by capability gating.
- **Partial implementation**: unsupported families do not disable unrelated API
  groups.

## Test Scenarios

| Scenario | AC ID | Input / State | Action | Expected Result |
|----------|-------|---------------|--------|-----------------|
| Trait suite passes | US-016-AC1 | Backend implements `OffsetStore` per TRAIT-001 | Run the offset-store conformance suite | Suite passes without importing the `heimq` bin crate |
| Missing family is gated | US-016-AC2 | Backend has no group-coordinator family | Compute ApiVersions | Group APIs are absent from the advertised set |
| Gating is narrow | US-016-AC3 | Backend has log and offset families, no group family | Compute ApiVersions | Produce/fetch/offset APIs remain advertised; group APIs are absent |
| Context reaches backend | US-016-AC4 | Request context has principal and client id | Execute a client-serving trait method | Backend observes the context values |

## Dependencies

- **Stories**: None.
- **Feature Spec**: FEAT-008
- **Feature Requirements**: EMB-01, EMB-02, EMB-03
- **PRD Requirements**: FR-12
- **External**: TRAIT-001 for exact trait families, capability rules, and
  conformance obligations.

## Out of Scope

- Changing trait signatures.
- Implementing a new durable backend.
- Defining Kafka wire framing or handler registry behavior; US-017 and
  WIRE-001 own that path.
