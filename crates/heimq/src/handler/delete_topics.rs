//! DeleteTopics request handler (API Key 20)

use crate::error::Result;
use crate::storage::LogBackend;
use bytes::Bytes;
use kafka_protocol::messages::delete_topics_request::DeleteTopicsRequest;
use kafka_protocol::messages::delete_topics_response::DeletableTopicResult;
use kafka_protocol::messages::{DeleteTopicsResponse, TopicName};
use kafka_protocol::protocol::{Decodable, StrBytes};
use std::sync::Arc;
use tracing::info;

pub fn handle(
    api_version: i16,
    body: &[u8],
    storage: &Arc<dyn LogBackend>,
) -> Result<DeleteTopicsResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    let request = match DeleteTopicsRequest::decode(&mut buf, api_version) {
        Ok(r) => r,
        Err(_) => return Ok(DeleteTopicsResponse::default()),
    };

    let mut response = DeleteTopicsResponse::default();

    // v0-5: topic_names field; v6+: topics field (with optional uuid)
    // kafka-protocol crate populates the appropriate field based on api_version.
    let names: Vec<String> = if api_version >= 6 {
        request
            .topics
            .iter()
            .filter_map(|t| t.name.as_ref().map(|n| n.0.to_string()))
            .collect()
    } else {
        request
            .topic_names
            .iter()
            .map(|n| n.0.to_string())
            .collect()
    };

    for topic_name in names {
        let mut topic_result = DeletableTopicResult::default();
        topic_result.name = Some(TopicName(StrBytes::from_string(topic_name.clone())));

        match storage.delete_topic(&topic_name) {
            Ok(()) => {
                info!(topic = %topic_name, "Deleted topic");
                topic_result.error_code = 0;
            }
            Err(_) => {
                topic_result.error_code = 3; // UNKNOWN_TOPIC_OR_PARTITION
            }
        }

        response.responses.push(topic_result);
    }

    Ok(response)
}
