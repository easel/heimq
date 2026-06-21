//! Heartbeat request handler (API Key 12)

use crate::consumer_group::GroupCoordinatorBackend;
use crate::error::Result;
use crate::storage::RequestContext;
use bytes::Bytes;
use kafka_protocol::messages::heartbeat_request::HeartbeatRequest;
use kafka_protocol::messages::HeartbeatResponse;
use kafka_protocol::protocol::Decodable;

pub fn handle(
    api_version: i16,
    body: &[u8],
    coordinator: &dyn GroupCoordinatorBackend,
) -> Result<HeartbeatResponse> {
    handle_with_context(api_version, body, coordinator, &RequestContext::ANONYMOUS)
}

pub fn handle_with_context(
    api_version: i16,
    body: &[u8],
    coordinator: &dyn GroupCoordinatorBackend,
    ctx: &RequestContext,
) -> Result<HeartbeatResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    let request = match HeartbeatRequest::decode(&mut buf, api_version) {
        Ok(r) => r,
        Err(_) => {
            let mut r = HeartbeatResponse::default();
            r.error_code = 35;
            return Ok(r);
        }
    };

    let group_id = request.group_id.0.to_string();
    let member_id = request.member_id.to_string();
    let result =
        coordinator.heartbeat_with_context(ctx, &group_id, request.generation_id, &member_id);
    let mut response = HeartbeatResponse::default();
    response.error_code = result.error_code;
    Ok(response)
}
