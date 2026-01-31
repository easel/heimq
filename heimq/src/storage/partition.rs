//! Partition storage implementation

use crate::error::{HeimqError, Result};
use crate::storage::Segment;
use parking_lot::RwLock;
use std::sync::atomic::{AtomicI64, Ordering};

/// A partition is an ordered, immutable sequence of records
pub struct Partition {
    id: i32,
    /// The current segment being written to
    active_segment: RwLock<Segment>,
    /// Historical segments (read-only)
    segments: RwLock<Vec<Segment>>,
    /// Next offset to assign
    next_offset: AtomicI64,
    /// First available offset
    log_start_offset: AtomicI64,
}

impl Partition {
    /// Create a new partition
    pub fn new(id: i32) -> Self {
        Self {
            id,
            active_segment: RwLock::new(Segment::new(0)),
            segments: RwLock::new(Vec::new()),
            next_offset: AtomicI64::new(0),
            log_start_offset: AtomicI64::new(0),
        }
    }

    /// Get the partition ID
    pub fn id(&self) -> i32 {
        self.id
    }

    /// Append a record batch to this partition
    ///
    /// Returns (base_offset, record_count)
    pub fn append(&self, record_batch_data: &[u8]) -> Result<(i64, i64)> {
        // Parse the record batch to count records and update offsets
        // For now, we store the raw bytes and track offsets

        // Get the base offset for this batch
        let base_offset = self.next_offset.load(Ordering::SeqCst);

        // Parse record batch header to get record count
        // Kafka record batch format:
        // - baseOffset (8 bytes)
        // - batchLength (4 bytes)
        // - partitionLeaderEpoch (4 bytes)
        // - magic (1 byte)
        // - crc (4 bytes)
        // - attributes (2 bytes)
        // - lastOffsetDelta (4 bytes)
        // ... more fields
        // - recordCount (4 bytes at offset 57)

        let record_count = if record_batch_data.len() >= 61 {
            // Read recordCount from the batch header (little endian at offset 57-60)
            let count_bytes = &record_batch_data[57..61];
            i32::from_be_bytes([count_bytes[0], count_bytes[1], count_bytes[2], count_bytes[3]]) as i64
        } else {
            // Fallback: assume 1 record
            1
        };

        // Update the next offset
        self.next_offset.fetch_add(record_count, Ordering::SeqCst);

        // Create a modified batch with correct base offset
        let mut batch = record_batch_data.to_vec();
        if batch.len() >= 8 {
            // Write the base offset into the batch
            batch[0..8].copy_from_slice(&base_offset.to_be_bytes());
        }

        // Append to active segment
        let mut segment = self.active_segment.write();
        segment.append(base_offset, batch);

        Ok((base_offset, record_count))
    }

    /// Fetch records starting from the given offset
    ///
    /// Returns (record_batch_data, high_watermark)
    pub fn fetch(&self, start_offset: i64, max_bytes: usize) -> Result<(Vec<u8>, i64)> {
        let high_watermark = self.high_watermark();

        if start_offset >= high_watermark {
            // No new records
            return Ok((Vec::new(), high_watermark));
        }

        if start_offset < self.log_start_offset.load(Ordering::SeqCst) {
            return Err(HeimqError::InvalidOffset(start_offset));
        }

        // Read from active segment
        let segment = self.active_segment.read();
        let data = segment.read(start_offset, max_bytes);

        Ok((data, high_watermark))
    }

    /// Get the high watermark (next offset to be written)
    pub fn high_watermark(&self) -> i64 {
        self.next_offset.load(Ordering::SeqCst)
    }

    /// Get the log start offset (earliest available offset)
    pub fn log_start_offset(&self) -> i64 {
        self.log_start_offset.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_partition() {
        let partition = Partition::new(0);
        assert_eq!(partition.id(), 0);
        assert_eq!(partition.high_watermark(), 0);
        assert_eq!(partition.log_start_offset(), 0);
    }
}
