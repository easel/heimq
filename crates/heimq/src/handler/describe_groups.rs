//! DescribeGroups request handler (API Key 15)

use crate::consumer_group::GroupCoordinatorBackend;
use crate::error::Result;
use bytes::Bytes;
use kafka_protocol::messages::describe_groups_request::DescribeGroupsRequest;
use kafka_protocol::messages::describe_groups_response::{DescribedGroup, DescribedGroupMember};
use kafka_protocol::messages::DescribeGroupsResponse;
use kafka_protocol::protocol::{Decodable, StrBytes};

pub fn handle(
    api_version: i16,
    body: &[u8],
    coordinator: &dyn GroupCoordinatorBackend,
) -> Result<DescribeGroupsResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    let request = match DescribeGroupsRequest::decode(&mut buf, api_version) {
        Ok(r) => r,
        Err(_) => {
            let mut r = DescribeGroupsResponse::default();
            r.groups.push(error_group("", 35));
            return Ok(r);
        }
    };

    let mut response = DescribeGroupsResponse::default();
    for gid in &request.groups {
        let group_id = gid.0.as_str();
        let described = match coordinator.describe_group(group_id) {
            Some(d) => {
                let mut dg = DescribedGroup::default();
                dg.group_id = kafka_protocol::messages::GroupId(StrBytes::from_string(
                    d.group_id.clone(),
                ));
                dg.group_state = StrBytes::from_string(d.group_state);
                dg.protocol_type = StrBytes::from_string(d.protocol_type);
                dg.protocol_data = StrBytes::from_string(d.protocol_name);
                for m in d.members {
                    let mut dm = DescribedGroupMember::default();
                    dm.member_id = StrBytes::from_string(m.member_id);
                    dm.client_id = StrBytes::from_string(m.client_id);
                    dm.client_host = StrBytes::from_string(m.client_host);
                    dm.member_metadata = bytes::Bytes::from(m.member_metadata);
                    dm.member_assignment = bytes::Bytes::from(m.member_assignment);
                    dg.members.push(dm);
                }
                dg
            }
            None => error_group(group_id, 16), // UNKNOWN_MEMBER_ID / GROUP_ID_NOT_FOUND
        };
        response.groups.push(described);
    }
    Ok(response)
}

fn error_group(group_id: &str, error_code: i16) -> DescribedGroup {
    let mut dg = DescribedGroup::default();
    dg.error_code = error_code;
    dg.group_id =
        kafka_protocol::messages::GroupId(StrBytes::from_string(group_id.to_string()));
    dg
}
