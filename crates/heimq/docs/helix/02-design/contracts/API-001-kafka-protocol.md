---
ddx:
  id: API-001
  type: contract
  activity: design
  status: draft
  depends_on:
    - helix.prd
    - FEAT-001
    - FEAT-002
    - FEAT-006
    - ADR-003
  review:
    self_hash: d85884155fba0fe46b242b6bdc939b612c55ea033bc2d68b6cf0c50c114d18b4
    deps:
      ADR-003: c06761cf28323ddb6162d309cfcfe4db1037f915a2c05a716ad4262222193145
      FEAT-001: 7133c264bc364ec4535c1d6b6187a90c9ba66d1fa3df30731ade260c2e092479
      FEAT-002: 164350929c7bbc09a589f3cd1a80b685e88cce1054445fe5373aec566464636f
      FEAT-006: e59d3b8965ebd35b4bbe9c5302f4218432ad83ec27691989dfd4c345ac2ae004
      helix.prd: 236574e8f31d3847bb8269d538fe07d0a47376aa7d7e75c30dca783e479ad4ab
    reviewed_at: "2026-06-22T21:30:26Z"
---

# API Contract: Kafka Wire Protocol [heimq]

**Contract ID**: API-001
**Features**: FEAT-001 (Wire-Protocol Compatibility), FEAT-002 (Core Kafka Semantics), FEAT-006 (Flexible-Version Protocol)
**Type**: Protocol (Kafka TCP Wire)
**Status**: Draft
**Version**: 0.2.0
**Source**: Kafka Protocol Guide (`https://kafka.apache.org/protocol/`)

## Purpose

*This contract defines heimq's Kafka wire-protocol surface area, supported versions, exclusions, and test coverage expectations.*

## Scope and Boundaries

- Single-node Kafka-compatible broker focused on transport speed over durability.
- No distributed consensus, replication, or controller responsibilities.
- No security or ACLs in scope for this contract.
- **Transactions and idempotent producers are in scope** per FEAT-002. The single-coordinator implementation operates without a replicated transaction log; loss of in-memory transactional/producer-id state across restart is acceptable per PRD non-goal #1, with clients re-initializing per Kafka spec for a fresh broker.

## Normative Surface

### Transport and Framing

- **Transport**: TCP, binary Kafka protocol.
- **Framing**: 4-byte big-endian message length, followed by request header + body.
- **Request header (non-flexible)**: api_key, api_version, correlation_id, client_id.
- **Request header (flexible v2)**: api_key, api_version, correlation_id, client_id, tagged_fields block (compact types/varints per FEAT-006; see ADR-003).
- **Response header (non-flexible / v0)**: correlation_id.
- **Response header (flexible v1)**: correlation_id, tagged_fields block (compact encoding per FEAT-006; see ADR-003). Exception: ApiVersions responses always use v0 even for flexible requests (KIP-482 quirk).
- **Flexible versions**: In scope per FEAT-006 (compact strings, unsigned varints, tagged fields; codec via `kafka-protocol` crate, see ADR-003).

### Version Policy

- **Flexible versions are in scope (FEAT-006).** Per-API `max_version`
  targets the current Kafka spec for the in-scope semantic surface.
  Flexible-version APIs use compact strings, unsigned varints, and
  tagged fields; the codec is delivered via the `kafka-protocol` crate's
  generated encode/decode (see ADR-003); a thin dispatch shim under
  `src/protocol/codec/` may or may not be introduced during FEAT-006
  implementation.
- The static version table is `SUPPORTED_APIS` in `src/protocol/mod.rs`;
  the matrix below mirrors it. Any change to advertised versions must
  update both.
