//! IncrementalAlterConfigs request handler (API Key 44)
//!
//! heimq does not maintain mutable broker/topic config objects, so we return
//! success with no-op for every requested resource.

use crate::error::Result;
use bytes::Bytes;
use kafka_protocol::messages::incremental_alter_configs_request::IncrementalAlterConfigsRequest;
use kafka_protocol::messages::incremental_alter_configs_response::AlterConfigsResourceResponse;
use kafka_protocol::messages::IncrementalAlterConfigsResponse;
use kafka_protocol::protocol::Decodable;

pub fn handle(api_version: i16, body: &[u8]) -> Result<IncrementalAlterConfigsResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    let request = match IncrementalAlterConfigsRequest::decode(&mut buf, api_version) {
        Ok(r) => r,
        Err(_) => return Ok(IncrementalAlterConfigsResponse::default()),
    };

    let mut response = IncrementalAlterConfigsResponse::default();
    for resource in &request.resources {
        let mut rr = AlterConfigsResourceResponse::default();
        rr.error_code = 0;
        rr.resource_type = resource.resource_type;
        rr.resource_name = resource.resource_name.clone();
        response.responses.push(rr);
    }
    Ok(response)
}
