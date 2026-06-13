//! Conformance suite run against the consumer-shaped fixture backends.
//!
//! These tests prove that WAL-shape, queue-sink-shape, and object-store-shape
//! fixture backends satisfy the PartitionLog contract (TRAIT-001).
//! Shapes are "proven" when they pass the same suites as the memory backend.

use heimq_testkit::{
    fixtures::{
        object_store_shape::ObjectStoreShapePartitionLog,
        queue_sink_shape::QueueSinkPartitionLog,
        wal_shape::WalShapePartitionLog,
    },
    suites,
};
use std::sync::Arc;

// ── WAL-shape ─────────────────────────────────────────────────────────────────

#[test]
fn wal_shape_partition_log_suite() {
    suites::partition_log::run_all(&|| {
        Arc::new(WalShapePartitionLog::new(0)) as Arc<_>
    });
}

#[test]
fn wal_shape_records_in_wal_before_ack() {
    use heimq::storage::{PartitionLog, RecordBatchView};
    use heimq::test_support::encode_record_batch;
    use kafka_protocol::records::Record;

    let log = WalShapePartitionLog::new(0);
    let wal = log.wal();

    let raw = encode_record_batch(&[Record {
        transactional: false,
        control: false,
        partition_leader_epoch: 0,
        producer_id: -1,
        producer_epoch: -1,
        timestamp_type: kafka_protocol::records::TimestampType::Creation,
        timestamp: 0,
        sequence: 0,
        offset: 0,
        key: Some("k".into()),
        value: Some("v".into()),
        headers: Default::default(),
    }]);
    let raw_bytes = raw.to_vec();
    let view = RecordBatchView::from_bytes(&raw_bytes).unwrap();

    let log_arc: Arc<dyn PartitionLog> = Arc::new(log);
    log_arc.append(&view, Some(&raw_bytes)).unwrap();

    let wal_entries = wal.lock().unwrap();
    assert_eq!(wal_entries.len(), 1, "WAL must have one entry after append");
    assert!(!wal_entries[0].is_empty(), "WAL entry must not be empty");
}

// ── Queue-sink-shape ──────────────────────────────────────────────────────────

#[test]
fn queue_sink_partition_log_suite() {
    suites::partition_log::run_all(&|| {
        let (log, _rx) = QueueSinkPartitionLog::new(0);
        Arc::new(log) as Arc<_>
    });
}

#[test]
fn queue_sink_delivers_to_channel() {
    use heimq::storage::{PartitionLog, RecordBatchView};
    use heimq::test_support::encode_record_batch;
    use kafka_protocol::records::Record;

    let (log, rx) = QueueSinkPartitionLog::new(0);

    let raw = encode_record_batch(&[Record {
        transactional: false,
        control: false,
        partition_leader_epoch: 0,
        producer_id: -1,
        producer_epoch: -1,
        timestamp_type: kafka_protocol::records::TimestampType::Creation,
        timestamp: 0,
        sequence: 0,
        offset: 0,
        key: Some("k".into()),
        value: Some("v".into()),
        headers: Default::default(),
    }]);
    let raw_bytes = raw.to_vec();
    let view = RecordBatchView::from_bytes(&raw_bytes).unwrap();
    let log_arc: Arc<dyn PartitionLog> = Arc::new(log);
    log_arc.append(&view, Some(&raw_bytes)).unwrap();

    let received = rx.try_recv().expect("queue sink must deliver to channel");
    assert!(!received.is_empty(), "delivered batch must not be empty");
}

// ── Object-store-shape ────────────────────────────────────────────────────────

#[test]
fn object_store_shape_partition_log_suite() {
    suites::partition_log::run_all(&|| {
        Arc::new(ObjectStoreShapePartitionLog::new(0)) as Arc<_>
    });
}

#[test]
fn object_store_shape_records_in_manifest() {
    use heimq::storage::{PartitionLog, RecordBatchView};
    use heimq::test_support::encode_record_batch;
    use kafka_protocol::records::Record;

    let log = ObjectStoreShapePartitionLog::new(0);
    let manifest = log.manifest();

    let raw = encode_record_batch(&[Record {
        transactional: false,
        control: false,
        partition_leader_epoch: 0,
        producer_id: -1,
        producer_epoch: -1,
        timestamp_type: kafka_protocol::records::TimestampType::Creation,
        timestamp: 0,
        sequence: 0,
        offset: 0,
        key: Some("k".into()),
        value: Some("v".into()),
        headers: Default::default(),
    }]);
    let raw_bytes = raw.to_vec();
    let view = RecordBatchView::from_bytes(&raw_bytes).unwrap();
    let log_arc: Arc<dyn PartitionLog> = Arc::new(log);
    log_arc.append(&view, Some(&raw_bytes)).unwrap();

    let entries = manifest.lock().unwrap();
    assert_eq!(entries.len(), 1, "manifest must have one entry after append");
    assert_eq!(entries[0].base_offset, 0);
    assert!(entries[0].batch_len > 0, "manifest entry must record batch length");
}
