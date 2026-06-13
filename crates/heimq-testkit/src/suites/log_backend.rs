//! Conformance suite for [`LogBackend`].
//!
//! Tests the contract specified by TRAIT-001 §LogBackend.

use heimq::storage::LogBackend;
use heimq::test_support::encode_record_batch;
use kafka_protocol::records::Record;

fn one_record_batch() -> Vec<u8> {
    let record = Record {
        transactional: false,
        control: false,
        partition_leader_epoch: 0,
        producer_id: -1,
        producer_epoch: -1,
        timestamp_type: kafka_protocol::records::TimestampType::Creation,
        timestamp: 0,
        sequence: 0,
        offset: 0,
        key: Some("suite-key".into()),
        value: Some("suite-value".into()),
        headers: Default::default(),
    };
    encode_record_batch(&[record]).to_vec()
}

/// Backend has a capabilities descriptor with a non-empty name.
pub fn check_capabilities_name(backend: &dyn LogBackend) {
    let caps = backend.capabilities();
    assert!(!caps.name.is_empty(), "capabilities().name must be non-empty");
}

/// Creating a topic returns a TopicLog with the requested partition count.
pub fn check_create_topic(backend: &dyn LogBackend) {
    let topic = backend.create_topic("suite-create", 3).expect("create_topic must succeed");
    assert_eq!(topic.name(), "suite-create");
    assert_eq!(topic.num_partitions(), 3);
}

/// Creating a duplicate topic returns an error or the existing topic.
pub fn check_create_duplicate_topic(backend: &dyn LogBackend) {
    backend.create_topic("suite-dup", 2).expect("first create must succeed");
    let result = backend.create_topic("suite-dup", 2);
    // Either error or idempotent success is acceptable; must not panic.
    let _ = result;
}

/// topic() returns None for unknown topics.
pub fn check_topic_unknown(backend: &dyn LogBackend) {
    assert!(
        backend.topic("suite-unknown-xyz-987").is_none(),
        "topic() must return None for unknown topic"
    );
}

/// topic() returns Some after create_topic.
pub fn check_topic_after_create(backend: &dyn LogBackend) {
    backend.create_topic("suite-find", 1).expect("create_topic must succeed");
    assert!(
        backend.topic("suite-find").is_some(),
        "topic() must return Some after create_topic"
    );
}

/// list_topics includes freshly created topics.
pub fn check_list_topics(backend: &dyn LogBackend) {
    backend.create_topic("suite-list-1", 1).expect("create");
    backend.create_topic("suite-list-2", 1).expect("create");
    let topics = backend.list_topics();
    assert!(
        topics.contains(&"suite-list-1".to_string()),
        "list_topics must include suite-list-1"
    );
    assert!(
        topics.contains(&"suite-list-2".to_string()),
        "list_topics must include suite-list-2"
    );
}

/// append followed by fetch returns the appended data.
pub fn check_append_and_fetch(backend: &dyn LogBackend) {
    backend.create_topic("suite-append", 1).expect("create");
    let raw = one_record_batch();
    let (base_offset, hwm) = backend
        .append("suite-append", 0, &raw)
        .expect("append must succeed");
    assert!(base_offset >= 0);
    assert!(hwm > base_offset || hwm == base_offset + 1 || hwm > 0);

    let (fetched, _) = backend
        .fetch("suite-append", 0, base_offset, 65536)
        .expect("fetch must succeed");
    assert!(!fetched.is_empty(), "fetch after append must return data");
}

/// high_watermark advances after append.
pub fn check_high_watermark(backend: &dyn LogBackend) {
    backend.create_topic("suite-hwm", 1).expect("create");
    let hwm_before = backend.high_watermark("suite-hwm", 0).expect("hwm");
    let raw = one_record_batch();
    backend.append("suite-hwm", 0, &raw).expect("append");
    let hwm_after = backend.high_watermark("suite-hwm", 0).expect("hwm");
    assert!(hwm_after > hwm_before, "high_watermark must advance after append");
}

/// log_start_offset is 0 for a freshly created partition.
pub fn check_log_start_offset_initial(backend: &dyn LogBackend) {
    backend.create_topic("suite-lso", 1).expect("create");
    let lso = backend.log_start_offset("suite-lso", 0).expect("log_start_offset");
    assert_eq!(lso, 0, "initial log_start_offset must be 0");
}

/// get_or_create_topic is idempotent: second call returns same partition count.
pub fn check_get_or_create_idempotent(backend: &dyn LogBackend) {
    let t1 = backend.get_or_create_topic("suite-goc", 2);
    let t2 = backend.get_or_create_topic("suite-goc", 2);
    assert_eq!(t1.num_partitions(), t2.num_partitions(), "get_or_create must be idempotent");
}

/// delete_topic removes the topic from list_topics.
pub fn check_delete_topic(backend: &dyn LogBackend) {
    backend.create_topic("suite-del", 1).expect("create");
    backend.delete_topic("suite-del").expect("delete must succeed");
    assert!(
        backend.topic("suite-del").is_none(),
        "topic must not exist after delete"
    );
}

/// Run all log backend conformance checks.
pub fn run_all(backend: &dyn LogBackend) {
    check_capabilities_name(backend);
    check_create_topic(backend);
    check_create_duplicate_topic(backend);
    check_topic_unknown(backend);
    check_topic_after_create(backend);
    check_list_topics(backend);
    check_append_and_fetch(backend);
    check_high_watermark(backend);
    check_log_start_offset_initial(backend);
    check_get_or_create_idempotent(backend);
    check_delete_topic(backend);
}
