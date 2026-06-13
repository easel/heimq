//! ListOffsets request handler (API Key 2)

use crate::error::Result;
use crate::storage::LogBackend;
use bytes::Buf;
use kafka_protocol::messages::list_offsets_response::{
    ListOffsetsPartitionResponse, ListOffsetsTopicResponse,
};
use kafka_protocol::messages::{ListOffsetsResponse, TopicName};
use kafka_protocol::protocol::StrBytes;
use std::io::Cursor;
use std::sync::Arc;

/// Handle ListOffsets request
pub fn handle(
    api_version: i16,
    body: &[u8],
    storage: &Arc<dyn LogBackend>,
) -> Result<ListOffsetsResponse> {
    let mut response = ListOffsetsResponse::default();
    let mut cursor = Cursor::new(body);

    // replica_id (INT32)
    if cursor.remaining() < 4 {
        return Ok(response);
    }
    let _replica_id = cursor.get_i32();

    // isolation_level (INT8) - version 2+
    if api_version >= 2 && cursor.remaining() >= 1 {
        let _isolation_level = cursor.get_i8();
    }

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

        let mut topic_response = ListOffsetsTopicResponse::default();
        topic_response.name = TopicName(StrBytes::from_string(topic_name.clone()));

        // partitions array
        if cursor.remaining() < 4 {
            break;
        }
        let partition_count = cursor.get_i32();

        for _ in 0..partition_count {
            // partition index (INT32)
            if cursor.remaining() < 4 {
                break;
            }
            let partition = cursor.get_i32();

            // current_leader_epoch (INT32) - version 4+
            if api_version >= 4 && cursor.remaining() >= 4 {
                let _current_leader_epoch = cursor.get_i32();
            }

            // timestamp (INT64)
            if cursor.remaining() < 8 {
                break;
            }
            let timestamp = cursor.get_i64();

            let mut partition_response = ListOffsetsPartitionResponse::default();
            partition_response.partition_index = partition;

            // Get offset based on timestamp
            // -1 = latest, -2 = earliest, otherwise find by timestamp
            match timestamp {
                -1 => {
                    // Latest offset (high watermark)
                    match storage.high_watermark(&topic_name, partition) {
                        Ok(hw) => {
                            partition_response.offset = hw;
                            partition_response.timestamp = -1;
                            partition_response.error_code = 0;
                        }
                        Err(e) => {
                            partition_response.error_code = e.to_error_code();
                        }
                    }
                }
                -2 => {
                    // Earliest offset
                    match storage.log_start_offset(&topic_name, partition) {
                        Ok(offset) => {
                            partition_response.offset = offset;
                            partition_response.timestamp = -2;
                            partition_response.error_code = 0;
                        }
                        Err(e) => {
                            partition_response.error_code = e.to_error_code();
                        }
                    }
                }
                _ => {
                    // Timestamp-based lookup - for simplicity, return earliest
                    match storage.log_start_offset(&topic_name, partition) {
                        Ok(offset) => {
                            partition_response.offset = offset;
                            partition_response.timestamp = timestamp;
                            partition_response.error_code = 0;
                        }
                        Err(e) => {
                            partition_response.error_code = e.to_error_code();
                        }
                    }
                }
            }

            topic_response.partitions.push(partition_response);
        }

        response.topics.push(topic_response);
    }

    Ok(response)
}
