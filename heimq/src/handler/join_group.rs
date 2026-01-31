//! JoinGroup request handler (API Key 11)

use crate::consumer_group::{ConsumerGroup, ConsumerGroupManager, Member};
use crate::error::Result;
use bytes::Buf;
use kafka_protocol::messages::join_group_response::JoinGroupResponseMember;
use kafka_protocol::messages::JoinGroupResponse;
use kafka_protocol::protocol::StrBytes;
use std::io::Cursor;
use std::sync::Arc;
use tracing::debug;

/// Handle JoinGroup request
pub fn handle(
    api_version: i16,
    body: &[u8],
    consumer_groups: &Arc<ConsumerGroupManager>,
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

    // Get or create the group
    let group = consumer_groups.get_or_create_group(&group_id);

    // Generate member ID if empty
    let actual_member_id = if member_id.is_empty() {
        group.generate_member_id("consumer")
    } else {
        member_id.clone()
    };

    // If member_id was empty, client needs to rejoin with the assigned ID
    if member_id.is_empty() {
        response.error_code = 79; // MEMBER_ID_REQUIRED
        response.member_id = StrBytes::from_string(actual_member_id);
        response.generation_id = -1;
        return Ok(response);
    }

    // Create member and add to group
    let member = Member::new(
        actual_member_id.clone(),
        "client".to_string(), // Would come from header
        "127.0.0.1".to_string(),
        session_timeout_ms,
        rebalance_timeout_ms,
        protocol_type.clone(),
        protocols.clone(),
    );

    let generation_id = group.add_member(member);

    // Select protocol
    let protocol = group.select_protocol().unwrap_or_else(|| {
        protocols.first().map(|(n, _)| n.clone()).unwrap_or_default()
    });

    debug!(
        group = %group_id,
        member = %actual_member_id,
        generation = generation_id,
        "Member joined group"
    );

    response.error_code = 0;
    response.generation_id = generation_id;
    response.protocol_type = Some(StrBytes::from_string(protocol_type));
    response.protocol_name = Some(StrBytes::from_string(protocol.clone()));
    response.leader = StrBytes::from_string(group.leader_id().unwrap_or_default());
    response.member_id = StrBytes::from_string(actual_member_id.clone());

    // If this member is the leader, include all members
    if group.leader_id() == Some(actual_member_id.clone()) { add_leader_members(&mut response, &group, &protocol); }

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

fn add_leader_members(
    response: &mut JoinGroupResponse,
    group: &Arc<ConsumerGroup>,
    protocol: &str,
) {
    for m in group.members() {
        let mut member_info = JoinGroupResponseMember::default();
        member_info.member_id = StrBytes::from_string(m.member_id.clone());
        // Find the metadata for the selected protocol
        if let Some((_, metadata)) = m.protocols.iter().find(|(n, _)| n == protocol) {
            member_info.metadata = bytes::Bytes::from(metadata.clone());
        }
        response.members.push(member_info);
    }
}
