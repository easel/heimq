//! ClusterView trait and single-node reference implementation (TRAIT-001 §4).
use crate::config::Config;
use std::sync::Arc;

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
    fn partition_leader(&self, topic: &str, partition: i32) -> Result<BrokerInfo, ClusterViewError>;
    /// Coordinator broker for group_id.
    fn find_coordinator(&self, group_id: &str) -> Result<BrokerInfo, ClusterViewError>;
}

/// Single-node reference implementation of ClusterView.
/// Returns `self_broker()` for all queries — correct for a single-node broker.
pub struct SingleNodeClusterView {
    broker: BrokerInfo,
    cluster_id: String,
}

impl SingleNodeClusterView {
    pub fn new(config: &Config) -> Self {
        let host = if config.host == "0.0.0.0" {
            "127.0.0.1".to_string()
        } else {
            config.host.clone()
        };
        Self {
            broker: BrokerInfo { node_id: config.broker_id, host, port: config.port as u16 },
            cluster_id: config.cluster_id.clone(),
        }
    }

    pub fn arc_from_config(config: &Config) -> Arc<dyn ClusterView> {
        Arc::new(Self::new(config))
    }
}

impl ClusterView for SingleNodeClusterView {
    fn self_broker(&self) -> BrokerInfo { self.broker.clone() }
    fn brokers(&self) -> Vec<BrokerInfo> { vec![self.broker.clone()] }
    fn cluster_id(&self) -> String { self.cluster_id.clone() }
    fn partition_leader(&self, _topic: &str, _partition: i32) -> Result<BrokerInfo, ClusterViewError> {
        Ok(self.broker.clone())
    }
    fn find_coordinator(&self, _group_id: &str) -> Result<BrokerInfo, ClusterViewError> {
        Ok(self.broker.clone())
    }
}