- **Per-API maxima** (pinned by FEAT-006 FR-03):
  Produce v11, Fetch v12 (v13+ drops topic name — capped), ListOffsets v9,
  Metadata v12, OffsetCommit v9, OffsetFetch v9 (v8+ uses groups structure),
  FindCoordinator v6, JoinGroup v9, Heartbeat v4, LeaveGroup v5, SyncGroup v5,
  ApiVersions v3, CreateTopics v7, DeleteTopics v6, InitProducerId v5,
  AddPartitionsToTxn v5, AddOffsetsToTxn v4, EndTxn v4, WriteTxnMarkers v1,
  TxnOffsetCommit v4. The matrix below mirrors `SUPPORTED_APIS` in
  `src/protocol/mod.rs`; any change to advertised versions must update both.
- **Capability-derived advertisement**: The ApiVersions response is not a verbatim copy of `SUPPORTED_APIS`. At runtime, `compute_supported_apis` (`src/protocol/mod.rs`) intersects the static table with the per-API `CapabilityGate` against each backend's descriptor (`BackendCapabilities`, `OffsetStoreCapabilities`, `GroupCoordinatorCapabilities`). APIs whose required backend is absent (e.g. no group coordinator) are filtered out before the response is encoded, so heimq advertises only what its currently configured backends can actually serve. Gating is per-API, not a global meet — a backend that lacks compaction does not lose unrelated APIs.

### Support Matrix (Kafka API Keys)

Kafka API keys listed here follow the current Apache Kafka protocol spec.

Status values:
- **Supported**: Implemented in heimq.
- **Planned (FEAT-002)**: In scope per FEAT-002 (transactions / idempotent producer); not yet implemented. Tracked as upcoming work, not excluded.
- **Excluded**: Out of scope for single-node/no-durability design (reason noted).

Reason codes (Exclusions):
- **R1**: Requires multi-broker/controller/replication/KRaft.
- **R2**: Requires security/authentication/authorization.
- **R4**: Admin/control-plane APIs not yet in scope.
- **R5**: Telemetry/share-group/new-group APIs beyond current scope.

