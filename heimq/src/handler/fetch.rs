//! Fetch request handler (API Key 1)

use crate::error::Result;
use crate::storage::LogBackend;
use bytes::Buf;
use kafka_protocol::messages::fetch_response::{
    FetchableTopicResponse, PartitionData,
};
use kafka_protocol::messages::{FetchResponse, TopicName};
use kafka_protocol::protocol::StrBytes;
use std::io::Cursor;
use std::sync::Arc;
use tracing::debug;

/// Handle Fetch request
pub fn handle(
    api_version: i16,
    body: &[u8],
    storage: &Arc<dyn LogBackend>,
) -> Result<FetchResponse> {
    let mut response = FetchResponse::default();
    let mut cursor = Cursor::new(body);

    // Parse request header
    // replica_id (INT32)
    if cursor.remaining() < 4 {
        return Ok(response);
    }
    let _replica_id = cursor.get_i32();

    // max_wait_ms (INT32)
    if cursor.remaining() < 4 {
        return Ok(response);
    }
    let _max_wait_ms = cursor.get_i32();

    // min_bytes (INT32)
    if cursor.remaining() < 4 {
        return Ok(response);
    }
    let _min_bytes = cursor.get_i32();

    // max_bytes (INT32) - version 3+
    let max_bytes = if api_version >= 3 && cursor.remaining() >= 4 {
        cursor.get_i32()
    } else {
        1024 * 1024 // Default 1MB
    };

    // isolation_level (INT8) - version 4+
    if api_version >= 4 && cursor.remaining() >= 1 {
        let _isolation_level = cursor.get_i8();
    }

    // session_id (INT32) - version 7+
    if api_version >= 7 && cursor.remaining() >= 4 {
        let _session_id = cursor.get_i32();
    }

    // session_epoch (INT32) - version 7+
    if api_version >= 7 && cursor.remaining() >= 4 {
        let _session_epoch = cursor.get_i32();
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

        let mut topic_response = FetchableTopicResponse::default();
        topic_response.topic = TopicName(StrBytes::from_string(topic_name.clone()));

        // partitions array
        if cursor.remaining() < 4 {
            break;
        }
        let partition_count = cursor.get_i32();

        for _ in 0..partition_count {
            // partition (INT32)
            if cursor.remaining() < 4 {
                break;
            }
            let partition = cursor.get_i32();

            // current_leader_epoch (INT32) - version 9+
            if api_version >= 9 && cursor.remaining() >= 4 {
                let _current_leader_epoch = cursor.get_i32();
            }

            // fetch_offset (INT64)
            if cursor.remaining() < 8 {
                break;
            }
            let fetch_offset = cursor.get_i64();

            // last_fetched_epoch (INT32) - version 12+
            if api_version >= 12 && cursor.remaining() >= 4 {
                let _last_fetched_epoch = cursor.get_i32();
            }

            // log_start_offset (INT64) - version 5+
            if api_version >= 5 && cursor.remaining() >= 8 {
                let _log_start_offset = cursor.get_i64();
            }

            // partition_max_bytes (INT32)
            if cursor.remaining() < 4 {
                break;
            }
            let partition_max_bytes = cursor.get_i32();

            let mut partition_data = PartitionData::default();
            partition_data.partition_index = partition;

            // Fetch from storage
            match storage.fetch(
                &topic_name,
                partition,
                fetch_offset,
                partition_max_bytes.min(max_bytes),
            ) {
                Ok((records, high_watermark)) => {
                    debug!(
                        topic = %topic_name,
                        partition = partition,
                        offset = fetch_offset,
                        bytes = records.len(),
                        "Fetched records"
                    );
                    partition_data.error_code = 0;
                    partition_data.high_watermark = high_watermark;

                    // Set log start offset
                    partition_data.log_start_offset = storage
                        .log_start_offset(&topic_name, partition)
                        .unwrap_or(-1);

                    // Set records as raw bytes
                    if !records.is_empty() {
                        partition_data.records = Some(bytes::Bytes::from(records).into());
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
