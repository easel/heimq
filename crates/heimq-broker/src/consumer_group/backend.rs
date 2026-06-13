//! Pluggable group-coordinator backend.
//!
//! `GroupCoordinatorBackend` is the trait higher layers use to drive consumer
//! group lifecycle (join / sync / heartbeat / leave). Unlike a snapshot store,
//! the coordinator owns live state — members, generations, assignments, and
//! the rebalance state machine — so the trait is request/response shaped
//! rather than a simple KV.
//!
//! The in-memory implementation lives on `ConsumerGroupManager` and matches
//! the pre-trait behaviour exactly. A future durable backend (e.g. one that
//! persists membership to a coordinator log) can implement this trait without
//! touching protocol handlers.

#![allow(dead_code)]

use crate::storage::{Durability, OffsetStore};
use std::sync::Arc;

/// Declarative description of what a group-coordinator backend can do.
#[derive(Debug, Clone)]
pub struct GroupCoordinatorCapabilities {
    /// Human-readable backend name (e.g. `"in-memory"`).
    pub name: &'static str,
    /// Backend implementation version string.
    pub version: &'static str,
    /// Durability guarantee for acknowledged group-membership changes.
    pub durability: Durability,
    /// Whether group state survives a process restart.
    pub survives_restart: bool,
    /// Whether the backend coordinates state across multiple nodes.
    pub multi_node: bool,
}

impl GroupCoordinatorCapabilities {
    /// Default capabilities for a non-durable, single-node coordinator.
    pub const fn minimal() -> Self {
        Self {
            name: "unknown",
            version: "0.0.0",
            durability: Durability::None,
            survives_restart: false,
            multi_node: false,
        }
    }
}

impl Default for GroupCoordinatorCapabilities {
    fn default() -> Self {
        Self::minimal()
    }
}

/// Inputs to a `JoinGroup` call.
#[derive(Debug, Clone)]
pub struct JoinRequest {
    pub group_id: String,
    /// Empty when the client is requesting a fresh member id.
    pub member_id: String,
    pub client_id: String,
    pub client_host: String,
    pub session_timeout_ms: i32,
    pub rebalance_timeout_ms: i32,
    pub protocol_type: String,
    pub protocols: Vec<(String, Vec<u8>)>,
}

/// One entry in `JoinResult::members` (returned only to the elected leader).
#[derive(Debug, Clone)]
pub struct JoinMember {
    pub member_id: String,
    /// Protocol-specific subscription metadata supplied by the member.
    pub metadata: Vec<u8>,
}

/// Result of a `JoinGroup` call.
#[derive(Debug, Clone)]
pub struct JoinResult {
    /// Kafka error code (0 == success). `MEMBER_ID_REQUIRED` (79) is returned
    /// when the request had an empty member id and a fresh one was minted.
    pub error_code: i16,
    pub generation_id: i32,
    pub member_id: String,
    pub leader_id: String,
    pub protocol_type: String,
    pub protocol_name: String,
    /// Populated only when this member is the elected leader.
    pub members: Vec<JoinMember>,
}

/// Inputs to a `SyncGroup` call.
#[derive(Debug, Clone)]
pub struct SyncRequest {
    pub group_id: String,
    pub generation_id: i32,
    pub member_id: String,
    /// Per-member assignments — non-empty only when the caller is the leader.
    pub assignments: Vec<(String, Vec<u8>)>,
}

/// Result of a `SyncGroup` call.
#[derive(Debug, Clone)]
pub struct SyncResult {
    pub error_code: i16,
    pub assignment: Vec<u8>,
}

/// Result of a `Heartbeat` call.
#[derive(Debug, Clone)]
pub struct HeartbeatResult {
    pub error_code: i16,
}

/// Result of a `LeaveGroup` call.
#[derive(Debug, Clone)]
pub struct LeaveResult {
    pub error_code: i16,
}

/// Member description returned by `describe_group`.
#[derive(Debug, Clone)]
pub struct MemberDescription {
    pub member_id: String,
    pub client_id: String,
    pub client_host: String,
    /// Protocol-specific subscription metadata from the member's JoinGroup request.
    pub member_metadata: Vec<u8>,
    /// Assignment bytes set by the group leader in SyncGroup.
    pub member_assignment: Vec<u8>,
}

/// Group description returned by `describe_group`.
#[derive(Debug, Clone)]
pub struct GroupDescription {
    pub group_id: String,
    /// Kafka group state string (e.g. "Stable", "Empty", "PreparingRebalance").
    pub group_state: String,
    pub protocol_type: String,
    /// Selected protocol name (e.g. "range").
    pub protocol_name: String,
    pub members: Vec<MemberDescription>,
}

/// Pluggable group-coordinator backend.
///
/// All methods are infallible at the trait level — protocol-level error
/// conditions are returned as Kafka error codes inside the result types so
/// handlers can map them to wire responses without losing information.
pub trait GroupCoordinatorBackend: Send + Sync {
    /// Join (or rejoin) a member to a group. When `req.member_id` is empty,
    /// returns `MEMBER_ID_REQUIRED` and a freshly minted id without adding
    /// the member — the client must reissue the request with that id.
    fn join_group(&self, req: JoinRequest) -> JoinResult;

    /// Sync assignments. When the caller is the elected leader, the supplied
    /// per-member assignments are stored and the group transitions to
    /// `Stable`. Followers receive their previously-stored assignment.
    fn sync_group(&self, req: SyncRequest) -> SyncResult;

    /// Refresh a member's session timeout.
    fn heartbeat(&self, group_id: &str, generation_id: i32, member_id: &str) -> HeartbeatResult;

    /// Remove one or more members from a group.
    fn leave_group(&self, group_id: &str, member_ids: &[String]) -> LeaveResult;

    /// List all known group IDs.
    fn list_groups(&self) -> Vec<String>;

    /// Describe a single group. Returns `None` if the group does not exist.
    fn describe_group(&self, group_id: &str) -> Option<GroupDescription>;

    /// Delete a group. Returns true if the group existed and was removed.
    fn delete_group(&self, group_id: &str) -> bool;

    /// Backend capabilities descriptor.
    fn capabilities(&self) -> &GroupCoordinatorCapabilities;

    /// Offset store associated with this coordinator.
    fn offset_store(&self) -> Arc<dyn OffsetStore>;
}
