//! In-memory implementation of the [`OffsetStore`] trait.
//!
//! `MemoryOffsetStore` is the default single-node backend: commits live in
//! a `DashMap` and are lost on restart. Durable backends (Postgres, etc.)
//! implement the same trait in separate modules.

use crate::storage::{CommittedOffset, Durability, OffsetStore, OffsetStoreCapabilities};
use dashmap::DashMap;
use std::collections::HashMap;

/// Key for offset storage: (group_id, topic, partition)
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct OffsetKey {
    group_id: String,
    topic: String,
    partition: i32,
}

const MEMORY_CAPABILITIES: OffsetStoreCapabilities = OffsetStoreCapabilities {
    name: "in-memory",
    version: "0.1.0",
    durability: Durability::None,
    survives_restart: false,
};

/// In-memory offset storage backend.
pub struct MemoryOffsetStore {
    offsets: DashMap<OffsetKey, CommittedOffset>,
    capabilities: OffsetStoreCapabilities,
}

impl MemoryOffsetStore {
    pub fn new() -> Self {
        Self {
            offsets: DashMap::new(),
            capabilities: MEMORY_CAPABILITIES,
        }
    }
}

impl Default for MemoryOffsetStore {
    fn default() -> Self {
        Self::new()
    }
}

impl OffsetStore for MemoryOffsetStore {
    fn commit(
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

    fn fetch(&self, group_id: &str, topic: &str, partition: i32) -> Option<CommittedOffset> {
        let key = OffsetKey {
            group_id: group_id.to_string(),
            topic: topic.to_string(),
            partition,
        };

        self.offsets.get(&key).map(|e| e.clone())
    }

    fn fetch_all_for_group(
        &self,
        group_id: &str,
    ) -> HashMap<(String, i32), CommittedOffset> {
        self.offsets
            .iter()
            .filter(|e| e.key().group_id == group_id)
            .map(|e| ((e.key().topic.clone(), e.key().partition), e.value().clone()))
            .collect()
    }

    fn delete_group(&self, group_id: &str) {
        self.offsets.retain(|k, _| k.group_id != group_id);
    }

    fn capabilities(&self) -> &OffsetStoreCapabilities {
        &self.capabilities
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_and_fetch() {
        let store = MemoryOffsetStore::new();
        store.commit("group1", "topic1", 0, 100, 0, None);

        let offset = store.fetch("group1", "topic1", 0).unwrap();
        assert_eq!(offset.offset, 100);
    }

    #[test]
    fn test_fetch_missing() {
        let store = MemoryOffsetStore::new();
        assert!(store.fetch("group1", "topic1", 0).is_none());
    }

    #[test]
    fn test_fetch_all_and_delete_group() {
        let store = MemoryOffsetStore::new();
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
        let store = MemoryOffsetStore::default();
        assert!(store.fetch("missing", "topic", 0).is_none());
    }

    #[test]
    fn test_capabilities() {
        let store = MemoryOffsetStore::new();
        let caps = store.capabilities();
        assert_eq!(caps.name, "in-memory");
        assert_eq!(caps.durability, Durability::None);
        assert!(!caps.survives_restart);
    }

    #[test]
    fn test_trait_object() {
        let store: std::sync::Arc<dyn OffsetStore> =
            std::sync::Arc::new(MemoryOffsetStore::new());
        store.commit("g", "t", 0, 5, 0, None);
        assert_eq!(store.fetch("g", "t", 0).unwrap().offset, 5);
    }
}
