//! ClusterView trait and related types (TRAIT-001 §4).

#[derive(Debug, Clone)]
pub struct BrokerInfo {
    pub node_id: i32,
    pub host: String,
    pub port: u16,
}

#[derive(Debug)]
pub enum ClusterViewError {
    NotLeaderOrFollower { topic: String, partition: i32 },
    NotCoordinator { group_id: String },
}

pub trait ClusterView: Send + Sync {
    fn self_broker(&self) -> BrokerInfo;
    fn brokers(&self) -> Vec<BrokerInfo>;
    fn cluster_id(&self) -> String;
    /// Leader for (topic, partition); Err on stale/unknown.
    fn partition_leader(&self, topic: &str, partition: i32)
        -> Result<BrokerInfo, ClusterViewError>;
    /// Coordinator broker for group_id.
    fn find_coordinator(&self, group_id: &str) -> Result<BrokerInfo, ClusterViewError>;
}
