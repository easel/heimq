//! heimq: A fast, lightweight, single-node Kafka-compatible API server
//!
//! Focused on transport, simplicity, and speed at the expense of durability.

#![allow(clippy::uninlined_format_args)]

pub mod config;
pub mod consumer_group;
pub mod error;
pub mod handler;
pub mod protocol;
pub mod server;
pub mod storage;

// Request-level handlers and their supporting state now live in heimq-handlers
// so embedders can reuse them. Re-exported at the original module paths to keep
// the rest of the binary's `crate::…` references unchanged.
pub use heimq_handlers::{config_store, producer_state, transaction_state};

// Test support module - available during tests and when test-support feature is enabled
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
