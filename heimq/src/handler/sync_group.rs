//! SyncGroup request handler (API Key 14)

use crate::consumer_group::{ConsumerGroup, ConsumerGroupManager};
use crate::error::Result;
use bytes::Buf;
use kafka_protocol::messages::SyncGroupResponse;
use std::io::Cursor;
use std::sync::Arc;
use tracing::debug;

/// Handle SyncGroup request
pub fn handle(
    api_version: i16,
    body: &[u8],
    consumer_groups: &Arc<ConsumerGroupManager>,
) -> Result<SyncGroupResponse> {
    let mut response = SyncGroupResponse::default();
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

    // group_instance_id (NULLABLE_STRING) - version 3+
    if api_version >= 3 {
        let _ = read_string(&mut cursor);
    }

    // protocol_type (STRING) - version 5+
    if api_version >= 5 {
        let _ = read_string(&mut cursor);
    }

    // protocol_name (STRING) - version 5+
    if api_version >= 5 {
        let _ = read_string(&mut cursor);
    }

    // assignments array
    if cursor.remaining() < 4 {
        response.error_code = 35;
        return Ok(response);
    }
    let assignment_count = cursor.get_i32();

    let mut assignments = Vec::new();
    for _ in 0..assignment_count {
        // member_id
        let assign_member_id = read_string(&mut cursor).unwrap_or_default();

        // assignment (BYTES)
        if cursor.remaining() < 4 {
            break;
        }
        let assignment_len = cursor.get_i32();
        let assignment = if assignment_len > 0 && cursor.remaining() >= assignment_len as usize {
            let mut buf = vec![0u8; assignment_len as usize];
            cursor.copy_to_slice(&mut buf);
            buf
        } else {
            vec![]
        };

        assignments.push((assign_member_id, assignment));
    }

    // Get the group
    let group = match consumer_groups.get_group(&group_id) {
        Some(g) => g,
        None => {
            response.error_code = 16; // NOT_COORDINATOR
            return Ok(response);
        }
    };

    // Verify generation
    if group.generation_id() != generation_id {
        response.error_code = 22; // ILLEGAL_GENERATION
        return Ok(response);
    }

    // Verify member exists
    if group.get_member(&member_id).is_none() {
        response.error_code = 25; // UNKNOWN_MEMBER_ID
        return Ok(response);
    }

    // If this is the leader, store the assignments
    if group.leader_id() == Some(member_id.clone()) { apply_leader_assignments(&group, &assignments); }

    // Return this member's assignment
    let assignment = group.get_assignment(&member_id).unwrap_or_default();
    response.assignment = bytes::Bytes::from(assignment);

    debug!(
        group = %group_id,
        member = %member_id,
        generation = generation_id,
        "Member synced"
    );

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

fn apply_leader_assignments(group: &Arc<ConsumerGroup>, assignments: &[(String, Vec<u8>)]) {
    for (assign_member_id, assignment) in assignments {
        group.set_assignment(assign_member_id, assignment.clone());
    }

    // Select protocol and complete rebalance
    let protocol = group.select_protocol().unwrap_or_default();
    group.complete_rebalance(protocol);
}