| API Key | Name | heimq Status | Supported Versions | Tests | Notes |
| --- | --- | --- | --- | --- | --- |
| 0 | Produce | Supported | 0-11 | `src/handler/tests.rs`; `tests/contract.rs`; `tests/integration.rs` | In-memory append only |
| 1 | Fetch | Supported | 0-12 | `src/handler/tests.rs`; `tests/contract.rs`; `tests/integration.rs` | In-memory read only; capped at v12 (v13+ uses topic_id UUID, name-based lookup unsupported) |
| 2 | ListOffsets | Supported | 0-9 | `src/handler/tests.rs`; `tests/contract.rs` | Timestamp lookups simplified |
| 3 | Metadata | Supported | 0-12 | `src/handler/tests.rs`; `tests/contract.rs`; `tests/integration.rs` | Single broker only |
| 8 | OffsetCommit | Supported | 0-9 | `src/handler/tests.rs`; `tests/contract.rs` | In-memory offsets |
| 9 | OffsetFetch | Supported | 0-9 | `src/handler/tests.rs`; `tests/contract.rs`; `tests/integration.rs` | In-memory offsets; v8+ uses groups response structure |
| 10 | FindCoordinator | Supported | 0-6 | `src/handler/tests.rs` | Single coordinator (self) |
| 11 | JoinGroup | Supported | 0-9 | `src/handler/tests.rs` | Simplified group state |
| 12 | Heartbeat | Supported | 0-4 | `src/handler/tests.rs` | Simplified liveness |
| 13 | LeaveGroup | Supported | 0-5 | `src/handler/tests.rs` | Member removal only |
| 14 | SyncGroup | Supported | 0-5 | `src/handler/tests.rs` | Simplified assignment |
| 15 | DescribeGroups | Excluded (R4) | N/A | N/A | Out of scope for current single-node implementation |
| 16 | ListGroups | Excluded (R4) | N/A | N/A | Out of scope for current single-node implementation |
| 17 | SaslHandshake | Excluded (R2) | N/A | N/A | Out of scope for current single-node implementation |
| 18 | ApiVersions | Supported | 0-3 | `src/handler/tests.rs`; `tests/contract.rs`; `src/protocol/router.rs` | Version negotiation only |
| 19 | CreateTopics | Supported | 0-7 | `src/handler/tests.rs`; `tests/contract.rs` | No config validation |
| 20 | DeleteTopics | Supported | 0-6 | `src/handler/tests.rs`; `tests/contract.rs` | Best-effort delete |
| 21 | DeleteRecords | Excluded (R4) | N/A | N/A | Out of scope for current single-node implementation |
| 22 | InitProducerId | Supported (FEAT-002) | 0-5 | `tests/contract.rs` | Required for idempotent producer + transactions |
| 23 | OffsetForLeaderEpoch | Excluded (R1) | N/A | N/A | Out of scope for current single-node implementation |
| 24 | AddPartitionsToTxn | Supported (FEAT-002) | 0-5 | `tests/contract.rs` | Single-coordinator transaction state machine |
| 25 | AddOffsetsToTxn | Supported (FEAT-002) | 0-4 | `tests/contract.rs` | Single-coordinator transaction state machine |
| 26 | EndTxn | Supported (FEAT-002) | 0-4 | `tests/contract.rs` | Single-coordinator transaction state machine |
| 27 | WriteTxnMarkers | Supported (FEAT-002) | 0-1 | `tests/contract.rs` | Control batches drive read_committed visibility |
| 28 | TxnOffsetCommit | Supported (FEAT-002) | 0-4 | `tests/contract.rs` | EOS consumer offset commits |
| 29 | DescribeAcls | Excluded (R2) | N/A | N/A | Out of scope for current single-node implementation |
| 30 | CreateAcls | Excluded (R2) | N/A | N/A | Out of scope for current single-node implementation |
| 31 | DeleteAcls | Excluded (R2) | N/A | N/A | Out of scope for current single-node implementation |
| 32 | DescribeConfigs | Excluded (R4) | N/A | N/A | Out of scope for current single-node implementation |
| 33 | AlterConfigs | Excluded (R4) | N/A | N/A | Out of scope for current single-node implementation |
| 34 | AlterReplicaLogDirs | Excluded (R1) | N/A | N/A | Out of scope for current single-node implementation |
| 35 | DescribeLogDirs | Excluded (R1) | N/A | N/A | Out of scope for current single-node implementation |
| 36 | SaslAuthenticate | Excluded (R2) | N/A | N/A | Out of scope for current single-node implementation |
| 37 | CreatePartitions | Excluded (R4) | N/A | N/A | Out of scope for current single-node implementation |
| 38 | CreateDelegationToken | Excluded (R2) | N/A | N/A | Out of scope for current single-node implementation |
| 39 | RenewDelegationToken | Excluded (R2) | N/A | N/A | Out of scope for current single-node implementation |
| 40 | ExpireDelegationToken | Excluded (R2) | N/A | N/A | Out of scope for current single-node implementation |
| 41 | DescribeDelegationToken | Excluded (R2) | N/A | N/A | Out of scope for current single-node implementation |
| 42 | DeleteGroups | Excluded (R4) | N/A | N/A | Out of scope for current single-node implementation |
| 43 | ElectLeaders | Excluded (R1) | N/A | N/A | Out of scope for current single-node implementation |
| 44 | IncrementalAlterConfigs | Excluded (R4) | N/A | N/A | Out of scope for current single-node implementation |
| 45 | AlterPartitionReassignments | Excluded (R1) | N/A | N/A | Out of scope for current single-node implementation |
| 46 | ListPartitionReassignments | Excluded (R1) | N/A | N/A | Out of scope for current single-node implementation |
| 47 | OffsetDelete | Excluded (R4) | N/A | N/A | Out of scope for current single-node implementation |
| 48 | DescribeClientQuotas | Excluded (R4) | N/A | N/A | Out of scope for current single-node implementation |
| 49 | AlterClientQuotas | Excluded (R4) | N/A | N/A | Out of scope for current single-node implementation |
| 50 | DescribeUserScramCredentials | Excluded (R2) | N/A | N/A | Out of scope for current single-node implementation |
| 51 | AlterUserScramCredentials | Excluded (R2) | N/A | N/A | Out of scope for current single-node implementation |
| 55 | DescribeQuorum | Excluded (R1) | N/A | N/A | Out of scope for current single-node implementation |
| 57 | UpdateFeatures | Excluded (R1) | N/A | N/A | Out of scope for current single-node implementation |
| 60 | DescribeCluster | Excluded (R4) | N/A | N/A | Out of scope for current single-node implementation |
| 61 | DescribeProducers | Planned (FEAT-002) | TBD | Pending (`tests/contract/transactions.rs`) | Producer-id state introspection |
| 64 | UnregisterBroker | Excluded (R1) | N/A | N/A | Out of scope for current single-node implementation |
| 65 | DescribeTransactions | Planned (FEAT-002) | TBD | Pending (`tests/contract/transactions.rs`) | Transaction-state introspection |
| 66 | ListTransactions | Planned (FEAT-002) | TBD | Pending (`tests/contract/transactions.rs`) | Transaction-state introspection |
| 68 | ConsumerGroupHeartbeat | Excluded (R5) | N/A | N/A | Out of scope for current single-node implementation |
| 69 | ConsumerGroupDescribe | Excluded (R5) | N/A | N/A | Out of scope for current single-node implementation |
| 71 | GetTelemetrySubscriptions | Excluded (R5) | N/A | N/A | Out of scope for current single-node implementation |
| 72 | PushTelemetry | Excluded (R5) | N/A | N/A | Out of scope for current single-node implementation |
| 74 | ListConfigResources | Excluded (R4) | N/A | N/A | Out of scope for current single-node implementation |
| 75 | DescribeTopicPartitions | Excluded (R4) | N/A | N/A | Out of scope for current single-node implementation |
| 76 | ShareGroupHeartbeat | Excluded (R5) | N/A | N/A | Out of scope for current single-node implementation |
| 77 | ShareGroupDescribe | Excluded (R5) | N/A | N/A | Out of scope for current single-node implementation |
| 78 | ShareFetch | Excluded (R5) | N/A | N/A | Out of scope for current single-node implementation |
| 79 | ShareAcknowledge | Excluded (R5) | N/A | N/A | Out of scope for current single-node implementation |
| 80 | AddRaftVoter | Excluded (R1) | N/A | N/A | Out of scope for current single-node implementation |
| 81 | RemoveRaftVoter | Excluded (R1) | N/A | N/A | Out of scope for current single-node implementation |
| 83 | InitializeShareGroupState | Excluded (R5) | N/A | N/A | Out of scope for current single-node implementation |
| 84 | ReadShareGroupState | Excluded (R5) | N/A | N/A | Out of scope for current single-node implementation |
| 85 | WriteShareGroupState | Excluded (R5) | N/A | N/A | Out of scope for current single-node implementation |
| 86 | DeleteShareGroupState | Excluded (R5) | N/A | N/A | Out of scope for current single-node implementation |
| 87 | ReadShareGroupStateSummary | Excluded (R5) | N/A | N/A | Out of scope for current single-node implementation |
| 90 | DescribeShareGroupOffsets | Excluded (R5) | N/A | N/A | Out of scope for current single-node implementation |
| 91 | AlterShareGroupOffsets | Excluded (R5) | N/A | N/A | Out of scope for current single-node implementation |
| 92 | DeleteShareGroupOffsets | Excluded (R5) | N/A | N/A | Out of scope for current single-node implementation |

