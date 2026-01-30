//! Topic management

use crate::error::{KafkaLiteError, Result};
use crate::storage::Partition;
use std::sync::Arc;

/// A Kafka topic containing multiple partitions
pub struct Topic {
    name: String,
    partitions: Vec<Arc<Partition>>,
}

impl Topic {
    /// Create a new topic with the specified number of partitions
    pub fn new(name: String, num_partitions: i32) -> Self {
        let partitions = (0..num_partitions)
            .map(|i| Arc::new(Partition::new(i)))
            .collect();

        Self { name, partitions }
    }

    /// Get the topic name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the number of partitions
    pub fn num_partitions(&self) -> i32 {
        self.partitions.len() as i32
    }

    /// Get a specific partition
    pub fn get_partition(&self, partition: i32) -> Result<&Arc<Partition>> {
        self.partitions.get(partition as usize).ok_or_else(|| {
            KafkaLiteError::PartitionNotFound {
                topic: self.name.clone(),
                partition,
            }
        })
    }

    /// Get all partitions
    pub fn partitions(&self) -> &[Arc<Partition>] {
        &self.partitions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_topic() {
        let topic = Topic::new("test".to_string(), 3);
        assert_eq!(topic.name(), "test");
        assert_eq!(topic.num_partitions(), 3);
    }

    #[test]
    fn test_get_partition() {
        let topic = Topic::new("test".to_string(), 3);
        assert!(topic.get_partition(0).is_ok());
        assert!(topic.get_partition(2).is_ok());
        assert!(topic.get_partition(3).is_err());
    }
}
