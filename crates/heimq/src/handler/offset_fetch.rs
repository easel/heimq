//! OffsetFetch request handler (API Key 9)

use crate::error::Result;
use crate::storage::OffsetStore;
use bytes::Buf;
use kafka_protocol::messages::offset_fetch_response::{
    OffsetFetchResponsePartition, OffsetFetchResponseTopic,
};
use kafka_protocol::messages::{OffsetFetchResponse, TopicName};
use kafka_protocol::protocol::StrBytes;
use std::io::Cursor;
use std::sync::Arc;

/// Handle OffsetFetch request
pub fn handle(
    _api_version: i16,
    body: &[u8],
    offset_store: &Arc<dyn OffsetStore>,
) -> Result<OffsetFetchResponse> {
    let mut response = OffsetFetchResponse::default();
    let mut cursor = Cursor::new(body);

    // group_id (STRING)
    let group_id = read_string(&mut cursor).unwrap_or_default();

    // topics array (null = all topics)
    if cursor.remaining() < 4 {
        return Ok(response);
    }
    let topic_count = cursor.get_i32();

    if topic_count < 0 {
        // Fetch all offsets for this group
        let all_offsets = offset_store.fetch_all_for_group(&group_id);

        // Group by topic
        let mut topic_map: std::collections::HashMap<String, Vec<(i32, i64, Option<String>)>> =
            std::collections::HashMap::new();

        for ((topic, partition), committed) in all_offsets {
            topic_map
                .entry(topic)
                .or_default()
                .push((partition, committed.offset, committed.metadata));
        }

        for (topic_name, partitions) in topic_map {
            let mut topic_response = OffsetFetchResponseTopic::default();
            topic_response.name = TopicName(StrBytes::from_string(topic_name));

            for (partition, offset, metadata) in partitions {
                let mut partition_response = OffsetFetchResponsePartition::default();
                partition_response.partition_index = partition;
                partition_response.committed_offset = offset;
                partition_response.metadata = metadata.map(StrBytes::from_string);
                partition_response.error_code = 0;
                topic_response.partitions.push(partition_response);
            }

            response.topics.push(topic_response);
        }
    } else {
        // Fetch specific topics/partitions
        for _ in 0..topic_count {
            let topic_name = read_string(&mut cursor).unwrap_or_default();

            let mut topic_response = OffsetFetchResponseTopic::default();
            topic_response.name = TopicName(StrBytes::from_string(topic_name.clone()));

            // partitions array
            if cursor.remaining() < 4 {
                break;
            }
            let partition_count = cursor.get_i32();

            for _ in 0..partition_count {
                if cursor.remaining() < 4 {
                    break;
                }
                let partition = cursor.get_i32();

                let mut partition_response = OffsetFetchResponsePartition::default();
                partition_response.partition_index = partition;

                if let Some(committed) = offset_store.fetch(&group_id, &topic_name, partition) {
                    partition_response.committed_offset = committed.offset;
                    partition_response.metadata = committed.metadata.map(StrBytes::from_string);
                    partition_response.error_code = 0;
                } else {
                    partition_response.committed_offset = -1; // No offset committed
                    partition_response.error_code = 0;
                }

                topic_response.partitions.push(partition_response);
            }

            response.topics.push(topic_response);
        }
    }

    response.error_code = 0;
    Ok(response)
}

fn read_string(cursor: &mut Cursor<&[u8]>) -> Option<String> {
    if cursor.remaining() < 2 {
        return None;
    }
    let len = cursor.get_i16();
    if len < 0 {
        return None;
    }
    if cursor.remaining() < len as usize {
        return None;
    }
    let mut buf = vec![0u8; len as usize];
    cursor.copy_to_slice(&mut buf);
    Some(String::from_utf8_lossy(&buf).to_string())
}
