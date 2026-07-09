//! CreateTopics request handler (API Key 19)

use crate::error::Result;
use crate::storage::LogBackend;
use bytes::Bytes;
use heimq_protocol::messages::create_topics_request::CreateTopicsRequest;
use heimq_protocol::messages::create_topics_response::CreatableTopicResult;
use heimq_protocol::messages::{CreateTopicsResponse, TopicName};
use heimq_protocol::protocol::{Decodable, StrBytes};
use std::sync::Arc;
use tracing::{info, warn};

/// Upper bound on partitions per topic. Far above any real single-node use, but
/// low enough that the per-partition allocation can't be turned into a DoS by an
/// attacker-supplied `num_partitions` (which can be up to i32::MAX on the wire).
const MAX_PARTITIONS: i32 = 100_000;

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

        // Reject an absurd partition count before allocating per-partition state:
        // an attacker-supplied num_partitions (up to i32::MAX) would otherwise
        // reserve a multi-GB partition vector. INVALID_PARTITIONS (37).
        if partitions > MAX_PARTITIONS {
            warn!(topic = %topic_name, partitions, "Rejecting create-topic: partition count exceeds limit");
            topic_result.error_code = 37; // INVALID_PARTITIONS
            response.topics.push(topic_result);
            continue;
        }

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
