//! DeleteTopics request handler (API Key 20)

use crate::error::Result;
use crate::storage::Storage;
use bytes::Buf;
use kafka_protocol::messages::delete_topics_response::DeletableTopicResult;
use kafka_protocol::messages::{DeleteTopicsResponse, TopicName};
use kafka_protocol::protocol::StrBytes;
use std::io::Cursor;
use std::sync::Arc;
use tracing::info;

/// Handle DeleteTopics request
pub fn handle(
    _api_version: i16,
    body: &[u8],
    storage: &Arc<Storage>,
) -> Result<DeleteTopicsResponse> {
    let mut response = DeleteTopicsResponse::default();
    let mut cursor = Cursor::new(body);

    // topic_names array (older versions) or topics array (newer versions)
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
