//! Produce request handler (API Key 0)

use crate::error::Result;
use crate::storage::LogBackend;
use bytes::Bytes;
use kafka_protocol::messages::produce_request::ProduceRequest;
use kafka_protocol::messages::produce_response::{PartitionProduceResponse, TopicProduceResponse};
use kafka_protocol::messages::{ProduceResponse, TopicName};
use kafka_protocol::protocol::{Decodable, StrBytes};
use std::sync::Arc;
use tracing::{debug, warn};

/// Handle Produce request
pub fn handle(
    api_version: i16,
    body: &[u8],
    storage: &Arc<dyn LogBackend>,
) -> Result<ProduceResponse> {
    debug!(api_version = api_version, body_len = body.len(), "Handling produce request");

    // Use kafka-protocol's decoder for proper version-aware parsing
    let mut buf = Bytes::copy_from_slice(body);
    let request = match ProduceRequest::decode(&mut buf, api_version) {
        Ok(req) => req,
        Err(e) => {
            warn!(error = %e, "Failed to decode produce request");
            return Ok(ProduceResponse::default());
        }
    };

    let mut response = ProduceResponse::default();

    let caps = storage.capabilities();
    let transactional_attempted = request.transactional_id.is_some();
    let transactions_unsupported = transactional_attempted && !caps.transactions;
    let max_message_bytes = caps.max_message_bytes;
    let max_batch_bytes = caps.max_batch_bytes;

    for topic_data in request.topic_data {
        let topic_name = topic_data.name.0.to_string();
        debug!(topic = %topic_name, partitions = topic_data.partition_data.len(), "Processing topic");

        let mut topic_response = TopicProduceResponse::default();
        topic_response.name = TopicName(StrBytes::from_string(topic_name.clone()));

        for partition_data in topic_data.partition_data {
            let partition = partition_data.index;
            let mut partition_response = PartitionProduceResponse::default();
            partition_response.index = partition;

            if transactions_unsupported {
                warn!(topic = %topic_name, partition, "Rejecting transactional produce: backend does not support transactions");
                partition_response.error_code = 48; // INVALID_TXN_STATE
                partition_response.base_offset = -1;
                topic_response.partition_responses.push(partition_response);
                continue;
            }

            if let Some(records) = partition_data.records {
                if !records.is_empty() {
                    if records.len() > max_batch_bytes || records.len() > max_message_bytes {
                        warn!(
                            topic = %topic_name,
                            partition,
                            len = records.len(),
                            max_message_bytes,
                            max_batch_bytes,
                            "Rejecting oversized produce batch"
                        );
                        partition_response.error_code = 10; // MESSAGE_TOO_LARGE
                        partition_response.base_offset = -1;
                        topic_response.partition_responses.push(partition_response);
                        continue;
                    }
                    // Append to storage
                    match storage.append(&topic_name, partition, &records) {
                        Ok((base_offset, count)) => {
                            debug!(
                                topic = %topic_name,
                                partition = partition,
                                base_offset = base_offset,
                                count = count,
                                "Produced records"
                            );
                            partition_response.base_offset = base_offset;
                            partition_response.error_code = 0;
                            partition_response.log_append_time_ms =
                                chrono::Utc::now().timestamp_millis();
                        }
                        Err(e) => {
                            warn!(error = %e, topic = %topic_name, partition, "Failed to append records");
                            partition_response.error_code = e.to_error_code();
                            partition_response.base_offset = -1;
                        }
                    }
                } else {
                    // Empty records - still valid, just no-op
                    partition_response.base_offset = storage
                        .high_watermark(&topic_name, partition)
                        .unwrap_or(0);
                    partition_response.error_code = 0;
                }
            } else {
                // Null records
                partition_response.error_code = 87; // INVALID_RECORD
                partition_response.base_offset = -1;
            }

            topic_response.partition_responses.push(partition_response);
        }

        response.responses.push(topic_response);
    }

    Ok(response)
}
