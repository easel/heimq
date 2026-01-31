//! Offset storage for consumer groups

use dashmap::DashMap;
use std::collections::HashMap;

/// Key for offset storage: (group_id, topic, partition)
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct OffsetKey {
    pub group_id: String,
    pub topic: String,
    pub partition: i32,
}

/// Committed offset with metadata
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CommittedOffset {
    pub offset: i64,
    pub leader_epoch: i32,
    pub metadata: Option<String>,
    pub commit_timestamp: i64,
}

/// In-memory offset storage
pub struct OffsetStore {
    offsets: DashMap<OffsetKey, CommittedOffset>,
}

impl OffsetStore {
    pub fn new() -> Self {
        Self {
            offsets: DashMap::new(),
        }
    }

    /// Commit an offset
    pub fn commit(
        &self,
        group_id: &str,
        topic: &str,
        partition: i32,
        offset: i64,
        leader_epoch: i32,
        metadata: Option<String>,
    ) {
        let key = OffsetKey {
            group_id: group_id.to_string(),
            topic: topic.to_string(),
            partition,
        };

        let committed = CommittedOffset {
            offset,
            leader_epoch,
            metadata,
            commit_timestamp: chrono::Utc::now().timestamp_millis(),
        };

        self.offsets.insert(key, committed);
    }

    /// Fetch committed offset
    pub fn fetch(
        &self,
        group_id: &str,
        topic: &str,
        partition: i32,
    ) -> Option<CommittedOffset> {
        let key = OffsetKey {
            group_id: group_id.to_string(),
            topic: topic.to_string(),
            partition,
        };

        self.offsets.get(&key).map(|e| e.clone())
    }

    /// Fetch all offsets for a group
    pub fn fetch_all_for_group(
        &self,
        group_id: &str,
    ) -> HashMap<(String, i32), CommittedOffset> {
        self.offsets
            .iter()
            .filter(|e| e.key().group_id == group_id)
            .map(|e| ((e.key().topic.clone(), e.key().partition), e.value().clone()))
            .collect()
    }

    /// Delete all offsets for a group
    #[allow(dead_code)]
    pub fn delete_group(&self, group_id: &str) {
        self.offsets.retain(|k, _| k.group_id != group_id);
    }
}

impl Default for OffsetStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_and_fetch() {
        let store = OffsetStore::new();
        store.commit("group1", "topic1", 0, 100, 0, None);

        let offset = store.fetch("group1", "topic1", 0).unwrap();
        assert_eq!(offset.offset, 100);
    }

    #[test]
    fn test_fetch_missing() {
        let store = OffsetStore::new();
        assert!(store.fetch("group1", "topic1", 0).is_none());
    }

    #[test]
    fn test_fetch_all_and_delete_group() {
        let store = OffsetStore::new();
        store.commit("group1", "topic1", 0, 10, 0, None);
        store.commit("group1", "topic2", 1, 20, 0, Some("meta".into()));
        store.commit("group2", "topic1", 0, 30, 0, None);

        let all = store.fetch_all_for_group("group1");
        assert_eq!(all.len(), 2);

        store.delete_group("group1");
        assert!(store.fetch("group1", "topic1", 0).is_none());
        assert!(store.fetch("group1", "topic2", 1).is_none());
        assert!(store.fetch("group2", "topic1", 0).is_some());
    }

    #[test]
    fn test_default_store() {
        let store = OffsetStore::default();
        assert!(store.fetch("missing", "topic", 0).is_none());
    }
}
