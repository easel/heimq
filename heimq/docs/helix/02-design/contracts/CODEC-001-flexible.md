# CODEC-001: Flexible-Version Codec Module Surface

**Contract ID**: CODEC-001
**Type**: Protocol (Flexible-Version Codec)
**Status**: Draft v0.1.0
**Feature**: FEAT-006
**Depends on**: API-001-kafka-protocol.md, ADR-003

---

## Scope

This contract specifies the module surface for flexible-version Kafka protocol
support in heimq. It covers:

- How flexible-version codec primitives are provided (via `kafka-protocol` crate)
- The `is_flexible(api_key, api_version) -> bool` dispatch function
- How `decode_request` selects request header v0/v1 (legacy) vs v1/v2 (flexible)
- How `encode_response` appends the tagged-fields trailer for flexible responses
- The per-API flexible-version-min table

Out of scope: varint/compact-string implementation (delegated to `kafka-protocol`
crate per ADR-003), handler-level API semantics, FEAT-002 transactional APIs.

---

## Codec Primitives

Per ADR-003, heimq **does not implement** compact-string, compact-array,
unsigned-varint, signed-zigzag-varint, or tagged-fields codec primitives.
All such encoding and decoding is performed by the `kafka-protocol` crate
(v0.15, Apache-2.0), which is code-generated from Kafka's official
`protocol.json` schema files. The crate selects legacy or flexible wire format
automatically based on the API version passed to `encode`/`decode`.

The following table maps FEAT-006 primitive names to their crate-provided
equivalents for reference:

| Primitive | Kafka spec | Provided by |
|-----------|------------|-------------|
| `unsigned_varint` | base-128 unsigned varint (LEB128) | `kafka_protocol::protocol::buf` varint routines |
| `signed_varint` | zigzag-encoded signed varint | `kafka_protocol::protocol::buf` varint routines |
| `compact_string` | varint(len+1) + UTF-8 bytes; varint(0) = null | `kafka_protocol::protocol::StrBytes` encode/decode at flexible version |
| `compact_bytes` | varint(len+1) + raw bytes; varint(0) = null | `kafka_protocol::protocol::types::CompactBytes` |
| `compact_array` | varint(count+1) + elements; varint(0) = null | `kafka_protocol::protocol::types::CompactArray` |
| `tagged_fields` | varint(count) + tag/length/value tuples | `kafka_protocol::protocol::types::TagBuffer` |

heimq handlers call the crate's generated `encode`/`decode` methods with the
negotiated `api_version`; the crate emits the correct wire format for that
version. No heimq-owned codec primitive module is required.

---

## Flexible-Status Dispatch

### `is_flexible(api_key: i16, api_version: i16) -> bool`

**Location**: `src/protocol/mod.rs`

Returns `true` when the request/response pair for `(api_key, api_version)`
must use flexible framing. Implemented as a lookup against the
`FLEXIBLE_VERSION_MIN` table (see below). Used by `decode_request` and
`encode_response` to select the correct header format.

```rust
/// Returns true if api_version >= the flexible-version boundary for api_key.
pub fn is_flexible(api_key: i16, api_version: i16) -> bool
```

This function is pure and O(1) via a match arm over known API keys.

---

## Request Header Decoding

### Header Versions

| Header version | Wire layout | Used when |
|----------------|-------------|-----------|
| v0 (legacy) | `api_key` i16, `api_version` i16, `correlation_id` i32, `client_id` nullable-string (i16 len + bytes) | `is_flexible == false` |
| v1 (legacy + tagged fields) | same as v0, then tagged-fields block | not used by any in-scope API |
| v2 (flexible) | `api_key` i16, `api_version` i16, `correlation_id` i32, `client_id` compact-nullable-string (varint len+1 + bytes), tagged-fields block | `is_flexible == true` |

**Note on ApiVersions**: ApiVersions v3 uses a flexible request header (v2).
The request body also becomes flexible (compact types), including optional
`client_software_name` and `client_software_version` fields in tagged fields.
heimq must read and discard these fields without error; it need not store them.

### Updated `decode_request` interface

**Location**: `src/protocol/codec.rs`

The current `decode_request(data: &[u8])` decodes a legacy (header v0) request.
Under FEAT-006, `decode_request` becomes header-format-aware:

1. Always reads the first 8 bytes as `api_key` (i16), `api_version` (i16),
   `correlation_id` (i32) — these fields are identical across all header versions.
2. Calls `is_flexible(api_key, api_version)`.
3. **If legacy**: reads `client_id` as i16-length-prefixed nullable string
   (existing logic). No tagged-fields block follows the header.
4. **If flexible**: reads `client_id` as compact-nullable-string (unsigned
   varint for `len+1`, then `len` bytes; varint 0 = null), then reads and
   discards the tagged-fields block (varint count followed by tag/length/value
   tuples — count must be consumed even if zero).
5. Returns `(RequestHeader, remaining_body_bytes)` as before.

The `RequestHeader` struct is unchanged. The body bytes passed to each handler
contain only the API-specific payload; the header (including tagged fields) is
fully consumed by `decode_request`.

---

## Response Header Encoding

### Header Versions

| Header version | Wire layout | Used when |
|----------------|-------------|-----------|
| v0 (legacy) | `correlation_id` i32 | `is_flexible == false` |
| v1 (flexible) | `correlation_id` i32, tagged-fields block (varint 0 for empty) | `is_flexible == true` |

