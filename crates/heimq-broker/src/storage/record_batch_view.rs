//! `RecordBatchView` — a structured view over a Kafka v2 record batch.
//!
//! Today the log trait accepts raw record-batch bytes. To let backends
//! implement retention-by-timestamp, key-based compaction, header-aware
//! filtering, etc., we need a structured handle that exposes parsed batch
//! metadata plus a borrowed iterator over records without losing access to
//! the original bytes for fast-path pass-through.
//!
//! Construction decodes the batch via `kafka_protocol::records` (same entry
//! point produce handlers will use) and caches the resulting `RecordSet`
//! alongside the original `&[u8]`.

#![allow(dead_code)]

use crate::error::{HeimqError, Result};
use crate::storage::CompressionCodec;
use bytes::Bytes;
use kafka_protocol::protocol::StrBytes;
use kafka_protocol::records::{
    Compression, Record, RecordBatchDecoder, RecordCompression, RecordSet,
};

/// Cheap, O(1) read of the fixed fields of a Kafka v2 RecordBatch header — the
/// producer/transaction metadata that lives at constant offsets before the
/// (variable-length, possibly compressed) records. Use this on the produce hot
/// path to avoid fully decoding every record just to inspect the header; reserve
/// [`RecordBatchView`] for paths that actually need the decoded records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordBatchHeader {
    pub producer_id: i64,
    pub producer_epoch: i16,
    pub base_sequence: i32,
    pub record_count: i32,
    pub is_transactional: bool,
    pub is_control: bool,
}

impl RecordBatchHeader {
    /// Read the fixed v2 header fields. Returns `None` if `raw` is too short to
    /// hold a v2 batch header (61 bytes through `record_count`).
    pub fn peek(raw: &[u8]) -> Option<Self> {
        // v2 layout: base_offset(0..8) batch_length(8..12) leader_epoch(12..16)
        // magic(16) crc(17..21) attributes(21..23) last_offset_delta(23..27)
        // base_ts(27..35) max_ts(35..43) producer_id(43..51) producer_epoch(51..53)
        // base_sequence(53..57) record_count(57..61) records...
        // Only valid for v2 batches (magic == 2); legacy v0/v1 message sets have a
        // different layout, so return None rather than reading garbage fields.
        if raw.len() < 61 || raw.get(16) != Some(&2) {
            return None;
        }
        let attributes = i16::from_be_bytes([raw[21], raw[22]]);
        Some(Self {
            producer_id: i64::from_be_bytes(raw[43..51].try_into().ok()?),
            producer_epoch: i16::from_be_bytes([raw[51], raw[52]]),
            base_sequence: i32::from_be_bytes(raw[53..57].try_into().ok()?),
            record_count: i32::from_be_bytes(raw[57..61].try_into().ok()?),
            is_transactional: attributes & 0x10 != 0,
            is_control: attributes & 0x20 != 0,
        })
    }
}

/// Stamp the assigned `base_offset` into a Kafka v2 record batch, in place.
///
/// `base_offset` lives at bytes `0..8` (big-endian), *before* the CRC at `17..21`,
/// so rewriting it does not invalidate the batch CRC (which covers `attributes`
/// onward). Storage backends that assign offsets out of band — e.g. a diskless
/// broker whose coordinator sequences offsets after the bytes are produced — call
/// this on the read path instead of poking the bytes themselves; heimq owns the v2
/// wire layout, so the knowledge lives here.
///
/// Returns `false` and does nothing if `batch` is not a well-formed v2 batch
/// header (shorter than the 61-byte fixed header, or `magic != 2`).
pub fn stamp_base_offset(batch: &mut [u8], base_offset: i64) -> bool {
    if batch.len() < 61 || batch.get(16) != Some(&2) {
        return false;
    }
    batch[0..8].copy_from_slice(&base_offset.to_be_bytes());
    true
}

/// Structured view of a single Kafka record batch.
///
/// Carries parsed batch-level metadata plus the decoded records and a
/// reference to the original serialized bytes. The original `&[u8]` is kept
/// so backends that don't care about parsed records can pass the batch
/// through to the wire without re-encoding.
pub struct RecordBatchView<'a> {
    raw: &'a [u8],
    producer_id: i64,
    producer_epoch: i16,
    base_offset: i64,
    base_timestamp: i64,
    max_timestamp: i64,
    is_transactional: bool,
    is_control: bool,
    compression: CompressionCodec,
    record_count: usize,
    records: Vec<Record>,
}

/// Borrowed view of a single record inside a `RecordBatchView`.
///
/// Offsets and timestamps are exposed as *deltas* relative to the batch's
/// `base_offset` / `base_timestamp` so backends working at the batch level
/// don't need to repeat the subtraction.
pub struct RecordView<'a> {
    pub offset_delta: i32,
    pub timestamp_delta: i64,
    pub key: Option<&'a Bytes>,
    pub value: Option<&'a Bytes>,
    record: &'a Record,
}

