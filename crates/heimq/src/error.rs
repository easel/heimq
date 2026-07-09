//! Error types for heimq

use heimq_protocol::protocol::StrBytes;

// Re-export core error types from heimq-broker so all crates use one error type.
pub use heimq_broker::error::{HeimqError, Result};

/// Extension trait for converting heimq errors to Kafka wire error codes.
pub trait ErrorCode {
    fn to_error_code(&self) -> i16;
}

impl ErrorCode for HeimqError {
    fn to_error_code(&self) -> i16 {
        match self {
            HeimqError::TopicNotFound(_) => 3, // UNKNOWN_TOPIC_OR_PARTITION
            HeimqError::PartitionNotFound { .. } => 3,
            HeimqError::InvalidOffset(_) => 1, // OFFSET_OUT_OF_RANGE
            HeimqError::ConsumerGroup(_) => 16, // NOT_COORDINATOR
            HeimqError::Protocol(_) => 35,     // UNSUPPORTED_VERSION
            HeimqError::StorageFull(_) => 56,  // KAFKA_STORAGE_ERROR (retriable backpressure)
            _ => -1,                           // UNKNOWN_SERVER_ERROR
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
