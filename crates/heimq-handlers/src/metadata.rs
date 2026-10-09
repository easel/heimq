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

fn decode_request(api_version: i16, body: &[u8]) -> MetadataRequest {
    if body.is_empty() {
        MetadataRequest::default()
    } else {
        let mut buf = Bytes::copy_from_slice(body);
        MetadataRequest::decode(&mut buf, api_version).unwrap_or_default()
    }
}

fn requested_names(request: &MetadataRequest) -> Option<Vec<String>> {
    request
        .topics
        .as_ref()
        .filter(|topics| !topics.is_empty())
        .map(|topics| {
            topics
                .iter()
                .filter_map(|topic| topic.name.as_ref().map(|name| name.0.to_string()))
                .collect()
        })
}

/// Metadata handler for synchronous in-memory catalog backends.
pub fn handle(
    api_version: i16,
    body: &[u8],
    storage: &Arc<dyn LogBackend>,
    cluster_view: &dyn ClusterView,
) -> Result<MetadataResponse> {
    let request = decode_request(api_version, body);
    let metadata = match requested_names(&request) {
        None => storage.get_all_topic_metadata(),
        Some(names) => {
            let mut metadata = Vec::with_capacity(names.len());
            let mut missing = Vec::new();
            for name in names {
                if let Some(topic) = storage.topic(&name) {
                    metadata.push((name, topic.num_partitions()));
                } else {
                    missing.push(name);
                }
            }
            if storage.auto_create_topics() {
                for name in missing {
                    let topic =
                        storage.get_or_create_topic(&name, storage.default_num_partitions());
                    metadata.push((name, topic.num_partitions()));
                }
            }
            metadata
        }
    };
    render_metadata(api_version, &request, metadata, cluster_view)
}

/// Metadata handler for backends whose catalog operations require awaited I/O.
pub async fn handle_async(
    api_version: i16,
    body: &[u8],
    storage: &Arc<dyn LogBackend>,
    cluster_view: &dyn ClusterView,
) -> Result<MetadataResponse> {
    let request = decode_request(api_version, body);
    let metadata = match requested_names(&request) {
        None => storage.get_all_topic_metadata_async().await?,
        Some(names) => {
            let mut metadata = Vec::with_capacity(names.len());
            let mut missing = Vec::new();
            for name in names {
                if let Some(topic) = storage.topic_async(&name).await? {
                    metadata.push((name, topic.num_partitions()));
                } else {
                    missing.push(name);
                }
            }
            if storage.auto_create_topics() {
                for name in missing {
                    let topic = storage
                        .get_or_create_topic_async(&name, storage.default_num_partitions())
                        .await?;
                    metadata.push((name, topic.num_partitions()));
                }
            }
            metadata
        }
    };
    render_metadata(api_version, &request, metadata, cluster_view)
}

fn render_metadata(
    api_version: i16,
    request: &MetadataRequest,
    topic_metadata: Vec<(String, i32)>,
    cluster_view: &dyn ClusterView,
) -> Result<MetadataResponse> {
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
                    let mut topic_meta = MetadataResponseTopic::default();
                    topic_meta.name = Some(TopicName(StrBytes::from_string(s)));
                    topic_meta.error_code = 3; // UNKNOWN_TOPIC_OR_PARTITION
                    response.topics.push(topic_meta);
                }
            }
        }
    }

    Ok(response)
}
