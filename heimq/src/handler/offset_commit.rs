//! OffsetCommit request handler (API Key 8)

use crate::consumer_group::ConsumerGroupManager;
use crate::error::Result;
use bytes::Buf;
use kafka_protocol::messages::offset_commit_response::{
    OffsetCommitResponsePartition, OffsetCommitResponseTopic,
};
use kafka_protocol::messages::{OffsetCommitResponse, TopicName};
use kafka_protocol::protocol::StrBytes;
use std::io::Cursor;
use std::sync::Arc;
use tracing::debug;

/// Handle OffsetCommit request
pub fn handle(
    api_version: i16,
    body: &[u8],
    consumer_groups: &Arc<ConsumerGroupManager>,
) -> Result<OffsetCommitResponse> {
    let mut response = OffsetCommitResponse::default();
    let mut cursor = Cursor::new(body);

    // group_id (STRING)
    let group_id = read_string(&mut cursor).unwrap_or_default();

    // generation_id (INT32) - version 1+
    let _generation_id = if api_version >= 1 && cursor.remaining() >= 4 {
        cursor.get_i32()
    } else {
        -1
    };

    // member_id (STRING) - version 1+
    let _member_id = if api_version >= 1 {
        read_string(&mut cursor).unwrap_or_default()
    } else {
        String::new()
    };

    // group_instance_id (NULLABLE_STRING) - version 7+
    if api_version >= 7 {
        let _ = read_string(&mut cursor);
    }

    // retention_time_ms (INT64) - version 2-4
    if api_version >= 2 && api_version <= 4 && cursor.remaining() >= 8 {
        let _retention = cursor.get_i64();
    }

    // topics array
    if cursor.remaining() < 4 {
        return Ok(response);
    }
    let topic_count = cursor.get_i32();

    let offset_store = consumer_groups.offset_store();

    for _ in 0..topic_count {
        // topic name
        let topic_name = read_string(&mut cursor).unwrap_or_default();

        let mut topic_response = OffsetCommitResponseTopic::default();
        topic_response.name = TopicName(StrBytes::from_string(topic_name.clone()));

        // partitions array
        if cursor.remaining() < 4 {
            break;
        }
        let partition_count = cursor.get_i32();

        for _ in 0..partition_count {
            // partition_index (INT32)
            if cursor.remaining() < 4 {
                break;
            }
            let partition = cursor.get_i32();

            // committed_offset (INT64)
            if cursor.remaining() < 8 {
                break;
            }
            let committed_offset = cursor.get_i64();

            // committed_leader_epoch (INT32) - version 6+
            let leader_epoch = if api_version >= 6 && cursor.remaining() >= 4 {
                cursor.get_i32()
            } else {
                -1
            };

            // commit_timestamp (INT64) - version 1 only
            if api_version == 1 && cursor.remaining() >= 8 {
                let _timestamp = cursor.get_i64();
            }

            // metadata (NULLABLE_STRING)
            let metadata = read_string(&mut cursor);

            // Commit the offset
            offset_store.commit(
                &group_id,
                &topic_name,
                partition,
                committed_offset,
                leader_epoch,
                metadata,
            );

            debug!(
                group = %group_id,
                topic = %topic_name,
                partition = partition,
                offset = committed_offset,
                "Committed offset"
            );

            let mut partition_response = OffsetCommitResponsePartition::default();
            partition_response.partition_index = partition;
            partition_response.error_code = 0;
            topic_response.partitions.push(partition_response);
        }

        response.topics.push(topic_response);
    }

    Ok(response)
}

fn read_string(cursor: &mut Cursor<&[u8]>) -> Option<String> {
    if cursor.remaining() < 2 {
        return None;
    }
    let len = cursor.get_i16();
    if len < 0 {
        return None; // null string
    }
    if cursor.remaining() < len as usize {
        return None;
    }
    let mut buf = vec![0u8; len as usize];
    cursor.copy_to_slice(&mut buf);
    Some(String::from_utf8_lossy(&buf).to_string())
}
