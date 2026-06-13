//! FindCoordinator request handler (API Key 10)

use crate::error::Result;
use crate::storage::ClusterView;
use kafka_protocol::messages::{BrokerId, FindCoordinatorResponse};
use kafka_protocol::protocol::StrBytes;

/// Handle FindCoordinator request
pub fn handle(
    _api_version: i16,
    _body: &[u8],
    cluster_view: &dyn ClusterView,
) -> Result<FindCoordinatorResponse> {
    let broker = cluster_view.find_coordinator("").unwrap_or_else(|_| cluster_view.self_broker());
    let mut response = FindCoordinatorResponse::default();
    response.error_code = 0;
    response.node_id = BrokerId(broker.node_id);
    response.host = StrBytes::from_string(broker.host);
    response.port = broker.port as i32;
    Ok(response)
}
