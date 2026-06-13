//! heimq-broker: trait families (LogBackend/TopicLog/PartitionLog, OffsetStore,
//! GroupCoordinatorBackend, ClusterView) and supporting types.

pub mod error;
pub mod storage;
pub mod consumer_group;

pub use error::{HeimqError, Result};
