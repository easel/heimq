//! In-memory implementation of the log-backend traits.
//!
//! Designed for speed over durability. Uses a simple segment-based log
//! structure. This was previously the concrete `Storage` struct; it now
//! implements [`LogBackend`] alongside keeping its legacy inherent API
//! so existing call sites (and tests) continue to compile.

use crate::config::Config;
use crate::error::{HeimqError, Result};
use crate::storage::{BackendCapabilities, LogBackend, MemoryTopicLog, TopicLog};
use dashmap::DashMap;
use metrics::{counter, gauge};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use tracing::{debug, info};

/// The in-memory log backend.
pub struct MemoryLog {
    /// Topics by name.
    topics: DashMap<String, Arc<MemoryTopicLog>>,
    /// Configuration.
    config: Arc<Config>,
    /// Global message counter (for debugging/diagnostics).
    total_messages: AtomicI64,
    /// Total bytes of stored record batches across all partitions. Bounds the
    /// memory cap and feeds backpressure.
    total_bytes: AtomicI64,
    /// Backend capabilities descriptor.
    capabilities: BackendCapabilities,
}

impl MemoryLog {
    /// Create a new in-memory backend.
    pub fn new(config: Arc<Config>) -> Self {
        Self::with_capabilities(
            config,
            BackendCapabilities {
                name: "in-memory",
                transactions: true,
                idempotent_producer: true,
                // Memory is the only limit; no artificial cap on message size.
                max_message_bytes: usize::MAX,
                max_batch_bytes: usize::MAX,
                ..BackendCapabilities::minimal()
            },
        )
    }

    /// Create a new in-memory backend with caller-supplied capabilities.
    ///
    /// Used in tests to exercise call-time rejection paths (e.g. forcing a
    /// small `max_message_bytes` so oversized produce requests are rejected).
    pub fn with_capabilities(config: Arc<Config>, capabilities: BackendCapabilities) -> Self {
        info!("Initializing in-memory storage backend");
        Self {
            topics: DashMap::new(),
            config,
            total_messages: AtomicI64::new(0),
            total_bytes: AtomicI64::new(0),
            capabilities,
        }
    }

    /// Total messages written across all partitions (diagnostic).
    #[allow(dead_code)]
    pub fn total_messages(&self) -> i64 {
        self.total_messages.load(Ordering::Relaxed)
    }

    /// Total bytes of stored record batches across all partitions.
    pub fn total_bytes(&self) -> i64 {
        self.total_bytes.load(Ordering::Relaxed)
    }

    fn set_total_bytes_metric(&self) {
        gauge!("heimq_memory_log_bytes").set(self.total_bytes() as f64);
    }

    fn record_reclaimed_bytes(reason: &'static str, bytes: usize) {
        if bytes > 0 {
            counter!("heimq_retention_reclaimed_bytes_total", "reason" => reason)
                .increment(bytes as u64);
        }
    }

    fn resolve_topic_for_append(&self, name: &str) -> Result<Arc<MemoryTopicLog>> {
        if self.config.auto_create_topics {
            Ok(self.get_or_create_memory_topic(name, self.config.default_partitions))
        } else {
            self.get_memory_topic(name)
                .ok_or_else(|| HeimqError::TopicNotFound(name.to_string()))
        }
    }

    fn get_memory_topic(&self, name: &str) -> Option<Arc<MemoryTopicLog>> {
        self.topics.get(name).map(|t| t.clone())
    }

    fn get_or_create_memory_topic(&self, name: &str, num_partitions: i32) -> Arc<MemoryTopicLog> {
        if let Some(topic) = self.topics.get(name) {
            return topic.clone();
        }
        let topic = Arc::new(MemoryTopicLog::new(name.to_string(), num_partitions));
        self.topics.insert(name.to_string(), topic.clone());
        info!(topic = name, partitions = num_partitions, "Created topic");
        topic
    }
}

impl LogBackend for MemoryLog {
    fn create_topic(&self, name: &str, num_partitions: i32) -> Result<Arc<dyn TopicLog>> {
        if self.topics.contains_key(name) {
            return Err(HeimqError::Protocol(format!(
                "Topic '{}' already exists",
                name
            )));
        }

        let topic = Arc::new(MemoryTopicLog::new(name.to_string(), num_partitions));
        self.topics.insert(name.to_string(), topic.clone());
        info!(topic = name, partitions = num_partitions, "Created topic");
        Ok(topic as Arc<dyn TopicLog>)
    }

