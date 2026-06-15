//! heimq-broker: trait families (LogBackend/TopicLog/PartitionLog, OffsetStore,
//! GroupCoordinatorBackend, ClusterView) and supporting types.

pub mod consumer_group;
pub mod error;
pub mod produce;
pub mod storage;

pub use error::{HeimqError, Result};
