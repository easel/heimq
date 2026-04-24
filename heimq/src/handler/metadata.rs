//! Metadata request handler (API Key 3)

use crate::config::Config;
use crate::error::Result;
use crate::storage::LogBackend;
use bytes::Buf;
use kafka_protocol::messages::metadata_response::{
    MetadataResponseBroker, MetadataResponsePartition, MetadataResponseTopic,
};
use kafka_protocol::messages::{BrokerId, MetadataResponse, TopicName};
use kafka_protocol::protocol::StrBytes;
use std::io::Cursor;
use std::sync::Arc;

/// Handle Metadata request
pub fn handle(
    api_version: i16,
    body: &[u8],
    storage: &Arc<dyn LogBackend>,
    config: &Arc<Config>,
) -> Result<MetadataResponse> {
    let mut response = MetadataResponse::default();

    // Add broker information (single node)
    let mut broker = MetadataResponseBroker::default();
    broker.node_id = BrokerId(config.broker_id);
    broker.host = StrBytes::from_string(if config.host == "0.0.0.0" {
        "127.0.0.1".to_string()  // Use IPv4 to avoid IPv6 resolution issues
    } else {
        config.host.clone()
    });
    broker.port = config.port as i32;
    response.brokers.push(broker);

    // Parse request to get topic list
    let topics = parse_topic_list(body, api_version);

    // Get all topics if request is empty, otherwise filter
    let topic_metadata = if topics.is_empty() {
        storage.get_all_topic_metadata()
    } else {
        topics
            .iter()
            .filter_map(|name| {
                storage
                    .topic(name)
                    .map(|t| (name.clone(), t.num_partitions()))
            })
            .collect()
    };

    // Add topic metadata
    for (topic_name, num_partitions) in topic_metadata {
        let mut topic = MetadataResponseTopic::default();
        topic.name = Some(TopicName(StrBytes::from_string(topic_name.clone())));
        topic.error_code = 0;

        // Add partition metadata
        for partition_id in 0..num_partitions {
            let mut partition = MetadataResponsePartition::default();
            partition.partition_index = partition_id;
            partition.error_code = 0;
            partition.leader_id = BrokerId(config.broker_id);
            partition.replica_nodes = vec![BrokerId(config.broker_id)];
            partition.isr_nodes = vec![BrokerId(config.broker_id)];
            topic.partitions.push(partition);
        }

        response.topics.push(topic);
    }

    // Handle unknown topics in request
    for topic_name in topics {
        if !response.topics.iter().any(|t| {
            t.name
                .as_ref()
                .map(|n| n.0.as_str() == topic_name)
                .unwrap_or(false)
        }) {
            // Auto-create topic if enabled
            if storage.config().auto_create_topics {
                let topic = storage.get_or_create_topic(&topic_name, config.default_partitions);
                let mut topic_meta = MetadataResponseTopic::default();
                topic_meta.name = Some(TopicName(StrBytes::from_string(topic_name)));
                topic_meta.error_code = 0;

                for partition_id in 0..topic.num_partitions() {
                    let mut partition = MetadataResponsePartition::default();
                    partition.partition_index = partition_id;
                    partition.error_code = 0;
                    partition.leader_id = BrokerId(config.broker_id);
                    partition.replica_nodes = vec![BrokerId(config.broker_id)];
                    partition.isr_nodes = vec![BrokerId(config.broker_id)];
                    topic_meta.partitions.push(partition);
                }

                response.topics.push(topic_meta);
            } else {
                let mut topic = MetadataResponseTopic::default();
                topic.name = Some(TopicName(StrBytes::from_string(topic_name)));
                topic.error_code = 3; // UNKNOWN_TOPIC_OR_PARTITION
                response.topics.push(topic);
            }
        }
    }

    // Set cluster ID for newer versions
    if api_version >= 2 {
        response.cluster_id = Some(StrBytes::from_string(config.cluster_id.clone()));
    }

    // Set controller ID for newer versions
    if api_version >= 1 {
        response.controller_id = BrokerId(config.broker_id);
    }

    Ok(response)
}

/// Parse topic list from request body
fn parse_topic_list(body: &[u8], _api_version: i16) -> Vec<String> {
    if body.is_empty() {
        return Vec::new();
    }

    let mut cursor = Cursor::new(body);
    let mut topics = Vec::new();

    // Read array length
    if cursor.remaining() < 4 {
        return topics;
    }
    let count = cursor.get_i32();

    if count <= 0 {
        return topics; // All topics requested
    }

    for _ in 0..count {
        if cursor.remaining() < 2 {
            break;
        }

        let len = cursor.get_i16();
        if len < 0 || cursor.remaining() < len as usize {
            break;
        }

        let mut buf = vec![0u8; len as usize];
        cursor.copy_to_slice(&mut buf);
        topics.push(String::from_utf8_lossy(&buf).to_string());
    }

    topics
}
