//! ApiVersions request handler (API Key 18)

use heimq_protocol::messages::api_versions_response::ApiVersion;
use heimq_protocol::messages::ApiVersionsResponse;

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

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(response: &ApiVersionsResponse) -> Vec<i16> {
        response.api_keys.iter().map(|api| api.api_key).collect()
    }

    #[test]
    // @covers US-016-AC2
    fn api_versions_omits_apis_missing_from_capability_gated_input() {
        let advertised = [(0, 0, 11), (1, 0, 12), (18, 0, 3)];

        let response = handle(3, &advertised);
        let keys = keys(&response);

        assert!(keys.contains(&0), "Produce remains advertised");
        assert!(keys.contains(&1), "Fetch remains advertised");
        assert!(keys.contains(&18), "ApiVersions remains advertised");
        assert!(
            !keys.contains(&8) && !keys.contains(&9),
            "Offset APIs omitted by capability computation must not be reintroduced"
        );
        assert!(
            !keys.contains(&10) && !keys.contains(&11),
            "Group APIs omitted by capability computation must not be reintroduced"
        );
    }

    #[test]
    // @covers US-016-AC3
    fn api_versions_preserves_unrelated_apis_when_one_family_is_omitted() {
        let advertised = [(0, 0, 11), (1, 0, 12), (8, 0, 9), (9, 0, 9), (18, 0, 3)];

        let response = handle(3, &advertised);
        let keys = keys(&response);

        for key in [0, 1, 8, 9, 18] {
            assert!(keys.contains(&key), "api key {key} must remain advertised");
        }
        for key in [10, 11, 12, 13, 14] {
            assert!(
                !keys.contains(&key),
                "missing group-family key {key} must stay omitted"
            );
        }
    }
}
