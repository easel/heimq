/// Returns true if the given (api_key, api_version) pair requires flexible framing.
///
/// Flexible framing uses compact types (varint-length-prefixed strings/arrays) and
/// tagged fields per KIP-482.  The threshold per API key is defined by the
/// FLEXIBLE_VERSION_MIN table in CODEC-001.
///
/// FEAT-006: not yet implemented — panics until the dispatch table is wired.
pub fn is_flexible(_api_key: i16, _api_version: i16) -> bool {
    unimplemented!("FEAT-006: is_flexible dispatch table not yet implemented")
}
