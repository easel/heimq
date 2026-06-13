//! Conformance suite for [`OffsetStore`].
//!
//! Tests the contract specified by TRAIT-001 §OffsetStore.

use heimq::storage::OffsetStore;

/// Fetching an uncommitted offset returns None.
pub fn check_fetch_missing(store: &dyn OffsetStore) {
    let result = store.fetch("suite-group-miss", "t", 0);
    assert!(result.is_none(), "uncommitted offset must return None");
}

/// Commit followed by fetch returns the committed offset.
pub fn check_commit_and_fetch(store: &dyn OffsetStore) {
    store
        .commit("suite-grp-cf", "t", 0, 42, 0, None)
        .expect("commit must succeed");
    let fetched = store
        .fetch("suite-grp-cf", "t", 0)
        .expect("fetch must return Some after commit");
    assert_eq!(fetched.offset, 42, "fetched offset must match committed");
}

/// Committing a higher offset replaces the previous value.
pub fn check_commit_overwrites(store: &dyn OffsetStore) {
    store.commit("suite-grp-ow", "t", 0, 10, 0, None).expect("first commit");
    store.commit("suite-grp-ow", "t", 0, 20, 0, None).expect("second commit");
    let fetched = store.fetch("suite-grp-ow", "t", 0).expect("fetch");
    assert_eq!(fetched.offset, 20, "second commit must overwrite first");
}

/// fetch_all_for_group returns all committed offsets for a group.
pub fn check_fetch_all_for_group(store: &dyn OffsetStore) {
    store.commit("suite-grp-fa", "t", 0, 1, 0, None).expect("commit p0");
    store.commit("suite-grp-fa", "t", 1, 2, 0, None).expect("commit p1");
    let all = store.fetch_all_for_group("suite-grp-fa");
    assert!(!all.is_empty(), "fetch_all_for_group must return entries after commit");
    assert_eq!(
        all.get(&("t".to_string(), 0)).map(|c| c.offset),
        Some(1),
        "partition 0 offset must be 1"
    );
}

/// delete_group removes all offsets for a group.
pub fn check_delete_group(store: &dyn OffsetStore) {
    store.commit("suite-grp-del", "t", 0, 5, 0, None).expect("commit");
    store.delete_group("suite-grp-del");
    let fetched = store.fetch("suite-grp-del", "t", 0);
    assert!(fetched.is_none(), "fetch after delete_group must return None");
}

/// delete_offset removes a single committed offset without affecting others.
pub fn check_delete_offset(store: &dyn OffsetStore) {
    store.commit("suite-grp-do", "t", 0, 10, 0, None).expect("commit p0");
    store.commit("suite-grp-do", "t", 1, 20, 0, None).expect("commit p1");
    store.delete_offset("suite-grp-do", "t", 0);
    assert!(store.fetch("suite-grp-do", "t", 0).is_none(), "fetch after delete_offset must return None");
    assert_eq!(
        store.fetch("suite-grp-do", "t", 1).map(|c| c.offset),
        Some(20),
        "sibling partition must be unaffected by delete_offset"
    );
}

/// capabilities() returns a descriptor with a non-empty name.
pub fn check_capabilities(store: &dyn OffsetStore) {
    let caps = store.capabilities();
    assert!(!caps.name.is_empty(), "capabilities().name must be non-empty");
}

/// Run all offset store conformance checks.
pub fn run_all(store: &dyn OffsetStore) {
    check_capabilities(store);
    check_fetch_missing(store);
    check_commit_and_fetch(store);
    check_commit_overwrites(store);
    check_fetch_all_for_group(store);
    check_delete_group(store);
    check_delete_offset(store);
}
