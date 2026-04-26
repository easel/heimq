//! Heartbeat request handler (API Key 12)

use crate::consumer_group::GroupCoordinatorBackend;
use crate::error::Result;
use bytes::Buf;
use kafka_protocol::messages::HeartbeatResponse;
use std::io::Cursor;

/// Handle Heartbeat request
pub fn handle(
    _api_version: i16,
    body: &[u8],
    coordinator: &dyn GroupCoordinatorBackend,
) -> Result<HeartbeatResponse> {
    let mut response = HeartbeatResponse::default();
    let mut cursor = Cursor::new(body);

    // group_id (STRING)
    let group_id = read_string(&mut cursor).unwrap_or_default();

    // generation_id (INT32)
    if cursor.remaining() < 4 {
        response.error_code = 35;
        return Ok(response);
    }
    let generation_id = cursor.get_i32();

    // member_id (STRING)
    let member_id = read_string(&mut cursor).unwrap_or_default();

    let result = coordinator.heartbeat(&group_id, generation_id, &member_id);
    response.error_code = result.error_code;
    Ok(response)
}

fn read_string(cursor: &mut Cursor<&[u8]>) -> Option<String> {
    if cursor.remaining() < 2 {
        return None;
    }
    let len = cursor.get_i16();
    if len < 0 {
        return Some(String::new());
    }
    if cursor.remaining() < len as usize {
        return None;
    }
    let mut buf = vec![0u8; len as usize];
    cursor.copy_to_slice(&mut buf);
    Some(String::from_utf8_lossy(&buf).to_string())
}
