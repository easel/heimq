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

Legend:
- **Supported**: Implemented in heimq.
- **Gap**: Expected for functional completeness but not yet implemented.
- **Excluded**: Out of scope for single-node/no-durability design (reason noted).

Reason codes (Exclusions):
- **R1**: Requires multi-broker/controller/replication.
- **R2**: Requires security/authentication/authorization.
- **R3**: Requires transactions/idempotent producer.
- **R4**: Admin/config/quotas not yet in scope (control plane absent).
- **R5**: Newer group/telemetry/share-group APIs beyond current scope.

| API Key | Name | heimq Status | Supported Versions | Tests | Notes |
| --- | --- | --- | --- | --- | --- |
| 0 | Produce | Supported | 0-8 | `tests/integration.rs` legacy + rdkafka | Writes to in-memory storage only |
| 1 | Fetch | Supported | 0-11 | `tests/integration.rs` roundtrip consume | Reads from in-memory storage only |
| 2 | ListOffsets | Supported | 0-7 | None | Timestamp lookups simplified |
| 3 | Metadata | Supported | 0-8 | `tests/integration.rs` metadata_fetch | Single broker only |
| 4 | LeaderAndIsr | Excluded (R1) | N/A | N/A | Controller/replica protocol |
| 5 | StopReplica | Excluded (R1) | N/A | N/A | Controller/replica protocol |
| 6 | UpdateMetadata | Excluded (R1) | N/A | N/A | Controller/replica protocol |
| 7 | ControlledShutdown | Excluded (R1) | N/A | N/A | Broker lifecycle in multi-node clusters |
| 8 | OffsetCommit | Supported | 0-7 | None | Offset store is in-memory only |
| 9 | OffsetFetch | Supported | 0-7 | None | No tests yet for group offset retrieval |
| 10 | FindCoordinator | Supported | 0-3 | Implicit via rdkafka consumer | Single coordinator (self) |
| 11 | JoinGroup | Supported | 0-8 | Implicit via rdkafka consumer | Simplified group state |
| 12 | Heartbeat | Supported | 0-4 | Implicit via rdkafka consumer | Simplified liveness |
| 13 | LeaveGroup | Supported | 0-4 | None | Not directly tested |
| 14 | SyncGroup | Supported | 0-4 | Implicit via rdkafka consumer | Simplified assignment |
| 15 | DescribeGroups | Gap | N/A | None | Required for admin/visibility |
| 16 | ListGroups | Gap | N/A | None | Required for admin/visibility |
| 17 | SaslHandshake | Excluded (R2) | N/A | N/A | No auth support |
| 18 | ApiVersions | Supported | 0-3 | Implicit via clients | Explicit contract tests needed |
| 19 | CreateTopics | Supported | 0-6 | None | No validation of configs |
| 20 | DeleteTopics | Supported | 0-5 | None | No tests yet |
| 21 | DeleteRecords | Gap | N/A | None | Log truncation API |
| 22 | InitProducerId | Excluded (R3) | N/A | N/A | Transactions/idempotence not supported |
| 23 | OffsetForLeaderEpoch | Excluded (R1) | N/A | N/A | Leader epoch management |
| 24 | AddPartitionsToTxn | Excluded (R3) | N/A | N/A | Transactional |
| 25 | AddOffsetsToTxn | Excluded (R3) | N/A | N/A | Transactional |
| 26 | EndTxn | Excluded (R3) | N/A | N/A | Transactional |
| 27 | WriteTxnMarkers | Excluded (R3) | N/A | N/A | Transactional |
| 28 | TxnOffsetCommit | Excluded (R3) | N/A | N/A | Transactional |
| 29 | DescribeAcls | Excluded (R2) | N/A | N/A | No ACLs |
| 30 | CreateAcls | Excluded (R2) | N/A | N/A | No ACLs |
| 31 | DeleteAcls | Excluded (R2) | N/A | N/A | No ACLs |
| 32 | DescribeConfigs | Gap | N/A | None | Admin API expected for completeness |
| 33 | AlterConfigs | Gap | N/A | None | Admin API expected for completeness |
| 34 | AlterReplicaLogDirs | Excluded (R1) | N/A | N/A | Multi-broker log dirs |
| 35 | DescribeLogDirs | Excluded (R1) | N/A | N/A | Multi-broker log dirs |
| 36 | SaslAuthenticate | Excluded (R2) | N/A | N/A | No auth support |
| 37 | CreatePartitions | Gap | N/A | None | Partition scaling |
| 38 | CreateDelegationToken | Excluded (R2) | N/A | N/A | No auth support |
| 39 | RenewDelegationToken | Excluded (R2) | N/A | N/A | No auth support |
| 40 | ExpireDelegationToken | Excluded (R2) | N/A | N/A | No auth support |
| 41 | DescribeDelegationToken | Excluded (R2) | N/A | N/A | No auth support |
| 42 | DeleteGroups | Gap | N/A | None | Admin/cleanup API |
| 43 | ElectLeaders | Excluded (R1) | N/A | N/A | Controller protocol |
| 44 | IncrementalAlterConfigs | Gap | N/A | None | Admin API expected for completeness |
| 45 | AlterPartitionReassignments | Excluded (R1) | N/A | N/A | Multi-broker reassignment |
| 46 | ListPartitionReassignments | Excluded (R1) | N/A | N/A | Multi-broker reassignment |
| 47 | OffsetDelete | Gap | N/A | None | Group offset management |
| 48 | DescribeClientQuotas | Excluded (R4) | N/A | N/A | No quota management |
| 49 | AlterClientQuotas | Excluded (R4) | N/A | N/A | No quota management |
| 50 | DescribeUserScramCredentials | Excluded (R2) | N/A | N/A | No auth support |
| 51 | AlterUserScramCredentials | Excluded (R2) | N/A | N/A | No auth support |
| 52 | AlterIsr | Excluded (R1) | N/A | N/A | Controller protocol |
| 53 | UpdateFeatures | Excluded (R1) | N/A | N/A | Controller protocol |
| 54 | DescribeCluster | Gap | N/A | None | Admin/metadata completeness |
| 55 | DescribeProducers | Excluded (R3) | N/A | N/A | Idempotent/transactional tracking |
| 56 | DescribeTransactions | Excluded (R3) | N/A | N/A | Transactional |
| 57 | ListTransactions | Excluded (R3) | N/A | N/A | Transactional |
| 58 | AllocateProducerIds | Excluded (R3) | N/A | N/A | Idempotent/transactional |
| 59 | ConsumerGroupHeartbeat | Gap | N/A | None | New group protocol (KIP-848) |
| 60 | ConsumerGroupDescribe | Gap | N/A | None | New group protocol (KIP-848) |
| 61 | ControllerRegistration | Excluded (R1) | N/A | N/A | Controller protocol |
| 62 | GetTelemetrySubscriptions | Excluded (R5) | N/A | N/A | Telemetry not supported |
| 63 | PushTelemetry | Excluded (R5) | N/A | N/A | Telemetry not supported |
| 64 | AssignReplicasToDirs | Excluded (R1) | N/A | N/A | Multi-broker log dirs |
| 65 | ListClientMetricsResources | Excluded (R5) | N/A | N/A | Metrics API not supported |
| 66 | DescribeTopicPartitions | Gap | N/A | None | Newer admin metadata API |
| 67 | DescribeShareGroupOffsets | Excluded (R5) | N/A | N/A | Share groups not supported |
| 68 | AlterShareGroupOffsets | Excluded (R5) | N/A | N/A | Share groups not supported |
| 69 | DeleteShareGroupOffsets | Excluded (R5) | N/A | N/A | Share groups not supported |
| 70 | InitializeShareGroupState | Excluded (R5) | N/A | N/A | Share groups not supported |
| 71 | ReadShareGroupState | Excluded (R5) | N/A | N/A | Share groups not supported |
| 72 | WriteShareGroupState | Excluded (R5) | N/A | N/A | Share groups not supported |
| 73 | DeleteShareGroupState | Excluded (R5) | N/A | N/A | Share groups not supported |
| 74 | ReadShareGroupStateSummary | Excluded (R5) | N/A | N/A | Share groups not supported |
| 75 | DescribeShareGroupState | Excluded (R5) | N/A | N/A | Share groups not supported |

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
- **Tests**: `tests/integration.rs`, unit tests in storage modules, property tests in storage modules.
- **Related Doc**: `docs/helix/03-test/test-plan.md`.

