//! LeaveGroup request handler (API Key 13)

use crate::consumer_group::ConsumerGroupManager;
use crate::error::Result;
use bytes::Buf;
use kafka_protocol::messages::LeaveGroupResponse;
use std::io::Cursor;
use std::sync::Arc;
use tracing::debug;

/// Handle LeaveGroup request
pub fn handle(
    api_version: i16,
    body: &[u8],
    consumer_groups: &Arc<ConsumerGroupManager>,
) -> Result<LeaveGroupResponse> {
    let mut response = LeaveGroupResponse::default();
    let mut cursor = Cursor::new(body);

    // group_id (STRING)
    let group_id = read_string(&mut cursor).unwrap_or_default();

    // member_id (STRING) - older versions
    // members array - newer versions (v3+)
    let member_ids = if api_version >= 3 {
        // members array
        if cursor.remaining() < 4 {
            vec![]
        } else {
            let count = cursor.get_i32();
            let mut ids = Vec::new();
            for _ in 0..count {
                if let Some(member_id) = read_string(&mut cursor) {
                    ids.push(member_id);
                }
                // Skip group_instance_id
                let _ = read_string(&mut cursor);
            }
            ids
        }
    } else {
        // Single member_id
        match read_string(&mut cursor) {
            Some(id) => vec![id],
            None => vec![],
        }
    };

    // Get the group
    let group = match consumer_groups.get_group(&group_id) {
        Some(g) => g,
        None => {
            response.error_code = 16; // NOT_COORDINATOR
            return Ok(response);
        }
    };

    // Remove members
    for member_id in member_ids {
        if group.remove_member(&member_id) {
            debug!(group = %group_id, member = %member_id, "Member left group");
        }
    }

    response.error_code = 0;
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
