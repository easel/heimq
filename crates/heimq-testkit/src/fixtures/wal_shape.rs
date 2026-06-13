//! WAL-shape fixture backend: simulates niflheim's write-ahead-log pattern.
//!
//! `append()` records the batch in a WAL (pre-ack work hook) BEFORE
//! delegating to in-memory storage. This proves the LogBackend abstraction
//! supports durability-before-ack shapes (TRAIT-001 §complete-work-before-ack).

use heimq::error::Result;
use heimq::storage::{FetchWait, MemoryPartitionLog, PartitionLog, RecordBatchView};
use std::sync::{Arc, Mutex};

/// Wal-shape partition log.
///
/// The `wal` field accumulates raw batches that were "written to WAL" before
/// the ack was sent. A real implementation would flush to disk; here we use a
/// Vec as a stand-in for the durability boundary.
pub struct WalShapePartitionLog {
    inner: Arc<MemoryPartitionLog>,
    wal: Arc<Mutex<Vec<Vec<u8>>>>,
}

impl WalShapePartitionLog {
    pub fn new(partition_id: i32) -> Self {
        Self {
            inner: Arc::new(MemoryPartitionLog::new(partition_id)),
            wal: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Returns a reference to the WAL for inspection in tests.
    pub fn wal(&self) -> Arc<Mutex<Vec<Vec<u8>>>> {
        self.wal.clone()
    }
}

impl PartitionLog for WalShapePartitionLog {
    fn id(&self) -> i32 {
        self.inner.id()
    }

    fn append(&self, view: &RecordBatchView<'_>, raw_bytes: Option<&[u8]>) -> Result<(i64, i64)> {
        // Pre-ack WAL write (durability hook fires before ack)
        if let Some(raw) = raw_bytes {
            self.wal.lock().unwrap().push(raw.to_vec());
        }
        self.inner.append(view, raw_bytes)
    }

    fn read(&self, offset: i64, max_bytes: usize, wait: FetchWait) -> Result<(Vec<u8>, i64)> {
        self.inner.read(offset, max_bytes, wait)
    }

    fn log_start_offset(&self) -> i64 {
        self.inner.log_start_offset()
    }

    fn high_watermark(&self) -> i64 {
        self.inner.high_watermark()
    }

    fn truncate_before(&self, offset: i64) -> Result<()> {
        self.inner.truncate_before(offset)
    }
}
