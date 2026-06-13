//! Kafka protocol handling
//!
//! Decodes incoming requests, routes to appropriate handlers,
//! and encodes responses.

mod codec;
mod router;

pub use codec::{decode_request, encode_response, RequestHeader};
pub use router::Router;
pub use flexible::is_flexible;

mod flexible;

use crate::consumer_group::GroupCoordinatorCapabilities;
use crate::storage::{BackendCapabilities, OffsetStoreCapabilities};

/// API keys we support, with version ranges per API-001 contract targets.
///
/// FEAT-006 (flexible-version codec) is implemented: the `kafka-protocol`
/// crate's decode/encode handles both legacy and flexible encoding transparently
/// based on the api_version passed to each handler. Maxima here follow the
/// API-001 per-API target table; future bumps beyond these targets are tracked
/// in the parking lot.
pub const SUPPORTED_APIS: &[(i16, i16, i16)] = &[
    // (api_key, min_version, max_version)
    (0, 0, 11),  // Produce
    (1, 0, 17),  // Fetch
    (2, 0, 9),   // ListOffsets
    (3, 0, 12),  // Metadata
    (8, 0, 9),   // OffsetCommit
    (9, 0, 9),   // OffsetFetch
    (10, 0, 6),  // FindCoordinator
    (11, 0, 9),  // JoinGroup
    (12, 0, 4),  // Heartbeat
    (13, 0, 5),  // LeaveGroup
    (14, 0, 5),  // SyncGroup
    (18, 0, 3),  // ApiVersions
    (19, 0, 7),  // CreateTopics
    (20, 0, 6),  // DeleteTopics
    (22, 0, 5),  // InitProducerId
    (24, 0, 5),  // AddPartitionsToTxn
    (25, 0, 4),  // AddOffsetsToTxn
    (26, 0, 4),  // EndTxn
    (27, 0, 1),  // WriteTxnMarkers
    (28, 0, 4),  // TxnOffsetCommit
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

/// Which backend a given API depends on for its capability gate.
///
/// Per-API (not a global meet): Produce/Fetch must not be constrained by the
/// group-coordinator backend, OffsetCommit must not be constrained by the
/// log backend's compaction flag, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CapabilityGate {
    /// Always advertised — no backend-specific gating.
    Always,
    /// Requires a functional log backend.
    Log,
    /// Requires a functional offset store.
    OffsetStore,
    /// Requires a functional group-coordinator backend.
    GroupCoordinator,
}

fn capability_gate(api_key: i16) -> CapabilityGate {
    match api_key {
        // Log-backend APIs.
        0 | 1 | 2 | 3 | 19 | 20 => CapabilityGate::Log,
        // Offset-store APIs.
        8 | 9 => CapabilityGate::OffsetStore,
        // Group-coordinator APIs.
        10 | 11 | 12 | 13 | 14 => CapabilityGate::GroupCoordinator,
        // Transaction APIs and InitProducerId and ApiVersions are always available.
        22 | 24 | 25 | 26 | 27 | 28 | _ => CapabilityGate::Always,
    }
}

fn log_backend_present(caps: &BackendCapabilities) -> bool {
    caps.name != "unknown"
}

fn offset_store_present(caps: &OffsetStoreCapabilities) -> bool {
    caps.name != "unknown"
}

fn group_coordinator_present(caps: &GroupCoordinatorCapabilities) -> bool {
    caps.name != "unknown"
}

