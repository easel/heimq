//! AlterConfigs request handler (API Key 33)
//!
//! heimq does not maintain mutable broker/topic config objects, so we return
//! success with no-op for every requested resource (same approach as
//! DescribeConfigs).

use crate::error::Result;
use bytes::Bytes;
use kafka_protocol::messages::alter_configs_request::AlterConfigsRequest;
use kafka_protocol::messages::alter_configs_response::AlterConfigsResourceResponse;
use kafka_protocol::messages::AlterConfigsResponse;
use kafka_protocol::protocol::Decodable;

pub fn handle(api_version: i16, body: &[u8]) -> Result<AlterConfigsResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    let request = match AlterConfigsRequest::decode(&mut buf, api_version) {
        Ok(r) => r,
        Err(_) => return Ok(AlterConfigsResponse::default()),
    };

    let mut response = AlterConfigsResponse::default();
    for resource in &request.resources {
        let mut rr = AlterConfigsResourceResponse::default();
        rr.error_code = 0;
        rr.resource_type = resource.resource_type;
        rr.resource_name = resource.resource_name.clone();
        response.responses.push(rr);
    }
    Ok(response)
}
