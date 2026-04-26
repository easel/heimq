# API Contract: Kafka Wire Protocol [heimq]

**Contract ID**: API-001
**Feature**: FEAT-001 (Kafka Compatibility)
**Type**: Protocol (Kafka TCP Wire)
**Status**: Draft
**Version**: 0.1.0
**Source**: Kafka Protocol Guide (`https://kafka.apache.org/protocol/`)

*This contract defines heimq's Kafka wire-protocol surface area, supported versions, exclusions, and test coverage expectations.*

## Scope

- Single-node Kafka-compatible broker focused on transport speed over durability.
- No distributed consensus, replication, or controller responsibilities.
- No security, transactions, or ACLs in scope for this contract.

## Protocol Interface Contract

### Transport and Framing

- **Transport**: TCP, binary Kafka protocol.
- **Framing**: 4-byte big-endian message length, followed by request header + body.
- **Request header (non-flexible)**: api_key, api_version, correlation_id, client_id.
- **Response header**: correlation_id.
- **Flexible versions**: Not supported (requires compact types/varints).

### Version Policy

- **Flexible-version boundary**: Each API's `max_version` is held strictly below the Kafka flexible-version boundary for that API. Flexible versions introduce compact strings, varints, and tagged fields, which heimq's legacy-layout handlers do not parse. Per-API boundaries (first flexible version): Produce v9, Fetch v12, ListOffsets v6, Metadata v9, OffsetCommit v8, OffsetFetch v6, FindCoordinator v3, JoinGroup v6, Heartbeat v4, LeaveGroup v4, SyncGroup v4, ApiVersions v3, CreateTopics v5, DeleteTopics v4.
- The static version table is `SUPPORTED_APIS` in `src/protocol/mod.rs`; the matrix below mirrors it. Any change to advertised versions must update both.
- **Capability-derived advertisement**: The ApiVersions response is not a verbatim copy of `SUPPORTED_APIS`. At runtime, `compute_supported_apis` (`src/protocol/mod.rs`) intersects the static table with the per-API `CapabilityGate` against each backend's descriptor (`BackendCapabilities`, `OffsetStoreCapabilities`, `GroupCoordinatorCapabilities`). APIs whose required backend is absent (e.g. no group coordinator) are filtered out before the response is encoded, so heimq advertises only what its currently configured backends can actually serve. Gating is per-API, not a global meet — a backend that lacks compaction does not lose unrelated APIs.

## Support Matrix (Kafka API Keys)

Kafka API keys listed here follow the current Apache Kafka protocol spec.

Legend:
- **Supported**: Implemented in heimq.
- **Excluded**: Out of scope for single-node/no-durability design (reason noted).

Reason codes (Exclusions):
- **R1**: Requires multi-broker/controller/replication/KRaft.
- **R2**: Requires security/authentication/authorization.
- **R3**: Requires transactions/idempotent producer.
- **R4**: Admin/control-plane APIs not yet in scope.
- **R5**: Telemetry/share-group/new-group APIs beyond current scope.

