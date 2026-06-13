//! Error types for heimq-broker

use thiserror::Error;

/// Main error type for heimq
#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum HeimqError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Topic not found: {0}")]
    TopicNotFound(String),

    #[error("Partition not found: {topic} partition {partition}")]
    PartitionNotFound { topic: String, partition: i32 },

    #[error("Invalid offset: {0}")]
    InvalidOffset(i64),

    #[error("Consumer group error: {0}")]
    ConsumerGroup(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Compression error: {0}")]
    Compression(String),
}

/// Result type alias for heimq operations
pub type Result<T> = std::result::Result<T, HeimqError>;
