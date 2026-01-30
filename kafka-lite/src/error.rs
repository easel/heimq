//! Error types for kafka-lite

use kafka_protocol::protocol::StrBytes;
use thiserror::Error;

/// Main error type for kafka-lite
#[derive(Error, Debug)]
pub enum KafkaLiteError {
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

/// Result type alias for kafka-lite operations
pub type Result<T> = std::result::Result<T, KafkaLiteError>;

/// Convert kafka-lite errors to Kafka error codes
impl KafkaLiteError {
    pub fn to_error_code(&self) -> i16 {
        match self {
            KafkaLiteError::TopicNotFound(_) => 3,  // UNKNOWN_TOPIC_OR_PARTITION
            KafkaLiteError::PartitionNotFound { .. } => 3,
            KafkaLiteError::InvalidOffset(_) => 1,  // OFFSET_OUT_OF_RANGE
            KafkaLiteError::ConsumerGroup(_) => 16, // NOT_COORDINATOR
            KafkaLiteError::Protocol(_) => 35,      // UNSUPPORTED_VERSION
            _ => -1,                                // UNKNOWN_SERVER_ERROR
        }
    }
}

/// Helper to convert String to StrBytes
pub fn str_bytes(s: String) -> StrBytes {
    StrBytes::from_string(s)
}
