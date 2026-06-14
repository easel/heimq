//! AlterConfigs request handler (API Key 33)
//!
//! For TOPIC resources, supported config keys (see `config_store`) are stored
//! and reflected by DescribeConfigs; unsupported keys are rejected with
//! INVALID_CONFIG (40) rather than silently accepted. Non-topic resources keep
//! the historical no-op success.

use crate::config_store::ConfigStore;
use crate::error::Result;
use bytes::Bytes;
use kafka_protocol::messages::alter_configs_request::AlterConfigsRequest;
use kafka_protocol::messages::alter_configs_response::AlterConfigsResourceResponse;
use kafka_protocol::messages::AlterConfigsResponse;
use kafka_protocol::protocol::Decodable;

const RESOURCE_TYPE_TOPIC: i8 = 2;

pub fn handle(
    api_version: i16,
    body: &[u8],
    config_store: &ConfigStore,
) -> Result<AlterConfigsResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    let request = match AlterConfigsRequest::decode(&mut buf, api_version) {
        Ok(r) => r,
        Err(_) => return Ok(AlterConfigsResponse::default()),
    };

    let mut response = AlterConfigsResponse::default();
    for resource in &request.resources {
        let mut rr = AlterConfigsResourceResponse::default();
        rr.resource_type = resource.resource_type;
        rr.resource_name = resource.resource_name.clone();

        if resource.resource_type == RESOURCE_TYPE_TOPIC {
            let configs: Vec<(String, Option<String>)> = resource
                .configs
                .iter()
                .map(|c| (c.name.to_string(), c.value.as_ref().map(|v| v.to_string())))
                .collect();
            match config_store.alter_full(resource.resource_name.as_str(), &configs) {
                Ok(()) => rr.error_code = 0,
                Err(code) => rr.error_code = code,
            }
        } else {
            // Broker/other resource types: unchanged no-op success.
            rr.error_code = 0;
        }
        response.responses.push(rr);
    }
    Ok(response)
}