impl<'a> RecordView<'a> {
    /// Iterate over the record's headers as `(name, value)` pairs.
    pub fn headers(&self) -> impl Iterator<Item = (&'a StrBytes, Option<&'a Bytes>)> + 'a {
        self.record.headers.iter().map(|(k, v)| (k, v.as_ref()))
    }

    pub fn header_count(&self) -> usize {
        self.record.headers.len()
    }
}

impl<'a> RecordBatchView<'a> {
    /// Decode a record batch from the given raw bytes.
    ///
    /// `base_offset` is read from the batch header and treated as a placeholder:
    /// it reflects what the producer sent (typically `0`) and will be rewritten
    /// by the log when the batch is actually appended.
    pub fn from_bytes(raw: &'a [u8]) -> Result<Self> {
        let mut buf = Bytes::copy_from_slice(raw);
        let set = RecordBatchDecoder::decode(&mut buf)
            .map_err(|e| HeimqError::Protocol(format!("decode record batch: {e}")))?;
        Self::from_raw_and_set(raw, set)
    }

    /// Build a view from an already-decoded `RecordSet` together with the
    /// original bytes the set was decoded from.
    pub fn from_raw_and_set(raw: &'a [u8], set: RecordSet) -> Result<Self> {
        let compression = match set.compression {
            RecordCompression::RecordBatch(c) => compression_to_codec(c),
            RecordCompression::MessageSet => CompressionCodec::None,
        };

        let record_count = set.records.len();

        // Batch-level fields are identical across all records in the batch;
        // take them from the first record, or fall back to sentinels when
        // the batch is empty.
        let (producer_id, producer_epoch, is_transactional, is_control) =
            if let Some(first) = set.records.first() {
                (
                    first.producer_id,
                    first.producer_epoch,
                    first.transactional,
                    first.control,
                )
            } else {
                (-1, -1, false, false)
            };

        // base_offset / base_timestamp: minimum across decoded records. For
        // a well-formed batch the first record carries the minima; computing
        // the min defensively protects against out-of-order encodings.
        let base_offset = set.records.iter().map(|r| r.offset).min().unwrap_or(0);
        let base_timestamp = set.records.iter().map(|r| r.timestamp).min().unwrap_or(-1);
        let max_timestamp = set.records.iter().map(|r| r.timestamp).max().unwrap_or(-1);

        Ok(Self {
            raw,
            producer_id,
            producer_epoch,
            base_offset,
            base_timestamp,
            max_timestamp,
            is_transactional,
            is_control,
            compression,
            record_count,
            records: set.records,
        })
    }

    pub fn raw(&self) -> &'a [u8] {
        self.raw
    }

    pub fn producer_id(&self) -> i64 {
        self.producer_id
    }

    pub fn producer_epoch(&self) -> i16 {
        self.producer_epoch
    }

    pub fn base_offset(&self) -> i64 {
        self.base_offset
    }

    pub fn base_timestamp(&self) -> i64 {
        self.base_timestamp
    }

    pub fn max_timestamp(&self) -> i64 {
        self.max_timestamp
    }

    pub fn is_transactional(&self) -> bool {
        self.is_transactional
    }

    pub fn is_control(&self) -> bool {
        self.is_control
    }

    pub fn compression(&self) -> CompressionCodec {
        self.compression
    }

    pub fn record_count(&self) -> usize {
        self.record_count
    }

    /// The base sequence number of this batch (first record's sequence).
    /// Returns -1 for non-idempotent batches (producer_id == -1).
    pub fn base_sequence(&self) -> i32 {
        self.records.first().map(|r| r.sequence).unwrap_or(-1)
    }

    /// Iterate records as borrowed `RecordView`s.
    pub fn records(&self) -> impl Iterator<Item = RecordView<'_>> + '_ {
        let base_offset = self.base_offset;
        let base_timestamp = self.base_timestamp;
        self.records.iter().map(move |r| RecordView {
            offset_delta: (r.offset - base_offset) as i32,
            timestamp_delta: r.timestamp - base_timestamp,
            key: r.key.as_ref(),
            value: r.value.as_ref(),
            record: r,
        })
    }
}