    fn delete_topic(&self, name: &str) -> Result<()> {
        if self.topics.remove(name).is_none() {
            return Err(HeimqError::TopicNotFound(name.to_string()));
        }
        info!(topic = name, "Deleted topic");
        Ok(())
    }

    fn list_topics(&self) -> Vec<String> {
        self.topics.iter().map(|e| e.key().clone()).collect()
    }

    fn topic(&self, name: &str) -> Option<Arc<dyn TopicLog>> {
        self.get_memory_topic(name).map(|t| t as Arc<dyn TopicLog>)
    }

    fn capabilities(&self) -> &BackendCapabilities {
        &self.capabilities
    }

    fn get_or_create_topic(&self, name: &str, num_partitions: i32) -> Arc<dyn TopicLog> {
        self.get_or_create_memory_topic(name, num_partitions) as Arc<dyn TopicLog>
    }

    fn get_all_topic_metadata(&self) -> Vec<(String, i32)> {
        self.topics
            .iter()
            .map(|e| (e.key().clone(), e.value().num_partitions()))
            .collect()
    }

    fn default_num_partitions(&self) -> i32 {
        self.config.default_partitions
    }

    fn auto_create_topics(&self) -> bool {
        self.config.auto_create_topics
    }

    fn expand_topic_partitions(&self, name: &str, new_count: i32) -> Result<()> {
        let existing = self
            .get_memory_topic(name)
            .ok_or_else(|| crate::error::HeimqError::TopicNotFound(name.to_string()))?;
        let current = existing.num_partitions();
        if new_count <= current {
            return Err(crate::error::HeimqError::Protocol(format!(
                "topic '{}' already has {} partitions; new count {} must be greater",
                name, current, new_count
            )));
        }
        let expanded = Arc::new(MemoryTopicLog::new_expanded(
            name.to_string(),
            existing.partitions(),
            new_count,
        ));
        self.topics.insert(name.to_string(), expanded);
        info!(
            topic = name,
            from = current,
            to = new_count,
            "Expanded topic partitions"
        );
        Ok(())
    }

    fn append(&self, topic_name: &str, partition: i32, records: &[u8]) -> Result<(i64, i64)> {
        // Memory cap + backpressure: if accepting this batch would exceed the cap,
        // first drop expired data; if it is still over, reject with a retriable
        // storage error so the producer backs off. This keeps memory bounded
        // without evicting un-expired data (preserving the retention.ms contract).
        let cap = self.config.max_memory_bytes;
        if cap > 0 && self.total_bytes.load(Ordering::Relaxed) as u64 + records.len() as u64 > cap {
            let now_ms = chrono::Utc::now().timestamp_millis();
            self.reclaim_expired(now_ms, self.config.retention_ms);
            if self.total_bytes.load(Ordering::Relaxed) as u64 + records.len() as u64 > cap {
                counter!("heimq_storage_full_errors_total").increment(1);
                self.set_total_bytes_metric();
                return Err(HeimqError::StorageFull(format!(
                    "in-memory cap {} bytes reached and no expired data to reclaim",
                    cap
                )));
            }
        }

        let topic = self.resolve_topic_for_append(topic_name)?;
        let partition_log = topic.get_memory_partition(partition)?;
        let (base_offset, count) = partition_log.append_raw(records);
        self.total_messages.fetch_add(count, Ordering::Relaxed);
        self.total_bytes
            .fetch_add(records.len() as i64, Ordering::Relaxed);
        counter!("heimq_memory_log_messages_total").increment(count as u64);
        self.set_total_bytes_metric();

        debug!(
            topic = topic_name,
            partition = partition_log.id(),
            base_offset = base_offset,
            count = count,
            "Appended records"
        );

        Ok((base_offset, count))
    }

    fn reclaim_expired(&self, now_ms: i64, retention_ms: u64) -> usize {
        let cutoff = now_ms.saturating_sub(retention_ms as i64);
        let mut freed = 0usize;
        for topic in self.topics.iter() {
            for p in 0..topic.num_partitions() {
                if let Ok(part) = topic.get_memory_partition(p) {
                    freed += part.reclaim_expired(cutoff);
                }
            }
        }
        if freed > 0 {
            self.total_bytes.fetch_sub(freed as i64, Ordering::Relaxed);
            Self::record_reclaimed_bytes("retention_ms", freed);
            self.set_total_bytes_metric();
        }
        freed
    }

