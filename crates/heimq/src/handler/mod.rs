//! Request handlers for each Kafka API

pub mod add_offsets_to_txn;
pub mod add_partitions_to_txn;
pub mod alter_configs;
pub mod api_versions;
pub mod create_topics;
pub mod delete_topics;
pub mod delete_groups;
pub mod describe_configs;
pub mod describe_groups;
pub mod end_txn;
pub mod fetch;
pub mod find_coordinator;
pub mod heartbeat;
pub mod incremental_alter_configs;
pub mod init_producer_id;
pub mod join_group;
pub mod leave_group;
pub mod list_groups;
pub mod list_offsets;
pub mod metadata;
pub mod offset_commit;
pub mod offset_delete;
pub mod offset_fetch;
pub mod produce;
pub mod sync_group;
pub mod txn_offset_commit;
pub mod write_txn_markers;

#[cfg(test)]
mod tests;
