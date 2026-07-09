//! `RecordBatchView` — a structured view over a Kafka v2 record batch.
//!
//! Today the log trait accepts raw record-batch bytes. To let backends
//! implement retention-by-timestamp, key-based compaction, header-aware
//! filtering, etc., we need a structured handle that exposes parsed batch
//! metadata plus a borrowed iterator over records without losing access to
//! the original bytes for fast-path pass-through.
//!
//! Decoding is implemented here rather than delegated to `kafka-protocol` so
//! that `heimq-broker` — the crate embedders depend on — carries no Kafka
//! protocol crate in its public dependency tree. Embedders that link their own
//! `kafka-protocol` (at whatever version) therefore never resolve a second,
//! conflicting copy through us.
//!
//! Only v2 batches (`magic == 2`) are decoded. Legacy v0/v1 message sets are
//! rejected, consistent with [`RecordBatchHeader::peek`] and
//! [`stamp_base_offset`], which have always been v2-only.

#![allow(dead_code)]

use crate::error::{HeimqError, Result};
use crate::storage::CompressionCodec;
use bytes::Bytes;
use std::borrow::Cow;
use std::io::Read;

/// Byte length of the fixed v2 batch header, through `record_count`.
const V2_HEADER_LEN: usize = 61;
const MAGIC_V2: u8 = 2;
/// The CRC covers everything from `attributes` to the end of the batch.
const CRC_COVERAGE_START: usize = 21;

fn proto(msg: impl Into<String>) -> HeimqError {
    HeimqError::Protocol(msg.into())
}

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
        if raw.len() < V2_HEADER_LEN || raw.get(16) != Some(&MAGIC_V2) {
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
    if batch.len() < V2_HEADER_LEN || batch.get(16) != Some(&MAGIC_V2) {
        return false;
    }
    batch[0..8].copy_from_slice(&base_offset.to_be_bytes());
    true
}

/// A record decoded out of the (possibly compressed) records section.
///
/// Offsets and timestamps are stored as the wire-format deltas relative to the
/// batch header's `base_offset` / `base_timestamp`.
struct DecodedRecord {
    offset_delta: i32,
    timestamp_delta: i64,
    key: Option<Bytes>,
    value: Option<Bytes>,
    headers: Vec<(String, Option<Bytes>)>,
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
    base_sequence: i32,
    base_offset: i64,
    base_timestamp: i64,
    max_timestamp: i64,
    is_transactional: bool,
    is_control: bool,
    compression: CompressionCodec,
    records: Vec<DecodedRecord>,
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
    headers: &'a [(String, Option<Bytes>)],
}

impl<'a> RecordView<'a> {
    /// Iterate over the record's headers as `(name, value)` pairs.
    pub fn headers(&self) -> impl Iterator<Item = (&'a str, Option<&'a [u8]>)> + 'a {
        self.headers
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_ref().map(Bytes::as_ref)))
    }

    pub fn header_count(&self) -> usize {
        self.headers.len()
    }
}

