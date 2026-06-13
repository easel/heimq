//! CreateTopics request handler (API Key 19)

use crate::error::Result;
use crate::storage::LogBackend;
use bytes::Bytes;
use kafka_protocol::messages::create_topics_request::CreateTopicsRequest;
use kafka_protocol::messages::create_topics_response::CreatableTopicResult;
use kafka_protocol::messages::{CreateTopicsResponse, TopicName};
use kafka_protocol::protocol::{Decodable, StrBytes};
use std::sync::Arc;
use tracing::info;

pub fn handle(
    api_version: i16,
    body: &[u8],
    storage: &Arc<dyn LogBackend>,
) -> Result<CreateTopicsResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    let request = match CreateTopicsRequest::decode(&mut buf, api_version) {
        Ok(r) => r,
        Err(_) => return Ok(CreateTopicsResponse::default()),
    };

    let mut response = CreateTopicsResponse::default();

    for topic in &request.topics {
        let topic_name = topic.name.0.to_string();
        let partitions = if topic.num_partitions <= 0 {
            storage.default_num_partitions()
        } else {
            topic.num_partitions
        };

        let mut topic_result = CreatableTopicResult::default();
        topic_result.name = TopicName(StrBytes::from_string(topic_name.clone()));

        match storage.create_topic(&topic_name, partitions) {
            Ok(_) => {
                info!(topic = %topic_name, partitions = partitions, "Created topic");
                topic_result.error_code = 0;
                topic_result.num_partitions = partitions;
                topic_result.replication_factor = 1;
            }
            Err(_) => {
                topic_result.error_code = 36; // TOPIC_ALREADY_EXISTS
                let existing = storage
                    .topic(&topic_name)
                    .expect("topic exists after duplicate create");
                topic_result.num_partitions = existing.num_partitions();
                topic_result.replication_factor = 1;
            }
        }

        response.topics.push(topic_result);
    }

    Ok(response)
}
