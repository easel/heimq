---
ddx:
  id: WIRE-001
  type: contract
  activity: design
  status: draft
  version: 0.1.0
  depends_on:
    - helix.prd
    - CODEC-001
    - API-001
  review:
    self_hash: de364438fe7c52ed13c6debb0f5a76ac7f4e7ee50ad3e3a27852c7dbe601eaa9
    deps:
      API-001: 85aaecdeafe47af73d0e287b3851f021313a23a96dc492d2646f78913978d5de
      CODEC-001: d83709237e2ca19409a81cc063bfba856b91e67d8231964561686d2ec602c919
      helix.prd: debc0a32007f0c42db51e82f47848c7b988c3f67f6f97171069170492f9b5b95
    reviewed_at: "2026-07-14T06:48:37Z"
---

# WIRE-001: Wire Scaffolding Contract

**Contract ID**: WIRE-001
**Type**: Protocol (Wire Scaffolding)
**Version**: 0.1.0
**Status**: Draft
**Depends on**: CODEC-001, API-001, helix.prd

---

## Purpose

Specifies the framing, connection lifecycle, and handler/registry seam for
`heimq-wire` — the normative boundary handler authors and wire-crate
implementors program against. Byte-level codec (varints, compact types, tagged
fields, header field layouts) is owned by CODEC-001. Per-API version matrices
are owned by API-001.

Provenance: behaviors normativized from
`niflheim-protocol/src/kafka/{framing.rs,connection.rs}` (same owner), adapted
for heimq-wire's configurable defaults and typed handler seam.

---

## Scope and Boundaries

In scope: frame structure, connection lifecycle, malformed-request policy,
handler/registry contract (async shape, typed-vs-raw, deferred acks,
backpressure mapping), response encoding ownership, SASL/TLS capability gate,
intentional deltas from pre-split heimq.

Out of scope: varint/compact-string/tagged-field codec (CODEC-001); per-API
version ranges and advertisement (API-001); handler business logic.

Owning crate: `heimq-wire`

---

## Normative Surface

### 1. Framing

A Kafka frame MUST consist of a 4-byte big-endian signed integer (`i32`) length
prefix followed by exactly that many payload bytes.

| Field | Type | Rules |
|---|---|---|
| `length` | `i32` big-endian | MUST be non-negative; see Size Cap below |
| `payload` | `[u8; length]` | Request header + body per CODEC-001 |

**Size cap**: The gateway MUST enforce a configurable `max_frame_bytes` limit.
The normative default is **100 MiB** (104_857_600 bytes), matching Kafka's
`socket.request.max.bytes` default. The niflheim reference ships 16 MiB as its
own configured value — that is a deployment knob, not this contract's default.

**Rejection rule**: A frame with a negative declared length OR a declared length
exceeding `max_frame_bytes` MUST be treated as a connection-fatal codec error;
the connection MUST close immediately without sending a response frame.

**Mid-frame EOF**: Connection closes immediately; no response is sent.

Header and body wire encoding is fully delegated to CODEC-001. Response framing
follows the same length-prefix rule; the gateway writes the prefix after
encoding the response header and body.

---

### 2. Connection Lifecycle

| Property | Value |
|---|---|
| Reader/writer split | Reader task reads frames and forwards via channel; writer task handles dispatch and sends responses |
| Channel type | `mpsc` bounded channel, depth default **64** |
| Response ordering | FIFO per connection; responses MUST be sent in the order requests were received |
| Reader-on-writer-exit | When the writer exits (for any reason), the reader task MUST be aborted; the underlying stream MUST be fully dropped before the connection function returns |
| TCP option | `TCP_NODELAY` MUST be set on each accepted connection |

The reader task MUST NOT call handler registry functions or mutate connection
state. All dispatch and state mutation occurs on the writer side. Queued frames
MUST NOT be discarded when the writer exits — `recv()` drains the channel
before returning `None`.

---

### 3. Malformed-Request Policy

**Peeking**: Before dispatch, the gateway MUST peek the first 8 bytes of each
frame to extract `api_key` (i16), `api_version` (i16), and `correlation_id`
(i32). If fewer than 8 bytes are available, no error frame is sent and the
connection closes.

**Typed error frame**: On dispatch or header decode failure, the gateway SHOULD
send a best-effort typed error response using the peeked `api_key` and
`correlation_id`. If the api_key is not covered by the error-frame table, the
connection closes without a response.

**Consecutive-error limit**: The gateway MUST track consecutive errors (errors
where a response was successfully sent). Default limit: **10**. When the limit
is reached, the connection MUST close after the final error response. A
successful response resets the counter to zero.

