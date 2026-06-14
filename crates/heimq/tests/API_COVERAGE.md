# Kafka API Coverage Matrix

Provable functional baseline for heimq. Every Kafka API key that heimq
advertises (`heimq::protocol::SUPPORTED_APIS`) is exercised by at least one
passing test. The guard test `tests/api_coverage.rs` enforces that this matrix
stays in sync with the advertised surface: advertising a new API without adding
a row here (and a `COVERAGE` entry in the guard) fails the build.

Coverage layers referenced below:
- **contract** — `tests/contract.rs`, byte-level wire request/response per API.
- **integration** — `tests/integration.rs`, real librdkafka (rdkafka crate).
- **oracle** — `tests/compat.rs` driving real franz-go / sarama / Java / KafkaJS / kcat clients.
- **parity** — `tests/parity/`, differential vs Apache Kafka + Redpanda.

| Key | API | Max ver | Primary test (contract) | Also covered by |
|----:|-----|--------:|-------------------------|-----------------|
| 0  | Produce | 11 | `contract_produce_fetch_roundtrip` | integration `rdkafka_simple_produce`; oracle `produce` |
| 1  | Fetch | 12 | `contract_produce_fetch_roundtrip` | integration `rdkafka_produce_consume_roundtrip` |
| 2  | ListOffsets | 9 | `contract_list_offsets_earliest_latest` | oracle `list-offsets` |
| 3  | Metadata | 12 | `contract_metadata_auto_creates_topic` | every client handshake |
| 8  | OffsetCommit | 9 | `contract_offset_commit_and_fetch` | integration `rdkafka_manual_commit_offset_roundtrip` |
| 9  | OffsetFetch | 9 | `contract_offset_fetch_unknown_group_returns_minus_one` | oracle `list-consumer-group-offsets` |
| 10 | FindCoordinator | 6 | `contract_find_coordinator_returns_self` | every group client |
| 11 | JoinGroup | 9 | `contract_join_group_new_member` | oracle `multi-member-group` |
| 12 | Heartbeat | 4 | `contract_heartbeat_active_member` | `contract_session_timeout_evicts_member` |
| 13 | LeaveGroup | 5 | `contract_leave_group_success` | parity `consumer_group_lifecycle` |
| 14 | SyncGroup | 5 | `contract_sync_group_leader` | unit `sync_group_*`; oracle `multi-member-group` |
| 15 | DescribeGroups | 5 | `contract_describe_groups_active_member` | oracle `describe-groups` |
| 16 | ListGroups | 4 | `contract_list_groups_returns_joined_group` | oracle `list-groups` |
| 18 | ApiVersions | 3 | `contract_api_versions_matches_supported_range` | every client handshake |
| 19 | CreateTopics | 7 | `contract_create_and_delete_topics` | oracle `create-topic` |
| 20 | DeleteTopics | 6 | `contract_create_and_delete_topics` | oracle `delete-topic` |
| 21 | DeleteRecords | 2 | `contract_delete_records_advances_lso` | oracle `delete-records` |
| 22 | InitProducerId | 5 | `contract_init_producer_id_returns_valid_id` | `contract_init_producer_id_with_transactional_id`; oracle `transactional-produce` |
| 23 | OffsetForLeaderEpoch | 4 | `contract_offset_for_leader_epoch_returns_hwm` | oracle `offset-for-leader-epoch` |
| 24 | AddPartitionsToTxn | 5 | `contract_add_partitions_to_txn_basic` | oracle `transactional-produce` |
| 25 | AddOffsetsToTxn | 4 | `contract_add_offsets_to_txn_basic` | oracle `transactional-produce` |
| 26 | EndTxn | 4 | `contract_end_txn_basic` | `contract_transactional_produce_commit_fetch`; parity `transactional_produce_roundtrip` |
| 27 | WriteTxnMarkers | 1 | `contract_write_txn_markers_basic` | broker-internal control records |
| 28 | TxnOffsetCommit | 4 | `contract_txn_offset_commit_basic` | oracle `transactional-produce` |
| 32 | DescribeConfigs | 4 | `contract_describe_configs_returns_topic_configs` | oracle `describe-configs` |
| 33 | AlterConfigs | 2 | `contract_alter_configs_no_op_success` | oracle `alter-configs` |
| 35 | DescribeLogDirs | 4 | `contract_describe_log_dirs_returns_memory_dir` | oracle `describe-log-dirs` |
| 37 | CreatePartitions | 3 | `contract_create_partitions_increases_count` | oracle `create-partitions` |
| 42 | DeleteGroups | 2 | `contract_delete_groups_removes_group` | oracle `delete-groups` |
| 43 | ElectLeaders | 2 | `contract_elect_leaders_succeeds_on_single_node` | oracle `elect-leaders` |
| 44 | IncrementalAlterConfigs | 1 | `contract_incremental_alter_configs_succeeds` | oracle `incremental-alter-configs` |
| 47 | OffsetDelete | 0 | `contract_offset_delete_clears_committed_offset` | oracle `offset-delete` |
| 60 | DescribeCluster | 1 | `contract_describe_cluster_returns_broker` | oracle `describe-cluster` |
| 66 | ListTransactions | 1 | `contract_list_transactions_returns_empty` | oracle `list-transactions` |
| 75 | DescribeTopicPartitions | 0 | `contract_describe_topic_partitions_returns_partition_leader` | oracle `describe-topic-partitions` |

## Enforcement / negative-path coverage

Beyond the happy-path matrix above, the EOS / idempotency enforcement paths are
contract-tested (and being extended to differential parity under `heimq-2c7afefc`):

| Behavior | Error / effect | Test |
|----------|----------------|------|
| Duplicate producer sequence | `DUPLICATE_SEQUENCE_NUMBER` (46) | `contract_duplicate_sequence_returns_error` |
| Out-of-order producer sequence | `OUT_OF_ORDER_SEQUENCE_NUMBER` (45) | `contract_out_of_order_sequence_returns_error` |
| Zombie producer epoch | `INVALID_PRODUCER_EPOCH` (47) | `contract_stale_epoch_returns_invalid_producer_epoch` |
| Aborted txn under read_committed | records invisible | `contract_aborted_transaction_invisible_to_read_committed` |
| Aborted txn under read_uncommitted | records visible | `contract_read_uncommitted_observes_aborted_records` |
