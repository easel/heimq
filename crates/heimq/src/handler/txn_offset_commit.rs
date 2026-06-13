//! TxnOffsetCommit handler (API Key 28) — US-004

use crate::error::Result;
use crate::transaction_state::TransactionManager;
use bytes::Bytes;
use kafka_protocol::messages::txn_offset_commit_request::TxnOffsetCommitRequest;
use kafka_protocol::messages::txn_offset_commit_response::{
    TxnOffsetCommitResponse, TxnOffsetCommitResponsePartition, TxnOffsetCommitResponseTopic,
};
use kafka_protocol::messages::TopicName;
use kafka_protocol::protocol::{Decodable, StrBytes};
use std::sync::Arc;
use tracing::debug;

pub fn handle(
    api_version: i16,
    body: &[u8],
    transaction_manager: &Arc<TransactionManager>,
) -> Result<TxnOffsetCommitResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    let request = match TxnOffsetCommitRequest::decode(&mut buf, api_version) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to decode TxnOffsetCommit request");
            return Ok(TxnOffsetCommitResponse::default());
        }
    };

    let txn_id = request.transactional_id.0.to_string();
    let group_id = request.group_id.0.to_string();
    let producer_id = request.producer_id.0;
    let epoch = request.producer_epoch;

    debug!(
        txn_id = %txn_id,
        group_id = %group_id,
        producer_id,
        epoch,
        "TxnOffsetCommit"
    );

    let mut response = TxnOffsetCommitResponse::default();

    for topic in &request.topics {
        let topic_name = topic.name.0.to_string();
        let mut topic_result = TxnOffsetCommitResponseTopic::default();
        topic_result.name = TopicName(StrBytes::from_string(topic_name.clone()));

        for partition in &topic.partitions {
            let metadata = partition
                .committed_metadata
                .as_ref()
                .map(|m| m.to_string());

            let error_code = transaction_manager.commit_offset(
                &txn_id,
                producer_id,
                epoch,
                &group_id,
                &topic_name,
                partition.partition_index,
                partition.committed_offset,
                metadata,
            );

            let mut part_result = TxnOffsetCommitResponsePartition::default();
            part_result.partition_index = partition.partition_index;
            part_result.error_code = error_code;
            topic_result.partitions.push(part_result);
        }

        response.topics.push(topic_result);
    }

    Ok(response)
}