## Error Semantics

- **Error codes**: Use Kafka standard error codes per API response.
- **Unsupported APIs**: Respond via ApiVersions with supported range only.
- **Unsupported versions**: Return version error as defined by kafka-protocol decoder where applicable.

## Validation Checklist

### Required Contract Tests (per supported API)
1. **ApiVersions**: version negotiation reflects `compute_supported_apis(SUPPORTED_APIS, backend capabilities)` — i.e. the static table intersected with the per-API capability gates of the configured backends. Unit tests in `src/protocol/mod.rs` pin the intersection behaviour (memory default advertises full set; missing group coordinator drops only group APIs; missing offset store drops only offset APIs; capability flags do not leak across APIs).
2. **Metadata**: topic discovery, auto-create behavior, error for disabled auto-create.
3. **Produce**: empty batch, keyed/unkeyed, large batch, partition error codes.
4. **Fetch**: offsets, high watermark, empty responses, max_bytes enforcement.
5. **ListOffsets**: earliest/latest/timestamp behavior.
6. **CreateTopics/DeleteTopics**: create/delete idempotency and errors.
7. **Consumer Groups**: FindCoordinator, JoinGroup, SyncGroup, Heartbeat, LeaveGroup, OffsetFetch, OffsetCommit.
8. **Idempotent Producer (FEAT-002)**: InitProducerId returns producerId/epoch; sequence-number tracking dedups retries; out-of-order returns OUT_OF_ORDER_SEQUENCE_NUMBER; duplicate returns DUPLICATE_SEQUENCE_NUMBER (or is silently de-duped per Kafka semantics).
9. **Transactions (FEAT-002)**: AddPartitionsToTxn, AddOffsetsToTxn, EndTxn (commit/abort), WriteTxnMarkers, TxnOffsetCommit drive a single-coordinator transaction state machine; control batches make read_committed consumers skip aborted records; stale epoch returns INVALID_PRODUCER_EPOCH; transaction.timeout.ms is enforced.
10. **Differential parity (FEAT-003)**: every supported and planned API is exercised by the parity harness against Redpanda; zero diffs at the gating workload.

