//! Storage abstraction for heimq
//!
//! Re-exports the `LogBackend` / `TopicLog` / `PartitionLog` trait families
//! from `heimq-broker` and provides in-memory implementations.

mod cluster_view;
mod dispatch;
mod memory;
mod partition;
#[cfg(feature = "backend-postgres")]
mod postgres_offsets;
mod segment;
mod topic;

// Trait families and supporting types live in heimq-broker.
pub use heimq_broker::storage::{
    AtomicAppendScope, BackendCapabilities, CommittedOffset, CompressionCodec, Durability,
    FetchWait, LogBackend, OffsetStore, OffsetStoreCapabilities, PartitionLog, Record,
    RetentionMode, TopicConfig, TopicLog,
};
pub use heimq_broker::storage::{BrokerInfo, ClusterView, ClusterViewError};
pub use heimq_broker::storage::{RecordBatchView, RecordView};

pub use cluster_view::SingleNodeClusterView;
pub use dispatch::{dispatch_group_coordinator, dispatch_log_backend, dispatch_offset_store};
pub use memory::MemoryLog;
pub use partition::MemoryPartitionLog;
#[cfg(feature = "backend-postgres")]
pub use postgres_offsets::PostgresOffsetStore;
pub use segment::Segment;
pub use topic::MemoryTopicLog;
