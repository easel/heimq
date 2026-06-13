//! Per-trait conformance suites.
//!
//! Each module exposes a `run_all` function (plus individual check functions)
//! that can be called from any test context.

pub mod cluster_view;
pub mod group_coordinator;
pub mod log_backend;
pub mod offset_store;
pub mod partition_log;