**Mid-frame EOF**: Closes the connection immediately (covered in §1).

---

### 4. Handler and Registry Contract

This is the primary seam. Handler authors implement against this surface;
`heimq-wire` enforces it.

#### 4.1 Typed-vs-Raw Body Policy

Handlers registered for a specific `api_key` MUST receive a
**`kafka-protocol`-typed decoded request** at the negotiated `api_version`.
Decoding is performed by the gateway before the handler is invoked. Raw-bytes
registration (receiving `&[u8]` body without decoding) is the escape hatch for
API keys not yet covered by a typed handler; raw handlers MUST be explicitly
registered as such in the registry.

#### 4.2 Handler Signature

```rust
async fn handle(
    header: &RequestHeader,  // api_key, api_version, correlation_id, client_id
    request: TypedOrRaw,     // decoded typed request OR raw body bytes
    ctx: &RequestContext,
) -> Result<HandlerResponse, HandlerError>
```

`RequestHeader` fields are as parsed by CODEC-001.

#### 4.3 Per-Request Context

| Field | Type | Notes |
|---|---|---|
| `principal` | `Option<AuthenticatedPrincipal>` | `None` when gate is off or pre-auth |
| `cancellation_token` | `CancellationToken` | Cancelled on connection close |

#### 4.4 Handler Response and Deferred Acks

A handler MAY return either:
1. A **typed response** (encoded by the gateway; see §5).
2. A **deferred ack future** (`Future<Output = TypedResponse>`) that the
   writer awaits while the reader continues accepting and queueing later frames
   (ING-6). The gateway MUST preserve per-connection request order: later
   request handlers MUST NOT observe state before earlier no-reply Produce side
   effects complete, and responses MUST be sent in FIFO order.

#### 4.5 Backpressure Mapping

When a handler returns `HandlerError::Overload`, the gateway MUST map it per:

| API Context | Kafka Error Code | Code |
|---|---|---|
| Produce | `KAFKA_STORAGE_ERROR` | 56 |
| Fetch | `KAFKA_STORAGE_ERROR` | 56 |
| All other APIs | `UNKNOWN_SERVER_ERROR` | 10 |

Handlers MUST NOT embed raw error codes; they return `HandlerError::Overload`
and the gateway applies this table.

---

### 5. Response Encoding Ownership

The **gateway** (not handlers) MUST:
1. Select response header version via `kafka_protocol::messages::ApiKey::response_header_version(api_version)` (the retired `FLEXIBLE_VERSION_MIN` table is replaced by this crate call, per CODEC-001).
2. Encode the response header per CODEC-001 §Response Header Encoding.
3. Encode the typed response body via `kafka-protocol` crate.
4. Prepend the 4-byte big-endian length prefix.

Handlers return typed response structs or deferred futures. They MUST NOT write
frames or length prefixes.

---

### 6. SASL/TLS Capability Gate

The SASL/TLS feature is behind a **capability gate**, default **OFF**.

**When OFF** (heimq distribution default): SaslHandshake (17) and
SaslAuthenticate (36) MUST NOT be advertised. No SASL state machine runs;
all connections are pre-authenticated (`principal = None`; all APIs accessible).
TLS acceptor is not installed.

**When ON**: APIs 17 and 36 are advertised. After TCP accept, SASL MUST
complete before any API other than ApiVersions (18) is served. All other APIs
sent pre-auth MUST be rejected with `ILLEGAL_SASL_STATE` (34). Non-PLAIN
mechanisms MUST return `UNSUPPORTED_SASL_MECHANISM` (33). TLS acceptor, when
configured, wraps the stream before the SASL exchange. Provider-init MUST be
idempotent (multiple calls safe).

The heimq distribution ships with the gate off.

---

### 7. Intentional Deltas from Pre-Split heimq

Each delta is a behavioral change from the pre-restructure codebase and carries
a mandatory contract-test obligation.

| Delta | Previous behavior | New behavior | Test obligation |
|---|---|---|---|
| Frame-size cap | No enforcement | Max 100 MiB; oversized → connection close | `test_frame_size_cap_enforced` |
| Typed malformed-request error frames | Error swallowed; connection closed silently | Best-effort typed error frame sent before close (or error-counter path) | `test_malformed_request_typed_error_frame` |
| Consecutive-error close | None | Connection closes after 10 consecutive errors | `test_consecutive_error_limit` |

---

## Precedence and Compatibility

