//! In-memory storage engine for kafka-lite
//!
//! Designed for speed over durability. Uses a simple segment-based log
//! structure with optional persistence.

mod partition;
mod segment;
mod topic;

pub use partition::Partition;
pub use segment::Segment;
pub use topic::Topic;

use crate::config::Config;
use crate::error::{KafkaLiteError, Result};
use bytes::Bytes;
use dashmap::DashMap;
use kafka_protocol::records::RecordBatchDecoder;
use parking_lot::RwLock;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use tracing::{debug, info};

/// Record stored in a partition
#[derive(Debug, Clone)]
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
            return Err(KafkaLiteError::Protocol(format!(
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
            return Err(KafkaLiteError::TopicNotFound(name.to_string()));
        }
        info!(topic = name, "Deleted topic");
        Ok(())
    }

    /// List all topics
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
                .ok_or_else(|| KafkaLiteError::TopicNotFound(topic_name.to_string()))?
        };

        let partition = topic.get_partition(partition)?;
        let (base_offset, count) = partition.append(records)?;
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
            .ok_or_else(|| KafkaLiteError::TopicNotFound(topic_name.to_string()))?;

        let partition = topic.get_partition(partition)?;
        partition.fetch(offset, max_bytes as usize)
    }

    /// Get the high watermark for a partition
    pub fn high_watermark(&self, topic_name: &str, partition: i32) -> Result<i64> {
        let topic = self
            .get_topic(topic_name)
            .ok_or_else(|| KafkaLiteError::TopicNotFound(topic_name.to_string()))?;

        let partition = topic.get_partition(partition)?;
        Ok(partition.high_watermark())
    }

    /// Get the log start offset for a partition
    pub fn log_start_offset(&self, topic_name: &str, partition: i32) -> Result<i64> {
        let topic = self
            .get_topic(topic_name)
            .ok_or_else(|| KafkaLiteError::TopicNotFound(topic_name.to_string()))?;

        let partition = topic.get_partition(partition)?;
        Ok(partition.log_start_offset())
    }

    /// Get total message count
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
            data_dir: std::path::PathBuf::from("/tmp/kafka-lite-test"),
            memory_only: true,
            segment_size: 1024 * 1024,
            retention_ms: 60000,
            default_partitions: 1,
            auto_create_topics: true,
            broker_id: 0,
            cluster_id: "test".to_string(),
            metrics: false,
            metrics_port: 9093,
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
}
