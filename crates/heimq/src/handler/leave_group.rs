//! LeaveGroup request handler (API Key 13)

use crate::consumer_group::GroupCoordinatorBackend;
use crate::error::Result;
use crate::storage::RequestContext;
use bytes::Bytes;
use heimq_protocol::messages::leave_group_request::LeaveGroupRequest;
use heimq_protocol::messages::LeaveGroupResponse;
use heimq_protocol::protocol::Decodable;
use tracing::debug;

pub fn handle(
    api_version: i16,
    body: &[u8],
    coordinator: &dyn GroupCoordinatorBackend,
) -> Result<LeaveGroupResponse> {
    handle_with_context(api_version, body, coordinator, &RequestContext::ANONYMOUS)
}

pub fn handle_with_context(
    api_version: i16,
    body: &[u8],
    coordinator: &dyn GroupCoordinatorBackend,
    ctx: &RequestContext,
) -> Result<LeaveGroupResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    let request = match LeaveGroupRequest::decode(&mut buf, api_version) {
        Ok(r) => r,
        Err(_) => return Ok(LeaveGroupResponse::default()),
    };

    let group_id = request.group_id.0.to_string();

    // v0-2: single member_id field; v3+: members array
    let member_ids: Vec<String> = if api_version >= 3 {
        request
            .members
            .iter()
            .map(|m| m.member_id.to_string())
            .collect()
    } else {
        vec![request.member_id.to_string()]
    };

    let result = coordinator.leave_group_with_context(ctx, &group_id, &member_ids);
    debug!(
        group = %group_id,
        members = member_ids.len(),
        error = result.error_code,
        "LeaveGroup result"
    );
    let mut response = LeaveGroupResponse::default();
    response.error_code = result.error_code;
    Ok(response)
}
