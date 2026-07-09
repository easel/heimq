//! DescribeCluster request handler (API Key 60)
//!
//! Returns the single-node cluster topology from the ClusterView.

use crate::error::Result;
use crate::storage::ClusterView;
use bytes::Bytes;
use heimq_protocol::messages::describe_cluster_request::DescribeClusterRequest;
use heimq_protocol::messages::describe_cluster_response::DescribeClusterBroker;
use heimq_protocol::messages::BrokerId;
use heimq_protocol::messages::DescribeClusterResponse;
use heimq_protocol::protocol::{Decodable, StrBytes};

pub fn handle(
    api_version: i16,
    body: &[u8],
    cluster_view: &dyn ClusterView,
) -> Result<DescribeClusterResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    let _request = match DescribeClusterRequest::decode(&mut buf, api_version) {
        Ok(r) => r,
        Err(_) => return Ok(DescribeClusterResponse::default()),
    };

    let cluster_id = cluster_view.cluster_id();
    let brokers = cluster_view.brokers();

    let mut response = DescribeClusterResponse::default();
    response.error_code = 0;
    response.cluster_id = StrBytes::from_string(cluster_id);
    response.controller_id = BrokerId(brokers.first().map(|b| b.node_id).unwrap_or(1));
    response.brokers = brokers
        .iter()
        .map(|b| {
            let mut cb = DescribeClusterBroker::default();
            cb.broker_id = BrokerId(b.node_id);
            cb.host = StrBytes::from_string(b.host.clone());
            cb.port = b.port as i32;
            cb
        })
        .collect();
    Ok(response)
}
