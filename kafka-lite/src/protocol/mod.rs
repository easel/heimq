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
pub const SUPPORTED_APIS: &[(i16, i16, i16)] = &[
    // (api_key, min_version, max_version)
    (0, 0, 9),   // Produce
    (1, 0, 13),  // Fetch
    (2, 0, 7),   // ListOffsets
    (3, 0, 12),  // Metadata
    (8, 0, 8),   // OffsetCommit
    (9, 0, 8),   // OffsetFetch
    (10, 0, 4),  // FindCoordinator
    (11, 0, 9),  // JoinGroup
    (12, 0, 4),  // Heartbeat
    (13, 0, 5),  // LeaveGroup
    (14, 0, 5),  // SyncGroup
    (18, 0, 3),  // ApiVersions
    (19, 0, 7),  // CreateTopics
    (20, 0, 6),  // DeleteTopics
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
