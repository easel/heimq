//! FindCoordinator request handler (API Key 10)

use crate::error::Result;
use crate::storage::ClusterView;
use bytes::Bytes;
use kafka_protocol::messages::find_coordinator_request::FindCoordinatorRequest;
use kafka_protocol::messages::{BrokerId, FindCoordinatorResponse};
use kafka_protocol::protocol::{Decodable, StrBytes};

pub fn handle(
    api_version: i16,
    body: &[u8],
    cluster_view: &dyn ClusterView,
) -> Result<FindCoordinatorResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    // Decode to get the key; ignore decode errors (fall through to single-node reply).
    let key = FindCoordinatorRequest::decode(&mut buf, api_version)
        .map(|r| {
            // v4+ has coordinator_keys; v0-3 has a single `key` field.
            if api_version >= 4 {
                r.coordinator_keys
                    .into_iter()
                    .next()
                    .map(|s| s.to_string())
                    .unwrap_or_default()
            } else {
                r.key.to_string()
            }
        })
        .unwrap_or_default();

    let broker = cluster_view
        .find_coordinator(&key)
        .unwrap_or_else(|_| cluster_view.self_broker());
    let mut response = FindCoordinatorResponse::default();
    response.error_code = 0;
    response.node_id = BrokerId(broker.node_id);
    response.host = StrBytes::from_string(broker.host);
    response.port = broker.port as i32;
    Ok(response)
}
