---
ddx:
  id: US-017
  depends_on:
    - FEAT-008
  links:
    - kind: derived_from
      to: FEAT-008
    - kind: constrained_by
      to: WIRE-001
    - kind: constrained_by
      to: TRAIT-001
    - kind: covers
      to: FR-13
  review:
    self_hash: 9a8ed6ad154999b4271a14704f58b6d0ba07c1ed61970fc54e42dbce77c07940
    deps:
      FEAT-008: 9c4681949c5ee4823eb76b3b78411e72752795175efa03ac41f2c30d38d8f7a4
    reviewed_at: "2026-07-14T06:48:37Z"
---
# US-017 — Request-level handler delegation for embedders

**Feature**: FEAT-008 — Engine Embedding Contract
**Feature Requirements**: EMB-04, EMB-05, EMB-06, EMB-07
**PRD Requirements**: FR-13
**Priority**: P0
**Status**: Specified

## Story

**As a** Broker Builder,
**I want** request handlers to delegate async work, request context, deferred
acknowledgements, and response encoding through the engine contract,
**So that** my embedded broker can perform product-specific pre-response work
without owning Kafka framing or forking handler behavior.

## Context

FR-13 requires async handlers, deferred produce acknowledgements, per-request
principal context, backpressure mapping, cancellation, response-encoding
ownership, and SASL/TLS capability gating. WIRE-001 owns the handler and
registry contract; TRAIT-001 owns the backend request-context expectations.
This story covers the request-level delegation side of FEAT-008.

## Walkthrough

1. Broker Builder configures an embedded engine with backend trait
   implementations.
2. A standard Kafka client sends a request through `heimq-wire`.
3. The gateway decodes the typed request, creates request context, and invokes
   the async handler.
4. The handler delegates to backend trait calls and may await pre-ack work
   before returning a response.
5. The gateway encodes the response and preserves per-connection ordering.

## Acceptance Criteria

- [ ] **US-017-AC1** — Given a typed Kafka request covered by WIRE-001, when the
  gateway dispatches it, then the async handler receives the decoded request
  and request context before backend delegation.
- [ ] **US-017-AC2** — Given produce handling requires pre-ack backend work, when
  the handler returns a deferred acknowledgement, then the gateway waits for
  that work before sending the Produce response.
- [ ] **US-017-AC3** — Given multiple requests are queued on one connection, when
  an earlier request is waiting on deferred work, then later responses are not
  sent before the earlier response.
- [ ] **US-017-AC4** — Given a handler reports overload, when the gateway builds
  the response, then WIRE-001 backpressure mapping determines the Kafka error
  and the handler does not write a raw frame.
- [ ] **US-017-AC5** — Given a handler returns a typed response, when the gateway
  sends it, then response header selection, body encoding, and frame prefixing
  are owned by the gateway per WIRE-001.

## Edge Cases

- **Connection closes during pending work**: the request context cancellation
  signal is set and backend completion policy follows the embedder's trait
  implementation.
- **Raw escape hatch request**: raw handlers remain explicit and are used only
  for API keys not covered by typed handlers.
- **SASL/TLS gate off**: SASL APIs are not advertised and request context may
  carry no principal.

## Test Scenarios

| Scenario | AC ID | Input / State | Action | Expected Result |
|----------|-------|---------------|--------|-----------------|
| Typed dispatch | US-017-AC1 | Typed Produce request and request context | Dispatch through the gateway | Handler receives decoded request plus context |
| Deferred produce ack | US-017-AC2 | Backend append future pending on pre-ack work | Handle Produce | Response is sent only after the future completes |
| FIFO with deferred work | US-017-AC3 | Produce followed by Metadata on same connection | Delay Produce completion | Metadata response is not sent before Produce response |
| Overload mapping | US-017-AC4 | Handler returns overload during Produce | Gateway builds error response | Produce receives WIRE-001 storage-error mapping; handler writes no frame |
| Gateway encoding | US-017-AC5 | Handler returns typed response | Gateway sends response | Response header, body, and length prefix are encoded by the gateway |

## Dependencies

- **Stories**: US-016 for backend trait families that handlers delegate into.
- **Feature Spec**: FEAT-008
- **Feature Requirements**: EMB-04, EMB-05, EMB-06, EMB-07
- **PRD Requirements**: FR-13
- **External**: WIRE-001 for exact handler, registry, cancellation,
  backpressure, and response-encoding rules; TRAIT-001 for backend
  request-context propagation.

## Out of Scope

- Defining exact handler signatures or trait method signatures inline.
- Implementing SASL/TLS in the `heimq` distribution.
- Changing Kafka protocol codec rules owned by CODEC-001.