fn compression_to_codec(c: Compression) -> CompressionCodec {
    match c {
        Compression::None => CompressionCodec::None,
        Compression::Gzip => CompressionCodec::Gzip,
        Compression::Snappy => CompressionCodec::Snappy,
        Compression::Lz4 => CompressionCodec::Lz4,
        Compression::Zstd => CompressionCodec::Zstd,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;
    use kafka_protocol::records::{RecordBatchEncoder, RecordEncodeOptions, TimestampType};

    fn make_record(offset: i64, timestamp: i64, key: &[u8], value: &[u8]) -> Record {
        let mut headers = kafka_protocol::indexmap::IndexMap::new();
        headers.insert(
            StrBytes::from_static_str("h1"),
            Some(Bytes::copy_from_slice(b"hv")),
        );
        Record {
            transactional: false,
            control: false,
            partition_leader_epoch: 0,
            producer_id: 42,
            producer_epoch: 7,
            timestamp_type: TimestampType::Creation,
            offset,
            sequence: offset as i32,
            timestamp,
            key: Some(Bytes::copy_from_slice(key)),
            value: Some(Bytes::copy_from_slice(value)),
            headers,
        }
    }

    fn encode(records: &[Record], compression: Compression) -> Vec<u8> {
        let mut buf = BytesMut::new();
        RecordBatchEncoder::encode(
            &mut buf,
            records,
            &RecordEncodeOptions {
                version: 2,
                compression,
            },
        )
        .expect("encode batch");
        buf.to_vec()
    }

    #[test]
    fn view_exposes_batch_metadata() {
        let records = vec![
            make_record(0, 1_000, b"k0", b"v0"),
            make_record(1, 1_050, b"k1", b"v1"),
            make_record(2, 1_100, b"k2", b"v2"),
        ];
        let raw = encode(&records, Compression::None);

        let view = RecordBatchView::from_bytes(&raw).expect("decode view");

        assert_eq!(view.record_count(), 3);
        assert_eq!(view.producer_id(), 42);
        assert_eq!(view.producer_epoch(), 7);
        assert_eq!(view.base_offset(), 0);
        assert_eq!(view.base_timestamp(), 1_000);
        assert_eq!(view.max_timestamp(), 1_100);
        assert!(!view.is_transactional());
        assert!(!view.is_control());
        assert_eq!(view.compression(), CompressionCodec::None);
        assert_eq!(view.raw(), raw.as_slice());
    }

    #[test]
    fn view_iterates_records_as_deltas() {
        let records = vec![
            make_record(0, 1_000, b"alpha", b"one"),
            make_record(1, 1_050, b"beta", b"two"),
            make_record(2, 1_100, b"gamma", b"three"),
        ];
        let raw = encode(&records, Compression::None);
        let view = RecordBatchView::from_bytes(&raw).expect("decode view");

        let collected: Vec<_> = view
            .records()
            .map(|r| {
                (
                    r.offset_delta,
                    r.timestamp_delta,
                    r.key.map(|b| b.to_vec()),
                    r.value.map(|b| b.to_vec()),
                    r.header_count(),
                )
            })
            .collect();

        assert_eq!(
            collected,
            vec![
                (0, 0, Some(b"alpha".to_vec()), Some(b"one".to_vec()), 1),
                (1, 50, Some(b"beta".to_vec()), Some(b"two".to_vec()), 1),
                (2, 100, Some(b"gamma".to_vec()), Some(b"three".to_vec()), 1),
            ]
        );
    }

    #[test]
    fn view_reports_compression_codec() {
        let records = vec![make_record(0, 1_000, b"k", b"v")];
        let raw = encode(&records, Compression::Gzip);
        let view = RecordBatchView::from_bytes(&raw).expect("decode view");
        assert_eq!(view.compression(), CompressionCodec::Gzip);
        assert_eq!(view.record_count(), 1);
    }

    #[test]
    fn view_rejects_malformed_bytes() {
        let bogus = [0u8; 4];
        assert!(RecordBatchView::from_bytes(&bogus).is_err());
    }

    #[test]
    fn stamp_base_offset_rewrites_header_crc_safely() {
        let records = vec![
            make_record(0, 1_000, b"k0", b"v0"),
            make_record(1, 1_050, b"k1", b"v1"),
        ];
        let mut raw = encode(&records, Compression::None);
        // Producer sent base_offset 0; the log assigns 1000.
        assert!(stamp_base_offset(&mut raw, 1_000));
        assert_eq!(i64::from_be_bytes(raw[0..8].try_into().unwrap()), 1_000);
        // The batch still decodes (CRC unaffected) and reports the new offsets.
        let view = RecordBatchView::from_bytes(&raw).expect("decode after stamp");
        assert_eq!(view.base_offset(), 1_000);
        assert_eq!(view.record_count(), 2);
        let offsets: Vec<i64> = view
            .records()
            .map(|r| 1_000 + r.offset_delta as i64)
            .collect();
        assert_eq!(offsets, vec![1_000, 1_001]);
        // Non-v2 / short buffers are rejected without mutation.
        let mut bogus = [0u8; 8];
        assert!(!stamp_base_offset(&mut bogus, 5));
        assert_eq!(bogus, [0u8; 8]);
    }
}
