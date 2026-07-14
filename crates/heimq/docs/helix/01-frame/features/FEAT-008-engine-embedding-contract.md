---
ddx:
  id: FEAT-008
  depends_on:
    - helix.prd
    - TRAIT-001
    - WIRE-001
  links:
    - kind: derived_from
      to: helix.prd
    - kind: constrained_by
      to: TRAIT-001
    - kind: constrained_by
      to: WIRE-001
    - kind: resolves
      to: AR-2026-07-13-repo
  review:
    self_hash: 9c4681949c5ee4823eb76b3b78411e72752795175efa03ac41f2c30d38d8f7a4
    deps:
      TRAIT-001: 0f7508ff4e3da65c8f84f465ea2b2c7db82797016f5333240148cdcaa7e7cc23
      WIRE-001: de364438fe7c52ed13c6debb0f5a76ac7f4e7ee50ad3e3a27852c7dbe601eaa9
      helix.prd: 96f0479e307f2c240e8f47b69fff510164d0b9eda132abb22cc4a860932984fe
    reviewed_at: "2026-07-14T05:12:26Z"
---
# Feature Specification: FEAT-008 — Engine Embedding Contract

**Feature ID**: FEAT-008
**Status**: Specified
**Priority**: P0
**Owner**: heimq core
**Covered PRD Subsystem(s)**: Engine trait surface (TRAIT-001)
**Covered PRD Requirements**: FR-12, FR-13
**Cross-Subsystem Rationale**: None — single PRD subsystem. FR-13 is included here because the PRD places the handler contract under the Engine trait surface subsystem and delegates normative handler/registry detail to WIRE-001.

## Overview

FEAT-008 defines the engine-embedding capability for Broker Builders who depend
on `heimq-wire`, `heimq-broker`, and `heimq-testkit` without depending on the
`heimq` distribution binary. It covers PRD FR-12 and FR-13 by making the trait
families, conformance suites, capability-gated advertisement, and request-level
handler delegation visible as one coherent engine contract.

## Ideal Future State

A Broker Builder can embed heimq as a Kafka front end over its own storage and
coordination plane, implement only the trait families it can serve, and prove
that implementation with per-trait conformance suites. Request handlers remain
async and delegate request context, deferred acknowledgements, backpressure, and
response encoding through the WIRE-001 handler contract, so embedders can add
durability, tenancy, quota, or WAL work behind the engine without forking the
Kafka wire surface.

## Problem Statement

- **Current situation**: TRAIT-001 and WIRE-001 define the engine seams, but no
  feature spec owns the PRD subsystem "Engine trait surface (TRAIT-001)" or
  traces FR-12/FR-13 into user stories.
- **Pain points**: Broker Builders cannot tell which frame artifact governs
  backend trait conformance versus request-level delegation, and deterministic
  HELIX alignment reports FR-12/FR-13 as uncovered.
- **Desired outcome**: FEAT-008 becomes the frame-level owner for engine
  embedding; US-016 covers backend-trait conformance for FR-12, and US-017
  covers request-level handler delegation for FR-13.

## Functional Areas

| Area | User question or job | Feature responsibility |
|------|----------------------|------------------------|
| Backend trait conformance | Can my backend safely implement only the trait families it owns? | Trace FR-12 to stable trait families, per-trait conformance suites, and capability-gated API advertisement in TRAIT-001. |
| Request-level handler delegation | Can I run product work before responses without owning Kafka framing? | Trace FR-13 to async handlers, request context, deferred acks, backpressure mapping, cancellation, and response-encoding ownership in WIRE-001. |

## Requirements

### Functional Requirements by Area

#### Backend trait conformance

- **EMB-01** — The engine must expose the trait families named by FR-12
  (`TopicLog`/`LogBackend`/`PartitionLog`, `OffsetStore`,
  `GroupCoordinatorBackend`, and `ClusterView`) through the normative surface
  in TRAIT-001.
- **EMB-02** — Each trait family must have a corresponding conformance suite in
  `heimq-testkit` so an embedder can verify its implementation without using
  the `heimq` bin crate.
- **EMB-03** — API advertisement must be capability-gated: absent or
  unsupported backend families remove only the Kafka APIs that require that
  family, while unrelated APIs remain advertised.

#### Request-level handler delegation

- **EMB-04** — Request handlers must be async and must receive the per-request
  context required by WIRE-001 and TRAIT-001 before delegating to backend trait
  calls.
- **EMB-05** — Produce and other state-changing handlers must be able to defer
  responses until required pre-ack work completes, preserving per-connection
  response ordering.
- **EMB-06** — Handler overload and backpressure must be reported through the
  WIRE-001 error-mapping contract rather than by handlers writing raw frames or
  embedding ad hoc Kafka error codes.
- **EMB-07** — The gateway, not request handlers, must own response header
  selection, body encoding, and length-prefix framing per WIRE-001.

### Non-Functional Requirements

- **Compatibility**: A backend that passes the relevant `heimq-testkit`
  conformance suite must interoperate with the in-scope Kafka API surface for
  that family.
- **Reliability**: Capability-gated advertisement must be covered by tests that
  fail if APIs are advertised for absent backend families.
- **Security**: Per-request principal context must be threadable to backend
  trait calls while authentication and authorization policy remain embedder
  responsibilities.

## User Stories

- [US-016 — Backend trait conformance for embedders](../user-stories/US-016-backend-trait-conformance.md)
- [US-017 — Request-level handler delegation for embedders](../user-stories/US-017-request-level-handler-delegation.md)

## Edge Cases and Error Handling

- **Partial backend implementation**: unsupported families remove only their
  dependent Kafka APIs from ApiVersions; unrelated APIs still work.
- **Pre-ack work fails**: the handler returns the standard Kafka error response
  defined by WIRE-001 and does not acknowledge the request as successful.
- **Connection closes while deferred work is pending**: cancellation follows the
  WIRE-001 request-context contract; backend policy determines rollback or
  completion semantics.

## Success Metrics

- HELIX alignment reports no FR-to-FEAT or subsystem-to-FEAT blocking finding
  for FR-12, FR-13, or "Engine trait surface (TRAIT-001)".
- `heimq-testkit` exposes per-trait conformance suites for every FR-12 trait
  family.
- Handler contract tests cover async delegation, deferred acknowledgements,
  per-request context, and WIRE-001 backpressure mapping.

## Constraints and Assumptions

- Broker Builders depend on the engine crates only: `heimq-wire`,
  `heimq-broker`, and `heimq-testkit`.
- TRAIT-001 owns exact trait signatures and conformance obligations.
- WIRE-001 owns exact handler, registry, context, cancellation, backpressure,
  and response-encoding surface.

## Dependencies

- **Other features**: FEAT-001 for ApiVersions advertisement and protocol
  response compatibility.
- **External services**: None required by the feature; backend implementations
  may bring their own storage or coordination systems.
- **PRD requirements**: FR-12, FR-13.

## Out of Scope

- Rust implementation changes to trait signatures or handlers.
- Durable message logs, distributed coordination, replication, or KRaft.
- SASL/TLS as a heimq distribution feature; WIRE-001 capability-gates that
  surface for embedders.
