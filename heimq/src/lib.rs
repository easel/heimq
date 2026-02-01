//! heimq: A fast, lightweight, single-node Kafka-compatible API server
//!
//! Focused on transport, simplicity, and speed at the expense of durability.

pub mod config;
pub mod consumer_group;
pub mod error;
pub mod handler;
pub mod protocol;
pub mod server;
pub mod storage;

// Test support module - available during tests and when test-support feature is enabled
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
