//! OffsetCommit request handler (API Key 8)

use crate::error::Result;
use crate::storage::OffsetStore;
use bytes::Bytes;
use kafka_protocol::messages::offset_commit_request::OffsetCommitRequest;
use kafka_protocol::messages::offset_commit_response::{
    OffsetCommitResponsePartition, OffsetCommitResponseTopic,
};
use kafka_protocol::messages::{OffsetCommitResponse, TopicName};
use kafka_protocol::protocol::{Decodable, StrBytes};
use std::sync::Arc;
use tracing::debug;

pub fn handle(
    api_version: i16,
    body: &[u8],
    offset_store: &Arc<dyn OffsetStore>,
) -> Result<OffsetCommitResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    let request = match OffsetCommitRequest::decode(&mut buf, api_version) {
        Ok(r) => r,
        Err(_) => return Ok(OffsetCommitResponse::default()),
    };

    let mut response = OffsetCommitResponse::default();
    let group_id = request.group_id.0.to_string();

    for topic in &request.topics {
        let topic_name = topic.name.0.to_string();
        let mut topic_response = OffsetCommitResponseTopic::default();
        topic_response.name = TopicName(StrBytes::from_string(topic_name.clone()));

        for partition in &topic.partitions {
            let metadata = partition.committed_metadata.as_ref().map(|s| s.to_string());
            let commit_error = offset_store
                .commit(
                    &group_id,
                    &topic_name,
                    partition.partition_index,
                    partition.committed_offset,
                    partition.committed_leader_epoch,
                    metadata,
                )
                .err();

            if commit_error.is_none() {
                debug!(
                    group = %group_id,
                    topic = %topic_name,
                    partition = partition.partition_index,
                    offset = partition.committed_offset,
                    "Committed offset"
                );
            }

            let mut partition_response = OffsetCommitResponsePartition::default();
            partition_response.partition_index = partition.partition_index;
            partition_response.error_code = if commit_error.is_some() { -1 } else { 0 };
            topic_response.partitions.push(partition_response);
        }

        response.topics.push(topic_response);
    }

    Ok(response)
}
