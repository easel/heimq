//! Consumer group state management

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::{Duration, Instant};

/// Member state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum MemberState {
    /// Member is joining the group
    Joining,
    /// Member is syncing partition assignments
    Syncing,
    /// Member is stable and consuming
    Stable,
    /// Member has left or been removed
    Dead,
}

/// A member of a consumer group
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Member {
    pub member_id: String,
    pub client_id: String,
    pub client_host: String,
    pub session_timeout_ms: i32,
    pub rebalance_timeout_ms: i32,
    pub protocol_type: String,
    pub protocols: Vec<(String, Vec<u8>)>,
    pub assignment: Vec<u8>,
    pub state: MemberState,
    pub last_heartbeat: Instant,
}

impl Member {
    pub fn new(
        member_id: String,
        client_id: String,
        client_host: String,
        session_timeout_ms: i32,
        rebalance_timeout_ms: i32,
        protocol_type: String,
        protocols: Vec<(String, Vec<u8>)>,
    ) -> Self {
        Self {
            member_id,
            client_id,
            client_host,
            session_timeout_ms,
            rebalance_timeout_ms,
            protocol_type,
            protocols,
            assignment: Vec::new(),
            state: MemberState::Joining,
            last_heartbeat: Instant::now(),
        }
    }

    /// Check if this member's session has expired
    #[allow(dead_code)]
    pub fn is_expired(&self) -> bool {
        self.last_heartbeat.elapsed() > Duration::from_millis(self.session_timeout_ms as u64)
    }

    /// Update the last heartbeat time
    pub fn heartbeat(&mut self) {
        self.last_heartbeat = Instant::now();
    }
}

/// Group state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum GroupState {
    /// No members
    Empty,
    /// Waiting for members to join
    PreparingRebalance,
    /// Waiting for leader to send assignments
    CompletingRebalance,
    /// Stable, all members consuming
    Stable,
    /// Group is being deleted
    Dead,
}

/// A consumer group
pub struct ConsumerGroup {
    #[allow(dead_code)]
    group_id: String,
    state: RwLock<GroupState>,
    generation_id: AtomicI32,
    protocol_type: RwLock<Option<String>>,
    protocol: RwLock<Option<String>>,
    leader_id: RwLock<Option<String>>,
    members: RwLock<HashMap<String, Member>>,
    next_member_id: AtomicI32,
}

impl ConsumerGroup {
    pub fn new(group_id: String) -> Self {
        Self {
            group_id,
            state: RwLock::new(GroupState::Empty),
            generation_id: AtomicI32::new(0),
            protocol_type: RwLock::new(None),
            protocol: RwLock::new(None),
            leader_id: RwLock::new(None),
            members: RwLock::new(HashMap::new()),
            next_member_id: AtomicI32::new(0),
        }
    }

    #[allow(dead_code)]
    pub fn group_id(&self) -> &str {
        &self.group_id
    }

    #[allow(dead_code)]
    pub fn state(&self) -> GroupState {
        *self.state.read()
    }

    pub fn generation_id(&self) -> i32 {
        self.generation_id.load(Ordering::SeqCst)
    }

    pub fn leader_id(&self) -> Option<String> {
        self.leader_id.read().clone()
    }

    #[allow(dead_code)]
    pub fn protocol(&self) -> Option<String> {
        self.protocol.read().clone()
    }

    #[allow(dead_code)]
    pub fn protocol_type(&self) -> Option<String> {
        self.protocol_type.read().clone()
    }

    /// Generate a new member ID
    pub fn generate_member_id(&self, client_id: &str) -> String {
        let _num = self.next_member_id.fetch_add(1, Ordering::SeqCst);
        format!("{}-{}", client_id, uuid_simple())
    }

    /// Add a member to the group
    pub fn add_member(&self, member: Member) -> i32 {
        let member_id = member.member_id.clone();
        let mut members = self.members.write();

        // First member becomes the leader
        if members.is_empty() {
            *self.leader_id.write() = Some(member_id.clone());
            *self.protocol_type.write() = Some(member.protocol_type.clone());
        }

        members.insert(member_id, member);

        // Trigger rebalance
        *self.state.write() = GroupState::PreparingRebalance;

        // Increment generation and return the new value
        let new_gen = self.generation_id.fetch_add(1, Ordering::SeqCst) + 1;
        new_gen
    }

