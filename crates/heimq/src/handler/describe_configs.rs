//! DescribeConfigs request handler (API Key 32)
//!
//! Returns a minimal but non-empty config set for each requested resource.
//! Reports the standard topic configs with static/default source so clients
//! that require at least one entry (e.g. sarama ClusterAdmin.DescribeConfig)
//! succeed without a full config store backend.

use crate::error::Result;
use bytes::Bytes;
use kafka_protocol::messages::describe_configs_request::DescribeConfigsRequest;
use kafka_protocol::messages::describe_configs_response::{
    DescribeConfigsResourceResult, DescribeConfigsResult,
};
use kafka_protocol::messages::DescribeConfigsResponse;
use kafka_protocol::protocol::{Decodable, StrBytes};

// CONFIG_SOURCE_STATIC_BROKER_CONFIG (5) — compiled-in default.
const CONFIG_SOURCE_DEFAULT: i8 = 5;

fn default_topic_configs(api_version: i16) -> Vec<DescribeConfigsResourceResult> {
    let entries: &[(&str, &str)] = &[
        ("cleanup.policy", "delete"),
        ("retention.ms", "604800000"),
        ("segment.ms", "604800000"),
        ("compression.type", "producer"),
        ("min.insync.replicas", "1"),
    ];
    entries
        .iter()
        .map(|(name, val)| {
            let mut cfg = DescribeConfigsResourceResult::default();
            cfg.name = StrBytes::from_static_str(name);
            cfg.value = Some(StrBytes::from_static_str(val));
            cfg.read_only = false;
            // v0 uses is_default; v1+ replaced it with config_source.
            if api_version == 0 {
                cfg.is_default = true;
            } else {
                cfg.config_source = CONFIG_SOURCE_DEFAULT;
            }
            cfg.is_sensitive = false;
            cfg
        })
        .collect()
}

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
        // Resource type 2 = TOPIC; return default configs for topics.
        // For broker/other resources return the same defaults to avoid empty responses.
        result.configs = default_topic_configs(api_version);
        response.results.push(result);
    }
    Ok(response)
}
