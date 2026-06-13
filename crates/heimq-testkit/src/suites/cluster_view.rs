//! Conformance suite for [`ClusterView`].
//!
//! Tests the contract specified by TRAIT-001 §ClusterView.

use heimq::storage::ClusterView;

/// self_broker returns a broker with a non-empty host.
pub fn check_self_broker(view: &dyn ClusterView) {
    let broker = view.self_broker();
    assert!(!broker.host.is_empty(), "self_broker().host must be non-empty");
    assert!(broker.port > 0, "self_broker().port must be > 0");
}

/// brokers() includes self_broker.
pub fn check_brokers_includes_self(view: &dyn ClusterView) {
    let self_id = view.self_broker().node_id;
    let brokers = view.brokers();
    assert!(
        !brokers.is_empty(),
        "brokers() must return at least one entry"
    );
    assert!(
        brokers.iter().any(|b| b.node_id == self_id),
        "brokers() must include self_broker"
    );
}

/// cluster_id() returns a non-empty string.
pub fn check_cluster_id(view: &dyn ClusterView) {
    let id = view.cluster_id();
    assert!(!id.is_empty(), "cluster_id() must be non-empty");
}

/// partition_leader returns Ok for any (topic, partition) on a single-node cluster.
pub fn check_partition_leader_single_node(view: &dyn ClusterView) {
    let result = view.partition_leader("any-topic", 0);
    assert!(
        result.is_ok(),
        "single-node ClusterView must return Ok for partition_leader"
    );
}

/// find_coordinator returns Ok on a single-node cluster.
pub fn check_find_coordinator_single_node(view: &dyn ClusterView) {
    let result = view.find_coordinator("any-group");
    assert!(
        result.is_ok(),
        "single-node ClusterView must return Ok for find_coordinator"
    );
}

/// Run all cluster view conformance checks.
pub fn run_all(view: &dyn ClusterView) {
    check_self_broker(view);
    check_brokers_includes_self(view);
    check_cluster_id(view);
    check_partition_leader_single_node(view);
    check_find_coordinator_single_node(view);
}
