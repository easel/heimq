//! AddPartitionsToTxn handler (API Key 24) — US-004

use crate::error::Result;
use crate::transaction_state::TransactionManager;
use bytes::Bytes;
use heimq_protocol::messages::add_partitions_to_txn_request::AddPartitionsToTxnRequest;
use heimq_protocol::messages::add_partitions_to_txn_response::{
    AddPartitionsToTxnPartitionResult, AddPartitionsToTxnResponse, AddPartitionsToTxnTopicResult,
};
use heimq_protocol::messages::TopicName;
use heimq_protocol::protocol::{Decodable, StrBytes};
use std::sync::Arc;
use tracing::debug;

pub fn handle(
    api_version: i16,
    body: &[u8],
    transaction_manager: &Arc<TransactionManager>,
) -> Result<AddPartitionsToTxnResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    let request = match AddPartitionsToTxnRequest::decode(&mut buf, api_version) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to decode AddPartitionsToTxn request");
            return Ok(AddPartitionsToTxnResponse::default());
        }
    };

    let mut response = AddPartitionsToTxnResponse::default();

    // v0-2: use v3_and_below fields
    let txn_id = request.v3_and_below_transactional_id.0.to_string();
    let producer_id = request.v3_and_below_producer_id.0;
    let epoch = request.v3_and_below_producer_epoch;

    debug!(
        txn_id = %txn_id,
        producer_id,
        epoch,
        "AddPartitionsToTxn"
    );

    for topic in &request.v3_and_below_topics {
        let topic_name = topic.name.0.to_string();
        let mut topic_result = AddPartitionsToTxnTopicResult::default();
        topic_result.name = TopicName(StrBytes::from_string(topic_name.clone()));

        for &partition in &topic.partitions {
            let error_code = transaction_manager.add_partitions(
                &txn_id,
                producer_id,
                epoch,
                &topic_name,
                partition,
                i64::MAX, // sentinel; overwritten with actual offset by record_produce
            );

            let mut partition_result = AddPartitionsToTxnPartitionResult::default();
            partition_result.partition_index = partition;
            partition_result.partition_error_code = error_code;
            topic_result.results_by_partition.push(partition_result);
        }

        response.results_by_topic_v3_and_below.push(topic_result);
    }

    Ok(response)
}
