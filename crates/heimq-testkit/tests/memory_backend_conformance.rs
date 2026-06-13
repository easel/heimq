//! Conformance suite run against the in-memory backend (Layer 1).
//!
//! These tests prove that the in-memory implementations satisfy each per-trait
//! contract. Any failure here is a genuine engine gap (not a test artifact).

use heimq::consumer_group::ConsumerGroupManager;
use heimq::storage::{MemoryLog, MemoryPartitionLog, SingleNodeClusterView};
use heimq::test_support::test_config;
use heimq_testkit::suites;
use std::sync::Arc;

// ── LogBackend ────────────────────────────────────────────────────────────────

#[test]
fn memory_log_backend_capabilities_name() {
    let cfg = test_config(true);
    let backend = MemoryLog::new(cfg);
    suites::log_backend::check_capabilities_name(&backend);
}

#[test]
fn memory_log_backend_create_and_find_topic() {
    let cfg = test_config(true);
    let backend = MemoryLog::new(cfg);
    suites::log_backend::check_topic_unknown(&backend);
    suites::log_backend::check_create_topic(&backend);
    suites::log_backend::check_topic_after_create(&backend);
}

#[test]
fn memory_log_backend_list_topics() {
    let cfg = test_config(true);
    let backend = MemoryLog::new(cfg);
    suites::log_backend::check_list_topics(&backend);
}

#[test]
fn memory_log_backend_append_and_fetch() {
    let cfg = test_config(true);
    let backend = MemoryLog::new(cfg);
    suites::log_backend::check_append_and_fetch(&backend);
}

#[test]
fn memory_log_backend_high_watermark() {
    let cfg = test_config(true);
    let backend = MemoryLog::new(cfg);
    suites::log_backend::check_high_watermark(&backend);
}

#[test]
fn memory_log_backend_log_start_offset_initial() {
    let cfg = test_config(true);
    let backend = MemoryLog::new(cfg);
    suites::log_backend::check_log_start_offset_initial(&backend);
}

#[test]
fn memory_log_backend_get_or_create_idempotent() {
    let cfg = test_config(true);
    let backend = MemoryLog::new(cfg);
    suites::log_backend::check_get_or_create_idempotent(&backend);
}

#[test]
fn memory_log_backend_delete_topic() {
    let cfg = test_config(true);
    let backend = MemoryLog::new(cfg);
    suites::log_backend::check_delete_topic(&backend);
}

// ── PartitionLog ──────────────────────────────────────────────────────────────

#[test]
fn memory_partition_log_suite() {
    suites::partition_log::run_all(&|| Arc::new(MemoryPartitionLog::new(0)) as Arc<_>);
}

// ── OffsetStore ───────────────────────────────────────────────────────────────

#[test]
fn memory_offset_store_suite() {
    let cfg = test_config(true);
    let manager = ConsumerGroupManager::new(cfg);
    let store = manager.offset_store();
    suites::offset_store::run_all(store.as_ref());
}

// ── ClusterView ───────────────────────────────────────────────────────────────

#[test]
fn single_node_cluster_view_suite() {
    let cfg = test_config(true);
    let view = SingleNodeClusterView::new(&cfg);
    suites::cluster_view::run_all(&view);
}

// ── GroupCoordinatorBackend ───────────────────────────────────────────────────

#[test]
fn memory_group_coordinator_suite() {
    let cfg = test_config(true);
    let manager = ConsumerGroupManager::new(cfg);
    suites::group_coordinator::run_all(&manager);
}
