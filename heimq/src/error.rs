//! Error types for heimq

use kafka_protocol::protocol::StrBytes;
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

/// Convert heimq errors to Kafka error codes
impl HeimqError {
    pub fn to_error_code(&self) -> i16 {
        match self {
            HeimqError::TopicNotFound(_) => 3,  // UNKNOWN_TOPIC_OR_PARTITION
            HeimqError::PartitionNotFound { .. } => 3,
            HeimqError::InvalidOffset(_) => 1,  // OFFSET_OUT_OF_RANGE
            HeimqError::ConsumerGroup(_) => 16, // NOT_COORDINATOR
            HeimqError::Protocol(_) => 35,      // UNSUPPORTED_VERSION
            _ => -1,                                // UNKNOWN_SERVER_ERROR
        }
    }
}

/// Helper to convert String to StrBytes
#[allow(dead_code)]
pub fn str_bytes(s: String) -> StrBytes {
    StrBytes::from_string(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(HeimqError::TopicNotFound("t".into()).to_error_code(), 3);
        assert_eq!(
            HeimqError::PartitionNotFound {
                topic: "t".into(),
                partition: 1
            }
            .to_error_code(),
            3
        );
        assert_eq!(HeimqError::InvalidOffset(1).to_error_code(), 1);
        assert_eq!(HeimqError::ConsumerGroup("g".into()).to_error_code(), 16);
        assert_eq!(HeimqError::Protocol("p".into()).to_error_code(), 35);
        assert_eq!(HeimqError::Config("c".into()).to_error_code(), -1);
    }

    #[test]
    fn test_str_bytes_helper() {
        let value = str_bytes("hello".to_string());
        assert_eq!(value.as_str(), "hello");
    }
}