    fn reclaim_topic(&self, topic: &str, cutoff_ms: i64, retention_bytes: i64) -> usize {
        let t = match self.get_memory_topic(topic) {
            Some(t) => t,
            None => return 0,
        };
        let mut freed = 0usize;
        let mut expired_freed = 0usize;
        let mut size_freed = 0usize;
        for p in 0..t.num_partitions() {
            if let Ok(part) = t.get_memory_partition(p) {
                let expired = part.reclaim_expired(cutoff_ms);
                expired_freed += expired;
                freed += expired;
                if retention_bytes >= 0 {
                    let size = part.reclaim_to_size(retention_bytes as usize);
                    size_freed += size;
                    freed += size;
                }
            }
        }
        if freed > 0 {
            self.total_bytes.fetch_sub(freed as i64, Ordering::Relaxed);
            Self::record_reclaimed_bytes("retention_ms", expired_freed);
            Self::record_reclaimed_bytes("retention_bytes", size_freed);
            self.set_total_bytes_metric();
        }
        freed
    }

    fn fetch(
        &self,
        topic_name: &str,
        partition: i32,
        offset: i64,
        max_bytes: i32,
    ) -> Result<(Vec<u8>, i64)> {
        let topic = self
            .get_memory_topic(topic_name)
            .ok_or_else(|| HeimqError::TopicNotFound(topic_name.to_string()))?;
        let partition_log = topic.get_memory_partition(partition)?;
        partition_log.fetch(offset, max_bytes as usize)
    }

    fn high_watermark(&self, topic_name: &str, partition: i32) -> Result<i64> {
        let topic = self
            .get_memory_topic(topic_name)
            .ok_or_else(|| HeimqError::TopicNotFound(topic_name.to_string()))?;
        let partition_log = topic.get_memory_partition(partition)?;
        Ok(partition_log.high_watermark())
    }

