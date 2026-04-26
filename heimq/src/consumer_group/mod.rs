//! Consumer group coordination
//!
//! Simplified implementation for single-node operation.
//! Handles group membership and offset storage.

mod backend;
mod coordinator;
mod group;
mod offset_store;

pub use backend::{
    GroupCoordinatorBackend, GroupCoordinatorCapabilities, HeartbeatResult, JoinMember,
    JoinRequest, JoinResult, LeaveResult, SyncRequest, SyncResult,
};
#[allow(unused_imports)]
pub use coordinator::Coordinator;
#[allow(unused_imports)]
pub use group::{ConsumerGroup, GroupState, Member, MemberState};
pub use offset_store::MemoryOffsetStore;

use crate::config::Config;
use crate::storage::{Durability, OffsetStore};
use dashmap::DashMap;
use std::sync::Arc;
use tracing::info;

/// Capabilities exposed by the in-memory `ConsumerGroupManager` backend.
const MEMORY_COORDINATOR_CAPABILITIES: GroupCoordinatorCapabilities = GroupCoordinatorCapabilities {
    name: "in-memory",
    version: env!("CARGO_PKG_VERSION"),
    durability: Durability::None,
    survives_restart: false,
    multi_node: false,
};

/// Consumer group manager
pub struct ConsumerGroupManager {
    /// All consumer groups
    groups: DashMap<String, Arc<ConsumerGroup>>,
    /// Offset storage
    offset_store: Arc<dyn OffsetStore>,
    /// Configuration
    #[allow(dead_code)]
    config: Arc<Config>,
}

impl ConsumerGroupManager {
    /// Create a new consumer group manager backed by the default in-memory
    /// offset store.
    pub fn new(config: Arc<Config>) -> Self {
        Self::with_offset_store(config, Arc::new(MemoryOffsetStore::new()))
    }

    /// Create a new consumer group manager with an injected offset store.
    pub fn with_offset_store(config: Arc<Config>, offset_store: Arc<dyn OffsetStore>) -> Self {
        info!("Initializing consumer group manager");
        Self {
            groups: DashMap::new(),
            offset_store,
            config,
        }
    }

    /// Get or create a consumer group
    pub fn get_or_create_group(&self, group_id: &str) -> Arc<ConsumerGroup> {
        if let Some(group) = self.groups.get(group_id) {
            return group.clone();
        }

        let group = Arc::new(ConsumerGroup::new(group_id.to_string()));
        self.groups.insert(group_id.to_string(), group.clone());
        info!(group = group_id, "Created consumer group");
        group
    }

    /// Get a consumer group if it exists
    pub fn get_group(&self, group_id: &str) -> Option<Arc<ConsumerGroup>> {
        self.groups.get(group_id).map(|g| g.clone())
    }

    /// Get the offset store
    pub fn offset_store(&self) -> &Arc<dyn OffsetStore> {
        &self.offset_store
    }

    /// List all groups
    #[allow(dead_code)]
    pub fn list_groups(&self) -> Vec<String> {
        self.groups.iter().map(|e| e.key().clone()).collect()
    }
}

impl GroupCoordinatorBackend for ConsumerGroupManager {
    fn join_group(&self, req: JoinRequest) -> JoinResult {
        let group = self.get_or_create_group(&req.group_id);

        // Generate a new member id when the client didn't supply one and
        // return MEMBER_ID_REQUIRED — the client must rejoin with that id.
        if req.member_id.is_empty() {
            let new_id = group.generate_member_id("consumer");
            return JoinResult {
                error_code: 79,
                generation_id: -1,
                member_id: new_id,
                leader_id: String::new(),
                protocol_type: req.protocol_type,
                protocol_name: String::new(),
                members: Vec::new(),
            };
        }

        let member = Member::new(
            req.member_id.clone(),
            req.client_id,
            req.client_host,
            req.session_timeout_ms,
            req.rebalance_timeout_ms,
            req.protocol_type.clone(),
            req.protocols.clone(),
        );

        let generation_id = group.add_member(member);
        let protocol_name = group
            .select_protocol()
            .or_else(|| req.protocols.first().map(|(n, _)| n.clone()))
            .unwrap_or_default();

        let leader_id = group.leader_id().unwrap_or_default();
        let mut members = Vec::new();
        if leader_id == req.member_id {
            for m in group.members() {
                let metadata = m
                    .protocols
                    .iter()
                    .find(|(n, _)| n == &protocol_name)
                    .map(|(_, md)| md.clone())
                    .unwrap_or_default();
                members.push(JoinMember {
                    member_id: m.member_id,
                    metadata,
                });
            }
        }

        JoinResult {
            error_code: 0,
            generation_id,
            member_id: req.member_id,
            leader_id,
            protocol_type: req.protocol_type,
            protocol_name,
            members,
        }
    }

