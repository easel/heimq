---
ddx:
  id: FEAT-006
  depends_on:
    - helix.prd
    - FEAT-001
  review:
    self_hash: e59d3b8965ebd35b4bbe9c5302f4218432ad83ec27691989dfd4c345ac2ae004
    deps:
      FEAT-001: 7133c264bc364ec4535c1d6b6187a90c9ba66d1fa3df30731ade260c2e092479
      helix.prd: 96f0479e307f2c240e8f47b69fff510164d0b9eda132abb22cc4a860932984fe
    reviewed_at: "2026-07-14T05:12:26Z"
---
# Feature Specification: FEAT-006 — Flexible-Version Kafka Protocol Support

**Feature ID**: FEAT-006
**Status**: Specified
**Priority**: P0
**Owner**: heimq core
**Covered PRD Subsystem(s)**: Wire protocol (FEAT-001 + FEAT-006)
**Covered PRD Requirements**: FR-4 (flexible-version portion of PRD P0 #1: modern per-API maxima)
**Cross-Subsystem Rationale**: None — single subsystem.

## Overview

heimq decodes and encodes flexible-version Kafka APIs — compact strings,
unsigned varints (varlong), and tagged fields — for every in-scope API
that has a flexible variant in current Kafka. Without this, modern
clients (librdkafka ≥ ~1.6, Java client ≥ 2.4, Kafka Connect, Flink,
Debezium, ksqlDB, Confluent serializers) and the modern transactional
APIs cannot interoperate with heimq. This unblocks FEAT-001 at modern
versions, FEAT-002 (transactions are flexible-only at the versions
clients negotiate), and FEAT-005 (ecosystem tools).

## Ideal Future State

Modern Kafka clients with default configuration (no
`api.version.request=false`, no manual version pin) connect to heimq,
negotiate the current Kafka per-API maxima for the in-scope semantic
surface, and exchange data without being forced onto a legacy downgrade
path — including the flexible-only modern transactional APIs.

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

- **FR-01** — A flexible-version codec provides compact strings, compact
  arrays, unsigned varints, signed zigzag varints, and tagged-fields
  blocks, and they round-trip correctly. Normative encoding rules per
  [CODEC-001 §Codec Primitives](../../02-design/contracts/CODEC-001-flexible.md#codec-primitives).
- **FR-02** — Every in-scope API key has its flexible-request and flexible-response
  variants implemented — flexible APIs use request header v2 and
  response header v1. Normative header wire layouts per
  [CODEC-001 §Request Header Decoding](../../02-design/contracts/CODEC-001-flexible.md#request-header-decoding)
  and [CODEC-001 §Response Header Encoding](../../02-design/contracts/CODEC-001-flexible.md#response-header-encoding).
- **FR-03** — `SUPPORTED_APIS` and `compute_supported_apis` are updated to advertise
  current Kafka per-API maxima for the in-scope semantic surface. The
  `API-001` matrix is updated to remove the "first flexible version"
  ceiling and replace it with the current spec maxima.
- **FR-04** — Tagged fields are forwarded through round-trip — heimq does not need
  to interpret unknown tagged fields, but it must not corrupt them on
  re-encode (or, where heimq generates the response, it must include
  the empty-tagged-fields trailer).
- **FR-05** — ApiVersions v3 is implemented: the flexible request body —
  including its client-software-reporting fields — is accepted and
  decoded without error, with the reported fields ignored. Normative
  field-level detail per
  [CODEC-001 §Request Header Decoding](../../02-design/contracts/CODEC-001-flexible.md#request-header-decoding)
  (Note on ApiVersions).

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
- **Compact-string null sentinel**: null and empty string are distinct
  values and must round-trip without conflation. Sentinel encoding per
  [CODEC-001 §Codec Primitives](../../02-design/contracts/CODEC-001-flexible.md#codec-primitives).

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
