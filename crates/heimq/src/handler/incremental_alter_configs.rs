//! IncrementalAlterConfigs request handler (API Key 44)
//!
//! For TOPIC resources, SET/DELETE of supported config keys (see `config_store`)
//! are applied and reflected by DescribeConfigs; unsupported keys or operations
//! are rejected with INVALID_CONFIG (40). Non-topic resources keep the historical
//! no-op success.

use crate::config_store::{ConfigStore, IncrementalOp};
use crate::error::Result;
use bytes::Bytes;
use kafka_protocol::messages::incremental_alter_configs_request::IncrementalAlterConfigsRequest;
use kafka_protocol::messages::incremental_alter_configs_response::AlterConfigsResourceResponse;
use kafka_protocol::messages::IncrementalAlterConfigsResponse;
use kafka_protocol::protocol::Decodable;

const RESOURCE_TYPE_TOPIC: i8 = 2;

pub fn handle(
    api_version: i16,
    body: &[u8],
    config_store: &ConfigStore,
) -> Result<IncrementalAlterConfigsResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    let request = match IncrementalAlterConfigsRequest::decode(&mut buf, api_version) {
        Ok(r) => r,
        Err(_) => return Ok(IncrementalAlterConfigsResponse::default()),
    };

    let mut response = IncrementalAlterConfigsResponse::default();
    for resource in &request.resources {
        let mut rr = AlterConfigsResourceResponse::default();
        rr.resource_type = resource.resource_type;
        rr.resource_name = resource.resource_name.clone();

        if resource.resource_type == RESOURCE_TYPE_TOPIC {
            let names: Vec<String> = resource.configs.iter().map(|c| c.name.to_string()).collect();
            let values: Vec<Option<String>> = resource
                .configs
                .iter()
                .map(|c| c.value.as_ref().map(|v| v.to_string()))
                .collect();
            let ops: Vec<IncrementalOp> = resource
                .configs
                .iter()
                .enumerate()
                .map(|(i, c)| IncrementalOp {
                    key: names[i].as_str(),
                    op: c.config_operation,
                    value: values[i].as_deref(),
                })
                .collect();
            match config_store.alter_incremental(resource.resource_name.as_str(), &ops) {
                Ok(()) => rr.error_code = 0,
                Err(code) => rr.error_code = code,
            }
        } else {
            rr.error_code = 0;
        }
        response.responses.push(rr);
    }
    Ok(response)
}
