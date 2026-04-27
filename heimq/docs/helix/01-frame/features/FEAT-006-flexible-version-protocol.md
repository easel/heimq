---
dun:
  id: FEAT-006
  depends_on:
    - helix.prd
    - FEAT-001
---
# Feature Specification: FEAT-006 — Flexible-Version Kafka Protocol Support

**Feature ID**: FEAT-006
**Status**: Specified
**Priority**: P0
**Owner**: heimq core

## Overview

heimq decodes and encodes flexible-version Kafka APIs — compact strings,
unsigned varints (varlong), and tagged fields — for every in-scope API
that has a flexible variant in current Kafka. Without this, modern
clients (librdkafka ≥ ~1.6, Java client ≥ 2.4, Kafka Connect, Flink,
Debezium, ksqlDB, Confluent serializers) and the modern transactional
APIs cannot interoperate with heimq. This unblocks FEAT-001 at modern
versions, FEAT-002 (transactions are flexible-only at the versions
clients negotiate), and FEAT-005 (ecosystem tools).

## Problem Statement

- **Current situation**: `API-001-kafka-protocol.md` caps every API
  strictly below its Kafka flexible-version boundary. heimq's handlers
  use legacy length-prefixed strings, fixed-width arrays, and no tagged
  fields.
- **Pain points**: Modern Kafka clients negotiate the highest version
  both sides advertise. They will downgrade against heimq, but many
  modern features (transactions v3+, Metadata v9+, Fetch v12+,
  ApiVersions v3+ flexible request header) only exist at flexible
  versions; the downgrade path is fragile or absent. Ecosystem tools
  routinely require flexible-only versions (e.g., the Confluent client's
  `ApiVersions` v3 includes a flexible request body for client-software
  reporting).
- **Desired outcome**: heimq advertises and serves the current Kafka
  per-API maxima (subject to in-scope semantic surface), with the
  flexible-version codec wired into every handler.

## Requirements

### Functional Requirements

1. A flexible-version codec implements: compact strings (length+1 as
   unsigned varint, then bytes), compact arrays (length+1 as unsigned
   varint, then elements), unsigned varint encoding, signed zigzag
   varint encoding, and tagged-fields blocks (varint count followed by
   tag/length/value tuples).
2. Every in-scope API key has its flexible-request and flexible-response
   variants implemented — request header v2 (with tagged fields and
   client_id still as nullable string) for flexible APIs, response
   header v1 for flexible APIs.
3. `SUPPORTED_APIS` and `compute_supported_apis` are updated to advertise
   current Kafka per-API maxima for the in-scope semantic surface. The
   `API-001` matrix is updated to remove the "first flexible version"
   ceiling and replace it with the current spec maxima.
4. Tagged fields are forwarded through round-trip — heimq does not need
   to interpret unknown tagged fields, but it must not corrupt them on
   re-encode (or, where heimq generates the response, it must include
   the empty-tagged-fields trailer).
5. ApiVersions v3 (flexible request body, including
   `client_software_name` / `client_software_version` ignored on the
   wire and tagged fields) is implemented.

### Non-Functional Requirements

- **Compatibility**: The default rdkafka and Java client configuration
  (no `api.version.request=false`, no manual version pin) connects and
  exchanges data without forcing a downgrade.
- **Reliability**: 100% of decode/encode round-trips on flexible
  variants pass property-based tests.
- **Determinism**: Differential parity (FEAT-003) reports zero diffs at
  flexible versions for in-scope APIs.

## User Stories

- [US-013 — Modern client connects without forcing legacy versions](../user-stories/US-013-modern-client-no-downgrade.md)

## Edge Cases and Error Handling

- **Unknown tagged fields**: ignored on decode, preserved or dropped per
  Kafka spec for that response.
- **Truncated tagged-fields block**: standard codec error.
- **Mixed legacy + flexible workload**: each request is decoded per its
  api_key + api_version; legacy handlers continue to serve legacy
  versions.
- **Compact-string null sentinel**: encoded as varint 0 (length+1 = 0
  meaning null), distinct from empty string (varint 1).

## Success Metrics

- All in-scope APIs advertise current Kafka per-API maxima.
- Property-based codec tests pass at 100%.
- Modern librdkafka and modern Java client connect with default
  configuration in differential parity tests.

## Constraints and Assumptions

- The `kafka-protocol` Rust crate (or equivalent) provides flexible
  codec primitives; if not, heimq implements them. Selection is a
  Design-phase decision (ADR).

## Dependencies

- **Other features**: FEAT-001 (foundation). Blocks FEAT-002 (modern
  transactional APIs are flexible) and FEAT-005 (ecosystem tools rely
  on modern versions).
- **PRD requirements**: P0 #1 (wire-protocol compatibility — explicitly
  including modern flexible versions).

## Out of Scope

- KIP-482 tagged-field semantic interpretation beyond pass-through.
- Header-versioning quirks for APIs not in the in-scope surface.
- Compression-format upgrades (already covered by Phase 5 client
  surface).
