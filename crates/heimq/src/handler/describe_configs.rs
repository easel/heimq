//! DescribeConfigs request handler (API Key 32)
//!
//! Returns an empty (but non-error) config set for all requested resources.
//! This is sufficient for clients that probe configs to check connectivity;
//! full config support would require a config store backend.

use crate::error::Result;
use bytes::Bytes;
use kafka_protocol::messages::describe_configs_request::DescribeConfigsRequest;
use kafka_protocol::messages::describe_configs_response::DescribeConfigsResult;
use kafka_protocol::messages::DescribeConfigsResponse;
use kafka_protocol::protocol::{Decodable, StrBytes};

pub fn handle(api_version: i16, body: &[u8]) -> Result<DescribeConfigsResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    let request = match DescribeConfigsRequest::decode(&mut buf, api_version) {
        Ok(r) => r,
        Err(_) => return Ok(DescribeConfigsResponse::default()),
    };

    let mut response = DescribeConfigsResponse::default();
    for res in &request.resources {
        let mut result = DescribeConfigsResult::default();
        result.resource_type = res.resource_type;
        result.resource_name = StrBytes::from_string(res.resource_name.as_str().to_string());
        result.error_code = 0;
        result.configs = vec![];
        response.results.push(result);
    }
    Ok(response)
}
