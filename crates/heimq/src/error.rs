//! Error types for heimq.
//!
//! The concrete error type lives in heimq-broker; the Kafka wire error-code
//! mapping (`ErrorCode`) and `str_bytes` helper live in heimq-handlers. Both are
//! re-exported here so the binary keeps using `crate::error::…`.

pub use heimq_broker::error::{HeimqError, Result};
pub use heimq_handlers::error::{str_bytes, ErrorCode};
