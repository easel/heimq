# API Contract: Kafka Wire Protocol [heimq]

**Contract ID**: API-001
**Feature**: FEAT-001 (Kafka Compatibility)
**Type**: Protocol (Kafka TCP Wire)
**Status**: Draft
**Version**: 0.1.0

*This contract defines heimq's Kafka wire-protocol surface area, supported versions, exclusions, and test coverage expectations.*

## Scope

- Single-node Kafka-compatible broker focused on transport speed over durability.
- No distributed consensus, replication, or controller responsibilities.
- No security, transactions, or ACLs in scope for this contract.

## Transport and Framing

- **Transport**: TCP, binary Kafka protocol.
- **Framing**: 4-byte big-endian message length, followed by request header + body.
- **Request header (non-flexible)**: api_key, api_version, correlation_id, client_id.
- **Response header**: correlation_id.
- **Flexible versions**: Not supported (requires compact types/varints).

## Version Policy

- API versions are limited to pre-flexible versions to avoid compact encodings.
- The supported version range for each API is reported via ApiVersions.

## Support Matrix (Kafka API Keys)

Kafka API keys listed here follow the current Apache Kafka protocol spec.

Legend:
- **Supported**: Implemented in heimq.
- **Gap**: Expected for functional completeness but not yet implemented.
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
| 2 | ListOffsets | Supported | 0-7 | `src/handler/tests.rs`; `tests/contract.rs` | Timestamp lookups simplified |
| 3 | Metadata | Supported | 0-8 | `src/handler/tests.rs`; `tests/contract.rs`; `tests/integration.rs` | Single broker only |
| 8 | OffsetCommit | Supported | 0-7 | `src/handler/tests.rs`; `tests/contract.rs` | In-memory offsets |
| 9 | OffsetFetch | Supported | 0-7 | `src/handler/tests.rs`; `tests/contract.rs` | In-memory offsets |
| 10 | FindCoordinator | Supported | 0-3 | `src/handler/tests.rs` | Single coordinator (self) |
| 11 | JoinGroup | Supported | 0-8 | `src/handler/tests.rs` | Simplified group state |
| 12 | Heartbeat | Supported | 0-4 | `src/handler/tests.rs` | Simplified liveness |
| 13 | LeaveGroup | Supported | 0-4 | `src/handler/tests.rs` | Member removal only |
| 14 | SyncGroup | Supported | 0-4 | `src/handler/tests.rs` | Simplified assignment |
| 15 | DescribeGroups | Gap | N/A | None | Admin/visibility (R4) |
| 16 | ListGroups | Gap | N/A | None | Admin/visibility (R4) |
| 17 | SaslHandshake | Excluded (R2) | N/A | N/A | No auth support |
| 18 | ApiVersions | Supported | 0-3 | `src/handler/tests.rs`; `tests/contract.rs`; `src/protocol/router.rs` | Version negotiation only |
| 19 | CreateTopics | Supported | 0-6 | `src/handler/tests.rs`; `tests/contract.rs` | No config validation |
| 20 | DeleteTopics | Supported | 0-5 | `src/handler/tests.rs`; `tests/contract.rs` | Best-effort delete |
| 21 | DeleteRecords | Gap | N/A | None | Log truncation API (R4) |
| 22 | InitProducerId | Excluded (R3) | N/A | N/A | Transactions/idempotence |
| 23 | OffsetForLeaderEpoch | Excluded (R1) | N/A | N/A | Leader epoch management |
| 24 | AddPartitionsToTxn | Excluded (R3) | N/A | N/A | Transactions |
| 25 | AddOffsetsToTxn | Excluded (R3) | N/A | N/A | Transactions |
| 26 | EndTxn | Excluded (R3) | N/A | N/A | Transactions |
| 27 | WriteTxnMarkers | Excluded (R3) | N/A | N/A | Transactions |
| 28 | TxnOffsetCommit | Excluded (R3) | N/A | N/A | Transactions |
| 29 | DescribeAcls | Excluded (R2) | N/A | N/A | No auth/ACLs |
| 30 | CreateAcls | Excluded (R2) | N/A | N/A | No auth/ACLs |
| 31 | DeleteAcls | Excluded (R2) | N/A | N/A | No auth/ACLs |
| 32 | DescribeConfigs | Gap | N/A | None | Admin/configs (R4) |
| 33 | AlterConfigs | Excluded (R4) | N/A | N/A | Admin/configs |
| 34 | AlterReplicaLogDirs | Excluded (R1) | N/A | N/A | Multi-broker log dirs |
| 35 | DescribeLogDirs | Excluded (R1) | N/A | N/A | Multi-broker log dirs |
| 36 | SaslAuthenticate | Excluded (R2) | N/A | N/A | No auth support |
| 37 | CreatePartitions | Gap | N/A | None | Admin/partition management (R4) |
| 38 | CreateDelegationToken | Excluded (R2) | N/A | N/A | No auth |
| 39 | RenewDelegationToken | Excluded (R2) | N/A | N/A | No auth |
| 40 | ExpireDelegationToken | Excluded (R2) | N/A | N/A | No auth |
| 41 | DescribeDelegationToken | Excluded (R2) | N/A | N/A | No auth |
| 42 | DeleteGroups | Gap | N/A | None | Admin/group cleanup (R4) |
| 43 | ElectLeaders | Excluded (R1) | N/A | N/A | Multi-broker leadership |
| 44 | IncrementalAlterConfigs | Excluded (R4) | N/A | N/A | Admin/configs |
| 45 | AlterPartitionReassignments | Excluded (R1) | N/A | N/A | Multi-broker reassignment |
| 46 | ListPartitionReassignments | Excluded (R1) | N/A | N/A | Multi-broker reassignment |
| 47 | OffsetDelete | Gap | N/A | None | Group offset management (R4) |
| 48 | DescribeClientQuotas | Excluded (R4) | N/A | N/A | Quotas not supported |
| 49 | AlterClientQuotas | Excluded (R4) | N/A | N/A | Quotas not supported |
| 50 | DescribeUserScramCredentials | Excluded (R2) | N/A | N/A | No auth |
| 51 | AlterUserScramCredentials | Excluded (R2) | N/A | N/A | No auth |
| 55 | DescribeQuorum | Excluded (R1) | N/A | N/A | KRaft/controller |
| 57 | UpdateFeatures | Excluded (R1) | N/A | N/A | KRaft/controller |
| 60 | DescribeCluster | Gap | N/A | None | Admin/cluster metadata (R4) |
| 61 | DescribeProducers | Excluded (R3) | N/A | N/A | Idempotent/transactional |
| 64 | UnregisterBroker | Excluded (R1) | N/A | N/A | KRaft/controller |
| 65 | DescribeTransactions | Excluded (R3) | N/A | N/A | Transactions |
| 66 | ListTransactions | Excluded (R3) | N/A | N/A | Transactions |
| 68 | ConsumerGroupHeartbeat | Gap | N/A | None | New group protocol (R5) |
| 69 | ConsumerGroupDescribe | Gap | N/A | None | New group protocol (R5) |
| 71 | GetTelemetrySubscriptions | Excluded (R5) | N/A | N/A | Telemetry not supported |
| 72 | PushTelemetry | Excluded (R5) | N/A | N/A | Telemetry not supported |
| 74 | ListConfigResources | Gap | N/A | None | Admin/configs (R4) |
| 75 | DescribeTopicPartitions | Gap | N/A | None | Admin metadata (R4) |
| 76 | ShareGroupHeartbeat | Excluded (R5) | N/A | N/A | Share groups not supported |
| 77 | ShareGroupDescribe | Excluded (R5) | N/A | N/A | Share groups not supported |
| 78 | ShareFetch | Excluded (R5) | N/A | N/A | Share groups not supported |
| 79 | ShareAcknowledge | Excluded (R5) | N/A | N/A | Share groups not supported |
| 80 | AddRaftVoter | Excluded (R1) | N/A | N/A | KRaft/controller |
| 81 | RemoveRaftVoter | Excluded (R1) | N/A | N/A | KRaft/controller |
| 83 | InitializeShareGroupState | Excluded (R5) | N/A | N/A | Share groups not supported |
| 84 | ReadShareGroupState | Excluded (R5) | N/A | N/A | Share groups not supported |
| 85 | WriteShareGroupState | Excluded (R5) | N/A | N/A | Share groups not supported |
| 86 | DeleteShareGroupState | Excluded (R5) | N/A | N/A | Share groups not supported |
| 87 | ReadShareGroupStateSummary | Excluded (R5) | N/A | N/A | Share groups not supported |
| 90 | DescribeShareGroupOffsets | Excluded (R5) | N/A | N/A | Share groups not supported |
| 91 | AlterShareGroupOffsets | Excluded (R5) | N/A | N/A | Share groups not supported |
| 92 | DeleteShareGroupOffsets | Excluded (R5) | N/A | N/A | Share groups not supported |


## Error Contracts

- **Error codes**: Use Kafka standard error codes per API response.
- **Unsupported APIs**: Respond via ApiVersions with supported range only.
- **Unsupported versions**: Return version error as defined by kafka-protocol decoder where applicable.

## Contract Validation

### Required Contract Tests (per supported API)
1. **ApiVersions**: version negotiation reflects `SUPPORTED_APIS`.
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
- **Related Doc**: `docs/helix/03-test/test-plan.md`.