## Precedence and Compatibility

- Versioning: per-API supported version ranges are advertised via ApiVersions
  (see Version Policy above); the static table is `SUPPORTED_APIS` in
  `src/protocol/mod.rs`.
- Flexible versions are supported per FEAT-006; legacy versions remain
  supported and are exercised by existing contract tests.
- All future changes must remain additive or gated by ApiVersions.

### FEAT-006 Tracking

Until FEAT-006 lands, the `Supported Versions` column in the matrix
above continues to reflect the current legacy-only range. As each
in-scope API gains its flexible variant, this contract is updated in
the same commit and the parity harness (FEAT-003) is re-run to confirm
zero diffs at flexible versions vs Redpanda.

## Examples

Concrete request/response examples are not inlined here. Wire-format examples
for each API follow the Kafka Protocol Guide (see **Source** above); per-API
executable examples are pinned by the contract tests listed in the Support
Matrix `Tests` column (`tests/contract.rs`, `tests/integration.rs`,
`src/handler/tests.rs`).

## Feature Traceability

- **PRD**: `docs/helix/01-frame/prd.md` (P0 #1 wire compat, #2 groups, #3 idempotent producers, #4 transactions, #5 parity, #6 benchmarks, #7 ecosystem).
- **Feature specs**: FEAT-001 (wire protocol), FEAT-002 (groups + transactions + idempotency), FEAT-003 (differential parity), FEAT-004 (benchmark conformance), FEAT-005 (ecosystem integrations), FEAT-006 (flexible-version protocol).
- **Implementation**: `src/handler/*.rs`, `src/protocol/*`, `src/storage/*`; transaction coordinator and producer-id manager TBD under FEAT-002.
- **Tests**: `tests/contract.rs`, `tests/integration.rs`, `src/handler/tests.rs`, `src/protocol/router.rs`, storage module unit/property tests; planned `tests/contract/transactions.rs`, `tests/parity/`, `tests/ecosystem/`, `scripts/bench/`.
- **Related Doc**: `docs/helix/03-test/test-plan/test-plan.md`.
