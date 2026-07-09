//! ListGroups request handler (API Key 16)

use crate::consumer_group::GroupCoordinatorBackend;
use crate::error::Result;
use crate::storage::RequestContext;
use bytes::Bytes;
use heimq_protocol::messages::list_groups_request::ListGroupsRequest;
use heimq_protocol::messages::list_groups_response::ListedGroup;
use heimq_protocol::messages::ListGroupsResponse;
use heimq_protocol::protocol::{Decodable, StrBytes};

pub fn handle(
    api_version: i16,
    body: &[u8],
    coordinator: &dyn GroupCoordinatorBackend,
) -> Result<ListGroupsResponse> {
    handle_with_context(api_version, body, coordinator, &RequestContext::ANONYMOUS)
}

pub fn handle_with_context(
    api_version: i16,
    body: &[u8],
    coordinator: &dyn GroupCoordinatorBackend,
    ctx: &RequestContext,
) -> Result<ListGroupsResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    // Ignore decode errors — use defaults (no filter).
    let _request = ListGroupsRequest::decode(&mut buf, api_version).unwrap_or_default();

    let group_ids = coordinator.list_groups_with_context(ctx);
    let mut response = ListGroupsResponse::default();
    for group_id in group_ids {
        let desc = coordinator.describe_group_with_context(ctx, &group_id);
        let protocol_type = desc.map(|d| d.protocol_type).unwrap_or_default();
        let mut listed = ListedGroup::default();
        listed.group_id = heimq_protocol::messages::GroupId(StrBytes::from_string(group_id));
        listed.protocol_type = StrBytes::from_string(protocol_type);
        response.groups.push(listed);
    }
    Ok(response)
}
