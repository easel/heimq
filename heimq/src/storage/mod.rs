//! In-memory storage engine for heimq
//!
//! Designed for speed over durability. Uses a simple segment-based log
//! structure with optional persistence.

mod capabilities;
mod partition;
mod record_batch_view;
mod segment;
mod topic;

pub use capabilities::{
    AtomicAppendScope, BackendCapabilities, CompressionCodec, Durability, RetentionMode,
};
pub use partition::Partition;
pub use record_batch_view::{RecordBatchView, RecordView};
pub use segment::Segment;
pub use topic::Topic;

use crate::config::Config;
use crate::error::{HeimqError, Result};
use bytes::Bytes;
use dashmap::DashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use tracing::{debug, info};

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

/// The main storage engine
pub struct Storage {
    /// Topics by name
    topics: DashMap<String, Arc<Topic>>,
    /// Configuration
    config: Arc<Config>,
    /// Global offset counter (for debugging)
    total_messages: AtomicI64,
}

impl Storage {
    /// Create a new storage engine
    pub fn new(config: Arc<Config>) -> Self {
        info!("Initializing storage engine");
        Self {
            topics: DashMap::new(),
            config,
            total_messages: AtomicI64::new(0),
        }
    }

    /// Get or create a topic
    pub fn get_or_create_topic(&self, name: &str, num_partitions: i32) -> Arc<Topic> {
        if let Some(topic) = self.topics.get(name) {
            return topic.clone();
        }

        let topic = Arc::new(Topic::new(name.to_string(), num_partitions));
        self.topics.insert(name.to_string(), topic.clone());
        info!(topic = name, partitions = num_partitions, "Created topic");
        topic
    }

    /// Get a topic if it exists
    pub fn get_topic(&self, name: &str) -> Option<Arc<Topic>> {
        self.topics.get(name).map(|t| t.clone())
    }

    /// Create a topic with specific configuration
    pub fn create_topic(&self, name: &str, num_partitions: i32) -> Result<Arc<Topic>> {
        if self.topics.contains_key(name) {
            return Err(HeimqError::Protocol(format!(
                "Topic '{}' already exists",
                name
            )));
        }

        let topic = Arc::new(Topic::new(name.to_string(), num_partitions));
        self.topics.insert(name.to_string(), topic.clone());
        info!(topic = name, partitions = num_partitions, "Created topic");
        Ok(topic)
    }

    /// Delete a topic
    pub fn delete_topic(&self, name: &str) -> Result<()> {
        if self.topics.remove(name).is_none() {
            return Err(HeimqError::TopicNotFound(name.to_string()));
        }
        info!(topic = name, "Deleted topic");
        Ok(())
    }

    /// List all topics
    #[allow(dead_code)]
    pub fn list_topics(&self) -> Vec<String> {
        self.topics.iter().map(|e| e.key().clone()).collect()
    }

    /// Get topic metadata for all topics
    pub fn get_all_topic_metadata(&self) -> Vec<(String, i32)> {
        self.topics
            .iter()
            .map(|e| (e.key().clone(), e.value().num_partitions()))
            .collect()
    }

    /// Append records to a partition
    pub fn append(
        &self,
        topic_name: &str,
        partition: i32,
        records: &[u8],
    ) -> Result<(i64, i64)> {
        let topic = if self.config.auto_create_topics {
            self.get_or_create_topic(topic_name, self.config.default_partitions)
        } else {
            self.get_topic(topic_name)
                .ok_or_else(|| HeimqError::TopicNotFound(topic_name.to_string()))?
        };

        let partition = topic.get_partition(partition)?;
        let (base_offset, count) = partition.append(records);
        self.total_messages.fetch_add(count, Ordering::Relaxed);

        debug!(
            topic = topic_name,
            partition = partition.id(),
            base_offset = base_offset,
            count = count,
            "Appended records"
        );

        Ok((base_offset, count))
    }

    /// Fetch records from a partition
    pub fn fetch(
        &self,
        topic_name: &str,
        partition: i32,
        offset: i64,
        max_bytes: i32,
    ) -> Result<(Vec<u8>, i64)> {
        let topic = self
            .get_topic(topic_name)
            .ok_or_else(|| HeimqError::TopicNotFound(topic_name.to_string()))?;

        let partition = topic.get_partition(partition)?;
        partition.fetch(offset, max_bytes as usize)
    }

    /// Get the high watermark for a partition
    pub fn high_watermark(&self, topic_name: &str, partition: i32) -> Result<i64> {
        let topic = self
            .get_topic(topic_name)
            .ok_or_else(|| HeimqError::TopicNotFound(topic_name.to_string()))?;

        let partition = topic.get_partition(partition)?;
        Ok(partition.high_watermark())
    }

    /// Get the log start offset for a partition
    pub fn log_start_offset(&self, topic_name: &str, partition: i32) -> Result<i64> {
        let topic = self
            .get_topic(topic_name)
            .ok_or_else(|| HeimqError::TopicNotFound(topic_name.to_string()))?;

        let partition = topic.get_partition(partition)?;
        Ok(partition.log_start_offset())
    }

    /// Get total message count
    #[allow(dead_code)]
    pub fn total_messages(&self) -> i64 {
        self.total_messages.load(Ordering::Relaxed)
    }

    /// Get the configuration
    pub fn config(&self) -> &Config {
        &self.config
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
            default_partitions: 1,
            auto_create_topics: true,
            broker_id: 0,
            cluster_id: "test".to_string(),
            metrics: false,
            metrics_port: 9093,
            create_topics: Vec::new(),
        })
    }

    #[test]
    fn test_create_topic() {
        let storage = Storage::new(test_config());
        let topic = storage.create_topic("test", 3).unwrap();
        assert_eq!(topic.name(), "test");
        assert_eq!(topic.num_partitions(), 3);
    }

    #[test]
    fn test_list_topics() {
        let storage = Storage::new(test_config());
        storage.create_topic("topic1", 1).unwrap();
        storage.create_topic("topic2", 2).unwrap();
        let topics = storage.list_topics();
        assert_eq!(topics.len(), 2);
    }

    #[test]
    fn test_get_all_metadata_and_delete_missing() {
        let storage = Storage::new(test_config());
        storage.create_topic("topic1", 1).unwrap();
        storage.create_topic("topic2", 2).unwrap();
        let metadata = storage.get_all_topic_metadata();
        assert_eq!(metadata.len(), 2);

        let missing = storage.delete_topic("missing");
        assert!(missing.is_err());
    }

    #[test]
    fn test_append_fetch_total_messages() {
        let storage = Storage::new(test_config());
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
    fn test_auto_create_disabled() {
        let mut config = (*test_config()).clone();
        config.auto_create_topics = false;
        let storage = Storage::new(Arc::new(config));
        let result = storage.append("missing", 0, &[1, 2, 3]);
        assert!(result.is_err());
    }

    #[test]
    fn test_fetch_invalid_offset() {
        let storage = Storage::new(test_config());
        storage.create_topic("topic", 1).unwrap();
        let result = storage.fetch("topic", 0, -1, 1024);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_partition_errors() {
        let storage = Storage::new(test_config());
        storage.create_topic("topic", 1).unwrap();

        assert!(storage.append("topic", 2, &[1, 2, 3]).is_err());
        assert!(storage.fetch("topic", 2, 0, 1024).is_err());
        assert!(storage.high_watermark("topic", 2).is_err());
        assert!(storage.log_start_offset("topic", 2).is_err());
    }
}
