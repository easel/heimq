//! DeleteRecords request handler (API Key 21)
//!
//! Advances the log start offset (low watermark) for each specified partition,
//! making records before that offset inaccessible. The response reports the
//! resulting low watermark for each partition.

use crate::error::Result;
use crate::storage::LogBackend;
use bytes::Bytes;
use heimq_protocol::messages::delete_records_request::DeleteRecordsRequest;
use heimq_protocol::messages::delete_records_response::{
    DeleteRecordsPartitionResult, DeleteRecordsTopicResult,
};
use heimq_protocol::messages::DeleteRecordsResponse;
use heimq_protocol::protocol::{Decodable, StrBytes};
use std::sync::Arc;

pub fn handle(
    api_version: i16,
    body: &[u8],
    storage: &Arc<dyn LogBackend>,
) -> Result<DeleteRecordsResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    let request = match DeleteRecordsRequest::decode(&mut buf, api_version) {
        Ok(r) => r,
        Err(_) => return Ok(DeleteRecordsResponse::default()),
    };

    let mut response = DeleteRecordsResponse::default();

    for topic in &request.topics {
        let topic_name = topic.name.0.as_str();
        let mut topic_result = DeleteRecordsTopicResult::default();
        topic_result.name =
            heimq_protocol::messages::TopicName(StrBytes::from_string(topic_name.to_string()));

        for partition in &topic.partitions {
            let mut pr = DeleteRecordsPartitionResult::default();
            pr.partition_index = partition.partition_index;

            match storage.topic(topic_name) {
                None => {
                    pr.error_code = 3; // UNKNOWN_TOPIC_OR_PARTITION
                    pr.low_watermark = -1;
                }
                Some(topic_log) => {
                    match topic_log.partition(partition.partition_index) {
                        Err(_) => {
                            pr.error_code = 3; // UNKNOWN_TOPIC_OR_PARTITION
                            pr.low_watermark = -1;
                        }
                        Ok(partition_log) => {
                            let offset = partition.offset;
                            // -2 = delete up to the current high watermark
                            let target = if offset == -1 {
                                partition_log.high_watermark()
                            } else {
                                offset
                            };
                            match partition_log.truncate_before(target) {
                                Ok(()) => {
                                    pr.error_code = 0;
                                    pr.low_watermark = partition_log.log_start_offset();
                                }
                                Err(_) => {
                                    pr.error_code = 1; // OFFSET_OUT_OF_RANGE
                                    pr.low_watermark = partition_log.log_start_offset();
                                }
                            }
                        }
                    }
                }
            }

            topic_result.partitions.push(pr);
        }

        response.topics.push(topic_result);
    }

    Ok(response)
}
