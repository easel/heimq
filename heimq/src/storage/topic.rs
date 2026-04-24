//! Topic management

use crate::error::{HeimqError, Result};
use crate::storage::{MemoryPartitionLog, PartitionLog, TopicConfig, TopicLog};
use std::sync::Arc;

/// A Kafka topic containing multiple partitions — in-memory implementation.
pub struct MemoryTopicLog {
    name: String,
    partitions: Vec<Arc<MemoryPartitionLog>>,
    config: TopicConfig,
}

impl MemoryTopicLog {
    /// Create a new topic with the specified number of partitions.
    pub fn new(name: String, num_partitions: i32) -> Self {
        let partitions = (0..num_partitions)
            .map(|i| Arc::new(MemoryPartitionLog::new(i)))
            .collect();

        Self {
            name,
            partitions,
            config: TopicConfig { num_partitions },
        }
    }

    /// Get a specific partition as the concrete in-memory type.
    pub(crate) fn get_memory_partition(
        &self,
        partition: i32,
    ) -> Result<&Arc<MemoryPartitionLog>> {
        self.partitions.get(partition as usize).ok_or_else(|| {
            HeimqError::PartitionNotFound {
                topic: self.name.clone(),
                partition,
            }
        })
    }

    /// Get all partitions (in-memory handles).
    #[allow(dead_code)]
    pub fn partitions(&self) -> &[Arc<MemoryPartitionLog>] {
        &self.partitions
    }
}

impl TopicLog for MemoryTopicLog {
    fn name(&self) -> &str {
        &self.name
    }

    fn num_partitions(&self) -> i32 {
        self.partitions.len() as i32
    }

    fn partition(&self, id: i32) -> Result<Arc<dyn PartitionLog>> {
        self.get_memory_partition(id)
            .map(|p| p.clone() as Arc<dyn PartitionLog>)
    }

    fn config(&self) -> &TopicConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_topic() {
        let topic = MemoryTopicLog::new("test".to_string(), 3);
        assert_eq!(topic.name(), "test");
        assert_eq!(topic.num_partitions(), 3);
        assert_eq!(topic.config().num_partitions, 3);
    }

    #[test]
    fn test_get_partition() {
        let topic = MemoryTopicLog::new("test".to_string(), 3);
        assert!(topic.partition(0).is_ok());
        assert!(topic.partition(2).is_ok());
        assert!(topic.partition(3).is_err());
    }

    #[test]
    fn test_partitions_slice() {
        let topic = MemoryTopicLog::new("test".to_string(), 2);
        assert_eq!(topic.partitions().len(), 2);
    }
}