### Updated `encode_response` interface

**Location**: `src/protocol/codec.rs`

The current `encode_response` writes `correlation_id` then delegates body
encoding to the `kafka-protocol` crate. Under FEAT-006:

1. Writes `correlation_id` (i32) as before.
2. Calls `is_flexible(api_key, api_version)`.
3. **If flexible**: writes an empty tagged-fields trailer (single 0x00 byte
   for varint count = 0) after `correlation_id` and before the response body.
4. Delegates body encoding to the crate (which adds the per-message empty
   tagged-fields trailers required at flexible versions).

`encode_response` gains an `api_key: i16` parameter to support the
`is_flexible` check:

```rust
pub fn encode_response<R: Encodable>(
    correlation_id: i32,
    api_key: i16,
    api_version: i16,
    response: &R,
) -> std::io::Result<Bytes>
```

All callers (`router.rs` `encode_response_bytes`) forward the `api_key`
from the `RequestHeader`.

---

## Router Integration

**Location**: `src/protocol/router.rs`

`Router::route` calls `decode_request`, which already returns the parsed
`RequestHeader` (containing `api_key` and `api_version`). No change to the
routing dispatch table is required by this design.

`encode_response_bytes` in `Router` must pass `header.api_key` to the updated
`encode_response` signature:

```rust
fn encode_response_bytes<R: Encodable>(
    &self,
    header: &RequestHeader,
    response: &R,
) -> Result<Bytes> {
    encode_response(header.correlation_id, header.api_key, header.api_version, response)
        .map_err(Into::into)
}
```

No new routing paths are added. The existing handler closures continue to
receive `api_version` and a `body: &[u8]` slice; the `kafka-protocol` crate
handles flexible decode inside its generated `decode` functions for the
API-specific request type.

---

## Per-API Flexible-Version-Min Table

The `FLEXIBLE_VERSION_MIN` table maps API key to the first version that
requires flexible framing. Values are derived from Kafka's official protocol
schema and confirmed by ADR-003.

| API Key | Name | Flexible from (request header v2) |
|---------|------|-----------------------------------|
| 0 | Produce | v9 |
| 1 | Fetch | v12 |
| 2 | ListOffsets | v6 |
| 3 | Metadata | v9 |
| 8 | OffsetCommit | v8 |
| 9 | OffsetFetch | v6 |
| 10 | FindCoordinator | v3 |
| 11 | JoinGroup | v6 |
| 12 | Heartbeat | v4 |
| 13 | LeaveGroup | v4 |
| 14 | SyncGroup | v4 |
| 18 | ApiVersions | v3 (request header v2; legacy response header v0 retained per Kafka spec) |
| 19 | CreateTopics | v5 |
| 20 | DeleteTopics | v4 |
| 22 | InitProducerId | v2 (FEAT-002; always flexible at all planned versions) |
| 24 | AddPartitionsToTxn | v3 (FEAT-002) |
| 25 | AddOffsetsToTxn | v2 (FEAT-002) |
| 26 | EndTxn | v2 (FEAT-002) |
| 27 | WriteTxnMarkers | v1 (FEAT-002) |
| 28 | TxnOffsetCommit | v3 (FEAT-002) |
| 61 | DescribeProducers | v0 (FEAT-002; always flexible) |
| 65 | DescribeTransactions | v0 (FEAT-002; always flexible) |
| 66 | ListTransactions | v0 (FEAT-002; always flexible) |

**ApiVersions response header**: Kafka specifies that ApiVersions responses
always use response header v0 (no tagged fields), even when the request is
flexible v3. `encode_response` must special-case ApiVersions (api_key = 18)
to suppress the flexible response-header tagged-fields trailer.

For API keys not in this table, `is_flexible` returns `false`.

---

## Contract Validation

The following tests verify this contract at FEAT-006 implementation time:

1. **`test_is_flexible_boundary`**: For every API in the table, assert
   `is_flexible(api_key, flexible_min - 1) == false` and
   `is_flexible(api_key, flexible_min) == true`.
2. **`test_decode_request_flexible_header`**: Decode a hand-crafted flexible
   request (compact client_id + empty tagged-fields block); assert `client_id`
   is parsed correctly and body starts at the right offset.
3. **`test_decode_request_legacy_header`**: Existing tests remain passing.
4. **`test_encode_response_flexible_header`**: Encode a response at a
   flexible version; assert byte 8 (after 4-byte length + 4-byte
   correlation_id) is `0x00` (empty tagged-fields varint).
5. **`test_encode_response_apiversions_no_flexible_header`**: ApiVersions v3
   response does not have the tagged-fields trailer byte in the response header.
6. **`test_router_flexible_roundtrip`**: Route a flexible-version request
   end-to-end; assert correlation_id is preserved and response decodes cleanly.

Test locations: `src/protocol/codec.rs` (unit tests 1–5),
`src/protocol/router.rs` (integration test 6).

---

## References

- [ADR-003: Flexible-Version Codec Strategy](../../adr/003-flexible-version-codec-strategy.md)
- [API-001: Kafka Protocol Contract](API-001-kafka-protocol.md)
- [FEAT-006: Flexible-Version Protocol Specification](../../01-frame/features/FEAT-006-flexible-version-protocol.md)
- [`kafka-protocol` crate v0.15](https://crates.io/crates/kafka-protocol)
- KIP-482: Tagged Fields
- KIP-368: Allow brokers to SASL handshake without encoding changes (flexible header context)