    fn log_start_offset(&self, topic_name: &str, partition: i32) -> Result<i64> {
        let topic = self
            .get_memory_topic(topic_name)
            .ok_or_else(|| HeimqError::TopicNotFound(topic_name.to_string()))?;
        let partition_log = topic.get_memory_partition(partition)?;
        Ok(partition_log.log_start_offset())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> Arc<Config> {
        Arc::new(Config {
            host: "127.0.0.1".to_string(),
            port: 9092,
            data_dir: std::path::PathBuf::from("/tmp/heimq-test"),
            memory_only: true,
            segment_size: 1024 * 1024,
            retention_ms: 60000,
            max_memory_bytes: 0,
            default_partitions: 1,
            auto_create_topics: true,
            broker_id: 0,
            cluster_id: "test".to_string(),
            metrics: false,
            metrics_port: 9093,
            create_topics: Vec::new(),
            storage_log: "memory://".to_string(),
            storage_offsets: "memory://".to_string(),
            storage_groups: "memory://".to_string(),
            advertised_host: None,
        })
    }

    #[test]
    fn test_create_topic() {
        let storage = MemoryLog::new(test_config());
        let topic = storage.create_topic("test", 3).unwrap();
        assert_eq!(topic.name(), "test");
        assert_eq!(topic.num_partitions(), 3);
    }

    #[test]
    fn test_list_topics() {
        let storage = MemoryLog::new(test_config());
        storage.create_topic("topic1", 1).unwrap();
        storage.create_topic("topic2", 2).unwrap();
        let topics = storage.list_topics();
        assert_eq!(topics.len(), 2);
    }

    #[test]
    fn test_get_all_metadata_and_delete_missing() {
        let storage = MemoryLog::new(test_config());
        storage.create_topic("topic1", 1).unwrap();
        storage.create_topic("topic2", 2).unwrap();
        let metadata = storage.get_all_topic_metadata();
        assert_eq!(metadata.len(), 2);

        let missing = storage.delete_topic("missing");
        assert!(missing.is_err());
    }

    #[test]
    fn test_append_fetch_total_messages() {
        let storage = MemoryLog::new(test_config());
        storage.create_topic("topic", 1).unwrap();
        let records = vec![1, 2, 3];
        let (base_offset, count) = storage.append("topic", 0, &records).unwrap();
        assert_eq!(base_offset, 0);
        assert_eq!(count, 1);
        assert_eq!(storage.total_messages(), 1);

        let (fetched, hw) = storage.fetch("topic", 0, 0, 1024).unwrap();
        assert_eq!(fetched, records);
        assert_eq!(hw, 1);
    }

    #[test]
    fn test_async_append_default_bridge_matches_sync_append() {
        let storage = MemoryLog::new(test_config());
        storage.create_topic("topic", 1).unwrap();
        let records = vec![1, 2, 3];

        let (base_offset, count) =
            tokio_test::block_on(LogBackend::append_async(&storage, "topic", 0, &records)).unwrap();

        assert_eq!(base_offset, 0);
        assert_eq!(count, 1);
        assert_eq!(storage.total_messages(), 1);

        let (fetched, hw) = storage.fetch("topic", 0, 0, 1024).unwrap();
        assert_eq!(fetched, records);
        assert_eq!(hw, 1);
    }

    #[test]
    fn test_auto_create_disabled() {
        let mut config = (*test_config()).clone();
        config.auto_create_topics = false;
        let storage = MemoryLog::new(Arc::new(config));
        let result = storage.append("missing", 0, &[1, 2, 3]);
        assert!(result.is_err());
    }

    #[test]
    fn test_fetch_invalid_offset() {
        let storage = MemoryLog::new(test_config());
        storage.create_topic("topic", 1).unwrap();
        let result = storage.fetch("topic", 0, -1, 1024);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_partition_errors() {
        let storage = MemoryLog::new(test_config());
        storage.create_topic("topic", 1).unwrap();

        assert!(storage.append("topic", 2, &[1, 2, 3]).is_err());
        assert!(storage.fetch("topic", 2, 0, 1024).is_err());
        assert!(storage.high_watermark("topic", 2).is_err());
        assert!(storage.log_start_offset("topic", 2).is_err());
    }

    #[test]
    fn test_capabilities_name_is_in_memory() {
        let storage = MemoryLog::new(test_config());
        assert_eq!(storage.capabilities().name, "in-memory");
    }

    #[test]
    fn test_trait_path_append_and_read() {
        use crate::storage::{FetchWait, RecordBatchView};
        use bytes::{Bytes, BytesMut};
        use kafka_protocol::indexmap::IndexMap;
        use kafka_protocol::protocol::StrBytes;
        use kafka_protocol::records::{
            Compression, Record, RecordBatchEncoder, RecordEncodeOptions, TimestampType,
        };

        let storage = MemoryLog::new(test_config());
        let _ = storage.create_topic("trait-t", 1).unwrap();

        let mut headers = IndexMap::new();
        headers.insert(
            StrBytes::from_static_str("h"),
            Some(Bytes::from_static(b"v")),
        );
        let records = vec![Record {
            transactional: false,
            control: false,
            partition_leader_epoch: 0,
            producer_id: 1,
            producer_epoch: 0,
            timestamp_type: TimestampType::Creation,
            offset: 0,
            sequence: 0,
            timestamp: 1,
            key: Some(Bytes::from_static(b"k")),
            value: Some(Bytes::from_static(b"v")),
            headers,
        }];
        let mut buf = BytesMut::new();
        RecordBatchEncoder::encode(
            &mut buf,
            &records,
            &RecordEncodeOptions {
                version: 2,
                compression: Compression::None,
            },
        )
        .unwrap();
        let raw = buf.freeze();

        let view = RecordBatchView::from_bytes(&raw).unwrap();

        let topic = storage.topic("trait-t").unwrap();
        let partition = topic.partition(0).unwrap();
        let (base, count) = partition
            .append(&view, Some(&raw))
            .expect("append via trait");
        assert_eq!(base, 0);
        assert_eq!(count, 1);

        let (data, hw) = partition
            .read(
                0,
                1024,
                FetchWait::LongPoll {
                    min_bytes: 1,
                    max_wait_ms: 0,
                },
            )
            .unwrap();
        assert_eq!(hw, 1);
        assert!(!data.is_empty());

        // `append` with `raw_bytes: None` falls back to `view.raw()`.
        let (base2, count2) = partition.append(&view, None).unwrap();
        assert_eq!(base2, 1);
        assert_eq!(count2, 1);

        assert_eq!(topic.name(), "trait-t");
        assert_eq!(topic.num_partitions(), 1);
        assert_eq!(topic.config().num_partitions, 1);
        assert_eq!(partition.id(), 0);
        assert_eq!(partition.high_watermark(), 2);
        assert_eq!(partition.log_start_offset(), 0);
        partition.truncate_before(1).unwrap();
        assert_eq!(partition.log_start_offset(), 1);
    }
}