impl<'a> RecordBatchView<'a> {
    /// Decode a record batch from the given raw bytes.
    ///
    /// `base_offset` is read from the batch header and treated as a placeholder:
    /// it reflects what the producer sent (typically `0`) and will be rewritten
    /// by the log when the batch is actually appended.
    pub fn from_bytes(raw: &'a [u8]) -> Result<Self> {
        if raw.len() < V2_HEADER_LEN {
            return Err(proto(format!(
                "record batch too short: {} bytes, need at least {V2_HEADER_LEN}",
                raw.len()
            )));
        }
        if raw[16] != MAGIC_V2 {
            return Err(proto(format!(
                "unsupported record batch magic {}: heimq decodes v2 batches only",
                raw[16]
            )));
        }

        // `batch_length` counts the bytes following it, i.e. from
        // `partition_leader_epoch` at offset 12 to the end of the batch.
        let batch_length = i32::from_be_bytes(raw[8..12].try_into().expect("4 bytes"));
        let batch_end = usize::try_from(batch_length)
            .ok()
            .and_then(|n| n.checked_add(12))
            .filter(|end| *end >= V2_HEADER_LEN && *end <= raw.len())
            .ok_or_else(|| proto(format!("invalid batch_length {batch_length}")))?;

        let expected_crc = u32::from_be_bytes(raw[17..21].try_into().expect("4 bytes"));
        let actual_crc = crc32c::crc32c(&raw[CRC_COVERAGE_START..batch_end]);
        if expected_crc != actual_crc {
            return Err(proto(format!(
                "record batch CRC mismatch: expected {expected_crc}, computed {actual_crc}"
            )));
        }

        let attributes = i16::from_be_bytes([raw[21], raw[22]]);
        let compression = compression_from_attributes(attributes)?;
        let base_timestamp = i64::from_be_bytes(raw[27..35].try_into().expect("8 bytes"));
        let max_timestamp = i64::from_be_bytes(raw[35..43].try_into().expect("8 bytes"));
        let record_count = i32::from_be_bytes(raw[57..61].try_into().expect("4 bytes"));
        let record_count = usize::try_from(record_count)
            .map_err(|_| proto(format!("negative record_count {record_count}")))?;

        let records_section = &raw[V2_HEADER_LEN..batch_end];
        let decompressed = decompress(compression, records_section)?;
        let records = decode_records(&decompressed, record_count)?;

        Ok(Self {
            raw,
            producer_id: i64::from_be_bytes(raw[43..51].try_into().expect("8 bytes")),
            producer_epoch: i16::from_be_bytes([raw[51], raw[52]]),
            base_sequence: i32::from_be_bytes(raw[53..57].try_into().expect("4 bytes")),
            base_offset: i64::from_be_bytes(raw[0..8].try_into().expect("8 bytes")),
            base_timestamp,
            max_timestamp,
            is_transactional: attributes & 0x10 != 0,
            is_control: attributes & 0x20 != 0,
            compression,
            records,
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
        self.records.len()
    }

    /// The base sequence number of this batch. Returns -1 for non-idempotent
    /// batches (producer_id == -1).
    pub fn base_sequence(&self) -> i32 {
        self.base_sequence
    }

    /// Iterate records as borrowed `RecordView`s.
    pub fn records(&self) -> impl Iterator<Item = RecordView<'_>> + '_ {
        self.records.iter().map(|r| RecordView {
            offset_delta: r.offset_delta,
            timestamp_delta: r.timestamp_delta,
            key: r.key.as_ref(),
            value: r.value.as_ref(),
            headers: &r.headers,
        })
    }
}

fn compression_from_attributes(attributes: i16) -> Result<CompressionCodec> {
    match attributes & 0x07 {
        0 => Ok(CompressionCodec::None),
        1 => Ok(CompressionCodec::Gzip),
        2 => Ok(CompressionCodec::Snappy),
        3 => Ok(CompressionCodec::Lz4),
        4 => Ok(CompressionCodec::Zstd),
        other => Err(proto(format!("unknown compression codec {other}"))),
    }
}

fn decompress(codec: CompressionCodec, data: &[u8]) -> Result<Cow<'_, [u8]>> {
    let out = match codec {
        CompressionCodec::None => return Ok(Cow::Borrowed(data)),
        CompressionCodec::Gzip => {
            let mut out = Vec::new();
            flate2::read::GzDecoder::new(data)
                .read_to_end(&mut out)
                .map_err(|e| proto(format!("gzip decompress: {e}")))?;
            out
        }
        CompressionCodec::Snappy => decompress_snappy(data)?,
        CompressionCodec::Lz4 => {
            let mut out = Vec::new();
            lz4::Decoder::new(data)
                .map_err(|e| proto(format!("lz4 decompress: {e}")))?
                .read_to_end(&mut out)
                .map_err(|e| proto(format!("lz4 decompress: {e}")))?;
            out
        }
        CompressionCodec::Zstd => {
            zstd::stream::decode_all(data).map_err(|e| proto(format!("zstd decompress: {e}")))?
        }
    };
    Ok(Cow::Owned(out))
}

/// Kafka's Java client frames snappy with the `xerial` block format; other
/// producers emit a bare snappy block. Detect the framing rather than assuming,
/// so batches from any client decode.
const XERIAL_MAGIC: [u8; 8] = [0x82, b'S', b'N', b'A', b'P', b'P', b'Y', 0x00];
/// 8-byte magic, then 4-byte version and 4-byte min-compatible-version.
const XERIAL_HEADER_LEN: usize = 16;

fn decompress_snappy(data: &[u8]) -> Result<Vec<u8>> {
    let raw = |block: &[u8]| {
        snap::raw::Decoder::new()
            .decompress_vec(block)
            .map_err(|e| proto(format!("snappy decompress: {e}")))
    };

    if data.len() < XERIAL_HEADER_LEN || data[..8] != XERIAL_MAGIC {
        return raw(data);
    }

    let mut out = Vec::new();
    let mut pos = XERIAL_HEADER_LEN;
    while pos < data.len() {
        let len_end = pos
            .checked_add(4)
            .filter(|e| *e <= data.len())
            .ok_or_else(|| proto("snappy: truncated xerial block length"))?;
        let block_len = u32::from_be_bytes(data[pos..len_end].try_into().expect("4 bytes"));
        pos = len_end;
        let block_end = usize::try_from(block_len)
            .ok()
            .and_then(|n| pos.checked_add(n))
            .filter(|e| *e <= data.len())
            .ok_or_else(|| proto("snappy: truncated xerial block"))?;
        out.extend_from_slice(&raw(&data[pos..block_end])?);
        pos = block_end;
    }
    Ok(out)
}

fn decode_records(section: &[u8], count: usize) -> Result<Vec<DecodedRecord>> {
    let mut reader = Reader::new(section);
    // A record occupies at least one byte, so `count` elements need at least
    // `count` bytes. Bounding the pre-allocation by what is actually present
    // keeps a bogus header count from triggering a multi-gigabyte reservation.
    let mut out = Vec::with_capacity(count.min(reader.remaining()));
    for i in 0..count {
        out.push(
            decode_record(&mut reader).map_err(|e| proto(format!("record {i} of {count}: {e}")))?,
        );
    }
    Ok(out)
}

fn decode_record(reader: &mut Reader<'_>) -> Result<DecodedRecord> {
    let length = reader.read_varint()?;
    let length =
        usize::try_from(length).map_err(|_| proto(format!("negative record length {length}")))?;
    let body_end = reader
        .pos
        .checked_add(length)
        .filter(|e| *e <= reader.buf.len())
        .ok_or_else(|| proto("record length overruns batch"))?;

    let _attributes = reader.read_u8()?;
    let timestamp_delta = reader.read_varlong()?;
    let offset_delta = reader.read_varint()?;
    let key = reader.read_nullable_bytes()?;
    let value = reader.read_nullable_bytes()?;

    let header_count = reader.read_varint()?;
    let header_count = usize::try_from(header_count)
        .map_err(|_| proto(format!("negative header count {header_count}")))?;
    let mut headers = Vec::with_capacity(header_count.min(reader.remaining()));
    for _ in 0..header_count {
        let key_len = reader.read_varint()?;
        let key_len = usize::try_from(key_len)
            .map_err(|_| proto(format!("null header key (length {key_len})")))?;
        let key_bytes = reader.read_exact(key_len)?;
        let name = std::str::from_utf8(key_bytes)
            .map_err(|e| proto(format!("header key not utf-8: {e}")))?
            .to_owned();
        headers.push((name, reader.read_nullable_bytes()?));
    }

    if reader.pos != body_end {
        return Err(proto(format!(
            "record length mismatch: declared body ends at {body_end}, parsed to {}",
            reader.pos
        )));
    }

    Ok(DecodedRecord {
        offset_delta,
        timestamp_delta,
        key,
        value,
        headers,
    })
}

/// A bounds-checked forward cursor over the decompressed records section.
struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }

    fn read_u8(&mut self) -> Result<u8> {
        let b = *self
            .buf
            .get(self.pos)
            .ok_or_else(|| proto("unexpected end of records section"))?;
        self.pos += 1;
        Ok(b)
    }

    fn read_exact(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|e| *e <= self.buf.len())
            .ok_or_else(|| proto("unexpected end of records section"))?;
        let slice = &self.buf[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    /// Kafka varints are zigzag-encoded. Accumulate in a wider type so an
    /// over-long encoding is reported rather than silently overflowing.
    fn read_unsigned_varint(&mut self) -> Result<u32> {
        let mut value: u64 = 0;
        for i in 0..5u32 {
            let b = self.read_u8()?;
            value |= u64::from(b & 0x7F) << (i * 7);
            if b & 0x80 == 0 {
                return u32::try_from(value).map_err(|_| proto("varint overflows u32"));
            }
        }
        Err(proto("varint longer than 5 bytes"))
    }

    fn read_unsigned_varlong(&mut self) -> Result<u64> {
        let mut value: u128 = 0;
        for i in 0..10u32 {
            let b = self.read_u8()?;
            value |= u128::from(b & 0x7F) << (i * 7);
            if b & 0x80 == 0 {
                return u64::try_from(value).map_err(|_| proto("varlong overflows u64"));
            }
        }
        Err(proto("varlong longer than 10 bytes"))
    }

    fn read_varint(&mut self) -> Result<i32> {
        let n = self.read_unsigned_varint()?;
        Ok(((n >> 1) as i32) ^ -((n & 1) as i32))
    }

    fn read_varlong(&mut self) -> Result<i64> {
        let n = self.read_unsigned_varlong()?;
        Ok(((n >> 1) as i64) ^ -((n & 1) as i64))
    }

    /// Length-prefixed bytes where a negative length encodes `null`.
    fn read_nullable_bytes(&mut self) -> Result<Option<Bytes>> {
        let len = self.read_varint()?;
        if len < 0 {
            return Ok(None);
        }
        let bytes = self.read_exact(len as usize)?;
        Ok(Some(Bytes::copy_from_slice(bytes)))
    }
}

