//! Consumer group coordination
//!
//! Simplified implementation for single-node operation.
//! Handles group membership and offset storage.

mod coordinator;
mod group;
mod offset_store;

#[allow(unused_imports)]
pub use coordinator::Coordinator;
#[allow(unused_imports)]
pub use group::{ConsumerGroup, GroupState, Member, MemberState};
#[allow(unused_imports)]
pub use offset_store::OffsetStore;

use crate::config::Config;
use dashmap::DashMap;
use std::sync::Arc;
use tracing::info;

/// Consumer group manager
pub struct ConsumerGroupManager {
    /// All consumer groups
    groups: DashMap<String, Arc<ConsumerGroup>>,
    /// Offset storage
    offset_store: Arc<OffsetStore>,
    /// Configuration
    #[allow(dead_code)]
    config: Arc<Config>,
}

impl ConsumerGroupManager {
    /// Create a new consumer group manager
    pub fn new(config: Arc<Config>) -> Self {
        info!("Initializing consumer group manager");
        Self {
            groups: DashMap::new(),
            offset_store: Arc::new(OffsetStore::new()),
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
    pub fn offset_store(&self) -> &Arc<OffsetStore> {
        &self.offset_store
    }

    /// List all groups
    #[allow(dead_code)]
    pub fn list_groups(&self) -> Vec<String> {
        self.groups.iter().map(|e| e.key().clone()).collect()
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
}
