//! Protocol encoding and decoding

use bytes::{Buf, BufMut, Bytes, BytesMut};
use kafka_protocol::protocol::Encodable;
use std::io::Cursor;
use tracing::trace;

/// Parsed request header
#[derive(Debug, Clone)]
pub struct RequestHeader {
    pub api_key: i16,
    pub api_version: i16,
    pub correlation_id: i32,
    #[allow(dead_code)]
    pub client_id: Option<String>,
}

/// Decode a request from bytes
///
/// Returns the header and the remaining bytes (request body)
pub fn decode_request(data: &[u8]) -> std::io::Result<(RequestHeader, Bytes)> {
    if data.len() < 8 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Request too short",
        ));
    }

    let mut cursor = Cursor::new(data);

    // Read header fields
    let api_key = cursor.get_i16();
    let api_version = cursor.get_i16();
    let correlation_id = cursor.get_i32();

    // Client ID (nullable string)
    let client_id = if cursor.remaining() >= 2 {
        let len = cursor.get_i16();
        if len >= 0 && cursor.remaining() >= len as usize {
            let mut buf = vec![0u8; len as usize];
            cursor.copy_to_slice(&mut buf);
            Some(String::from_utf8_lossy(&buf).to_string())
        } else if len == -1 {
            None
        } else {
            None
        }
    } else {
        None
    };

    let header = RequestHeader {
        api_key,
        api_version,
        correlation_id,
        client_id,
    };

    // Return remaining bytes as the body
    let pos = cursor.position() as usize;
    let body = Bytes::copy_from_slice(&data[pos..]);

    trace!(
        api_key = header.api_key,
        api_version = header.api_version,
        correlation_id = header.correlation_id,
        body_len = body.len(),
        "Decoded request header"
    );

    Ok((header, body))
}

/// Encode a response with the correlation ID
pub fn encode_response<R: Encodable>(
    correlation_id: i32,
    api_version: i16,
    response: &R,
) -> std::io::Result<Bytes> {
    let mut buf = BytesMut::new();

    // Reserve space for message length
    buf.put_i32(0);

    // Write correlation ID
    buf.put_i32(correlation_id);

    // Encode response body
    response
        .encode(&mut buf, api_version)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

    // Write message length
    let len = (buf.len() - 4) as i32;
    buf[0..4].copy_from_slice(&len.to_be_bytes());

    Ok(buf.freeze())
}

