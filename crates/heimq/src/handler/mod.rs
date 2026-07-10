//! Request handlers for each Kafka API

// Produce/Metadata/ApiVersions/InitProducerId handlers now live in
// heimq-handlers so embedders can delegate to them; re-exported here at their
// original `crate::handler::…` paths (the router reaches them via `handler::*`).
pub use heimq_handlers::{api_versions, init_producer_id, metadata, produce};

pub mod add_offsets_to_txn;
pub mod add_partitions_to_txn;
pub mod alter_configs;
pub mod create_partitions;
pub mod create_topics;
pub mod delete_groups;
pub mod delete_records;
pub mod delete_topics;
pub mod describe_cluster;
pub mod describe_configs;
pub mod describe_groups;
pub mod describe_log_dirs;
pub mod describe_topic_partitions;
pub mod elect_leaders;
pub mod end_txn;
pub mod fetch;
pub mod find_coordinator;
pub mod heartbeat;
pub mod incremental_alter_configs;
pub mod join_group;
pub mod leave_group;
pub mod list_groups;
pub mod list_offsets;
pub mod list_transactions;
pub mod offset_commit;
pub mod offset_delete;
pub mod offset_fetch;
pub mod offset_for_leader_epoch;
pub mod sync_group;
pub mod txn_offset_commit;
pub mod write_txn_markers;

#[cfg(test)]
mod tests;
