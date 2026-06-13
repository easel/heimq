//! DeleteGroups request handler (API Key 42)

use crate::consumer_group::GroupCoordinatorBackend;
use crate::error::Result;
use bytes::Bytes;
use kafka_protocol::messages::delete_groups_request::DeleteGroupsRequest;
use kafka_protocol::messages::delete_groups_response::DeletableGroupResult;
use kafka_protocol::messages::DeleteGroupsResponse;
use kafka_protocol::protocol::{Decodable, StrBytes};

pub fn handle(
    api_version: i16,
    body: &[u8],
    coordinator: &dyn GroupCoordinatorBackend,
) -> Result<DeleteGroupsResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    let request = match DeleteGroupsRequest::decode(&mut buf, api_version) {
        Ok(r) => r,
        Err(_) => return Ok(DeleteGroupsResponse::default()),
    };

    let mut response = DeleteGroupsResponse::default();
    for gid in &request.groups_names {
        let group_id = gid.0.as_str();
        // Also remove any committed offsets for the group.
        coordinator.offset_store().delete_group(group_id);
        let _existed = coordinator.delete_group(group_id);
        let mut result = DeletableGroupResult::default();
        result.group_id =
            kafka_protocol::messages::GroupId(StrBytes::from_string(group_id.to_string()));
        result.error_code = 0;
        response.results.push(result);
    }
    Ok(response)
}
