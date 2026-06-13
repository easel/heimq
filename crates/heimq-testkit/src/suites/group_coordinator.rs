//! Conformance suite for [`GroupCoordinatorBackend`].
//!
//! Tests the contract specified by TRAIT-001 §GroupCoordinatorBackend.
//! Covers the join → sync → heartbeat → leave rebalance trace.

use heimq::consumer_group::{GroupCoordinatorBackend, JoinRequest, SyncRequest};
use heimq::storage::OffsetStore;
use std::sync::Arc;

fn make_join_req(group_id: &str, member_id: &str) -> JoinRequest {
    JoinRequest {
        group_id: group_id.to_string(),
        member_id: member_id.to_string(),
        client_id: "testkit".to_string(),
        client_host: "127.0.0.1".to_string(),
        session_timeout_ms: 30000,
        rebalance_timeout_ms: 10000,
        protocol_type: "consumer".to_string(),
        protocols: vec![("range".to_string(), b"meta".to_vec())],
    }
}

/// capabilities() returns a descriptor with a non-empty name.
pub fn check_capabilities(backend: &dyn GroupCoordinatorBackend) {
    let caps = backend.capabilities();
    assert!(!caps.name.is_empty(), "capabilities().name must be non-empty");
}

/// offset_store() returns an OffsetStore with a non-empty capabilities name.
pub fn check_offset_store_accessible(backend: &dyn GroupCoordinatorBackend) {
    let store: Arc<dyn OffsetStore> = backend.offset_store();
    let caps = store.capabilities();
    assert!(
        !caps.name.is_empty(),
        "offset_store().capabilities().name must be non-empty"
    );
}

/// join_group with empty member_id returns MEMBER_ID_REQUIRED (79) + new id.
pub fn check_join_mints_member_id(backend: &dyn GroupCoordinatorBackend) {
    let result = backend.join_group(make_join_req("suite-grp-mint", ""));
    assert_eq!(
        result.error_code, 79,
        "empty member_id must return MEMBER_ID_REQUIRED (79)"
    );
    assert!(!result.member_id.is_empty(), "minted member_id must be non-empty");
}

/// Full join → sync → heartbeat → leave rebalance trace.
pub fn check_join_sync_heartbeat_leave_trace(backend: &dyn GroupCoordinatorBackend) {
    // 1. Mint a member_id
    let minted = backend.join_group(make_join_req("suite-grp-trace", ""));
    assert_eq!(minted.error_code, 79);
    let member_id = minted.member_id.clone();

    // 2. Rejoin with the minted id
    let joined = backend.join_group(make_join_req("suite-grp-trace", &member_id));
    assert_eq!(joined.error_code, 0, "rejoin must succeed: error_code={}", joined.error_code);
    let generation_id = joined.generation_id;

    // 3. Sync (leader assigns empty assignment to self)
    let sync_req = SyncRequest {
        group_id: "suite-grp-trace".to_string(),
        generation_id,
        member_id: member_id.clone(),
        assignments: vec![(member_id.clone(), vec![])],
    };
    let synced = backend.sync_group(sync_req);
    assert_eq!(synced.error_code, 0, "sync must succeed: error_code={}", synced.error_code);

    // 4. Heartbeat
    let hb = backend.heartbeat("suite-grp-trace", generation_id, &member_id);
    assert_eq!(hb.error_code, 0, "heartbeat must succeed: error_code={}", hb.error_code);

    // 5. Leave
    let leave = backend.leave_group("suite-grp-trace", &[member_id]);
    assert_eq!(leave.error_code, 0, "leave must succeed: error_code={}", leave.error_code);
}

/// Run all group coordinator conformance checks.
pub fn run_all(backend: &dyn GroupCoordinatorBackend) {
    check_capabilities(backend);
    check_offset_store_accessible(backend);
    check_join_mints_member_id(backend);
    check_join_sync_heartbeat_leave_trace(backend);
}
