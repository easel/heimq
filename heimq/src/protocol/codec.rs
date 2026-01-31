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

    #[test]
    fn test_decode_request_header() {
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
}
