//! SyncGroup request handler (API Key 14)

use crate::consumer_group::{GroupCoordinatorBackend, SyncRequest};
use crate::error::Result;
use bytes::Bytes;
use kafka_protocol::messages::sync_group_request::SyncGroupRequest;
use kafka_protocol::messages::SyncGroupResponse;
use kafka_protocol::protocol::Decodable;
use tracing::debug;

pub fn handle(
    api_version: i16,
    body: &[u8],
    coordinator: &dyn GroupCoordinatorBackend,
) -> Result<SyncGroupResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    let request = match SyncGroupRequest::decode(&mut buf, api_version) {
        Ok(r) => r,
        Err(_) => {
            let mut r = SyncGroupResponse::default();
            r.error_code = 35;
            return Ok(r);
        }
    };

    let group_id = request.group_id.0.to_string();
    let member_id = request.member_id.to_string();

    let assignments: Vec<(String, Vec<u8>)> = request
        .assignments
        .iter()
        .map(|a| (a.member_id.to_string(), a.assignment.to_vec()))
        .collect();

    let result = coordinator.sync_group(SyncRequest {
        group_id: group_id.clone(),
        generation_id: request.generation_id,
        member_id: member_id.clone(),
        assignments,
    });

    debug!(
        group = %group_id,
        member = %member_id,
        generation = request.generation_id,
        error = result.error_code,
        "SyncGroup result"
    );

    let mut response = SyncGroupResponse::default();
    response.error_code = result.error_code;
    response.assignment = bytes::Bytes::from(result.assignment);
    Ok(response)
}
