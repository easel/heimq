//! Functional baseline guard: every Kafka API key heimq advertises in
//! `SUPPORTED_APIS` must have a covering test recorded in the coverage matrix
//! (`API_COVERAGE.md`) and the `COVERAGE` table below. This turns "we test the
//! whole API surface" into a mechanically enforced invariant: advertising a new
//! API without adding coverage here fails the build.

use heimq::protocol::SUPPORTED_APIS;

/// (api_key, api_name, primary covering test). Kept in sync with API_COVERAGE.md
/// — the `coverage_matrix_doc_lists_every_mapped_test` test enforces that.
const COVERAGE: &[(i16, &str, &str)] = &[
    (0, "Produce", "contract_produce_fetch_roundtrip"),
    (1, "Fetch", "contract_produce_fetch_roundtrip"),
    (2, "ListOffsets", "contract_list_offsets_earliest_latest"),
    (3, "Metadata", "contract_metadata_auto_creates_topic"),
    (8, "OffsetCommit", "contract_offset_commit_and_fetch"),
    (9, "OffsetFetch", "contract_offset_fetch_unknown_group_returns_minus_one"),
    (10, "FindCoordinator", "contract_find_coordinator_returns_self"),
    (11, "JoinGroup", "contract_join_group_new_member"),
    (12, "Heartbeat", "contract_heartbeat_active_member"),
    (13, "LeaveGroup", "contract_leave_group_success"),
    (14, "SyncGroup", "contract_sync_group_leader"),
    (15, "DescribeGroups", "contract_describe_groups_active_member"),
    (16, "ListGroups", "contract_list_groups_returns_joined_group"),
    (18, "ApiVersions", "contract_api_versions_matches_supported_range"),
    (19, "CreateTopics", "contract_create_and_delete_topics"),
    (20, "DeleteTopics", "contract_create_and_delete_topics"),
    (21, "DeleteRecords", "contract_delete_records_advances_lso"),
    (22, "InitProducerId", "contract_init_producer_id_returns_valid_id"),
    (23, "OffsetForLeaderEpoch", "contract_offset_for_leader_epoch_returns_hwm"),
    (24, "AddPartitionsToTxn", "contract_add_partitions_to_txn_basic"),
    (25, "AddOffsetsToTxn", "contract_add_offsets_to_txn_basic"),
    (26, "EndTxn", "contract_end_txn_basic"),
    (27, "WriteTxnMarkers", "contract_write_txn_markers_basic"),
    (28, "TxnOffsetCommit", "contract_txn_offset_commit_basic"),
    (32, "DescribeConfigs", "contract_describe_configs_returns_topic_configs"),
    (33, "AlterConfigs", "contract_alter_configs_no_op_success"),
    (35, "DescribeLogDirs", "contract_describe_log_dirs_returns_memory_dir"),
    (37, "CreatePartitions", "contract_create_partitions_increases_count"),
    (42, "DeleteGroups", "contract_delete_groups_removes_group"),
    (43, "ElectLeaders", "contract_elect_leaders_succeeds_on_single_node"),
    (44, "IncrementalAlterConfigs", "contract_incremental_alter_configs_succeeds"),
    (47, "OffsetDelete", "contract_offset_delete_clears_committed_offset"),
    (60, "DescribeCluster", "contract_describe_cluster_returns_broker"),
    (66, "ListTransactions", "contract_list_transactions_returns_empty"),
    (75, "DescribeTopicPartitions", "contract_describe_topic_partitions_returns_partition_leader"),
];

const MATRIX_DOC: &str = include_str!("API_COVERAGE.md");

/// No advertised API may lack a coverage entry.
#[test]
fn every_advertised_api_has_coverage() {
    let uncovered: Vec<i16> = SUPPORTED_APIS
        .iter()
        .map(|(key, _, _)| *key)
        .filter(|key| !COVERAGE.iter().any(|(k, _, _)| k == key))
        .collect();
    assert!(
        uncovered.is_empty(),
        "advertised API keys with no coverage entry in COVERAGE/API_COVERAGE.md: {uncovered:?}"
    );
}

/// No coverage entry may reference an API key heimq does not advertise.
#[test]
fn no_stale_coverage_entries() {
    let stale: Vec<i16> = COVERAGE
        .iter()
        .map(|(key, _, _)| *key)
        .filter(|key| !SUPPORTED_APIS.iter().any(|(k, _, _)| k == key))
        .collect();
    assert!(
        stale.is_empty(),
        "COVERAGE references API keys not in SUPPORTED_APIS: {stale:?}"
    );
}

/// The human-readable matrix must reference every mapped covering test, so the
/// doc cannot silently drift from the enforced table.
#[test]
fn coverage_matrix_doc_lists_every_mapped_test() {
    let missing: Vec<&str> = COVERAGE
        .iter()
        .map(|(_, _, test)| *test)
        .filter(|test| !MATRIX_DOC.contains(*test))
        .collect();
    assert!(
        missing.is_empty(),
        "API_COVERAGE.md does not mention these covering tests: {missing:?}"
    );
}
