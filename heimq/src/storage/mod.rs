//! Storage abstraction for heimq
//!
//! Defines the `LogBackend` / `TopicLog` / `PartitionLog` traits and
//! re-exports the in-memory implementation (`MemoryLog`).

mod capabilities;
mod dispatch;
mod memory;
mod offset_store;
mod partition;
mod record_batch_view;
mod segment;
mod topic;

pub use capabilities::{
    AtomicAppendScope, BackendCapabilities, CompressionCodec, Durability, RetentionMode,
};
pub use dispatch::{dispatch_group_coordinator, dispatch_log_backend, dispatch_offset_store};
pub use memory::MemoryLog;
pub use offset_store::{CommittedOffset, OffsetStore, OffsetStoreCapabilities};
pub use partition::MemoryPartitionLog;
pub use record_batch_view::{RecordBatchView, RecordView};
pub use segment::Segment;
pub use topic::MemoryTopicLog;

use crate::config::Config;
use crate::error::Result;
use bytes::Bytes;
use std::sync::Arc;

/// Record stored in a partition
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Record {
    pub offset: i64,
    pub timestamp: i64,
    pub key: Option<Bytes>,
    pub value: Option<Bytes>,
    pub headers: Vec<(Bytes, Bytes)>,
}

/// Topic-level configuration exposed by [`TopicLog::config`].
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TopicConfig {
    pub num_partitions: i32,
}

/// Wait semantics for a fetch/read operation.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum FetchWait {
    /// Return immediately with whatever is available.
    Immediate,
    /// Wait up to `max_wait_ms` for at least `min_bytes` to be available.
    LongPoll { min_bytes: i32, max_wait_ms: i32 },
}

/// A pluggable log-storage backend.
pub trait LogBackend: Send + Sync {
    /// Create a new topic with the given partition count.
    fn create_topic(&self, name: &str, num_partitions: i32) -> Result<Arc<dyn TopicLog>>;

    /// Delete an existing topic.
    fn delete_topic(&self, name: &str) -> Result<()>;

    /// List all topic names.
    fn list_topics(&self) -> Vec<String>;

    /// Look up a topic by name.
    fn topic(&self, name: &str) -> Option<Arc<dyn TopicLog>>;

    /// Backend capabilities descriptor.
    fn capabilities(&self) -> &BackendCapabilities;

    /// Look up a topic, creating it with `num_partitions` if it doesn't exist.
    fn get_or_create_topic(&self, name: &str, num_partitions: i32) -> Arc<dyn TopicLog>;

    /// Return `(topic_name, num_partitions)` for every known topic.
    fn get_all_topic_metadata(&self) -> Vec<(String, i32)>;

    /// Runtime configuration of this backend.
    fn config(&self) -> &Config;

    /// Append a raw record-batch to a (topic, partition) pair.
    ///
    /// Convenience that replicates the pre-trait `Storage::append` behaviour
    /// and auto-creates the topic when `config().auto_create_topics` is set.
    fn append(&self, topic_name: &str, partition: i32, records: &[u8]) -> Result<(i64, i64)>;

    /// Fetch records starting at `offset`, up to `max_bytes`.
    fn fetch(
        &self,
        topic_name: &str,
        partition: i32,
        offset: i64,
        max_bytes: i32,
    ) -> Result<(Vec<u8>, i64)>;

    /// High watermark (next offset to be written) for a partition.
    fn high_watermark(&self, topic_name: &str, partition: i32) -> Result<i64>;

    /// Earliest available offset for a partition.
    fn log_start_offset(&self, topic_name: &str, partition: i32) -> Result<i64>;
}

/// A single topic inside a [`LogBackend`].
pub trait TopicLog: Send + Sync {
    fn name(&self) -> &str;
    fn num_partitions(&self) -> i32;
    fn partition(&self, id: i32) -> Result<Arc<dyn PartitionLog>>;
    fn config(&self) -> &TopicConfig;
}

/// A single partition log.
#[allow(dead_code)]
pub trait PartitionLog: Send + Sync {
    fn id(&self) -> i32;

    /// Append a structured record batch.
    ///
    /// `raw_bytes` — when provided — is the verbatim wire-format encoding of
    /// the batch. Backends that pass through raw bytes (like the in-memory
    /// backend) should prefer `raw_bytes` over re-encoding the view.
    fn append(
        &self,
        view: &RecordBatchView<'_>,
        raw_bytes: Option<&[u8]>,
    ) -> Result<(i64, i64)>;

    /// Read records starting at `offset`, up to `max_bytes`.
    fn read(
        &self,
        offset: i64,
        max_bytes: usize,
        wait: FetchWait,
    ) -> Result<(Vec<u8>, i64)>;

    fn log_start_offset(&self) -> i64;
    fn high_watermark(&self) -> i64;

    /// Truncate the log so offsets below `offset` are no longer retained.
    fn truncate_before(&self, offset: i64) -> Result<()>;
}
