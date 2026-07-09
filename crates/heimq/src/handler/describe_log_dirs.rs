//! DescribeLogDirs request handler (API Key 35)
//!
//! Returns log directory info for the in-memory backend. Reports a synthetic
//! "memory://" log dir with correct partition offsets and zero-byte sizes
//! (in-memory storage has no on-disk footprint).

use crate::error::Result;
use crate::storage::LogBackend;
use bytes::Bytes;
use heimq_protocol::messages::describe_log_dirs_request::DescribeLogDirsRequest;
use heimq_protocol::messages::describe_log_dirs_response::{
    DescribeLogDirsPartition, DescribeLogDirsResult, DescribeLogDirsTopic,
};
use heimq_protocol::messages::DescribeLogDirsResponse;
use heimq_protocol::messages::TopicName;
use heimq_protocol::protocol::{Decodable, StrBytes};
use std::sync::Arc;

const MEMORY_LOG_DIR: &str = "memory://";

pub fn handle(
    api_version: i16,
    body: &[u8],
    storage: &Arc<dyn LogBackend>,
) -> Result<DescribeLogDirsResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    let request = match DescribeLogDirsRequest::decode(&mut buf, api_version) {
        Ok(r) => r,
        Err(_) => return Ok(DescribeLogDirsResponse::default()),
    };

    // Determine which topics to describe.
    let topic_partitions: Vec<(String, Vec<i32>)> = match &request.topics {
        None => {
            // All topics.
            storage
                .get_all_topic_metadata()
                .into_iter()
                .map(|(name, np)| (name, (0..np).collect()))
                .collect()
        }
        Some(topics) => topics
            .iter()
            .map(|t| {
                let partitions = if t.partitions.is_empty() {
                    // All partitions for this topic.
                    let np = storage
                        .topic(t.topic.0.as_str())
                        .map(|tl| tl.num_partitions())
                        .unwrap_or(0);
                    (0..np).collect()
                } else {
                    t.partitions.clone()
                };
                (t.topic.0.as_str().to_string(), partitions)
            })
            .collect(),
    };

    let mut dir_topics: Vec<DescribeLogDirsTopic> = Vec::new();
    for (topic_name, partitions) in topic_partitions {
        let mut dps = Vec::new();
        for pid in partitions {
            let mut dp = DescribeLogDirsPartition::default();
            dp.partition_index = pid;
            dp.partition_size = 0;
            dp.offset_lag = 0;
            dp.is_future_key = false;
            dps.push(dp);
        }
        let mut dt = DescribeLogDirsTopic::default();
        dt.name = TopicName(StrBytes::from_string(topic_name));
        dt.partitions = dps;
        dir_topics.push(dt);
    }

    let mut result = DescribeLogDirsResult::default();
    result.error_code = 0;
    result.log_dir = StrBytes::from_static_str(MEMORY_LOG_DIR);
    result.topics = dir_topics;

    let mut response = DescribeLogDirsResponse::default();
    response.results = vec![result];
    Ok(response)
}
