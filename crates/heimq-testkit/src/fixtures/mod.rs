//! Consumer-shaped fixture backends for conformance testing.
//!
//! These are in-repo, non-product implementations that exercise different
//! backend shapes against the per-trait suites. IP-001 Slice 3.

pub mod object_store_shape;
pub mod queue_sink_shape;
pub mod wal_shape;
