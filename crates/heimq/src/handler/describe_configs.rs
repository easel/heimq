//! DescribeConfigs request handler (API Key 32)
//!
//! Returns the supported topic config keys (see `config_store`), reflecting any
//! values set via AlterConfigs / IncrementalAlterConfigs. Keys that have been
//! explicitly overridden are reported with the DYNAMIC_TOPIC_CONFIG source; the
//! rest report compiled-in defaults. A non-empty set is always returned so
//! clients that require at least one entry (e.g. sarama ClusterAdmin) succeed.

use crate::config_store::ConfigStore;
use crate::error::Result;
use bytes::Bytes;
use kafka_protocol::messages::describe_configs_request::DescribeConfigsRequest;
use kafka_protocol::messages::describe_configs_response::{
    DescribeConfigsResourceResult, DescribeConfigsResult,
};
use kafka_protocol::messages::DescribeConfigsResponse;
use kafka_protocol::protocol::{Decodable, StrBytes};

// CONFIG_SOURCE_DYNAMIC_TOPIC_CONFIG (1) — set via AlterConfigs.
const CONFIG_SOURCE_DYNAMIC_TOPIC: i8 = 1;
// CONFIG_SOURCE_STATIC_BROKER_CONFIG (5) — compiled-in default.
const CONFIG_SOURCE_DEFAULT: i8 = 5;

const RESOURCE_TYPE_TOPIC: i8 = 2;

/// Build the config result list for a topic from the effective config (defaults
/// merged with any stored overrides).
fn topic_configs(
    api_version: i16,
    topic: &str,
    config_store: &ConfigStore,
) -> Vec<DescribeConfigsResourceResult> {
    config_store
        .effective(topic)
        .into_iter()
        .map(|(name, value, overridden)| {
            let mut cfg = DescribeConfigsResourceResult::default();
            cfg.name = StrBytes::from_static_str(name);
            cfg.value = Some(StrBytes::from_string(value));
            cfg.read_only = false;
            // v0 uses is_default; v1+ replaced it with config_source.
            if api_version == 0 {
                cfg.is_default = !overridden;
            } else {
                cfg.config_source = if overridden {
                    CONFIG_SOURCE_DYNAMIC_TOPIC
                } else {
                    CONFIG_SOURCE_DEFAULT
                };
            }
            cfg.is_sensitive = false;
            cfg
        })
        .collect()
}

pub fn handle(
    api_version: i16,
    body: &[u8],
    config_store: &ConfigStore,
) -> Result<DescribeConfigsResponse> {
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
        // Topic configs reflect the store; broker/other resources fall back to
        // the same supported-key set with defaults so the response is non-empty.
        let topic = if res.resource_type == RESOURCE_TYPE_TOPIC {
            res.resource_name.as_str()
        } else {
            ""
        };
        result.configs = topic_configs(api_version, topic, config_store);
        response.results.push(result);
    }
    Ok(response)
}
