//! Single-node ClusterView implementation for heimq.
use crate::config::Config;
use heimq_broker::storage::{BrokerInfo, ClusterView, ClusterViewError};
use std::sync::Arc;

/// Single-node reference implementation of ClusterView.
/// Returns `self_broker()` for all queries — correct for a single-node broker.
pub struct SingleNodeClusterView {
    broker: BrokerInfo,
    cluster_id: String,
}

impl SingleNodeClusterView {
    pub fn new(config: &Config) -> Self {
        let host = if let Some(advertised) = &config.advertised_host {
            advertised.clone()
        } else if config.host == "0.0.0.0" {
            "127.0.0.1".to_string()
        } else {
            config.host.clone()
        };
        Self {
            broker: BrokerInfo {
                node_id: config.broker_id,
                host,
                port: config.port as u16,
            },
            cluster_id: config.cluster_id.clone(),
        }
    }

    pub fn arc_from_config(config: &Config) -> Arc<dyn ClusterView> {
        Arc::new(Self::new(config))
    }
}

impl ClusterView for SingleNodeClusterView {
    fn self_broker(&self) -> BrokerInfo {
        self.broker.clone()
    }
    fn brokers(&self) -> Vec<BrokerInfo> {
        vec![self.broker.clone()]
    }
    fn cluster_id(&self) -> String {
        self.cluster_id.clone()
    }
    fn partition_leader(
        &self,
        _topic: &str,
        _partition: i32,
    ) -> Result<BrokerInfo, ClusterViewError> {
        Ok(self.broker.clone())
    }
    fn find_coordinator(&self, _group_id: &str) -> Result<BrokerInfo, ClusterViewError> {
        Ok(self.broker.clone())
    }
}
