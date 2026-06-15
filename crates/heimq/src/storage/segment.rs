//! Log segment implementation

use std::collections::BTreeMap;

/// Read the `max_timestamp` field from a Kafka v2 record batch header
/// (constant offset 35..43), if the slice is long enough.
fn batch_max_timestamp(batch: &[u8]) -> Option<i64> {
    if batch.len() < 43 {
        return None;
    }
    batch[35..43].try_into().ok().map(i64::from_be_bytes)
}

/// A segment is a portion of the partition log
///
/// For simplicity and speed, we store raw record batches in memory
/// with an index for offset-based lookups.
pub struct Segment {
    /// Base offset of this segment
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub fn base_offset(&self) -> i64 {
        self.base_offset
    }

    /// Get the size of this segment in bytes
    #[allow(dead_code)]
    pub fn size(&self) -> usize {
        self.size
    }

    /// Append a record batch to this segment
    pub fn append(&mut self, offset: i64, data: Vec<u8>) {
        self.size += data.len();
        self.batches.insert(offset, data);
    }

    /// Lowest batch base-offset still present, if any.
    pub fn first_offset(&self) -> Option<i64> {
        self.batches.keys().next().copied()
    }

    /// Drop the leading (oldest) batches whose `max_timestamp` is older than
    /// `cutoff_ms`, returning the number of bytes freed. Batches are stored in
    /// ascending offset order, which is also produce-time order, so dropping from
    /// the front retires exactly the expired prefix. A batch with no usable
    /// timestamp (header too short, or <= 0) is treated as not-yet-expired.
    pub fn reclaim_expired(&mut self, cutoff_ms: i64) -> usize {
        let expired: Vec<i64> = self
            .batches
            .iter()
            .take_while(|(_, b)| batch_max_timestamp(b).map_or(false, |ts| ts > 0 && ts < cutoff_ms))
            .map(|(&k, _)| k)
            .collect();
        let mut freed = 0;
        for k in expired {
            if let Some(b) = self.batches.remove(&k) {
                freed += b.len();
            }
        }
        self.size -= freed;
        freed
    }

    /// Read record batches starting from the given offset
    pub fn read(&self, start_offset: i64, max_bytes: usize) -> Vec<u8> {
        let mut result = Vec::new();
        let mut bytes_read = 0;

        // Step back one entry so we include the batch that contains start_offset
        // when start_offset falls mid-batch (base_offset < start_offset but the
        // batch spans records up to or past start_offset).  The consumer is
        // responsible for skipping individual records before start_offset.
        let first_key = self
            .batches
            .range(..=start_offset)
            .next_back()
            .map(|(&k, _)| k)
            .unwrap_or(start_offset);

        for (_offset, batch) in self.batches.range(first_key..) {
            if bytes_read + batch.len() > max_bytes && !result.is_empty() {
                break;
            }
            result.extend_from_slice(batch);
            bytes_read += batch.len();
        }

        result
    }

    /// Check if this segment contains the given offset
    #[allow(dead_code)]
    pub fn contains(&self, offset: i64) -> bool {
        if let Some((&last_offset, _)) = self.batches.last_key_value() {
            offset >= self.base_offset && offset <= last_offset
        } else {
            false
        }
    }

    /// Get the last offset in this segment
    #[allow(dead_code)]
    pub fn last_offset(&self) -> Option<i64> {
        self.batches.last_key_value().map(|(&k, _)| k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

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

    #[test]
    fn test_contains_and_last_offset() {
        let mut segment = Segment::new(5);
        assert!(!segment.contains(5));
        assert_eq!(segment.last_offset(), None);

        segment.append(5, vec![1]);
        segment.append(6, vec![2]);

        assert!(segment.contains(5));
        assert!(segment.contains(6));
        assert!(!segment.contains(4));
        assert_eq!(segment.last_offset(), Some(6));
        assert_eq!(segment.base_offset(), 5);
    }

    #[test]
    fn test_read_mid_batch_offset() {
        // Simulate a batched producer: one batch at base_offset=0 spanning
        // records 0-4 (count=5), next batch at base_offset=5.
        let mut segment = Segment::new(0);
        segment.append(0, vec![1, 2, 3]); // batch spanning offsets 0-4
        segment.append(5, vec![4, 5, 6]); // batch spanning offsets 5-9

        // A consumer that committed offset=3 restarts and fetches from 3.
        // It must receive the batch starting at base_offset=0 (which contains
        // offsets 3-4) plus the batch at 5 — not just the batch at 5.
        let data = segment.read(3, 1000);
        assert_eq!(
            data,
            vec![1, 2, 3, 4, 5, 6],
            "read(3) must include the containing batch at base_offset=0"
        );
    }

    proptest! {
        #[test]
        fn prop_segment_size_matches_appends(batches in prop::collection::vec(prop::collection::vec(any::<u8>(), 1..64), 1..32)) {
            let mut segment = Segment::new(0);
            let mut expected_size = 0usize;

            for (i, batch) in batches.into_iter().enumerate() {
                expected_size += batch.len();
                segment.append(i as i64, batch);
            }

            prop_assert_eq!(segment.size(), expected_size);
        }

        #[test]
        fn prop_read_matches_reference(
            batches in prop::collection::vec(prop::collection::vec(any::<u8>(), 1..64), 1..32),
            start_index in 0usize..32,
            max_bytes in 1usize..512
        ) {
            let mut segment = Segment::new(0);
            for (i, batch) in batches.iter().enumerate() {
                segment.append(i as i64, batch.clone());
            }

            let start_offset = (start_index.min(batches.len().saturating_sub(1))) as i64;
            let mut expected = Vec::new();
            let mut bytes_read = 0usize;

            for (i, batch) in batches.iter().enumerate() {
                if (i as i64) < start_offset {
                    continue;
                }
                if bytes_read + batch.len() > max_bytes && !expected.is_empty() {
                    break;
                }
                expected.extend_from_slice(batch);
                bytes_read += batch.len();
            }

            let actual = segment.read(start_offset, max_bytes);
            prop_assert_eq!(actual, expected);
        }
    }
}