| API Key | Name | heimq Status | Supported Versions | Tests | Notes |
| --- | --- | --- | --- | --- | --- |
| 0 | Produce | Supported | 0-8 | `src/handler/tests.rs`; `tests/contract.rs`; `tests/integration.rs` | In-memory append only |
| 1 | Fetch | Supported | 0-11 | `src/handler/tests.rs`; `tests/contract.rs`; `tests/integration.rs` | In-memory read only |
| 2 | ListOffsets | Supported | 0-5 | `src/handler/tests.rs`; `tests/contract.rs` | Timestamp lookups simplified |
| 3 | Metadata | Supported | 0-8 | `src/handler/tests.rs`; `tests/contract.rs`; `tests/integration.rs` | Single broker only |
| 8 | OffsetCommit | Supported | 0-7 | `src/handler/tests.rs`; `tests/contract.rs` | In-memory offsets |
| 9 | OffsetFetch | Supported | 0-5 | `src/handler/tests.rs`; `tests/contract.rs` | In-memory offsets |
| 10 | FindCoordinator | Supported | 0-2 | `src/handler/tests.rs` | Single coordinator (self) |
| 11 | JoinGroup | Supported | 0-5 | `src/handler/tests.rs` | Simplified group state |
| 12 | Heartbeat | Supported | 0-3 | `src/handler/tests.rs` | Simplified liveness |
| 13 | LeaveGroup | Supported | 0-3 | `src/handler/tests.rs` | Member removal only |
| 14 | SyncGroup | Supported | 0-3 | `src/handler/tests.rs` | Simplified assignment |
| 15 | DescribeGroups | Excluded (R4) | N/A | N/A | Out of scope for current single-node implementation |
| 16 | ListGroups | Excluded (R4) | N/A | N/A | Out of scope for current single-node implementation |
| 17 | SaslHandshake | Excluded (R2) | N/A | N/A | Out of scope for current single-node implementation |
| 18 | ApiVersions | Supported | 0-2 | `src/handler/tests.rs`; `tests/contract.rs`; `src/protocol/router.rs` | Version negotiation only |
| 19 | CreateTopics | Supported | 0-4 | `src/handler/tests.rs`; `tests/contract.rs` | No config validation |
| 20 | DeleteTopics | Supported | 0-3 | `src/handler/tests.rs`; `tests/contract.rs` | Best-effort delete |
| 21 | DeleteRecords | Excluded (R4) | N/A | N/A | Out of scope for current single-node implementation |
| 22 | InitProducerId | Excluded (R3) | N/A | N/A | Out of scope for current single-node implementation |
| 23 | OffsetForLeaderEpoch | Excluded (R1) | N/A | N/A | Out of scope for current single-node implementation |
| 24 | AddPartitionsToTxn | Excluded (R3) | N/A | N/A | Out of scope for current single-node implementation |
| 25 | AddOffsetsToTxn | Excluded (R3) | N/A | N/A | Out of scope for current single-node implementation |
| 26 | EndTxn | Excluded (R3) | N/A | N/A | Out of scope for current single-node implementation |
| 27 | WriteTxnMarkers | Excluded (R3) | N/A | N/A | Out of scope for current single-node implementation |
| 28 | TxnOffsetCommit | Excluded (R3) | N/A | N/A | Out of scope for current single-node implementation |
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
| 61 | DescribeProducers | Excluded (R3) | N/A | N/A | Out of scope for current single-node implementation |
| 64 | UnregisterBroker | Excluded (R1) | N/A | N/A | Out of scope for current single-node implementation |
| 65 | DescribeTransactions | Excluded (R3) | N/A | N/A | Out of scope for current single-node implementation |
| 66 | ListTransactions | Excluded (R3) | N/A | N/A | Out of scope for current single-node implementation |
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

## Error Contracts

- **Error codes**: Use Kafka standard error codes per API response.
- **Unsupported APIs**: Respond via ApiVersions with supported range only.
- **Unsupported versions**: Return version error as defined by kafka-protocol decoder where applicable.

## Contract Validation

### Required Contract Tests (per supported API)
1. **ApiVersions**: version negotiation reflects `compute_supported_apis(SUPPORTED_APIS, backend capabilities)` — i.e. the static table intersected with the per-API capability gates of the configured backends. Unit tests in `src/protocol/mod.rs` pin the intersection behaviour (memory default advertises full set; missing group coordinator drops only group APIs; missing offset store drops only offset APIs; capability flags do not leak across APIs).
2. **Metadata**: topic discovery, auto-create behavior, error for disabled auto-create.
3. **Produce**: empty batch, keyed/unkeyed, large batch, partition error codes.
4. **Fetch**: offsets, high watermark, empty responses, max_bytes enforcement.
5. **ListOffsets**: earliest/latest/timestamp behavior.
6. **CreateTopics/DeleteTopics**: create/delete idempotency and errors.
7. **Consumer Groups**: FindCoordinator, JoinGroup, SyncGroup, Heartbeat, LeaveGroup, OffsetFetch, OffsetCommit.

### Backwards Compatibility
- No flexible versions; max version constrained to non-flexible range.
- All future changes must remain additive or gated by ApiVersions.

## Feature Traceability

- **Implementation**: `src/handler/*.rs`, `src/protocol/*`, `src/storage/*`.
- **Tests**: `tests/contract.rs`, `tests/integration.rs`, `src/handler/tests.rs`, `src/protocol/router.rs`, storage module unit/property tests.
- **Related Doc**: `docs/helix/03-test/test-plan/test-plan.md`.