/// Encode just the response body (without length prefix)
#[allow(dead_code)]
pub fn encode_response_body<R: Encodable>(
    correlation_id: i32,
    api_version: i16,
    response: &R,
) -> std::io::Result<BytesMut> {
    let mut buf = BytesMut::new();

    // Write correlation ID
    buf.put_i32(correlation_id);

    // Encode response body
    response
        .encode(&mut buf, api_version)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::init_tracing;
    use anyhow::anyhow;

    struct FailingEncode;

    impl Encodable for FailingEncode {
        fn encode<B: kafka_protocol::protocol::buf::ByteBufMut>(
            &self,
            _buf: &mut B,
            _version: i16,
        ) -> anyhow::Result<()> {
            Err(anyhow!("boom"))
        }

        fn compute_size(&self, _version: i16) -> anyhow::Result<usize> {
            Ok(0)
        }
    }

    #[test]
    fn test_decode_request_header() {
        init_tracing();
        // API key 18 (ApiVersions), version 0, correlation_id 1, no client_id
        let data = [
            0x00, 0x12, // api_key = 18
            0x00, 0x00, // api_version = 0
            0x00, 0x00, 0x00, 0x01, // correlation_id = 1
            0xFF, 0xFF, // client_id = null (-1)
        ];

        let (header, _) = decode_request(&data).unwrap();
        assert_eq!(header.api_key, 18);
        assert_eq!(header.api_version, 0);
        assert_eq!(header.correlation_id, 1);
    }

    #[test]
    fn test_decode_with_client_id() {
        let mut data = vec![];
        data.extend_from_slice(&18i16.to_be_bytes());
        data.extend_from_slice(&0i16.to_be_bytes());
        data.extend_from_slice(&7i32.to_be_bytes());
        data.extend_from_slice(&3i16.to_be_bytes());
        data.extend_from_slice(b"app");

        let (header, body) = decode_request(&data).unwrap();
        assert_eq!(header.correlation_id, 7);
        assert_eq!(header.client_id.as_deref(), Some("app"));
        assert!(body.is_empty());
    }

    #[test]
    fn test_encode_response_body() {
        use kafka_protocol::messages::ApiVersionsResponse;
        let response = ApiVersionsResponse::default();
        let buf = encode_response_body(5, 0, &response).unwrap();
        assert!(buf.len() >= 4);
    }

    #[test]
    fn test_decode_request_too_short() {
        let data = [0x00, 0x01, 0x02];
        assert!(decode_request(&data).is_err());
    }

    #[test]
    fn test_decode_request_invalid_client_id_length() {
        let data = [
            0x00, 0x12, // api_key
            0x00, 0x00, // api_version
            0x00, 0x00, 0x00, 0x01, // correlation_id
            0x00, 0x04, // client_id length 4
            0x61, // only one byte
        ];
        let (header, _) = decode_request(&data).unwrap();
        assert!(header.client_id.is_none());
    }

    #[test]
    fn test_decode_request_missing_client_id_field() {
        let data = [
            0x00, 0x12, // api_key
            0x00, 0x00, // api_version
            0x00, 0x00, 0x00, 0x01, // correlation_id
        ];
        let (header, _) = decode_request(&data).unwrap();
        assert!(header.client_id.is_none());
    }

    #[test]
    fn test_encode_response_error_mapping() {
        let response = FailingEncode;
        let err = encode_response(1, 0, &response).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn test_encode_response_body_error_mapping() {
        let response = FailingEncode;
        let err = encode_response_body(1, 0, &response).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn test_compute_size_for_failing_encode() {
        let response = FailingEncode;
        assert_eq!(response.compute_size(0).unwrap(), 0);
    }

    #[test]
    fn test_encode_response_invalid_versions_error() {
        use kafka_protocol::messages::api_versions_response::ApiVersionsResponse;
        use kafka_protocol::messages::create_topics_response::CreateTopicsResponse;
        use kafka_protocol::messages::delete_topics_response::DeleteTopicsResponse;
        use kafka_protocol::messages::fetch_response::FetchResponse;
        use kafka_protocol::messages::find_coordinator_response::FindCoordinatorResponse;
        use kafka_protocol::messages::heartbeat_response::HeartbeatResponse;
        use kafka_protocol::messages::join_group_response::JoinGroupResponse;
        use kafka_protocol::messages::leave_group_response::LeaveGroupResponse;
        use kafka_protocol::messages::list_offsets_response::ListOffsetsResponse;
        use kafka_protocol::messages::metadata_response::MetadataResponse;
        use kafka_protocol::messages::offset_commit_response::OffsetCommitResponse;
        use kafka_protocol::messages::offset_fetch_response::OffsetFetchResponse;
        use kafka_protocol::messages::produce_response::ProduceResponse;
        use kafka_protocol::messages::sync_group_response::SyncGroupResponse;

        let invalid_version = i16::MAX;
        let err = encode_response(1, invalid_version, &ApiVersionsResponse::default()).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        let err = encode_response(1, invalid_version, &MetadataResponse::default()).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        let err = encode_response(1, invalid_version, &ProduceResponse::default()).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        let err = encode_response(1, invalid_version, &FetchResponse::default()).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        let err = encode_response(1, invalid_version, &ListOffsetsResponse::default()).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        let err = encode_response(1, invalid_version, &CreateTopicsResponse::default()).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        let err = encode_response(1, invalid_version, &DeleteTopicsResponse::default()).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        let err = encode_response(1, invalid_version, &FindCoordinatorResponse::default()).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        let err = encode_response(1, invalid_version, &JoinGroupResponse::default()).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        let err = encode_response(1, invalid_version, &HeartbeatResponse::default()).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        let err = encode_response(1, invalid_version, &LeaveGroupResponse::default()).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        let err = encode_response(1, invalid_version, &SyncGroupResponse::default()).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        let err = encode_response(1, invalid_version, &OffsetCommitResponse::default()).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        let err = encode_response(1, invalid_version, &OffsetFetchResponse::default()).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn test_encode_response_body_invalid_version_error() {
        use kafka_protocol::messages::api_versions_response::ApiVersionsResponse;
        let err = encode_response_body(1, i16::MAX, &ApiVersionsResponse::default()).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }
}

/// Proptest-based round-trip tests for FEAT-006 flexible-version codec primitives.
///
/// Codec primitives (compact_string, compact_bytes, compact_array, unsigned_varint,
/// signed_varint, tagged_fields) are provided by the `kafka-protocol` crate per ADR-003.
/// These tests verify the crate's round-trip behaviour and drive the FEAT-006 build bead
/// by asserting heimq-side dispatch (is_flexible) and framing that are not yet implemented.
///
/// EXPECTED STATE: proptest round-trips pass (crate handles primitives); FEAT-006
/// integration tests (is_flexible, flexible decode/encode) fail until the build bead lands.
#[cfg(test)]
mod flexible_tests {
    use super::*;
    use crate::protocol::is_flexible;
    use bytes::BytesMut;
    use kafka_protocol::messages::TopicName;
    use kafka_protocol::protocol::{Decodable, Encodable, StrBytes};
    use proptest::prelude::*;

    fn topic_name(s: &str) -> TopicName {
        TopicName(StrBytes::from_string(s.to_owned()))
    }

    // -------------------------------------------------------------------------
    // Codec primitive round-trip baselines (via kafka-protocol crate)
    // -------------------------------------------------------------------------

    proptest! {
        /// compact_string round-trip: arbitrary ASCII strings encode and decode losslessly
        /// at flexible version v9 (MetadataRequest topic name field).
        /// Null-sentinel: varint(0) encodes null; empty string is distinct (varint(1) + 0 bytes).
        #[test]
        fn prop_compact_string_roundtrip(s in "[\\x20-\\x7e]{0,200}") {
            use kafka_protocol::messages::metadata_request::{MetadataRequest, MetadataRequestTopic};
            let mut topic = MetadataRequestTopic::default();
            topic.name = Some(topic_name(&s));
            let mut req = MetadataRequest::default();
            req.topics = Some(vec![topic]);
            let mut buf = BytesMut::new();
            req.encode(&mut buf, 9).expect("encode at flexible v9");
            let mut frozen = buf.freeze();
            let decoded = MetadataRequest::decode(&mut frozen, 9).expect("decode at flexible v9");
            let got = decoded.topics.unwrap().into_iter().next().unwrap().name.unwrap();
            prop_assert_eq!(got.to_string(), s);
        }

        /// compact_string null sentinel: None encodes as varint(0) and decodes as None.
        #[test]
        fn prop_compact_string_null_roundtrip(_unused in 0u8..1) {
            use kafka_protocol::messages::metadata_request::{MetadataRequest, MetadataRequestTopic};
            let mut topic = MetadataRequestTopic::default();
            topic.name = None;
            let mut req = MetadataRequest::default();
            req.topics = Some(vec![topic]);
            let mut buf = BytesMut::new();
            req.encode(&mut buf, 9).expect("encode null compact_string");
            let mut frozen = buf.freeze();
            let decoded = MetadataRequest::decode(&mut frozen, 9).expect("decode null compact_string");
            let got_name = decoded.topics.unwrap().into_iter().next().unwrap().name;
            prop_assert!(got_name.is_none(), "null compact_string must decode as None");
        }

        /// compact_array round-trip: vary the number of topics from 0 (null/empty array
        /// sentinel varint) to 32, exercising compact_array length encoding.
        #[test]
        fn prop_compact_array_roundtrip(count in 0usize..=32) {
            use kafka_protocol::messages::metadata_request::{MetadataRequest, MetadataRequestTopic};
            let topics: Vec<MetadataRequestTopic> = (0..count)
                .map(|i| {
                    let mut t = MetadataRequestTopic::default();
                    t.name = Some(topic_name(&format!("topic-{}", i)));
                    t
                })
                .collect();
            let mut req = MetadataRequest::default();
            req.topics = Some(topics);
            let mut buf = BytesMut::new();
            req.encode(&mut buf, 9).expect("encode compact_array");
            let mut frozen = buf.freeze();
            let decoded = MetadataRequest::decode(&mut frozen, 9).expect("decode compact_array");
            prop_assert_eq!(decoded.topics.map(|v| v.len()).unwrap_or(0), count);
        }

        /// compact_string boundary lengths cross the 1-byte → 2-byte varint boundary at
        /// length+1 = 128 (i.e., len=127 → varint 0x80 0x01).  Covers the thresholds
        /// the bead specifies: empty (0), single-byte (1), 1<<7-1 (127), 1<<7 (128).
        #[test]
        fn prop_compact_string_varint_boundary(len in prop::sample::select(vec![
            0usize, 1, 126, 127, 128, 200,
        ])) {
            use kafka_protocol::messages::metadata_request::{MetadataRequest, MetadataRequestTopic};
            let s: String = "a".repeat(len);
            let mut topic = MetadataRequestTopic::default();
            topic.name = Some(topic_name(&s));
            let mut req = MetadataRequest::default();
            req.topics = Some(vec![topic]);
            let mut buf = BytesMut::new();
            req.encode(&mut buf, 9).expect("encode boundary compact_string");
            let mut frozen = buf.freeze();
            let decoded = MetadataRequest::decode(&mut frozen, 9).expect("decode boundary compact_string");
            let got = decoded.topics.unwrap().into_iter().next().unwrap().name.unwrap();
            prop_assert_eq!(got.to_string(), s);
        }

        /// tagged_fields block round-trip: a flexible message always carries an empty
        /// tagged-fields block (varint 0).  Verify the round-trip is lossless.
        #[test]
        fn prop_tagged_fields_empty_block_roundtrip(_unused in 0u8..1) {
            use kafka_protocol::messages::api_versions_request::ApiVersionsRequest;
            let req = ApiVersionsRequest::default();
            let mut buf = BytesMut::new();
            req.encode(&mut buf, 3).expect("encode ApiVersionsRequest v3 (flexible)");
            let mut frozen = buf.freeze();
            ApiVersionsRequest::decode(&mut frozen, 3).expect("decode ApiVersionsRequest v3");
        }

        /// unsigned_varint / signed_varint: ListOffsets v6+ uses compact arrays with
        /// varint-encoded counts.  Round-trip with up to 16 partitions exercises the
        /// varint encoding path across multiple values including the 1<<7 threshold.
        #[test]
        fn prop_unsigned_varint_via_compact_array(count in 0usize..=16) {
            use kafka_protocol::messages::list_offsets_request::{
                ListOffsetsRequest, ListOffsetsTopic, ListOffsetsPartition,
            };
            let partitions: Vec<ListOffsetsPartition> = (0..count)
                .map(|i| {
                    let mut p = ListOffsetsPartition::default();
                    p.partition_index = i as i32;
                    p.timestamp = -1;
                    p
                })
                .collect();
            let mut topic = ListOffsetsTopic::default();
            topic.name = topic_name("bench");
            topic.partitions = partitions;
            let mut req = ListOffsetsRequest::default();
            use kafka_protocol::messages::BrokerId;
            req.replica_id = BrokerId(-1);
            req.topics = vec![topic];
            let mut buf = BytesMut::new();
            req.encode(&mut buf, 6).expect("encode ListOffsetsRequest v6 (flexible)");
            let mut frozen = buf.freeze();
            let decoded = ListOffsetsRequest::decode(&mut frozen, 6)
                .expect("decode ListOffsetsRequest v6");
            prop_assert_eq!(decoded.topics[0].partitions.len(), count);
        }
    }

    // -------------------------------------------------------------------------
    // FEAT-006 integration tests — ALL fail until the build bead lands.
    // -------------------------------------------------------------------------

    /// CODEC-001 §Contract Validation #1: is_flexible boundary per API key.
    ///
    /// Fails: is_flexible() panics with "not yet implemented".
    #[test]
    fn test_is_flexible_boundary() {
        // (api_key, flexible_from)
        let table: &[(i16, i16)] = &[
            (0, 9),   // Produce
            (1, 12),  // Fetch
            (2, 6),   // ListOffsets
            (3, 9),   // Metadata
            (8, 8),   // OffsetCommit
            (9, 6),   // OffsetFetch
            (10, 3),  // FindCoordinator
            (11, 6),  // JoinGroup
            (12, 4),  // Heartbeat
            (13, 4),  // LeaveGroup
            (14, 4),  // SyncGroup
            (18, 3),  // ApiVersions
            (19, 5),  // CreateTopics
            (20, 4),  // DeleteTopics
        ];
        for &(api_key, flex_min) in table {
            assert!(
                !is_flexible(api_key, flex_min - 1),
                "api_key={} version={} should be legacy",
                api_key,
                flex_min - 1,
            );
            assert!(
                is_flexible(api_key, flex_min),
                "api_key={} version={} should be flexible",
                api_key,
                flex_min,
            );
        }
        assert!(!is_flexible(999, 0), "unknown api_key must return false");
    }

    /// CODEC-001 §Contract Validation #2: decode_request consumes the tagged-fields
    /// block in a flexible header so the returned body starts at the right offset.
    ///
    /// Wire layout (flexible request header v2, ApiVersions v3, null client_id):
    ///   api_key=18 i16, api_version=3 i16, correlation_id=7 i32,
    ///   client_id=-1 i16, tagged-fields=varint(0)=0x00, body=0xAB
    ///
    /// Currently decode_request does not consume the 0x00 tagged-fields byte,
    /// so body.len() is 2 instead of 1 — fails.
    #[test]
    fn test_decode_request_flexible_header() {
        let data: &[u8] = &[
            0x00, 0x12, // api_key = 18 (ApiVersions)
            0x00, 0x03, // api_version = 3 (flexible)
            0x00, 0x00, 0x00, 0x07, // correlation_id = 7
            0xFF, 0xFF, // client_id = null
            0x00, // tagged-fields varint(0) = empty block
            0xAB, // first body byte
        ];
        let (header, body) = decode_request(data).expect("decode_request must succeed");
        assert_eq!(header.api_key, 18);
        assert_eq!(header.api_version, 3);
        assert_eq!(header.correlation_id, 7);
        assert_eq!(
            body.len(),
            1,
            "tagged-fields block must be consumed by decode_request (FEAT-006 not yet implemented)"
        );
        assert_eq!(body[0], 0xAB);
    }

    /// CODEC-001 §Contract Validation #4: encode_response at a flexible version writes
    /// an empty tagged-fields trailer (0x00 byte) after correlation_id and before body.
    ///
    /// Wire layout: [length i32][correlation_id i32][tagged-fields 0x00][body…]
    ///
    /// Verified by size: encode_response must produce exactly 1 more byte than
    /// encode_response_body for flexible versions (the tagged-fields trailer).
    /// Currently encode_response omits this byte — fails.
    #[test]
    fn test_encode_response_flexible_header() {
        use kafka_protocol::messages::metadata_response::MetadataResponse;
        let response = MetadataResponse::default();
        // Metadata v9 is flexible per CODEC-001.
        let body_only = encode_response_body(42, 9, &response)
            .expect("encode_response_body must succeed");
        let full = encode_response(42, 9, &response).expect("encode_response must succeed");
        // encode_response_body = [corr_id(4)][body...]
        // encode_response with FEAT-006 = [length(4)][corr_id(4)][tagged-fields(1)][body...]
        //   = 4 + body_only.len() + 1
        // encode_response without FEAT-006 = [length(4)][corr_id(4)][body...]
        //   = 4 + body_only.len()
        assert_eq!(
            full.len(),
            4 + body_only.len() + 1,
            "encode_response at flexible v9 must include 1-byte tagged-fields response header trailer (FEAT-006 not yet implemented)"
        );
        assert_eq!(full[8], 0x00, "tagged-fields trailer byte must be 0x00");
    }

    /// CODEC-001 §Contract Validation #5: ApiVersions (api_key=18) responses always use
    /// response header v0 (no tagged-fields trailer) even when the request is flexible v3.
    ///
    /// Pins the requirement: after FEAT-006 adds the trailer for other APIs, ApiVersions
    /// must NOT receive one.  Verified via legacy v2 where bytes[8] is the body start
    /// (error_code high byte = 0x00).
    #[test]
    fn test_encode_response_apiversions_no_flexible_header() {
        use kafka_protocol::messages::api_versions_response::ApiVersionsResponse;
        let response = ApiVersionsResponse::default();
        let bytes = encode_response(1, 2, &response).expect("encode_response must succeed");
        assert!(bytes.len() > 9);
        assert_eq!(bytes[8], 0x00, "ApiVersions v2: bytes[8] = error_code high byte");
        assert_eq!(bytes[9], 0x00, "ApiVersions v2: bytes[9] = error_code low byte");
    }
}
