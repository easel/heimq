//! DescribeTopicPartitions request handler (API Key 75)
//!
//! Returns partition metadata for requested topics. Added in Kafka 3.7 (KIP-848)
//! as a more efficient alternative to Metadata for clients that only need partition
//! assignments. For our single-node in-memory backend, all partitions are local
//! with a zero-epoch and no replication.

use crate::error::Result;
use crate::storage::{ClusterView, LogBackend};
use bytes::Bytes;
use kafka_protocol::messages::describe_topic_partitions_request::DescribeTopicPartitionsRequest;
use kafka_protocol::messages::describe_topic_partitions_response::{
    DescribeTopicPartitionsResponse, DescribeTopicPartitionsResponsePartition,
    DescribeTopicPartitionsResponseTopic,
};
use kafka_protocol::messages::{BrokerId, TopicName};
use kafka_protocol::protocol::{Decodable, StrBytes};
use std::sync::Arc;

pub fn handle(
    api_version: i16,
    body: &[u8],
    storage: &Arc<dyn LogBackend>,
    cluster_view: &dyn ClusterView,
) -> Result<DescribeTopicPartitionsResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    let request = match DescribeTopicPartitionsRequest::decode(&mut buf, api_version) {
        Ok(r) => r,
        Err(_) => return Ok(DescribeTopicPartitionsResponse::default()),
    };

    let self_broker = cluster_view.self_broker();
    let node_id = BrokerId(self_broker.node_id);

    let mut response = DescribeTopicPartitionsResponse::default();

    for topic_req in &request.topics {
        let topic_name = topic_req.name.0.as_str();
        let mut topic_result = DescribeTopicPartitionsResponseTopic::default();
        topic_result.name = Some(TopicName(StrBytes::from_string(topic_name.to_string())));
        // topic_id: leave as nil UUID (default) since we don't track UUIDs.
        topic_result.is_internal = false;

        match storage.topic(topic_name) {
            Some(topic_log) => {
                topic_result.error_code = 0;
                let mut partitions = Vec::new();
                for pid in 0..topic_log.num_partitions() {
                    let mut p = DescribeTopicPartitionsResponsePartition::default();
                    p.partition_index = pid;
                    p.error_code = 0;
                    p.leader_id = node_id;
                    p.leader_epoch = 0;
                    p.replica_nodes = vec![node_id];
                    p.isr_nodes = vec![node_id];
                    p.offline_replicas = vec![];
                    partitions.push(p);
                }
                topic_result.partitions = partitions;
            }
            None => {
                topic_result.error_code = 3; // UNKNOWN_TOPIC_OR_PARTITION
            }
        }

        response.topics.push(topic_result);
    }

    response.next_cursor = None;
    Ok(response)
}
