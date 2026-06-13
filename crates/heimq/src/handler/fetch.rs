//! Fetch request handler (API Key 1)

use crate::error::{ErrorCode, Result};
use crate::storage::LogBackend;
use bytes::Bytes;
use kafka_protocol::messages::fetch_request::FetchRequest;
use kafka_protocol::messages::fetch_response::{FetchableTopicResponse, PartitionData};
use kafka_protocol::messages::{FetchResponse, TopicName};
use kafka_protocol::protocol::{Decodable, StrBytes};
use std::sync::Arc;
use tracing::debug;

/// Handle Fetch request
pub fn handle(
    api_version: i16,
    body: &[u8],
    storage: &Arc<dyn LogBackend>,
) -> Result<FetchResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    let request = match FetchRequest::decode(&mut buf, api_version) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to decode fetch request");
            return Ok(FetchResponse::default());
        }
    };

    let max_bytes = request.max_bytes as usize;
    let mut response = FetchResponse::default();

    for topic in request.topics {
        let topic_name = topic.topic.0.to_string();
        let mut topic_response = FetchableTopicResponse::default();
        topic_response.topic = TopicName(StrBytes::from_string(topic_name.clone()));

        for fp in topic.partitions {
            let partition = fp.partition;
            let fetch_offset = fp.fetch_offset;
            let partition_max_bytes = fp.partition_max_bytes.min(max_bytes as i32);

            let mut partition_data = PartitionData::default();
            partition_data.partition_index = partition;

            match storage.fetch(&topic_name, partition, fetch_offset, partition_max_bytes) {
                Ok((records, high_watermark)) => {
                    debug!(
                        topic = %topic_name,
                        partition,
                        offset = fetch_offset,
                        bytes = records.len(),
                        "Fetched records"
                    );
                    partition_data.error_code = 0;
                    partition_data.high_watermark = high_watermark;
                    partition_data.log_start_offset = storage
                        .log_start_offset(&topic_name, partition)
                        .unwrap_or(-1);
                    if !records.is_empty() {
                        partition_data.records = Some(Bytes::from(records).into());
                    }
                }
                Err(e) => {
                    partition_data.error_code = e.to_error_code();
                    partition_data.high_watermark = -1;
                }
            }

            topic_response.partitions.push(partition_data);
        }

        response.responses.push(topic_response);
    }

    Ok(response)
}
