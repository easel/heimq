//! Log segment implementation

use bytes::Bytes;
use std::collections::BTreeMap;

/// A segment is a portion of the partition log
///
/// For simplicity and speed, we store raw record batches in memory
/// with an index for offset-based lookups.
pub struct Segment {
    /// Base offset of this segment
    base_offset: i64,
    /// Record batches stored by their base offset
    batches: BTreeMap<i64, Vec<u8>>,
    /// Total size in bytes
    size: usize,
}

impl Segment {
    /// Create a new segment starting at the given offset
    pub fn new(base_offset: i64) -> Self {
        Self {
            base_offset,
            batches: BTreeMap::new(),
            size: 0,
        }
    }

    /// Get the base offset of this segment
    pub fn base_offset(&self) -> i64 {
        self.base_offset
    }

    /// Get the size of this segment in bytes
    pub fn size(&self) -> usize {
        self.size
    }

    /// Append a record batch to this segment
    pub fn append(&mut self, offset: i64, data: Vec<u8>) {
        self.size += data.len();
        self.batches.insert(offset, data);
    }

    /// Read record batches starting from the given offset
    pub fn read(&self, start_offset: i64, max_bytes: usize) -> Vec<u8> {
        let mut result = Vec::new();
        let mut bytes_read = 0;

        for (offset, batch) in self.batches.range(start_offset..) {
            if bytes_read + batch.len() > max_bytes && !result.is_empty() {
                break;
            }
            result.extend_from_slice(batch);
            bytes_read += batch.len();
        }

        result
    }

    /// Check if this segment contains the given offset
    pub fn contains(&self, offset: i64) -> bool {
        if let Some((&last_offset, _)) = self.batches.last_key_value() {
            offset >= self.base_offset && offset <= last_offset
        } else {
            false
        }
    }

    /// Get the last offset in this segment
    pub fn last_offset(&self) -> Option<i64> {
        self.batches.last_key_value().map(|(&k, _)| k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_segment() {
        let segment = Segment::new(0);
        assert_eq!(segment.base_offset(), 0);
        assert_eq!(segment.size(), 0);
    }

    #[test]
    fn test_append_and_read() {
        let mut segment = Segment::new(0);
        segment.append(0, vec![1, 2, 3, 4]);
        segment.append(1, vec![5, 6, 7, 8]);

        let data = segment.read(0, 1000);
        assert_eq!(data, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn test_read_from_offset() {
        let mut segment = Segment::new(0);
        segment.append(0, vec![1, 2, 3, 4]);
        segment.append(1, vec![5, 6, 7, 8]);

        let data = segment.read(1, 1000);
        assert_eq!(data, vec![5, 6, 7, 8]);
    }
}
