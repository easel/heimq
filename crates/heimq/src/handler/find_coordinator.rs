//! FindCoordinator request handler (API Key 10)

use crate::error::Result;
use crate::storage::ClusterView;
use bytes::Bytes;
use kafka_protocol::messages::find_coordinator_request::FindCoordinatorRequest;
use kafka_protocol::messages::find_coordinator_response::Coordinator;
use kafka_protocol::messages::{BrokerId, FindCoordinatorResponse};
use kafka_protocol::protocol::{Decodable, StrBytes};

pub fn handle(
    api_version: i16,
    body: &[u8],
    cluster_view: &dyn ClusterView,
) -> Result<FindCoordinatorResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    // Decode to extract the coordinator key(s); fall back to self on error.
    let keys: Vec<String> = FindCoordinatorRequest::decode(&mut buf, api_version)
        .map(|r| {
            if api_version >= 4 {
                // v4+: coordinator_keys is a compact array of keys.
                r.coordinator_keys
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect()
            } else {
                vec![r.key.to_string()]
            }
        })
        .unwrap_or_default();

    let mut response = FindCoordinatorResponse::default();

    if api_version >= 4 {
        // v4+ response: per-key Coordinator array replaces the top-level fields.
        for key in &keys {
            let broker = cluster_view
                .find_coordinator(key)
                .unwrap_or_else(|_| cluster_view.self_broker());
            let mut coord = Coordinator::default();
            coord.key = StrBytes::from_string(key.clone());
            coord.node_id = BrokerId(broker.node_id);
            coord.host = StrBytes::from_string(broker.host);
            coord.port = broker.port as i32;
            coord.error_code = 0;
            response.coordinators.push(coord);
        }
        // Ensure at least one coordinator is returned (fallback for empty key list).
        if response.coordinators.is_empty() {
            let broker = cluster_view.self_broker();
            let mut coord = Coordinator::default();
            coord.node_id = BrokerId(broker.node_id);
            coord.host = StrBytes::from_string(broker.host);
            coord.port = broker.port as i32;
            coord.error_code = 0;
            response.coordinators.push(coord);
        }
    } else {
        // v0-v3 response: single top-level coordinator fields.
        let key = keys.into_iter().next().unwrap_or_default();
        let broker = cluster_view
            .find_coordinator(&key)
            .unwrap_or_else(|_| cluster_view.self_broker());
        response.error_code = 0;
        response.node_id = BrokerId(broker.node_id);
        response.host = StrBytes::from_string(broker.host);
        response.port = broker.port as i32;
    }

    Ok(response)
}
