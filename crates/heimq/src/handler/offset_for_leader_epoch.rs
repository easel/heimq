//! OffsetForLeaderEpoch request handler (API Key 23)
//!
//! Returns the current high-watermark as the end offset for each requested
//! partition. Leader epochs are not tracked in the in-memory backend (single
//! node, no election), so we always return leader_epoch=0 and the current
//! end offset.

use crate::error::Result;
use crate::storage::LogBackend;
use bytes::Bytes;
use kafka_protocol::messages::offset_for_leader_epoch_request::OffsetForLeaderEpochRequest;
use kafka_protocol::messages::offset_for_leader_epoch_response::{
    EpochEndOffset, OffsetForLeaderEpochResponse, OffsetForLeaderTopicResult,
};
use kafka_protocol::messages::TopicName;
use kafka_protocol::protocol::{Decodable, StrBytes};
use std::sync::Arc;

pub fn handle(
    api_version: i16,
    body: &[u8],
    storage: &Arc<dyn LogBackend>,
) -> Result<OffsetForLeaderEpochResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    let request = match OffsetForLeaderEpochRequest::decode(&mut buf, api_version) {
        Ok(r) => r,
        Err(_) => return Ok(OffsetForLeaderEpochResponse::default()),
    };

    let mut response = OffsetForLeaderEpochResponse::default();

    for topic_req in &request.topics {
        let topic_name = topic_req.topic.0.as_str();
        let mut partition_results = Vec::new();

        for partition_req in &topic_req.partitions {
            let pid = partition_req.partition;
            let mut ep = EpochEndOffset::default();
            ep.partition = pid;

            match storage.high_watermark(topic_name, pid) {
                Ok(hwm) => {
                    ep.error_code = 0;
                    ep.leader_epoch = 0;
                    ep.end_offset = hwm;
                }
                Err(_) => {
                    // UNKNOWN_TOPIC_OR_PARTITION
                    ep.error_code = 3;
                    ep.leader_epoch = -1;
                    ep.end_offset = -1;
                }
            }
            partition_results.push(ep);
        }

        let mut topic_result = OffsetForLeaderTopicResult::default();
        topic_result.topic = TopicName(StrBytes::from_string(topic_name.to_string()));
        topic_result.partitions = partition_results;
        response.topics.push(topic_result);
    }

    Ok(response)
}
