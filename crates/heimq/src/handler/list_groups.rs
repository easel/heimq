//! ListGroups request handler (API Key 16)

use crate::consumer_group::GroupCoordinatorBackend;
use crate::error::Result;
use bytes::Bytes;
use kafka_protocol::messages::list_groups_request::ListGroupsRequest;
use kafka_protocol::messages::list_groups_response::ListedGroup;
use kafka_protocol::messages::ListGroupsResponse;
use kafka_protocol::protocol::{Decodable, StrBytes};

pub fn handle(
    api_version: i16,
    body: &[u8],
    coordinator: &dyn GroupCoordinatorBackend,
) -> Result<ListGroupsResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    // Ignore decode errors — use defaults (no filter).
    let _request = ListGroupsRequest::decode(&mut buf, api_version).unwrap_or_default();

    let group_ids = coordinator.list_groups();
    let mut response = ListGroupsResponse::default();
    for group_id in group_ids {
        let desc = coordinator.describe_group(&group_id);
        let protocol_type = desc.map(|d| d.protocol_type).unwrap_or_default();
        let mut listed = ListedGroup::default();
        listed.group_id = kafka_protocol::messages::GroupId(StrBytes::from_string(group_id));
        listed.protocol_type = StrBytes::from_string(protocol_type);
        response.groups.push(listed);
    }
    Ok(response)
}
