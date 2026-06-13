//! heimq-testkit: Per-trait conformance suites, fixture backends,
//! and the differential parity harness for heimq.
//!
//! # Usage
//!
//! Call suite functions with your backend implementation:
//!
//! ```rust,ignore
//! use heimq_testkit::suites;
//! let backend: Arc<dyn LogBackend> = /* your backend */;
//! suites::log_backend::run_all(&backend);
//! ```

pub mod fixtures;
pub mod suites;
