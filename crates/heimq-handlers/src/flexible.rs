/// Returns true if the given (api_key, api_version) pair requires flexible framing.
///
/// Flexible framing uses compact types (varint-length-prefixed strings/arrays) and
/// tagged fields per KIP-482. Delegated to kafka-protocol's `ApiKey::request_header_version`:
/// flexible encoding corresponds to header_version >= 2.
pub fn is_flexible(api_key: i16, api_version: i16) -> bool {
    use heimq_protocol::messages::ApiKey;
    ApiKey::try_from(api_key)
        .map(|k| k.request_header_version(api_version) >= 2)
        .unwrap_or(false)
}
