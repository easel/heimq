//! JoinGroup request handler (API Key 11)

use crate::consumer_group::{GroupCoordinatorBackend, JoinRequest};
use crate::error::Result;
use crate::storage::RequestContext;
use bytes::Bytes;
use kafka_protocol::messages::join_group_request::JoinGroupRequest;
use kafka_protocol::messages::join_group_response::JoinGroupResponseMember;
use kafka_protocol::messages::JoinGroupResponse;
use kafka_protocol::protocol::{Decodable, StrBytes};
use tracing::debug;

pub fn handle(
    api_version: i16,
    body: &[u8],
    coordinator: &dyn GroupCoordinatorBackend,
) -> Result<JoinGroupResponse> {
    handle_with_context(api_version, body, coordinator, &RequestContext::ANONYMOUS)
}

pub fn handle_with_context(
    api_version: i16,
    body: &[u8],
    coordinator: &dyn GroupCoordinatorBackend,
    ctx: &RequestContext,
) -> Result<JoinGroupResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    let request = match JoinGroupRequest::decode(&mut buf, api_version) {
        Ok(r) => r,
        Err(_) => {
            let mut r = JoinGroupResponse::default();
            r.error_code = 35;
            return Ok(r);
        }
    };

    let group_id = request.group_id.0.to_string();
    let member_id = request.member_id.to_string();
    let protocol_type = request.protocol_type.to_string();

    let protocols: Vec<(String, Vec<u8>)> = request
        .protocols
        .iter()
        .map(|p| (p.name.to_string(), p.metadata.to_vec()))
        .collect();

    let result = coordinator.join_group_with_context(
        ctx,
        JoinRequest {
            group_id: group_id.clone(),
            member_id,
            client_id: "client".to_string(),
            client_host: "127.0.0.1".to_string(),
            session_timeout_ms: request.session_timeout_ms,
            rebalance_timeout_ms: request.rebalance_timeout_ms,
            protocol_type,
            protocols,
        },
    );

    debug!(
        group = %group_id,
        member = %result.member_id,
        generation = result.generation_id,
        error = result.error_code,
        "JoinGroup result"
    );

    let mut response = JoinGroupResponse::default();
    response.error_code = result.error_code;
    response.generation_id = result.generation_id;
    response.protocol_type = Some(StrBytes::from_string(result.protocol_type));
    response.protocol_name = Some(StrBytes::from_string(result.protocol_name));
    response.leader = StrBytes::from_string(result.leader_id);
    response.member_id = StrBytes::from_string(result.member_id);

    for m in result.members {
        let mut member_info = JoinGroupResponseMember::default();
        member_info.member_id = StrBytes::from_string(m.member_id);
        member_info.metadata = bytes::Bytes::from(m.metadata);
        response.members.push(member_info);
    }

    Ok(response)
}
