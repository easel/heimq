//! CreatePartitions request handler (API Key 37)
//!
//! Expands an existing topic's partition count. The in-memory backend supports
//! this via expand_topic_partitions(); other backends may return UNSUPPORTED.

use crate::error::Result;
use crate::storage::LogBackend;
use bytes::Bytes;
use kafka_protocol::messages::create_partitions_request::CreatePartitionsRequest;
use kafka_protocol::messages::create_partitions_response::CreatePartitionsTopicResult;
use kafka_protocol::messages::CreatePartitionsResponse;
use kafka_protocol::protocol::{Decodable, StrBytes};
use std::sync::Arc;

pub fn handle(
    api_version: i16,
    body: &[u8],
    storage: &Arc<dyn LogBackend>,
) -> Result<CreatePartitionsResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    let request = match CreatePartitionsRequest::decode(&mut buf, api_version) {
        Ok(r) => r,
        Err(_) => return Ok(CreatePartitionsResponse::default()),
    };

    let mut response = CreatePartitionsResponse::default();

    for topic in &request.topics {
        let topic_name = topic.name.0.as_str();
        let mut result = CreatePartitionsTopicResult::default();
        result.name = kafka_protocol::messages::TopicName(StrBytes::from_string(topic_name.to_string()));

        match storage.expand_topic_partitions(topic_name, topic.count) {
            Ok(()) => {
                result.error_code = 0;
                result.error_message = None;
            }
            Err(e) => {
                result.error_code = 36; // INVALID_PARTITIONS
                result.error_message = Some(StrBytes::from_string(e.to_string()));
            }
        }

        response.results.push(result);
    }

    Ok(response)
}