    /// Remove a member from the group
    pub fn remove_member(&self, member_id: &str) -> bool {
        let mut members = self.members.write();
        let removed = members.remove(member_id).is_some();

        if removed {
            // If leader left, elect new leader
            if self.leader_id.read().as_ref() == Some(&member_id.to_string()) {
                *self.leader_id.write() = members.keys().next().cloned();
            }

            // Trigger rebalance if members remain
            if !members.is_empty() {
                *self.state.write() = GroupState::PreparingRebalance;
                self.generation_id.fetch_add(1, Ordering::SeqCst);
            } else {
                *self.state.write() = GroupState::Empty;
            }
        }

        removed
    }

    /// Get member by ID
    pub fn get_member(&self, member_id: &str) -> Option<Member> {
        self.members.read().get(member_id).cloned()
    }

    /// Get all members
    pub fn members(&self) -> Vec<Member> {
        self.members.read().values().cloned().collect()
    }

    /// Update member heartbeat
    pub fn heartbeat(&self, member_id: &str) -> bool {
        if let Some(member) = self.members.write().get_mut(member_id) {
            member.heartbeat();
            true
        } else {
            false
        }
    }

    /// Set member assignment
    pub fn set_assignment(&self, member_id: &str, assignment: Vec<u8>) {
        if let Some(member) = self.members.write().get_mut(member_id) {
            member.assignment = assignment;
            member.state = MemberState::Stable;
        }
    }

    /// Get member assignment
    pub fn get_assignment(&self, member_id: &str) -> Option<Vec<u8>> {
        self.members.read().get(member_id).map(|m| m.assignment.clone())
    }

    /// Select a protocol that all members support
    pub fn select_protocol(&self) -> Option<String> {
        let members = self.members.read();
        if members.is_empty() {
            return None;
        }

        // Find protocols supported by all members
        let mut protocol_counts: HashMap<String, usize> = HashMap::new();
        for member in members.values() {
            for (protocol, _) in &member.protocols {
                *protocol_counts.entry(protocol.clone()).or_insert(0) += 1;
            }
        }

        // Select a deterministic protocol supported by all members
        let member_count = members.len();
        let mut supported: Vec<String> = protocol_counts
            .into_iter()
            .filter_map(|(protocol, count)| {
                if count == member_count {
                    Some(protocol)
                } else {
                    None
                }
            })
            .collect();
        supported.sort();
        supported.into_iter().next()
    }

    /// Complete rebalance - move to stable state
    pub fn complete_rebalance(&self, protocol: String) {
        *self.protocol.write() = Some(protocol);
        *self.state.write() = GroupState::Stable;

        // Mark all members as stable
        for member in self.members.write().values_mut() {
            member.state = MemberState::Stable;
        }
    }

    /// Check and remove expired members
    #[allow(dead_code)]
    pub fn remove_expired_members(&self) -> Vec<String> {
        let mut expired = Vec::new();
        let mut members = self.members.write();

        members.retain(|member_id, member| {
            if member.is_expired() {
                expired.push(member_id.clone());
                false
            } else {
                true
            }
        });

        if !expired.is_empty() && !members.is_empty() {
            *self.state.write() = GroupState::PreparingRebalance;
            self.generation_id.fetch_add(1, Ordering::SeqCst);
        }

        expired
    }
}

