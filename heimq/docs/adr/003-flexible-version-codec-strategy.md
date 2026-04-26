# ADR-003: Flexible-Version Kafka Protocol Codec Strategy

**Status**: Accepted
**Date**: 2026-04-26
**Feature**: FEAT-006 (Flexible-Version Kafka Protocol Support)

## Context

heimq currently caps every in-scope Kafka API strictly below its flexible-version boundary. Flexible versions (KIP-482) use compact strings (varint length+1), compact arrays, unsigned varints, and tagged-field blocks. Without flexible-version support, modern clients (librdkafka ≥ ~1.6, Java client ≥ 2.4, Kafka Connect, Flink, Debezium) cannot use current API versions, blocking FEAT-001 modern compatibility, FEAT-002 transactional APIs, and FEAT-005 ecosystem integrations.

The decision point: use a third-party crate that already implements flexible encoding, or implement the flexible codec primitives in-house under `src/protocol/codec/flexible.rs`.

### Dependency Landscape

| Option | Source | Flexible support | License | Downloads/mo | Notes |
|--------|--------|------------------|---------|--------------|-------|
| `kafka-protocol` v0.15 (already vendored) | tychedelia/kafka-protocol-rs | Yes — code-generated from Kafka schema JSON | Apache-2.0 | ~25K | Already in `Cargo.toml`; generates encode/decode per API version |
| `tansu-kafka-codec` | tansu-io/tansu | Partial | Apache-2.0 | <1K | Overlapping scope with tansu broker; uncertain independent maintenance |
| `ozzy` | small crate | Unknown/incomplete | Unclear | <100 | Experimental; no production evidence |
| `kpro` | Rust Kafka client | Client-oriented | MIT | <500 | Client framing, not server-side codec |
| In-house | heimq | To be written | MIT | N/A | ~400 lines; full control, full maintenance burden |

## Decision

**Use the existing `kafka-protocol` crate (v0.15) for flexible-version codec support. Do not add alternative crates. Do not implement codec primitives from scratch.**

The crate is already in the dependency tree. Its `encode`/`decode` traits emit flexible framing automatically for API versions at or above each API's flexible boundary — compact strings, compact arrays, varints, and tagged-field blocks are handled by generated code derived from Kafka's own schema JSON files. heimq's handlers call the generated `encode`/`decode` methods with the negotiated API version; the crate selects legacy or flexible format transparently.

### Rationale

1. **Zero additional dependency**: `kafka-protocol` is already vendored at v0.15. Pulling in a second protocol crate adds supply-chain surface with no benefit.
2. **Schema-authoritative**: The crate is code-generated from Apache Kafka's official `protocol.json` schema files — the same source Kafka's own Java client generator uses. The flexible boundary per API is correct by construction, not hand-maintained.
3. **Proven flexible support**: The crate's encode/decode paths have been exercised in production Kafka proxy and gateway projects at flexible versions.
4. **No in-house maintenance burden**: Varint, compact-string, and tagged-field code is subtle (null-sentinel encoding, zigzag signed varints, unknown-tag forwarding). Bugs here break all clients silently. Owning that code means owning all future Kafka schema changes that touch flexible framing.
5. **License**: Apache-2.0 is compatible with heimq's MIT license.

## Alternatives Considered

### tansu-kafka-codec (Rejected)
Designed for the tansu broker project; not clearly factored as a standalone codec library. Low download count, uncertain maintenance trajectory outside the tansu umbrella. Adds a second protocol dependency without offsetting `kafka-protocol`.

### ozzy / kpro (Rejected)
Experimental or client-oriented crates with no evidence of server-side flexible encoding in production. Unacceptable risk for a P0 feature.

### In-house implementation (Rejected)
Compact strings, unsigned varints, zigzag-signed varints, nullable-string null-sentinel (varint 0 vs 1), tagged-fields TLV blocks, and empty-tagged-fields trailers on every flexible response are ~400 lines of byte-manipulation code that must stay synchronized with the Kafka spec. The `kafka-protocol` crate already does this correctly. Building and owning it adds maintenance cost with no strategic advantage.

## Consequences

