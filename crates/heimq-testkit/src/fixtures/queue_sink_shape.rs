//! Queue-sink-shape fixture backend: simulates pqueue's produce-to-queue pattern.
//!
//! `append()` enqueues the raw batch to an mpsc channel (the downstream
//! consumer would drain it) AND stores in-memory for read-back. This proves
//! the LogBackend abstraction supports queue-enqueue shapes (TRAIT-001).

use heimq::error::Result;
use heimq::storage::{FetchWait, MemoryPartitionLog, PartitionLog, RecordBatchView};
use std::sync::{mpsc, Arc};

/// Queue-sink-shape partition log.
///
/// Produce batches are sent to `sink` for downstream processing AND stored
/// in `inner` so `read()` can verify round-trip integrity.
pub struct QueueSinkPartitionLog {
    inner: Arc<MemoryPartitionLog>,
    sink: mpsc::SyncSender<Vec<u8>>,
}

impl QueueSinkPartitionLog {
    /// Returns a `(log, receiver)` pair.
    pub fn new(partition_id: i32) -> (Self, mpsc::Receiver<Vec<u8>>) {
        let (tx, rx) = mpsc::sync_channel(64);
        let log = Self {
            inner: Arc::new(MemoryPartitionLog::new(partition_id)),
            sink: tx,
        };
        (log, rx)
    }
}

impl PartitionLog for QueueSinkPartitionLog {
    fn id(&self) -> i32 {
        self.inner.id()
    }

    fn append(&self, view: &RecordBatchView<'_>, raw_bytes: Option<&[u8]>) -> Result<(i64, i64)> {
        // Enqueue to downstream sink (queue-enqueue pattern)
        if let Some(raw) = raw_bytes {
            let _ = self.sink.try_send(raw.to_vec());
        }
        // Also store in-memory for read-back conformance
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
