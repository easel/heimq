//! Object-store-shape fixture backend: simulates manifest-ordered append pattern.
//!
//! `append()` records a manifest entry (offset + batch length) BEFORE storing
//! the batch in-memory. A real object-store backend would upload the batch as
//! a blob and record a manifest entry pointing to it. Here the manifest is an
//! in-memory Vec; storage is the standard MemoryPartitionLog.

use heimq::error::Result;
use heimq::storage::{FetchWait, MemoryPartitionLog, PartitionLog, RecordBatchView};
use std::sync::{Arc, Mutex};

/// One entry in the object-store manifest.
#[derive(Debug, Clone)]
pub struct ManifestEntry {
    pub base_offset: i64,
    pub batch_len: usize,
}

/// Object-store-shape partition log.
pub struct ObjectStoreShapePartitionLog {
    inner: Arc<MemoryPartitionLog>,
    manifest: Arc<Mutex<Vec<ManifestEntry>>>,
}

impl ObjectStoreShapePartitionLog {
    pub fn new(partition_id: i32) -> Self {
        Self {
            inner: Arc::new(MemoryPartitionLog::new(partition_id)),
            manifest: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Returns a reference to the manifest for inspection in tests.
    pub fn manifest(&self) -> Arc<Mutex<Vec<ManifestEntry>>> {
        self.manifest.clone()
    }
}

impl PartitionLog for ObjectStoreShapePartitionLog {
    fn id(&self) -> i32 {
        self.inner.id()
    }

    fn append(&self, view: &RecordBatchView<'_>, raw_bytes: Option<&[u8]>) -> Result<(i64, i64)> {
        // Record manifest entry BEFORE storing (upload-before-manifest pattern)
        let batch_len = raw_bytes.map(|b| b.len()).unwrap_or(0);
        let (base_offset, hwm) = self.inner.append(view, raw_bytes)?;
        self.manifest.lock().unwrap().push(ManifestEntry { base_offset, batch_len });
        Ok((base_offset, hwm))
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