- **Versioning**: Versioned independently of API-001. Breaking changes to handler signature or context shape require a version bump.
- **Ordering**: FIFO response ordering is a hard invariant; deferred acks MUST NOT reorder responses.
- **Backward compatibility**: Breaking changes to `RequestContext` or `HandlerResponse` require ADR amendment.
- **Codec precedence**: In any conflict with CODEC-001, CODEC-001 governs byte-level format; this contract governs connection-level policy.

---

## Error Semantics

| Condition | Error / Outcome | Retry | Recovery |
|---|---|---|---|
| Negative declared frame length | Connection close, no response | No | Client reconnects |
| Declared length > `max_frame_bytes` | Connection close, no response | No | Client reconnects |
| Mid-frame TCP EOF | Connection close, no response | Client-controlled | Client reconnects |
| Header decode failure, api_key peekable | Typed error frame (error code 10); consecutive-error counter incremented | Yes, up to limit | Connection survives if under limit |
| Header decode failure, api_key not peekable | Connection close | No | Client reconnects |
| Unsupported api_key (no handler) | Typed error frame (UNKNOWN_SERVER_ERROR, 10) | Yes | Counter incremented |
| Handler overload signal | Typed error frame per backpressure table | Client-controlled | Counter NOT incremented (successful send) |
| Consecutive-error limit reached | Close after final error frame | No | Client reconnects |
| API sent pre-auth (gate ON) | `ILLEGAL_SASL_STATE` (34) | No | Client must complete SASL first |
| SASL non-PLAIN mechanism | `UNSUPPORTED_SASL_MECHANISM` (33) | No | Client must use PLAIN |

---

## Examples

```text
Frame wire layout:
  [length i32 BE][api_key i16][api_version i16][correlation_id i32]
  [client_id: i16-len-prefix + UTF-8 bytes (nullable)]
  [tagged_fields: if flexible version per CODEC-001]
  [request body per API-001]

Consecutive-error sequence (limit=10):
  req 1: Produce v6 → ProduceResponse(error=35), counter=1
  ...
  req 10: Produce v6 → ProduceResponse(error=35), counter=10 → close

Deferred ack (ING-6):
  gateway receives Produce → dispatches, gets Future<ProduceResponse>
  reader queues next frame (no block)
  future resolves → gateway encodes response, sends in FIFO order
```

---

## Validation Checklist

- [x] §1 Framing: frame with declared length > 100 MiB causes connection close, no response (`test_frame_size_cap_enforced`).
- [x] §2 Lifecycle: FIFO ordering preserved when N requests queue concurrently in the reader→writer channel (`test_pipelined_requests_fifo_order`).
- [x] §3 Malformed request: error frame helpers verified; typed error frame sent on routing failure (`test_make_error_frame_structure`, `test_peek_correlation_id_happy_path`, `test_malformed_request_typed_error_frame`).
- [x] §3 Consecutive-error limit: exactly 10 consecutive errors close the connection; fewer do not (`test_consecutive_error_limit`).
- [ ] §4 Typed handler: typed handler receives decoded request struct; gateway encodes response header and length prefix (`test_typed_handler_receives_decoded_request`). Deferred to Slice 3 — current router dispatches typed handlers; end-to-end decode+encode path verified by `route_supported_apis_and_unsupported` suite.
- [x] §4 Deferred ack: handler-level futures are awaited by the writer while the reader can continue queueing frames; FIFO/state ordering is covered by `test_pipelined_requests_fifo_order` and `contract_produce_acks0_no_response_sent`, and WAL-shape backend shape is covered by `test_niflheim_shape_wal_adapter`.
- [ ] §4 Backpressure: `HandlerError::Overload` mapping deferred to Slice 3 (no current handler returns Overload; error path covered by `test_handle_connection_warn_on_route_error`).
- [x] §5 Encoding ownership: gateway adds length prefix and response header; handler returns typed struct. Verified by `encode_response` unit tests and the full router test suite.
- [x] §6 Gate OFF: SaslHandshake/SaslAuthenticate absent from ApiVersions response (`test_sasl_gate_off_no_advertise`).
- [ ] §6 Gate ON pre-auth: SASL state machine not implemented (gate is OFF in all deployments). Deferred to Slice 5 (SASL full implementation).

---

## References

- [CODEC-001: Flexible-Version Codec Module Surface](CODEC-001-flexible.md)
- [API-001: Kafka Protocol Contract](API-001-kafka-protocol.md)
- [IP-001: Implementation Plan §Slice 0](../../04-build/implementation-plan.md)
- Niflheim provenance: `niflheim-protocol/src/kafka/framing.rs`,
  `niflheim-protocol/src/kafka/connection.rs` (same owner; behaviors
  normativized above, not copied verbatim)
