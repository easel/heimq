//! Metadata request handler (API Key 3)

use crate::error::Result;
use crate::storage::{ClusterView, LogBackend};
use bytes::Bytes;
use heimq_protocol::messages::metadata_request::MetadataRequest;
use heimq_protocol::messages::metadata_response::{
    MetadataResponseBroker, MetadataResponsePartition, MetadataResponseTopic,
};
use heimq_protocol::messages::{BrokerId, MetadataResponse, TopicName};
use heimq_protocol::protocol::{Decodable, StrBytes};
use std::sync::Arc;

pub fn handle(
    api_version: i16,
    body: &[u8],
    storage: &Arc<dyn LogBackend>,
    cluster_view: &dyn ClusterView,
) -> Result<MetadataResponse> {
    // Empty body is treated as "all topics" request (Kafka spec: null topics array).
    let request: MetadataRequest = if body.is_empty() {
        MetadataRequest::default()
    } else {
        let mut buf = Bytes::copy_from_slice(body);
        MetadataRequest::decode(&mut buf, api_version).unwrap_or_default()
    };

    let self_broker = cluster_view.self_broker();
    let mut response = MetadataResponse::default();

    // Advertise the full cluster topology from the ClusterView. For a
    // single-node view this is just self; for a multi-node embedding (fjord)
    // this is every broker, so clients can spread connections across them.
    for b in cluster_view.brokers() {
        let mut broker = MetadataResponseBroker::default();
        broker.node_id = BrokerId(b.node_id);
        broker.host = StrBytes::from_string(b.host.clone());
        broker.port = b.port as i32;
        response.brokers.push(broker);
    }

    // Presented leader for a partition, from the ClusterView's (balanced)
    // assignment. Falls back to self if the view cannot resolve one — any
    // broker can serve, so self is always a safe answer.
    let leader_for = |topic: &str, partition: i32| -> i32 {
        cluster_view
            .partition_leader(topic, partition)
            .map(|b| b.node_id)
            .unwrap_or(self_broker.node_id)
    };

    if api_version >= 2 {
        response.cluster_id = Some(StrBytes::from_string(cluster_view.cluster_id()));
    }
    if api_version >= 1 {
        response.controller_id = BrokerId(self_broker.node_id);
    }

    // None means "all topics"; empty Some([]) also means all topics per Kafka spec (v1+)
    let requested_topics: Option<Vec<String>> = match &request.topics {
        None => None,
        Some(topics) if topics.is_empty() => None,
        Some(topics) => Some(
            topics
                .iter()
                .filter_map(|t| t.name.as_ref().map(|n| n.0.to_string()))
                .collect(),
        ),
    };

    let topic_metadata = match requested_topics {
        None => storage.get_all_topic_metadata(),
        Some(names) => names
            .iter()
            .filter_map(|name| {
                storage
                    .topic(name)
                    .map(|t| (name.clone(), t.num_partitions()))
            })
            .collect(),
    };

    let mut resolved_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (topic_name, num_partitions) in &topic_metadata {
        resolved_names.insert(topic_name.clone());
        let mut topic = MetadataResponseTopic::default();
        topic.name = Some(TopicName(StrBytes::from_string(topic_name.clone())));
        topic.error_code = 0;
        for partition_id in 0..*num_partitions {
            let mut partition = MetadataResponsePartition::default();
            partition.partition_index = partition_id;
            partition.error_code = 0;
            let leader = leader_for(topic_name, partition_id);
            partition.leader_id = BrokerId(leader);
            partition.replica_nodes = vec![BrokerId(leader)];
            partition.isr_nodes = vec![BrokerId(leader)];
            topic.partitions.push(partition);
        }
        response.topics.push(topic);
    }

    // Handle unknown topics in explicit request
    if let Some(names) = request.topics.as_ref() {
        for t in names {
            if let Some(name) = &t.name {
                let s = name.0.to_string();
                if !resolved_names.contains(&s) {
                    if storage.auto_create_topics() {
                        let topic =
                            storage.get_or_create_topic(&s, storage.default_num_partitions());
                        let mut topic_meta = MetadataResponseTopic::default();
                        topic_meta.name = Some(TopicName(StrBytes::from_string(s.clone())));
                        topic_meta.error_code = 0;
                        for partition_id in 0..topic.num_partitions() {
                            let mut partition = MetadataResponsePartition::default();
                            partition.partition_index = partition_id;
                            partition.error_code = 0;
                            let leader = leader_for(&s, partition_id);
                            partition.leader_id = BrokerId(leader);
                            partition.replica_nodes = vec![BrokerId(leader)];
                            partition.isr_nodes = vec![BrokerId(leader)];
                            topic_meta.partitions.push(partition);
                        }
                        response.topics.push(topic_meta);
                    } else {
                        let mut topic_meta = MetadataResponseTopic::default();
                        topic_meta.name = Some(TopicName(StrBytes::from_string(s)));
                        topic_meta.error_code = 3; // UNKNOWN_TOPIC_OR_PARTITION
                        response.topics.push(topic_meta);
                    }
                }
            }
        }
    }

    Ok(response)
}