/// Compute the effective set of advertised APIs by intersecting the static
/// protocol support with the per-API capability gate of each backend.
///
/// The intersection is per-API: e.g. a missing group coordinator removes
/// JoinGroup/Heartbeat/etc. but leaves Produce/Fetch alone.
pub fn compute_supported_apis(
    log: &BackendCapabilities,
    offset: &OffsetStoreCapabilities,
    group: &GroupCoordinatorCapabilities,
) -> Vec<(i16, i16, i16)> {
    SUPPORTED_APIS
        .iter()
        .copied()
        .filter(|(api_key, _, _)| match capability_gate(*api_key) {
            CapabilityGate::Always => true,
            CapabilityGate::Log => log_backend_present(log),
            CapabilityGate::OffsetStore => offset_store_present(offset),
            CapabilityGate::GroupCoordinator => group_coordinator_present(group),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Durability;

    #[test]
    fn test_api_supported_and_range() {
        assert!(is_api_supported(0, 0));
        assert!(is_api_supported(0, 11));
        assert!(!is_api_supported(0, 12));
        assert_eq!(get_api_version_range(0), Some((0, 11)));
        assert_eq!(get_api_version_range(999), None);
    }

    fn memory_log_caps() -> BackendCapabilities {
        BackendCapabilities {
            name: "in-memory",
            ..BackendCapabilities::minimal()
        }
    }

    fn memory_offset_caps() -> OffsetStoreCapabilities {
        OffsetStoreCapabilities {
            name: "in-memory",
            ..OffsetStoreCapabilities::minimal()
        }
    }

    fn memory_group_caps() -> GroupCoordinatorCapabilities {
        GroupCoordinatorCapabilities {
            name: "in-memory",
            ..GroupCoordinatorCapabilities::minimal()
        }
    }

    #[test]
    fn memory_default_advertises_full_supported_set() {
        let apis =
            compute_supported_apis(&memory_log_caps(), &memory_offset_caps(), &memory_group_caps());
        assert_eq!(apis, SUPPORTED_APIS.to_vec());
    }

    #[test]
    fn missing_group_coordinator_drops_only_group_apis() {
        let apis = compute_supported_apis(
            &memory_log_caps(),
            &memory_offset_caps(),
            &GroupCoordinatorCapabilities::minimal(), // name = "unknown"
        );
        let keys: Vec<i16> = apis.iter().map(|(k, _, _)| *k).collect();
        // Produce/Fetch/ListOffsets/Metadata still present.
        for k in [0, 1, 2, 3, 19, 20] {
            assert!(keys.contains(&k), "log API {} dropped unexpectedly", k);
        }
        // Offset store APIs still present.
        for k in [8, 9] {
            assert!(keys.contains(&k), "offset API {} dropped unexpectedly", k);
        }
        // Group APIs are filtered out.
        for k in [10, 11, 12, 13, 14] {
            assert!(!keys.contains(&k), "group API {} should be filtered", k);
        }
        // ApiVersions stays.
        assert!(keys.contains(&18));
    }

    #[test]
    fn missing_offset_store_drops_only_offset_apis() {
        let apis = compute_supported_apis(
            &memory_log_caps(),
            &OffsetStoreCapabilities::minimal(), // name = "unknown"
            &memory_group_caps(),
        );
        let keys: Vec<i16> = apis.iter().map(|(k, _, _)| *k).collect();
        assert!(!keys.contains(&8));
        assert!(!keys.contains(&9));
        // Group APIs survive.
        for k in [10, 11, 12, 13, 14] {
            assert!(keys.contains(&k));
        }
        // Log APIs survive.
        for k in [0, 1, 3] {
            assert!(keys.contains(&k));
        }
    }

    #[test]
    fn compaction_false_does_not_leak_compaction_specific_apis() {
        // SUPPORTED_APIS today contains no compaction-specific entries, so
        // a compaction=false backend must advertise the same effective set as
        // the memory default. This pins the contract: per-API gating is not
        // a global meet that would drop unrelated APIs when compaction=false.
        let log = BackendCapabilities {
            name: "in-memory",
            compaction: false,
            ..BackendCapabilities::minimal()
        };
        let apis = compute_supported_apis(&log, &memory_offset_caps(), &memory_group_caps());
        assert_eq!(apis, SUPPORTED_APIS.to_vec());
    }

    #[test]
    fn capability_gate_assigns_each_api_to_one_backend() {
        // Produce/Fetch are gated on the log backend only.
        assert_eq!(capability_gate(0), CapabilityGate::Log);
        assert_eq!(capability_gate(1), CapabilityGate::Log);
        // OffsetCommit/OffsetFetch are gated on the offset store only.
        assert_eq!(capability_gate(8), CapabilityGate::OffsetStore);
        assert_eq!(capability_gate(9), CapabilityGate::OffsetStore);
        // Group APIs are gated on the group coordinator only.
        assert_eq!(capability_gate(11), CapabilityGate::GroupCoordinator);
        // ApiVersions is always advertised.
        assert_eq!(capability_gate(18), CapabilityGate::Always);
    }

    #[test]
    fn presence_helpers_recognize_named_backends() {
        assert!(log_backend_present(&memory_log_caps()));
        assert!(!log_backend_present(&BackendCapabilities::minimal()));
        assert!(offset_store_present(&memory_offset_caps()));
        assert!(!offset_store_present(&OffsetStoreCapabilities::minimal()));
        assert!(group_coordinator_present(&memory_group_caps()));
        assert!(!group_coordinator_present(&GroupCoordinatorCapabilities::minimal()));
        // Durability has no effect on presence — presence is name-based.
        let _ = Durability::None;
    }
}