/// Generate a simple UUID-like string
fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", now)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_group() {
        let group = ConsumerGroup::new("test-group".to_string());
        assert_eq!(group.group_id(), "test-group");
        assert_eq!(group.state(), GroupState::Empty);
        assert_eq!(group.generation_id(), 0);
    }

    #[test]
    fn test_add_member() {
        let group = ConsumerGroup::new("test-group".to_string());
        let member = Member::new(
            "member-1".to_string(),
            "client-1".to_string(),
            "127.0.0.1".to_string(),
            30000,
            300000,
            "consumer".to_string(),
            vec![("range".to_string(), vec![])],
        );

        let gen = group.add_member(member);
        assert_eq!(gen, 1);
        assert_eq!(group.state(), GroupState::PreparingRebalance);
        assert_eq!(group.leader_id(), Some("member-1".to_string()));
    }

    #[test]
    fn test_remove_member_and_leader_election() {
        let group = ConsumerGroup::new("test-group".to_string());
        let member1 = Member::new(
            "member-1".to_string(),
            "client-1".to_string(),
            "127.0.0.1".to_string(),
            30000,
            300000,
            "consumer".to_string(),
            vec![("range".to_string(), vec![])],
        );
        let member2 = Member::new(
            "member-2".to_string(),
            "client-2".to_string(),
            "127.0.0.1".to_string(),
            30000,
            300000,
            "consumer".to_string(),
            vec![("range".to_string(), vec![])],
        );

        group.add_member(member1);
        group.add_member(member2);
        assert!(group.remove_member("member-1"));
        assert_eq!(group.leader_id(), Some("member-2".to_string()));

        assert!(group.remove_member("member-2"));
        assert_eq!(group.state(), GroupState::Empty);
    }

    #[test]
    fn test_remove_non_leader_keeps_leader() {
        let group = ConsumerGroup::new("test-group".to_string());
        let member1 = Member::new(
            "member-1".to_string(),
            "client-1".to_string(),
            "127.0.0.1".to_string(),
            30000,
            300000,
            "consumer".to_string(),
            vec![("range".to_string(), vec![])],
        );
        let member2 = Member::new(
            "member-2".to_string(),
            "client-2".to_string(),
            "127.0.0.1".to_string(),
            30000,
            300000,
            "consumer".to_string(),
            vec![("range".to_string(), vec![])],
        );

        group.add_member(member1);
        group.add_member(member2);

        assert!(group.remove_member("member-2"));
        assert_eq!(group.leader_id(), Some("member-1".to_string()));
    }

    #[test]
    fn test_set_assignment_missing_member() {
        let group = ConsumerGroup::new("test-group".to_string());
        group.set_assignment("missing", vec![1, 2, 3]);
        assert!(group.get_assignment("missing").is_none());
    }

    #[test]
    fn test_protocol_selection_and_rebalance() {
        let group = ConsumerGroup::new("test-group".to_string());
        assert!(group.select_protocol().is_none());

        let member = Member::new(
            "member-1".to_string(),
            "client-1".to_string(),
            "127.0.0.1".to_string(),
            30000,
            300000,
            "consumer".to_string(),
            vec![("range".to_string(), vec![]), ("roundrobin".to_string(), vec![])],
        );
        group.add_member(member);
        assert_eq!(group.select_protocol(), Some("range".to_string()));

        group.complete_rebalance("range".to_string());
        assert_eq!(group.state(), GroupState::Stable);
        assert_eq!(group.protocol(), Some("range".to_string()));
    }

    #[test]
    fn test_heartbeat_and_assignment() {
        let group = ConsumerGroup::new("test-group".to_string());
        let member = Member::new(
            "member-1".to_string(),
            "client-1".to_string(),
            "127.0.0.1".to_string(),
            30000,
            300000,
            "consumer".to_string(),
            vec![("range".to_string(), vec![])],
        );
        group.add_member(member);

        assert!(group.heartbeat("member-1"));
        assert!(!group.heartbeat("missing"));

        group.set_assignment("member-1", vec![1, 2, 3]);
        assert_eq!(group.get_assignment("member-1"), Some(vec![1, 2, 3]));
    }

    #[test]
    fn test_remove_expired_members() {
        let group = ConsumerGroup::new("test-group".to_string());
        let mut member = Member::new(
            "member-1".to_string(),
            "client-1".to_string(),
            "127.0.0.1".to_string(),
            1,
            300000,
            "consumer".to_string(),
            vec![("range".to_string(), vec![])],
        );
        member.last_heartbeat = Instant::now() - Duration::from_millis(10);
        group.add_member(member);

        let expired = group.remove_expired_members();
        assert_eq!(expired, vec!["member-1".to_string()]);
    }

    #[test]
    fn test_protocol_type_and_expired_member_retention() {
        let group = ConsumerGroup::new("test-group".to_string());

        let member1 = Member::new(
            "member-1".to_string(),
            "client".to_string(),
            "127.0.0.1".to_string(),
            5,
            5,
            "consumer".to_string(),
            vec![
                ("range".to_string(), vec![]),
                ("roundrobin".to_string(), vec![]),
            ],
        );
        let member2 = Member::new(
            "member-2".to_string(),
            "client".to_string(),
            "127.0.0.1".to_string(),
            10_000,
            10_000,
            "consumer".to_string(),
            vec![("range".to_string(), vec![])],
        );

        group.add_member(member1);
        group.add_member(member2);

        assert_eq!(group.protocol_type(), Some("consumer".to_string()));
        assert_eq!(group.select_protocol(), Some("range".to_string()));

        {
            let mut members = group.members.write();
            let expired = members.get_mut("member-1").unwrap();
            expired.last_heartbeat = Instant::now() - Duration::from_millis(50);
        }

        let expired = group.remove_expired_members();
        assert_eq!(expired, vec!["member-1".to_string()]);
        assert_eq!(group.state(), GroupState::PreparingRebalance);
    }
}
