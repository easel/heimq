//! Kafka protocol handling
//!
//! Decodes incoming requests, routes to appropriate handlers,
//! and encodes responses.

mod codec;
mod router;

pub use codec::{decode_request, encode_response, RequestHeader};
pub use router::Router;

/// API keys we support
///
/// Max versions are kept below "flexible versions" boundaries to avoid
/// needing compact string (varint) encoding. Most flexible versions start at:
/// - Produce v9, Fetch v12, Metadata v9, etc.
pub const SUPPORTED_APIS: &[(i16, i16, i16)] = &[
    // (api_key, min_version, max_version)
    // Max versions must stay below the flexible-version boundary
    // for each API (flexible = compact strings + varints + tagged
    // fields). The handlers parse the legacy layout only.
    (0, 0, 8),   // Produce (v9+ flexible)
    (1, 0, 11),  // Fetch (v12+ flexible)
    (2, 0, 5),   // ListOffsets (v6+ flexible)
    (3, 0, 8),   // Metadata (v9+ flexible)
    (8, 0, 7),   // OffsetCommit (v8+ flexible)
    (9, 0, 5),   // OffsetFetch (v6+ flexible)
    (10, 0, 2),  // FindCoordinator (v3+ flexible)
    (11, 0, 5),  // JoinGroup (v6+ flexible)
    (12, 0, 3),  // Heartbeat (v4+ flexible)
    (13, 0, 3),  // LeaveGroup (v4+ flexible)
    (14, 0, 3),  // SyncGroup (v4+ flexible)
    (18, 0, 2),  // ApiVersions (v3+ flexible for request only; keep v2 to avoid edge cases)
    (19, 0, 4),  // CreateTopics (v5+ flexible)
    (20, 0, 3),  // DeleteTopics (v4+ flexible)
];

/// Check if an API version is supported
#[allow(dead_code)]
pub fn is_api_supported(api_key: i16, api_version: i16) -> bool {
    SUPPORTED_APIS
        .iter()
        .any(|(key, min, max)| *key == api_key && api_version >= *min && api_version <= *max)
}

/// Get the version range for an API
#[allow(dead_code)]
pub fn get_api_version_range(api_key: i16) -> Option<(i16, i16)> {
    SUPPORTED_APIS
        .iter()
        .find(|(key, _, _)| *key == api_key)
        .map(|(_, min, max)| (*min, *max))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_supported_and_range() {
        assert!(is_api_supported(0, 0));
        assert!(!is_api_supported(0, 9));
        assert_eq!(get_api_version_range(0), Some((0, 8)));
        assert_eq!(get_api_version_range(999), None);
    }
}
