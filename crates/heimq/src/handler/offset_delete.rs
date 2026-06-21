//! OffsetDelete request handler (API Key 47)
//!
//! Deletes committed offsets for specified (group, topic, partition) tuples.
//! Used by kafka-consumer-groups.sh --delete-offsets.

use crate::error::Result;
use crate::storage::{OffsetStore, RequestContext};
use bytes::Bytes;
use kafka_protocol::messages::offset_delete_request::OffsetDeleteRequest;
use kafka_protocol::messages::offset_delete_response::{
    OffsetDeleteResponsePartition, OffsetDeleteResponseTopic,
};
use kafka_protocol::messages::OffsetDeleteResponse;
use kafka_protocol::protocol::{Decodable, StrBytes};
use std::sync::Arc;

pub fn handle(
    api_version: i16,
    body: &[u8],
    offset_store: &Arc<dyn OffsetStore>,
) -> Result<OffsetDeleteResponse> {
    handle_with_context(api_version, body, offset_store, &RequestContext::ANONYMOUS)
}

pub fn handle_with_context(
    api_version: i16,
    body: &[u8],
    offset_store: &Arc<dyn OffsetStore>,
    ctx: &RequestContext,
) -> Result<OffsetDeleteResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    let request = match OffsetDeleteRequest::decode(&mut buf, api_version) {
        Ok(r) => r,
        Err(_) => return Ok(OffsetDeleteResponse::default()),
    };

    let group_id = request.group_id.0.as_str();
    let mut response = OffsetDeleteResponse::default();

    for topic in &request.topics {
        let topic_name = topic.name.0.as_str();
        let mut response_partitions = Vec::new();
        for partition in &topic.partitions {
            offset_store.delete_offset_with_context(
                ctx,
                group_id,
                topic_name,
                partition.partition_index,
            );
            let mut rp = OffsetDeleteResponsePartition::default();
            rp.partition_index = partition.partition_index;
            rp.error_code = 0;
            response_partitions.push(rp);
        }
        let mut rt = OffsetDeleteResponseTopic::default();
        rt.name =
            kafka_protocol::messages::TopicName(StrBytes::from_string(topic_name.to_string()));
        rt.partitions = response_partitions;
        response.topics.push(rt);
    }

    Ok(response)
}