    fn sync_group(&self, req: SyncRequest) -> SyncResult {
        let group = match self.get_group(&req.group_id) {
            Some(g) => g,
            None => {
                return SyncResult {
                    error_code: 16,
                    assignment: Vec::new(),
                }
            }
        };

        if group.generation_id() != req.generation_id {
            return SyncResult {
                error_code: 22,
                assignment: Vec::new(),
            };
        }

        if group.get_member(&req.member_id).is_none() {
            return SyncResult {
                error_code: 25,
                assignment: Vec::new(),
            };
        }

        if group.leader_id() == Some(req.member_id.clone()) {
            for (assign_member_id, assignment) in &req.assignments {
                group.set_assignment(assign_member_id, assignment.clone());
            }
            let protocol = group.select_protocol().unwrap_or_default();
            group.complete_rebalance(protocol);
        }

        let assignment = group.get_assignment(&req.member_id).unwrap_or_default();
        SyncResult {
            error_code: 0,
            assignment,
        }
    }

    fn heartbeat(&self, group_id: &str, generation_id: i32, member_id: &str) -> HeartbeatResult {
        let group = match self.get_group(group_id) {
            Some(g) => g,
            None => return HeartbeatResult { error_code: 16 },
        };

        if group.generation_id() != generation_id {
            return HeartbeatResult { error_code: 22 };
        }

        if !group.heartbeat(member_id) {
            return HeartbeatResult { error_code: 25 };
        }

        HeartbeatResult { error_code: 0 }
    }

    fn leave_group(&self, group_id: &str, member_ids: &[String]) -> LeaveResult {
        let group = match self.get_group(group_id) {
            Some(g) => g,
            None => return LeaveResult { error_code: 16 },
        };

        for member_id in member_ids {
            let _ = group.remove_member(member_id);
        }

        LeaveResult { error_code: 0 }
    }

