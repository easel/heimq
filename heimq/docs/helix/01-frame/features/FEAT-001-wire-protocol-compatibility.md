---
dun:
  id: FEAT-001
  depends_on:
    - helix.prd
---
# Feature Specification: FEAT-001 — Kafka Wire-Protocol Compatibility

**Feature ID**: FEAT-001
**Status**: Specified
**Priority**: P0
**Owner**: heimq core

## Overview

heimq speaks the Kafka binary wire protocol such that standard Kafka
producers and consumers — using any client library (rdkafka, java client,
sarama, etc.) — connect with no code changes and observe the same protocol-
level behavior they would against Kafka or Redpanda for the in-scope API
surface. This addresses PRD goal 1 (clients connect unchanged) and is the
foundation other features build on.

## Problem Statement

- **Current situation**: heimq implements a non-flexible subset of the
  Kafka protocol (Produce, Fetch, Metadata, ListOffsets, OffsetCommit /
  Fetch, FindCoordinator, JoinGroup, SyncGroup, Heartbeat, LeaveGroup,
  ApiVersions, CreateTopics, DeleteTopics) per `API-001-kafka-protocol.md`.
- **Pain points**: Test authors and developers cannot point arbitrary
  Kafka tooling at a non-Kafka broker without behavioral surprises;
  divergences from Kafka are not always detectable until production.
- **Desired outcome**: For every in-scope API/version pair, a standard
  client request returns a byte-equivalent (modulo ids/timestamps)
  response to what Kafka/Redpanda would return, verified by contract and
  differential tests.

## Requirements

### Functional Requirements

1. ApiVersions advertises the runtime intersection of `SUPPORTED_APIS` and
   per-backend capability gates (`compute_supported_apis`).
2. All in-scope API keys decode and answer with semantically equivalent
   responses to Kafka/Redpanda for the same input.
3. Unsupported APIs and versions return the standard Kafka error codes.
4. Per-API maxima target current Kafka spec versions (subject to the
   in-scope semantic surface). Flexible-version decode/encode is
   delivered by FEAT-006; FEAT-001 alone is the legacy / pre-flexible
   layer of the same wire-compatibility goal.
5. Capability gating filters advertised APIs per-API (a missing group
   coordinator drops only group APIs, etc.).

### Non-Functional Requirements

- **Performance**: No hard latency target; standard Kafka benchmarks (see
  FEAT-004) must complete without protocol errors.
- **Compatibility**: Any client that talks to Redpanda for in-scope APIs
  must talk to heimq.
- **Reliability**: Wire-protocol contract tests pass at 100% for in-scope
  APIs.

## User Stories

- [US-001 — Standard client connects without code changes](../user-stories/US-001-standard-client-connects.md)

## Edge Cases and Error Handling

- **Unknown API key**: return `UNSUPPORTED_VERSION` per Kafka spec.
- **Flexible-version request**: rejected with the standard error; not
  silently downgraded.
- **ApiVersions probe with an unknown client_software_name field
  (flexible)**: handled per non-flexible policy.

## Success Metrics

- 100% contract test coverage for in-scope APIs.
- Differential parity (FEAT-003) reports zero diffs at the wire level for
  in-scope APIs.

## Constraints and Assumptions

- Single-node only; no controller/replication APIs.
- No flexible versions.

## Dependencies

- **Other features**: Foundation for FEAT-002, FEAT-003, FEAT-004, FEAT-005.
- **External services**: None at runtime; Redpanda for parity tests.
- **PRD requirements**: P0 #1 (wire-protocol compatibility).

## Out of Scope

- Multi-broker / replication / KRaft.
- Security (SASL, ACLs, delegation tokens).
- Share groups, telemetry APIs, admin reassignment APIs.
- Flexible-version protocol encoding (delivered by FEAT-006).
