//! FindCoordinator request handler (API Key 10)

use crate::config::Config;
use crate::error::Result;
use kafka_protocol::messages::{BrokerId, FindCoordinatorResponse};
use kafka_protocol::protocol::StrBytes;
use std::sync::Arc;

/// Handle FindCoordinator request
pub fn handle(
    _api_version: i16,
    _body: &[u8],
    config: &Arc<Config>,
) -> Result<FindCoordinatorResponse> {
    let mut response = FindCoordinatorResponse::default();

    // For single-node, we are always the coordinator
    response.error_code = 0;
    response.node_id = BrokerId(config.broker_id);
    response.host = StrBytes::from_string(if config.host == "0.0.0.0" {
        "127.0.0.1".to_string()  // Use IPv4 to avoid IPv6 resolution issues
    } else {
        config.host.clone()
    });
    response.port = config.port as i32;

    Ok(response)
}
