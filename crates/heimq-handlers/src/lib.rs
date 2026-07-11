//! heimq-handlers: request-level Kafka handlers extracted from the heimq binary
//! so embedders (niflheim) can delegate decode + dispatch + encode instead of
//! carrying their own in-tree Kafka codec and handlers.
//!
//! This crate sits above `heimq-broker` (which owns the version-neutral append
//! core and the backend trait families) and uses `heimq_protocol` wire types
//! directly on its public surface. Embedders adopt the same `heimq_protocol`
//! fork, so exposing those types here does not resolve a second, conflicting
//! copy of `kafka-protocol`.

// The handler and codec modules were extracted from the heimq binary, which
// allows this lint crate-wide; keep the same policy so the moved code's
// `format!("{}", x)` style stays consistent across the toolchains CI pins.
#![allow(clippy::uninlined_format_args)]

pub mod api_versions;
pub mod codec;
pub mod config_store;
pub mod error;
pub mod flexible;
pub mod init_producer_id;
pub mod metadata;
pub mod produce;
pub mod producer_state;
pub mod transaction_state;

/// Re-exports of the `heimq-broker` storage traits/types the handlers consume,
/// so the extracted handler modules keep their original `crate::storage::…`
/// import paths.
pub mod storage {
    pub use heimq_broker::storage::{ClusterView, LogBackend, RetentionPolicy};
    pub use heimq_broker::RequestContext;
}
