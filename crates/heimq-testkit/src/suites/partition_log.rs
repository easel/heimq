//! Conformance suite for [`PartitionLog`].
//!
//! Tests the contract specified by TRAIT-001 §PartitionLog.

use heimq::storage::{FetchWait, PartitionLog, RecordBatchView};
use heimq::test_support::encode_record_batch;
use heimq_protocol::records::Record;
use std::sync::Arc;

fn one_record_batch() -> Vec<u8> {
    let record = Record {
        transactional: false,
        control: false,
        partition_leader_epoch: 0,
        producer_id: -1,
        producer_epoch: -1,
        timestamp_type: heimq_protocol::records::TimestampType::Creation,
        timestamp: 0,
        sequence: 0,
        offset: 0,
        key: Some("k".into()),
        value: Some("v".into()),
        headers: Default::default(),
    };
    encode_record_batch(&[record]).to_vec()
}

/// New partition starts at offset 0 with no records.
pub fn check_initial_offsets(log: &Arc<dyn PartitionLog>) {
    assert_eq!(
        log.log_start_offset(),
        0,
        "new partition: log_start_offset must be 0"
    );
    assert_eq!(
        log.high_watermark(),
        0,
        "new partition: high_watermark must be 0"
    );
}

/// Appending a batch advances the high watermark.
pub fn check_append_advances_hwm(log: &Arc<dyn PartitionLog>) {
    let raw = one_record_batch();
    let view = RecordBatchView::from_bytes(&raw).expect("valid record batch");
    let (base_offset, _) = log.append(&view, Some(&raw)).expect("append must succeed");
    assert!(base_offset >= 0, "base_offset must be non-negative");
    assert!(
        log.high_watermark() > 0,
        "high_watermark must advance after append"
    );
}

/// Records appended can be read back starting at the base offset.
pub fn check_read_after_append(log: &Arc<dyn PartitionLog>) {
    let raw = one_record_batch();
    let view = RecordBatchView::from_bytes(&raw).expect("valid record batch");
    let (base_offset, _) = log.append(&view, Some(&raw)).expect("append must succeed");
    let (fetched, _hwm) = log
        .read(base_offset, 65536, FetchWait::Immediate)
        .expect("read must succeed");
    assert!(
        !fetched.is_empty(),
        "read after append must return non-empty bytes"
    );
}

/// Multiple appends are returned in order.
pub fn check_multiple_appends_in_order(log: &Arc<dyn PartitionLog>) {
    let raw = one_record_batch();
    let view = RecordBatchView::from_bytes(&raw).expect("valid record batch");
    let (offset1, _) = log.append(&view, Some(&raw)).expect("first append");
    let (offset2, _) = log.append(&view, Some(&raw)).expect("second append");
    assert!(offset2 > offset1, "second append offset must be > first");
}

/// truncate_before moves log_start_offset forward.
pub fn check_truncate_before(log: &Arc<dyn PartitionLog>) {
    let raw = one_record_batch();
    let view = RecordBatchView::from_bytes(&raw).expect("valid record batch");
    log.append(&view, Some(&raw)).expect("append must succeed");
    let hwm = log.high_watermark();
    if hwm > 0 {
        log.truncate_before(hwm)
            .expect("truncate_before must not fail");
        assert!(
            log.log_start_offset() >= hwm,
            "log_start_offset must advance after truncate_before(hwm)"
        );
    }
}

/// Run all partition log conformance checks.
/// `make_log` is called once per check to ensure a clean initial state.
pub fn run_all(make_log: &dyn Fn() -> Arc<dyn PartitionLog>) {
    check_initial_offsets(&make_log());
    check_append_advances_hwm(&make_log());
    check_read_after_append(&make_log());
    check_multiple_appends_in_order(&make_log());
    check_truncate_before(&make_log());
}
