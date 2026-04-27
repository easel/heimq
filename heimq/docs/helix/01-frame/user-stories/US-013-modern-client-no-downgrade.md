# US-013 — Modern Kafka client connects without forcing legacy versions

**Feature**: FEAT-006
**Priority**: P0

## Story

As a developer using a current-release Kafka client (librdkafka
≥ ~1.6, Java client ≥ 2.4) with default configuration,
I want my client to negotiate flexible-version APIs against heimq,
so that I can use modern features (transactions, modern Metadata /
Fetch / ApiVersions) without setting `api.version.request=false`,
pinning a legacy version, or otherwise diverging from how I'd
configure the client against Kafka or Redpanda.

## Acceptance Criteria

- An rdkafka producer + consumer with default configuration connect
  to heimq and complete a produce/fetch round-trip.
- ApiVersions v3 is served (flexible request and response).
- A workload exercising flexible-only API versions (Metadata v9+,
  Fetch v12+, Produce v9+, InitProducerId v2+, EndTxn v3+) succeeds.
- Differential parity vs Redpanda (FEAT-003) reports zero diffs at
  flexible versions for in-scope APIs.
