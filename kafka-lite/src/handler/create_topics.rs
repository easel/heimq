//! CreateTopics request handler (API Key 19)

use crate::error::Result;
use crate::storage::Storage;
use bytes::Buf;
use kafka_protocol::messages::create_topics_response::CreatableTopicResult;
use kafka_protocol::messages::{CreateTopicsResponse, TopicName};
use kafka_protocol::protocol::StrBytes;
use std::io::Cursor;
use std::sync::Arc;
use tracing::info;

/// Handle CreateTopics request
pub fn handle(
    _api_version: i16,
    body: &[u8],
    storage: &Arc<Storage>,
) -> Result<CreateTopicsResponse> {
    let mut response = CreateTopicsResponse::default();
    let mut cursor = Cursor::new(body);

    // topics array
    if cursor.remaining() < 4 {
        return Ok(response);
    }
    let topic_count = cursor.get_i32();

    for _ in 0..topic_count {
        // topic name
        if cursor.remaining() < 2 {
            break;
        }
        let name_len = cursor.get_i16();
        if name_len < 0 || cursor.remaining() < name_len as usize {
            break;
        }
        let mut name_buf = vec![0u8; name_len as usize];
        cursor.copy_to_slice(&mut name_buf);
        let topic_name = String::from_utf8_lossy(&name_buf).to_string();

        // num_partitions (INT32)
        if cursor.remaining() < 4 {
            break;
        }
        let num_partitions = cursor.get_i32();

        // replication_factor (INT16)
        if cursor.remaining() < 2 {
            break;
        }
        let _replication_factor = cursor.get_i16();

        // assignments array - skip for simplicity
        if cursor.remaining() < 4 {
            break;
        }
        let assignment_count = cursor.get_i32();
        for _ in 0..assignment_count {
            if cursor.remaining() < 4 {
                break;
            }
            let _partition_index = cursor.get_i32();
            if cursor.remaining() < 4 {
                break;
            }
            let broker_count = cursor.get_i32();
            for _ in 0..broker_count {
                if cursor.remaining() < 4 {
                    break;
                }
                let _broker_id = cursor.get_i32();
            }
        }

        // configs array - skip for simplicity
        if cursor.remaining() < 4 {
            break;
        }
        let config_count = cursor.get_i32();
        for _ in 0..config_count {
            // Skip config name
            if cursor.remaining() < 2 {
                break;
            }
            let name_len = cursor.get_i16();
            if name_len > 0 && cursor.remaining() >= name_len as usize {
                cursor.advance(name_len as usize);
            }
            // Skip config value
            if cursor.remaining() < 2 {
                break;
            }
            let value_len = cursor.get_i16();
            if value_len > 0 && cursor.remaining() >= value_len as usize {
                cursor.advance(value_len as usize);
            }
        }

        let mut topic_result = CreatableTopicResult::default();
        topic_result.name = TopicName(StrBytes::from_string(topic_name.clone()));

        // Use default partitions if -1
        let partitions = if num_partitions <= 0 {
            storage.config().default_partitions
        } else {
            num_partitions
        };

        // Create the topic
        match storage.create_topic(&topic_name, partitions) {
            Ok(_) => {
                info!(topic = %topic_name, partitions = partitions, "Created topic");
                topic_result.error_code = 0;
                topic_result.num_partitions = partitions;
                topic_result.replication_factor = 1; // Single node
            }
            Err(_) => {
                // Topic already exists
                topic_result.error_code = 36; // TOPIC_ALREADY_EXISTS
                if let Some(topic) = storage.get_topic(&topic_name) {
                    topic_result.num_partitions = topic.num_partitions();
                    topic_result.replication_factor = 1;
                }
            }
        }

        response.topics.push(topic_result);
    }

    Ok(response)
}
