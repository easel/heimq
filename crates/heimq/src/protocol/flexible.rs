/// Per-API flexible-version thresholds per CODEC-001 §Precedence and
/// Compatibility (FLEXIBLE_VERSION_MIN). An API key absent from this table
/// is treated as legacy framing (`is_flexible` returns false).
pub const FLEXIBLE_VERSION_MIN: &[(i16, i16)] = &[
    (0, 9),  // Produce
    (1, 12), // Fetch
    (2, 6),  // ListOffsets
    (3, 9),  // Metadata
    (8, 8),  // OffsetCommit
    (9, 6),  // OffsetFetch
    (10, 3), // FindCoordinator
    (11, 6), // JoinGroup
    (12, 4), // Heartbeat
    (13, 4), // LeaveGroup
    (14, 4), // SyncGroup
    (18, 3), // ApiVersions (request header only; response stays header v0)
    (19, 5), // CreateTopics
    (20, 4), // DeleteTopics
];

/// Returns true if the given (api_key, api_version) pair requires flexible framing.
///
/// Flexible framing uses compact types (varint-length-prefixed strings/arrays) and
/// tagged fields per KIP-482.  The threshold per API key is defined by the
/// FLEXIBLE_VERSION_MIN table in CODEC-001.
pub fn is_flexible(api_key: i16, api_version: i16) -> bool {
    FLEXIBLE_VERSION_MIN
        .iter()
        .find(|&&(key, _)| key == api_key)
        .is_some_and(|&(_, min)| api_version >= min)
}
