//! ApiVersions request handler (API Key 18)

use crate::protocol::SUPPORTED_APIS;
use kafka_protocol::messages::api_versions_response::ApiVersion;
use kafka_protocol::messages::ApiVersionsResponse;

/// Handle ApiVersions request
pub fn handle(_api_version: i16) -> ApiVersionsResponse {
    let mut response = ApiVersionsResponse::default();
    response.error_code = 0; // No error

    // Add all supported APIs
    for &(api_key, min_version, max_version) in SUPPORTED_APIS {
        let mut api = ApiVersion::default();
        api.api_key = api_key;
        api.min_version = min_version;
        api.max_version = max_version;
        response.api_keys.push(api);
    }

    response
}