    fn capabilities(&self) -> &GroupCoordinatorCapabilities {
        &MEMORY_COORDINATOR_CAPABILITIES
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use clap::Parser;

    #[test]
    fn test_list_groups() {
        let config = Arc::new(Config::parse_from(["heimq"]));
        let manager = ConsumerGroupManager::new(config);
        manager.get_or_create_group("g1");
        manager.get_or_create_group("g2");
        let groups = manager.list_groups();
        assert!(groups.contains(&"g1".to_string()));
        assert!(groups.contains(&"g2".to_string()));
    }

    #[test]
    fn test_with_injected_offset_store() {
        let config = Arc::new(Config::parse_from(["heimq"]));
        let store: Arc<dyn OffsetStore> = Arc::new(MemoryOffsetStore::new());
        let manager = ConsumerGroupManager::with_offset_store(config, store.clone());
        manager.offset_store().commit("g", "t", 0, 42, 0, None);
        assert_eq!(store.fetch("g", "t", 0).unwrap().offset, 42);
    }

    #[test]
    fn coordinator_capabilities_describe_memory_backend() {
        let config = Arc::new(Config::parse_from(["heimq"]));
        let manager = ConsumerGroupManager::new(config);
        let caps = manager.capabilities();
        assert_eq!(caps.name, "in-memory");
        assert_eq!(caps.durability, Durability::None);
        assert!(!caps.survives_restart);
        assert!(!caps.multi_node);
    }

    #[test]
    fn join_group_mints_member_id_when_empty() {
        let config = Arc::new(Config::parse_from(["heimq"]));
        let manager = ConsumerGroupManager::new(config);
        let result = manager.join_group(JoinRequest {
            group_id: "g".to_string(),
            member_id: String::new(),
            client_id: "c".to_string(),
            client_host: "h".to_string(),
            session_timeout_ms: 30_000,
            rebalance_timeout_ms: 30_000,
            protocol_type: "consumer".to_string(),
            protocols: vec![("range".to_string(), vec![])],
        });
        assert_eq!(result.error_code, 79);
        assert!(!result.member_id.is_empty());
        assert_eq!(result.generation_id, -1);
    }

    #[test]
    fn join_sync_heartbeat_leave_via_trait() {
        let config = Arc::new(Config::parse_from(["heimq"]));
        let manager: Arc<dyn GroupCoordinatorBackend> =
            Arc::new(ConsumerGroupManager::new(config));

        let join = manager.join_group(JoinRequest {
            group_id: "g".to_string(),
            member_id: "m1".to_string(),
            client_id: "c".to_string(),
            client_host: "h".to_string(),
            session_timeout_ms: 30_000,
            rebalance_timeout_ms: 30_000,
            protocol_type: "consumer".to_string(),
            protocols: vec![("range".to_string(), vec![1, 2])],
        });
        assert_eq!(join.error_code, 0);
        assert_eq!(join.leader_id, "m1");
        assert_eq!(join.members.len(), 1);
        assert_eq!(join.members[0].metadata, vec![1, 2]);

        let sync = manager.sync_group(SyncRequest {
            group_id: "g".to_string(),
            generation_id: join.generation_id,
            member_id: "m1".to_string(),
            assignments: vec![("m1".to_string(), vec![9, 9])],
        });
        assert_eq!(sync.error_code, 0);
        assert_eq!(sync.assignment, vec![9, 9]);

        let hb = manager.heartbeat("g", join.generation_id, "m1");
        assert_eq!(hb.error_code, 0);

        let hb_bad_gen = manager.heartbeat("g", join.generation_id + 1, "m1");
        assert_eq!(hb_bad_gen.error_code, 22);

        let hb_missing = manager.heartbeat("g", join.generation_id, "missing");
        assert_eq!(hb_missing.error_code, 25);

        let hb_no_group = manager.heartbeat("nope", 0, "m1");
        assert_eq!(hb_no_group.error_code, 16);

        let leave = manager.leave_group("g", &["m1".to_string()]);
        assert_eq!(leave.error_code, 0);

        let leave_no_group = manager.leave_group("nope", &["m1".to_string()]);
        assert_eq!(leave_no_group.error_code, 16);
    }

    #[test]
    fn sync_group_error_paths() {
        let config = Arc::new(Config::parse_from(["heimq"]));
        let manager = ConsumerGroupManager::new(config);

        let res = manager.sync_group(SyncRequest {
            group_id: "missing".to_string(),
            generation_id: 0,
            member_id: "m".to_string(),
            assignments: Vec::new(),
        });
        assert_eq!(res.error_code, 16);

        let join = manager.join_group(JoinRequest {
            group_id: "g".to_string(),
            member_id: "m1".to_string(),
            client_id: "c".to_string(),
            client_host: "h".to_string(),
            session_timeout_ms: 30_000,
            rebalance_timeout_ms: 30_000,
            protocol_type: "consumer".to_string(),
            protocols: vec![("range".to_string(), vec![])],
        });

        let bad_gen = manager.sync_group(SyncRequest {
            group_id: "g".to_string(),
            generation_id: join.generation_id + 1,
            member_id: "m1".to_string(),
            assignments: Vec::new(),
        });
        assert_eq!(bad_gen.error_code, 22);

        let bad_member = manager.sync_group(SyncRequest {
            group_id: "g".to_string(),
            generation_id: join.generation_id,
            member_id: "missing".to_string(),
            assignments: Vec::new(),
        });
        assert_eq!(bad_member.error_code, 25);
    }
}
