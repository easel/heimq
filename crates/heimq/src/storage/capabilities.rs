//! Backend capability descriptor.
//!
//! `BackendCapabilities` declares what a storage backend supports so that
//! higher layers (API version negotiation, feature gating, tests) can make
//! decisions without hard-coding per-backend knowledge. This module is
//! purely additive at introduction — no backend consumes it yet.

#![allow(dead_code)]

/// Durability guarantee provided by a backend for acknowledged writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Durability {
    /// Writes live only in memory; lost on restart.
    None,
    /// Writes are captured by periodic snapshots to stable storage.
    Snapshot,
    /// Writes are appended to a write-ahead log and fsynced before ack.
    WalFsync,
}

/// Scope over which a single append is atomic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AtomicAppendScope {
    /// Atomic within a single partition.
    Partition,
    /// Atomic across all partitions of a topic.
    Topic,
    /// Atomic across the whole cluster.
    Cluster,
}

/// Retention policies a backend can enforce.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RetentionMode {
    /// No automatic retention; data kept until manually removed.
    None,
    /// Retain data for a bounded time window.
    Time,
    /// Retain data up to a bounded byte size.
    Size,
    /// Combination of time and size limits.
    TimeAndSize,
    /// Compaction-based retention (latest value per key).
    Compact,
}

/// Compression codecs a backend can store / serve natively.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompressionCodec {
    None,
    Gzip,
    Snappy,
    Lz4,
    Zstd,
}

/// Declarative description of what a storage backend can do.
///
/// The struct is intentionally flat and `Clone`able so it can be embedded in
/// `ApiVersions`-style responses and test fixtures.
#[derive(Debug, Clone)]
pub struct BackendCapabilities {
    /// Human-readable backend name (e.g. `"in-memory"`, `"wal-rocksdb"`).
    pub name: &'static str,
    /// Backend implementation version string.
    pub version: &'static str,

    /// Durability guarantee for acknowledged writes.
    pub durability: Durability,
    /// Scope over which an append is atomic.
    pub atomic_append: AtomicAppendScope,
    /// Whether data survives a process restart.
    pub survives_restart: bool,

    /// Whether the backend performs log compaction.
    pub compaction: bool,
    /// Whether transactional writes are supported.
    pub transactions: bool,
    /// Whether idempotent producer semantics are supported.
    pub idempotent_producer: bool,

    /// Whether per-record timestamps are stored.
    pub timestamps: bool,
    /// Whether per-record headers are stored.
    pub headers: bool,

    /// Compression codecs the backend supports natively.
    pub compression: &'static [CompressionCodec],

    /// Maximum size of a single message in bytes.
    pub max_message_bytes: usize,
    /// Maximum size of a single produce batch in bytes.
    pub max_batch_bytes: usize,
    /// Maximum number of partitions per topic.
    pub max_partitions: u32,

    /// Whether fetch requests can wait for data (long poll).
    pub fetch_wait: bool,
    /// Whether a producer observes its own writes on a subsequent fetch.
    pub read_your_writes: bool,

    /// Retention policies the backend can enforce.
    pub retention: &'static [RetentionMode],
    /// Whether partitions can be truncated to a given offset.
    pub truncate: bool,
}

impl BackendCapabilities {
    /// Default capabilities describing a minimal, memory-only backend.
    pub const fn minimal() -> Self {
        Self {
            name: "unknown",
            version: "0.0.0",
            durability: Durability::None,
            atomic_append: AtomicAppendScope::Partition,
            survives_restart: false,
            compaction: false,
            transactions: false,
            idempotent_producer: false,
            timestamps: false,
            headers: false,
            compression: &[CompressionCodec::None],
            max_message_bytes: 1024 * 1024,
            max_batch_bytes: 1024 * 1024,
            max_partitions: 1,
            fetch_wait: false,
            read_your_writes: true,
            retention: &[RetentionMode::None],
            truncate: false,
        }
    }
}

impl Default for BackendCapabilities {
    fn default() -> Self {
        Self::minimal()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_minimal_capabilities() {
        let caps = BackendCapabilities::default();
        assert_eq!(caps.name, "unknown");
        assert_eq!(caps.version, "0.0.0");
        assert_eq!(caps.durability, Durability::None);
        assert_eq!(caps.atomic_append, AtomicAppendScope::Partition);
        assert!(!caps.survives_restart);
        assert!(!caps.compaction);
        assert!(!caps.transactions);
        assert!(!caps.idempotent_producer);
        assert!(!caps.timestamps);
        assert!(!caps.headers);
        assert_eq!(caps.compression, &[CompressionCodec::None]);
        assert_eq!(caps.max_message_bytes, 1024 * 1024);
        assert_eq!(caps.max_batch_bytes, 1024 * 1024);
        assert_eq!(caps.max_partitions, 1);
        assert!(!caps.fetch_wait);
        assert!(caps.read_your_writes);
        assert_eq!(caps.retention, &[RetentionMode::None]);
        assert!(!caps.truncate);
    }

    #[test]
    fn construct_custom_capabilities() {
        let caps = BackendCapabilities {
            name: "wal",
            version: "1.2.3",
            durability: Durability::WalFsync,
            atomic_append: AtomicAppendScope::Topic,
            survives_restart: true,
            compaction: true,
            transactions: true,
            idempotent_producer: true,
            timestamps: true,
            headers: true,
            compression: &[CompressionCodec::Zstd, CompressionCodec::Lz4],
            max_message_bytes: 16 * 1024 * 1024,
            max_batch_bytes: 64 * 1024 * 1024,
            max_partitions: 1024,
            fetch_wait: true,
            read_your_writes: true,
            retention: &[RetentionMode::TimeAndSize, RetentionMode::Compact],
            truncate: true,
        };
        assert_eq!(caps.durability, Durability::WalFsync);
        assert_eq!(caps.atomic_append, AtomicAppendScope::Topic);
        assert!(caps.survives_restart);
        assert_eq!(caps.compression.len(), 2);
        assert_eq!(caps.retention.len(), 2);
    }
}
