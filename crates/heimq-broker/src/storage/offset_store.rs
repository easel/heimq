//! Consumer-group offset storage abstraction.
//!
//! Declares the `OffsetStore` trait that pluggable offset backends implement
//! (in-memory today, Postgres or other durable stores later) and the
//! `CommittedOffset` / `OffsetStoreCapabilities` value types.

#![allow(dead_code)]

use crate::error::Result;
use crate::storage::Durability;
use crate::RequestContext;
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
    ///
    /// Returns `Ok(())` when the commit is durable per the backend's
    /// `OffsetStoreCapabilities::durability` guarantee. A durable backend MUST
    /// complete all persistence work before returning so callers can rely on
    /// read-your-writes semantics (TRAIT-001 complete-work-before-ack).
    fn commit(
        &self,
        group_id: &str,
        topic: &str,
        partition: i32,
        offset: i64,
        leader_epoch: i32,
        metadata: Option<String>,
    ) -> Result<()>;

    /// Context-aware commit entrypoint for request-serving callers.
    #[allow(clippy::too_many_arguments)]
    fn commit_with_context(
        &self,
        ctx: &RequestContext,
        group_id: &str,
        topic: &str,
        partition: i32,
        offset: i64,
        leader_epoch: i32,
        metadata: Option<String>,
    ) -> Result<()> {
        let _ = ctx;
        self.commit(group_id, topic, partition, offset, leader_epoch, metadata)
    }

    /// Fetch the committed offset for `(group, topic, partition)`.
    fn fetch(&self, group_id: &str, topic: &str, partition: i32) -> Option<CommittedOffset>;

    fn fetch_with_context(
        &self,
        ctx: &RequestContext,
        group_id: &str,
        topic: &str,
        partition: i32,
    ) -> Option<CommittedOffset> {
        let _ = ctx;
        self.fetch(group_id, topic, partition)
    }

    /// Fetch every committed offset for a group, keyed by `(topic, partition)`.
    fn fetch_all_for_group(&self, group_id: &str) -> HashMap<(String, i32), CommittedOffset>;

    fn fetch_all_for_group_with_context(
        &self,
        ctx: &RequestContext,
        group_id: &str,
    ) -> HashMap<(String, i32), CommittedOffset> {
        let _ = ctx;
        self.fetch_all_for_group(group_id)
    }

    /// Drop all committed offsets for a group.
    fn delete_group(&self, group_id: &str);

    fn delete_group_with_context(&self, ctx: &RequestContext, group_id: &str) {
        let _ = ctx;
        self.delete_group(group_id);
    }

    /// Drop the committed offset for a specific (group, topic, partition) triple.
    /// No-op if the offset does not exist.
    fn delete_offset(&self, group_id: &str, topic: &str, partition: i32);

    fn delete_offset_with_context(
        &self,
        ctx: &RequestContext,
        group_id: &str,
        topic: &str,
        partition: i32,
    ) {
        let _ = ctx;
        self.delete_offset(group_id, topic, partition);
    }

    /// Backend capabilities descriptor.
    fn capabilities(&self) -> &OffsetStoreCapabilities;
}
