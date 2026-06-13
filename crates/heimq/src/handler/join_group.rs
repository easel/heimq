//! JoinGroup request handler (API Key 11)

use crate::consumer_group::{GroupCoordinatorBackend, JoinRequest};
use crate::error::Result;
use bytes::Buf;
use kafka_protocol::messages::join_group_response::JoinGroupResponseMember;
use kafka_protocol::messages::JoinGroupResponse;
use kafka_protocol::protocol::StrBytes;
use std::io::Cursor;
use tracing::debug;

/// Handle JoinGroup request
pub fn handle(
    api_version: i16,
    body: &[u8],
    coordinator: &dyn GroupCoordinatorBackend,
) -> Result<JoinGroupResponse> {
    let mut response = JoinGroupResponse::default();
    let mut cursor = Cursor::new(body);

    // group_id (STRING)
    if cursor.remaining() < 2 {
        response.error_code = 35; // UNSUPPORTED_VERSION
        return Ok(response);
    }
    let group_id = read_string(&mut cursor).unwrap_or_default();

    // session_timeout_ms (INT32)
    if cursor.remaining() < 4 {
        response.error_code = 35;
        return Ok(response);
    }
    let session_timeout_ms = cursor.get_i32();

    // rebalance_timeout_ms (INT32) - version 1+
    let rebalance_timeout_ms = if api_version >= 1 && cursor.remaining() >= 4 {
        cursor.get_i32()
    } else {
        session_timeout_ms
    };

    // member_id (STRING)
    if cursor.remaining() < 2 {
        response.error_code = 35;
        return Ok(response);
    }
    let member_id = read_string(&mut cursor).unwrap_or_default();

    // group_instance_id (NULLABLE_STRING) - version 5+
    if api_version >= 5 && cursor.remaining() >= 2 {
        let _ = read_string(&mut cursor);
    }

    // protocol_type (STRING)
    if cursor.remaining() < 2 {
        response.error_code = 35;
        return Ok(response);
    }
    let protocol_type = read_string(&mut cursor).unwrap_or_default();

    // protocols array
    if cursor.remaining() < 4 {
        response.error_code = 35;
        return Ok(response);
    }
    let protocol_count = cursor.get_i32();
    let mut protocols = Vec::new();

    for _ in 0..protocol_count {
        // protocol name
        let name = read_string(&mut cursor).unwrap_or_default();

        // protocol metadata (BYTES)
        if cursor.remaining() < 4 {
            break;
        }
        let metadata_len = cursor.get_i32();
        let metadata = if metadata_len > 0 && cursor.remaining() >= metadata_len as usize {
            let mut buf = vec![0u8; metadata_len as usize];
            cursor.copy_to_slice(&mut buf);
            buf
        } else {
            vec![]
        };

        protocols.push((name, metadata));
    }

    let result = coordinator.join_group(JoinRequest {
        group_id: group_id.clone(),
        member_id,
        client_id: "client".to_string(), // Would come from header
        client_host: "127.0.0.1".to_string(),
        session_timeout_ms,
        rebalance_timeout_ms,
        protocol_type,
        protocols,
    });

    debug!(
        group = %group_id,
        member = %result.member_id,
        generation = result.generation_id,
        error = result.error_code,
        "JoinGroup result"
    );

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
