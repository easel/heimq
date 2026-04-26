//! ApiVersions request handler (API Key 18)

use kafka_protocol::messages::api_versions_response::ApiVersion;
use kafka_protocol::messages::ApiVersionsResponse;

/// Handle ApiVersions request given the effective set of advertised APIs.
///
/// `apis` is computed by `protocol::compute_supported_apis` from each
/// backend's capability descriptor at server startup. Passing the slice
/// keeps this handler pure and lets tests vary the advertised set.
pub fn handle(_api_version: i16, apis: &[(i16, i16, i16)]) -> ApiVersionsResponse {
    let mut response = ApiVersionsResponse::default();
    response.error_code = 0;

    for &(api_key, min_version, max_version) in apis {
        let mut api = ApiVersion::default();
        api.api_key = api_key;
        api.min_version = min_version;
        api.max_version = max_version;
        response.api_keys.push(api);
    }

    response
}