### Positive
- FEAT-006 implementation targets the generated `encode`/`decode` APIs directly; no new codec primitives need to be written.
- Future Kafka schema upgrades can be absorbed by bumping `kafka-protocol`; heimq handlers remain unchanged.
- `src/protocol/codec/flexible.rs` (per the bead's original description) is **not needed** as a new file — the crate provides the primitives. The module can be a thin dispatch shim or omitted entirely if the crate's encode/decode is invoked directly from existing handlers.

### Negative
- heimq is coupled to `kafka-protocol`'s API surface for codec internals. A breaking version bump requires updating call sites.
- The crate's generated types may not perfectly match heimq's internal request/response structs; adapter shims are likely required per handler.

### Neutral
- All legacy handler logic (sub-flexible-boundary versions) continues to function unchanged; the crate handles both paths.

## Per-API Impact: Legacy Handler vs. Flexible Upgrade

The table below records every in-scope API, the current legacy maximum advertised by `SUPPORTED_APIS` in `src/protocol/mod.rs`, the Kafka flexible-version boundary, and the planned new maximum after FEAT-006. APIs marked **Planned (FEAT-002)** are not yet implemented; their target maxima are recorded for design completeness.

| API Key | Name | Current max (legacy) | Flexible from | Target max (FEAT-006) | Handler status |
|---------|------|---------------------|---------------|----------------------|----------------|
| 0 | Produce | v8 | v9 | v11 | Legacy retained; flexible variant added |
| 1 | Fetch | v11 | v12 | v17 | Legacy retained; flexible variant added |
| 2 | ListOffsets | v5 | v6 | v9 | Legacy retained; flexible variant added |
| 3 | Metadata | v8 | v9 | v12 | Legacy retained; flexible variant added |
| 8 | OffsetCommit | v7 | v8 | v9 | Legacy retained; flexible variant added |
| 9 | OffsetFetch | v5 | v6 | v9 | Legacy retained; flexible variant added |
| 10 | FindCoordinator | v2 | v3 | v6 | Legacy retained; flexible variant added |
| 11 | JoinGroup | v5 | v6 | v9 | Legacy retained; flexible variant added |
| 12 | Heartbeat | v3 | v4 | v4 | Legacy retained; flexible variant added |
| 13 | LeaveGroup | v3 | v4 | v5 | Legacy retained; flexible variant added |
| 14 | SyncGroup | v3 | v4 | v5 | Legacy retained; flexible variant added |
| 18 | ApiVersions | v2 | v3 (req only) | v3 | Legacy retained; flexible request header + client_software fields added |
| 19 | CreateTopics | v4 | v5 | v7 | Legacy retained; flexible variant added |
| 20 | DeleteTopics | v3 | v4 | v6 | Legacy retained; flexible variant added |
| 22 | InitProducerId | — (not yet) | v2 | v5 | Planned (FEAT-002); implements at flexible target directly |
| 24 | AddPartitionsToTxn | — (not yet) | v3 | v5 | Planned (FEAT-002); implements at flexible target directly |
| 25 | AddOffsetsToTxn | — (not yet) | v2 | v4 | Planned (FEAT-002); implements at flexible target directly |
| 26 | EndTxn | — (not yet) | v2 | v4 | Planned (FEAT-002); implements at flexible target directly |
| 27 | WriteTxnMarkers | — (not yet) | v1 | v1 | Planned (FEAT-002); flexible-only at all supported versions |
| 28 | TxnOffsetCommit | — (not yet) | v3 | v4 | Planned (FEAT-002); implements at flexible target directly |
| 61 | DescribeProducers | — (not yet) | v0 | v0 | Planned (FEAT-002); always flexible |
| 65 | DescribeTransactions | — (not yet) | v0 | v0 | Planned (FEAT-002); always flexible |
| 66 | ListTransactions | — (not yet) | v0 | v1 | Planned (FEAT-002); always flexible |

### Notes on target maxima
- Targets are derived from the API-001 contract (v0.2.0) and the Kafka protocol spec as of April 2026.
- Final maxima must be confirmed against the `kafka-protocol` v0.15 schema at FEAT-006 implementation time; the crate's generated types set the practical ceiling.
- FEAT-002 APIs are listed for completeness; their implementation is gated on FEAT-006 landing first.

## References

- [FEAT-006 specification](../helix/01-frame/features/FEAT-006-flexible-version-protocol.md)
- [API-001 Kafka Protocol Contract](../helix/02-design/contracts/API-001-kafka-protocol.md)
- [`kafka-protocol` crate](https://crates.io/crates/kafka-protocol) — Apache Kafka wire protocol in Rust
- KIP-482: Tagged fields and flexible versions
- `src/protocol/mod.rs` — `SUPPORTED_APIS` static table
