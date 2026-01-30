//! Produce request handler (API Key 0)

use crate::error::Result;
use crate::storage::Storage;
use bytes::Buf;
use kafka_protocol::messages::produce_response::{
    PartitionProduceResponse, TopicProduceResponse,
};
use kafka_protocol::messages::{ProduceResponse, TopicName};
use kafka_protocol::protocol::StrBytes;
use std::io::Cursor;
use std::sync::Arc;
use tracing::debug;

/// Handle Produce request
pub fn handle(
    _api_version: i16,
    body: &[u8],
    storage: &Arc<Storage>,
) -> Result<ProduceResponse> {
    let mut response = ProduceResponse::default();
    let mut cursor = Cursor::new(body);

    // Parse request
    // acks (INT16)
    if cursor.remaining() < 2 {
        return Ok(response);
    }
    let _acks = cursor.get_i16();

    // timeout (INT32)
    if cursor.remaining() < 4 {
        return Ok(response);
    }
    let _timeout = cursor.get_i32();

    // topic_data array
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

        let mut topic_response = TopicProduceResponse::default();
        topic_response.name = TopicName(StrBytes::from_string(topic_name.clone()));

        // partition_data array
        if cursor.remaining() < 4 {
            break;
        }
        let partition_count = cursor.get_i32();

        for _ in 0..partition_count {
            // partition index
            if cursor.remaining() < 4 {
                break;
            }
            let partition = cursor.get_i32();

            // record set size
            if cursor.remaining() < 4 {
                break;
            }
            let record_set_size = cursor.get_i32();

            let mut partition_response = PartitionProduceResponse::default();
            partition_response.index = partition;

            if record_set_size > 0 && cursor.remaining() >= record_set_size as usize {
                // Extract record batch data
                let pos = cursor.position() as usize;
                let record_data = &body[pos..pos + record_set_size as usize];
                cursor.advance(record_set_size as usize);

                // Append to storage
                match storage.append(&topic_name, partition, record_data) {
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
                        partition_response.log_append_time_ms = chrono::Utc::now().timestamp_millis();
                    }
                    Err(e) => {
                        partition_response.error_code = e.to_error_code();
                        partition_response.base_offset = -1;
                    }
                }
            } else {
                partition_response.error_code = 87; // INVALID_RECORD
            }

            topic_response.partition_responses.push(partition_response);
        }

        response.responses.push(topic_response);
    }

    Ok(response)
}
