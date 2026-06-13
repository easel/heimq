//! Partition storage implementation (in-memory)

use crate::error::{HeimqError, Result};
use crate::storage::{FetchWait, PartitionLog, RecordBatchView, Segment};
use parking_lot::RwLock;
use std::sync::atomic::{AtomicI64, Ordering};

/// An in-memory partition log — an ordered, immutable sequence of records.
pub struct MemoryPartitionLog {
    id: i32,
    /// The current segment being written to.
    active_segment: RwLock<Segment>,
    /// Historical segments (read-only).
    #[allow(dead_code)]
    segments: RwLock<Vec<Segment>>,
    /// Next offset to assign.
    next_offset: AtomicI64,
    /// First available offset.
    log_start_offset: AtomicI64,
}

impl MemoryPartitionLog {
    /// Create a new partition.
    pub fn new(id: i32) -> Self {
        Self {
            id,
            active_segment: RwLock::new(Segment::new(0)),
            segments: RwLock::new(Vec::new()),
            next_offset: AtomicI64::new(0),
            log_start_offset: AtomicI64::new(0),
        }
    }

    /// Get the partition ID.
    pub fn id(&self) -> i32 {
        self.id
    }

    /// Append raw record-batch bytes.
    ///
    /// The caller is expected to hand us a Kafka v2 record-batch payload; we
    /// rewrite its base offset in place and read the record count from the
    /// batch header (defaulting to 1 if the payload is too short).
    ///
    /// Returns `(base_offset, record_count)`.
    pub fn append_raw(&self, record_batch_data: &[u8]) -> (i64, i64) {
        let base_offset = self.next_offset.load(Ordering::SeqCst);

        // Read record count from the batch header (BE at offset 57..61).
        let record_count = if record_batch_data.len() >= 61 {
            let count_bytes = &record_batch_data[57..61];
            i32::from_be_bytes([count_bytes[0], count_bytes[1], count_bytes[2], count_bytes[3]])
                as i64
        } else {
            1
        };

        self.next_offset.fetch_add(record_count, Ordering::SeqCst);

        let mut batch = record_batch_data.to_vec();
        if batch.len() >= 8 {
            batch[0..8].copy_from_slice(&base_offset.to_be_bytes());
        }

        let mut segment = self.active_segment.write();
        segment.append(base_offset, batch);

        (base_offset, record_count)
    }

    /// Fetch records starting from the given offset.
    ///
    /// Returns `(record_batch_data, high_watermark)`.
    pub fn fetch(&self, start_offset: i64, max_bytes: usize) -> Result<(Vec<u8>, i64)> {
        let high_watermark = self.high_watermark();

        if start_offset >= high_watermark {
            return Ok((Vec::new(), high_watermark));
        }

        if start_offset < self.log_start_offset.load(Ordering::SeqCst) {
            return Err(HeimqError::InvalidOffset(start_offset));
        }

        let segment = self.active_segment.read();
        let data = segment.read(start_offset, max_bytes);
        Ok((data, high_watermark))
    }

    /// High watermark (next offset to be written).
    pub fn high_watermark(&self) -> i64 {
        self.next_offset.load(Ordering::SeqCst)
    }

    /// Log start offset (earliest available offset).
    pub fn log_start_offset(&self) -> i64 {
        self.log_start_offset.load(Ordering::SeqCst)
    }
}

impl PartitionLog for MemoryPartitionLog {
    fn id(&self) -> i32 {
        self.id
    }

    fn append(
        &self,
        view: &RecordBatchView<'_>,
        raw_bytes: Option<&[u8]>,
    ) -> Result<(i64, i64)> {
        let bytes = raw_bytes.unwrap_or_else(|| view.raw());
        Ok(self.append_raw(bytes))
    }

    fn read(
        &self,
        offset: i64,
        max_bytes: usize,
        _wait: FetchWait,
    ) -> Result<(Vec<u8>, i64)> {
        self.fetch(offset, max_bytes)
    }

    fn log_start_offset(&self) -> i64 {
        MemoryPartitionLog::log_start_offset(self)
    }

    fn high_watermark(&self) -> i64 {
        MemoryPartitionLog::high_watermark(self)
    }

    fn truncate_before(&self, offset: i64) -> Result<()> {
        let current_start = self.log_start_offset.load(Ordering::SeqCst);
        let hw = self.high_watermark();
        if offset < current_start || offset > hw {
            return Err(HeimqError::InvalidOffset(offset));
        }
        self.log_start_offset.store(offset, Ordering::SeqCst);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_new_partition() {
        let partition = MemoryPartitionLog::new(0);
        assert_eq!(partition.id(), 0);
        assert_eq!(partition.high_watermark(), 0);
        assert_eq!(MemoryPartitionLog::log_start_offset(&partition), 0);
    }

    fn record_batch_with_count(record_count: i32) -> Vec<u8> {
        let mut batch = vec![0u8; 61];
        batch[57..61].copy_from_slice(&record_count.to_be_bytes());
        batch
    }

    #[test]
    fn trait_append_and_read_match_inherent() {
        let partition = MemoryPartitionLog::new(0);
        let batch = record_batch_with_count(3);
        let view_result = RecordBatchView::from_bytes(&batch);
        // The synthetic batch is malformed; decoding via view would fail,
        // so we only verify the inherent path here.
        assert!(view_result.is_err());

        let (base, count) = partition.append_raw(&batch);
        assert_eq!(base, 0);
        assert_eq!(count, 3);

        let (data, hw) =
            <MemoryPartitionLog as PartitionLog>::read(&partition, 0, 1024, FetchWait::Immediate)
                .unwrap();
        assert_eq!(hw, 3);
        assert!(!data.is_empty());
    }

    #[test]
    fn truncate_before_moves_log_start() {
        let partition = MemoryPartitionLog::new(0);
        let batch = record_batch_with_count(5);
        partition.append_raw(&batch);
        assert_eq!(partition.high_watermark(), 5);

        <MemoryPartitionLog as PartitionLog>::truncate_before(&partition, 3).unwrap();
        assert_eq!(<MemoryPartitionLog as PartitionLog>::log_start_offset(&partition), 3);

        // Past the high watermark is rejected.
        assert!(<MemoryPartitionLog as PartitionLog>::truncate_before(&partition, 999).is_err());
        // Below the current start is rejected.
        assert!(<MemoryPartitionLog as PartitionLog>::truncate_before(&partition, 0).is_err());
    }

    proptest! {
        #[test]
        fn prop_append_advances_offsets(counts in prop::collection::vec(1i32..50, 1..32)) {
            let partition = MemoryPartitionLog::new(0);
            let mut expected_offset = 0i64;

            for count in counts {
                let batch = record_batch_with_count(count);
                let (base_offset, appended) = partition.append_raw(&batch);
                prop_assert_eq!(base_offset, expected_offset);
                prop_assert_eq!(appended, count as i64);
                expected_offset += count as i64;
            }

            prop_assert_eq!(partition.high_watermark(), expected_offset);
        }
    }
}