#[cfg(test)]
mod tests {
    // Fixtures are encoded with `kafka-protocol` so these tests are differential:
    // they assert heimq's hand-rolled decoder agrees with the reference encoder.
    // It is a dev-dependency only — Cargo does not resolve dev-dependencies of
    // non-workspace packages, so embedders never see it.
    use super::*;
    use bytes::BytesMut;
    use kafka_protocol::protocol::StrBytes;
    use kafka_protocol::records::{
        Compression, Record, RecordBatchEncoder, RecordEncodeOptions, TimestampType,
    };

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

    /// `RecordBatchView` is not `Debug`, so `Result::expect_err` is unavailable.
    fn expect_decode_err(raw: &[u8], why: &str) -> HeimqError {
        match RecordBatchView::from_bytes(raw) {
            Ok(_) => panic!("{why}"),
            Err(e) => e,
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
    fn public_api_iterates_decoded_records_without_kafka_protocol_types() {
        let records = vec![make_record(0, 1_000, b"alpha", b"one")];
        let raw = encode(&records, Compression::None);
        let view = RecordBatchView::from_bytes(&raw).expect("decode view");

        let record = view.records().next().expect("one decoded record");
        let headers: Vec<(&str, Option<&[u8]>)> = record.headers().collect();

        assert_eq!(record.offset_delta, 0);
        assert_eq!(record.timestamp_delta, 0);
        assert_eq!(record.key.map(Bytes::as_ref), Some(b"alpha".as_slice()));
        assert_eq!(record.value.map(Bytes::as_ref), Some(b"one".as_slice()));
        assert_eq!(headers, vec![("h1", Some(b"hv".as_slice()))]);
    }

    #[test]
    fn view_reports_compression_codec() {
        let records = vec![make_record(0, 1_000, b"k", b"v")];
        let raw = encode(&records, Compression::Gzip);
        let view = RecordBatchView::from_bytes(&raw).expect("decode view");
        assert_eq!(view.compression(), CompressionCodec::Gzip);
        assert_eq!(view.record_count(), 1);
    }

    /// Every codec must round-trip through the hand-rolled decoder and yield
    /// exactly what the reference encoder was handed.
    #[test]
    fn view_decodes_every_compression_codec() {
        let cases = [
            (Compression::None, CompressionCodec::None),
            (Compression::Gzip, CompressionCodec::Gzip),
            (Compression::Snappy, CompressionCodec::Snappy),
            (Compression::Lz4, CompressionCodec::Lz4),
            (Compression::Zstd, CompressionCodec::Zstd),
        ];
        let records = vec![
            make_record(0, 2_000, b"k0", b"v0"),
            make_record(1, 2_010, b"k1", b"v1"),
        ];

        for (compression, expected) in cases {
            let raw = encode(&records, compression);
            let view = RecordBatchView::from_bytes(&raw)
                .unwrap_or_else(|e| panic!("decode {expected:?}: {e}"));

            assert_eq!(view.compression(), expected);
            assert_eq!(view.record_count(), 2);
            let values: Vec<_> = view.records().map(|r| r.value.unwrap().to_vec()).collect();
            assert_eq!(values, vec![b"v0".to_vec(), b"v1".to_vec()], "{expected:?}");
            let deltas: Vec<_> = view.records().map(|r| r.timestamp_delta).collect();
            assert_eq!(deltas, vec![0, 10], "{expected:?}");
        }
    }

    #[test]
    fn view_decodes_null_key_and_value_and_headerless_records() {
        let mut record = make_record(0, 1_000, b"k", b"v");
        record.key = None;
        record.value = None;
        record.headers = kafka_protocol::indexmap::IndexMap::new();
        let raw = encode(&[record], Compression::None);

        let view = RecordBatchView::from_bytes(&raw).expect("decode view");
        let decoded = view.records().next().expect("one record");
        assert!(decoded.key.is_none());
        assert!(decoded.value.is_none());
        assert_eq!(decoded.header_count(), 0);
    }

    #[test]
    fn view_rejects_malformed_bytes() {
        let bogus = [0u8; 4];
        assert!(RecordBatchView::from_bytes(&bogus).is_err());
    }

    #[test]
    fn view_rejects_legacy_message_set_magic() {
        let records = vec![make_record(0, 1_000, b"k", b"v")];
        let mut raw = encode(&records, Compression::None);
        raw[16] = 1; // pretend it is a v1 message set
        let err = expect_decode_err(&raw, "v1 must be rejected");
        assert!(format!("{err}").contains("magic"), "got: {err}");
    }

    #[test]
    fn view_rejects_crc_mismatch() {
        let records = vec![make_record(0, 1_000, b"k", b"v")];
        let mut raw = encode(&records, Compression::None);
        // Flip a bit inside the CRC-covered region (attributes onward).
        let last = raw.len() - 1;
        raw[last] ^= 0xFF;
        let err = expect_decode_err(&raw, "corrupt batch must be rejected");
        assert!(format!("{err}").contains("CRC"), "got: {err}");
    }

    /// A batch whose header claims a huge record count must fail cheaply rather
    /// than pre-allocating gigabytes — the bug this decoder's ancestor carried.
    #[test]
    fn view_rejects_inflated_record_count_without_huge_allocation() {
        let records = vec![make_record(0, 1_000, b"k", b"v")];
        let mut raw = encode(&records, Compression::None);
        raw[57..61].copy_from_slice(&842_150_450i32.to_be_bytes());
        // Re-stamp the CRC so we exercise the record loop, not the CRC check.
        let crc = crc32c::crc32c(&raw[CRC_COVERAGE_START..]);
        raw[17..21].copy_from_slice(&crc.to_be_bytes());

        let err = expect_decode_err(&raw, "must not decode");
        assert!(
            format!("{err}").contains("end of records section"),
            "got: {err}"
        );
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
