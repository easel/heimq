//! ListOffsets request handler (API Key 2)

use crate::error::{ErrorCode, Result};
use crate::storage::LogBackend;
use bytes::Bytes;
use heimq_protocol::messages::list_offsets_request::ListOffsetsRequest;
use heimq_protocol::messages::list_offsets_response::{
    ListOffsetsPartitionResponse, ListOffsetsTopicResponse,
};
use heimq_protocol::messages::{ListOffsetsResponse, TopicName};
use heimq_protocol::protocol::{Decodable, StrBytes};
use std::sync::Arc;

pub fn handle(
    api_version: i16,
    body: &[u8],
    storage: &Arc<dyn LogBackend>,
) -> Result<ListOffsetsResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    let request = match ListOffsetsRequest::decode(&mut buf, api_version) {
        Ok(r) => r,
        Err(_) => return Ok(ListOffsetsResponse::default()),
    };

    let mut response = ListOffsetsResponse::default();

    for topic in &request.topics {
        let topic_name = topic.name.0.to_string();
        let mut topic_response = ListOffsetsTopicResponse::default();
        topic_response.name = TopicName(StrBytes::from_string(topic_name.clone()));

        for partition in &topic.partitions {
            let mut partition_response = ListOffsetsPartitionResponse::default();
            partition_response.partition_index = partition.partition_index;

            match partition.timestamp {
                -1 => match storage.high_watermark(&topic_name, partition.partition_index) {
                    Ok(hw) => {
                        partition_response.offset = hw;
                        partition_response.timestamp = -1;
                        partition_response.error_code = 0;
                    }
                    Err(e) => {
                        partition_response.error_code = e.to_error_code();
                    }
                },
                -2 => match storage.log_start_offset(&topic_name, partition.partition_index) {
                    Ok(offset) => {
                        partition_response.offset = offset;
                        partition_response.timestamp = -2;
                        partition_response.error_code = 0;
                    }
                    Err(e) => {
                        partition_response.error_code = e.to_error_code();
                    }
                },
                ts => match storage.log_start_offset(&topic_name, partition.partition_index) {
                    Ok(offset) => {
                        partition_response.offset = offset;
                        partition_response.timestamp = ts;
                        partition_response.error_code = 0;
                    }
                    Err(e) => {
                        partition_response.error_code = e.to_error_code();
                    }
                },
            }

            topic_response.partitions.push(partition_response);
        }

        response.topics.push(topic_response);
    }

    Ok(response)
}
