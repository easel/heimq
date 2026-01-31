//! Kafka protocol handling
//!
//! Decodes incoming requests, routes to appropriate handlers,
//! and encodes responses.

mod codec;
mod router;

pub use codec::{decode_request, encode_response, RequestHeader};
pub use router::Router;

use bytes::Bytes;
use kafka_protocol::messages::*;

/// API keys we support
///
/// Max versions are kept below "flexible versions" boundaries to avoid
/// needing compact string (varint) encoding. Most flexible versions start at:
/// - Produce v9, Fetch v12, Metadata v9, etc.
pub const SUPPORTED_APIS: &[(i16, i16, i16)] = &[
    // (api_key, min_version, max_version)
    (0, 0, 8),   // Produce (v9+ uses flexible versions)
    (1, 0, 11),  // Fetch (v12+ uses flexible versions)
    (2, 0, 7),   // ListOffsets
    (3, 0, 8),   // Metadata (v9+ uses flexible versions)
    (8, 0, 7),   // OffsetCommit (v8+ uses flexible versions)
    (9, 0, 7),   // OffsetFetch (v8+ uses flexible versions)
    (10, 0, 3),  // FindCoordinator (v4+ uses flexible versions)
    (11, 0, 8),  // JoinGroup (v9+ uses flexible versions)
    (12, 0, 4),  // Heartbeat
    (13, 0, 4),  // LeaveGroup (v5+ uses flexible versions)
    (14, 0, 4),  // SyncGroup (v5+ uses flexible versions)
    (18, 0, 3),  // ApiVersions
    (19, 0, 6),  // CreateTopics (v7+ uses flexible versions)
    (20, 0, 5),  // DeleteTopics (v6+ uses flexible versions)
];

/// Check if an API version is supported
pub fn is_api_supported(api_key: i16, api_version: i16) -> bool {
    SUPPORTED_APIS
        .iter()
        .any(|(key, min, max)| *key == api_key && api_version >= *min && api_version <= *max)
}

/// Get the version range for an API
pub fn get_api_version_range(api_key: i16) -> Option<(i16, i16)> {
    SUPPORTED_APIS
        .iter()
        .find(|(key, _, _)| *key == api_key)
        .map(|(_, min, max)| (*min, *max))
}
