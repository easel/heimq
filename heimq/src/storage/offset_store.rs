//! Consumer-group offset storage abstraction.
//!
//! Declares the `OffsetStore` trait that pluggable offset backends implement
//! (in-memory today, Postgres or other durable stores later) and the
//! `CommittedOffset` / `OffsetStoreCapabilities` value types.
//!
//! The in-memory implementation (`MemoryOffsetStore`) lives in
//! `crate::consumer_group::offset_store` — kept alongside the rest of the
//! single-node consumer-group state so that a future durable backend can
//! replace it without touching consumer-group code.

#![allow(dead_code)]

use crate::storage::Durability;
use std::collections::HashMap;

/// Committed offset record stored by an [`OffsetStore`].
#[derive(Debug, Clone)]
pub struct CommittedOffset {
    pub offset: i64,
    pub leader_epoch: i32,
    pub metadata: Option<String>,
    pub commit_timestamp: i64,
}

/// Declarative description of what an offset-storage backend can do.
#[derive(Debug, Clone)]
pub struct OffsetStoreCapabilities {
    /// Human-readable backend name (e.g. `"in-memory"`, `"postgres"`).
    pub name: &'static str,
    /// Backend implementation version string.
    pub version: &'static str,
    /// Durability guarantee for acknowledged commits.
    pub durability: Durability,
    /// Whether committed offsets survive a process restart.
    pub survives_restart: bool,
}

impl OffsetStoreCapabilities {
    /// Default capabilities for a non-durable, in-memory offset store.
    pub const fn minimal() -> Self {
        Self {
            name: "unknown",
            version: "0.0.0",
            durability: Durability::None,
            survives_restart: false,
        }
    }
}

impl Default for OffsetStoreCapabilities {
    fn default() -> Self {
        Self::minimal()
    }
}

/// A pluggable consumer-group offset store.
pub trait OffsetStore: Send + Sync {
    /// Commit an offset for `(group, topic, partition)`.
    fn commit(
        &self,
        group_id: &str,
        topic: &str,
        partition: i32,
        offset: i64,
        leader_epoch: i32,
        metadata: Option<String>,
    );

    /// Fetch the committed offset for `(group, topic, partition)`.
    fn fetch(&self, group_id: &str, topic: &str, partition: i32) -> Option<CommittedOffset>;

    /// Fetch every committed offset for a group, keyed by `(topic, partition)`.
    fn fetch_all_for_group(
        &self,
        group_id: &str,
    ) -> HashMap<(String, i32), CommittedOffset>;

    /// Drop all committed offsets for a group.
    fn delete_group(&self, group_id: &str);

    /// Backend capabilities descriptor.
    fn capabilities(&self) -> &OffsetStoreCapabilities;
}
