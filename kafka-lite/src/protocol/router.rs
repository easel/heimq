//! Request routing

use crate::config::Config;
use crate::consumer_group::ConsumerGroupManager;
use crate::error::Result;
use crate::handler::*;
use crate::protocol::{decode_request, encode_response, RequestHeader};
use crate::storage::Storage;
use bytes::{Bytes, BytesMut};
use std::sync::Arc;
use tracing::{debug, error, warn};

/// Routes requests to appropriate handlers
pub struct Router {
    storage: Arc<Storage>,
    consumer_groups: Arc<ConsumerGroupManager>,
    config: Arc<Config>,
}

impl Router {
    pub fn new(
        storage: Arc<Storage>,
        consumer_groups: Arc<ConsumerGroupManager>,
        config: Arc<Config>,
    ) -> Self {
        Self {
            storage,
            consumer_groups,
            config,
        }
    }

    /// Route a request and return the response
    pub fn route(&self, data: &[u8]) -> Result<Bytes> {
        let (header, body) = decode_request(data)?;

        debug!(
            api_key = header.api_key,
            api_version = header.api_version,
            correlation_id = header.correlation_id,
            "Routing request"
        );

        let response = match header.api_key {
            0 => self.handle_produce(&header, &body),
            1 => self.handle_fetch(&header, &body),
            2 => self.handle_list_offsets(&header, &body),
            3 => self.handle_metadata(&header, &body),
            8 => self.handle_offset_commit(&header, &body),
            9 => self.handle_offset_fetch(&header, &body),
            10 => self.handle_find_coordinator(&header, &body),
            11 => self.handle_join_group(&header, &body),
            12 => self.handle_heartbeat(&header, &body),
            13 => self.handle_leave_group(&header, &body),
            14 => self.handle_sync_group(&header, &body),
            18 => self.handle_api_versions(&header, &body),
            19 => self.handle_create_topics(&header, &body),
            20 => self.handle_delete_topics(&header, &body),
            _ => {
                warn!(api_key = header.api_key, "Unsupported API");
                self.handle_unsupported(&header)
            }
        };

        response
    }

    fn handle_api_versions(&self, header: &RequestHeader, _body: &[u8]) -> Result<Bytes> {
        let response = api_versions::handle(header.api_version);
        Ok(encode_response(
            header.correlation_id,
            header.api_version,
            &response,
        )?)
    }

    fn handle_metadata(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        let response = metadata::handle(
            header.api_version,
            body,
            &self.storage,
            &self.config,
        )?;
        Ok(encode_response(
            header.correlation_id,
            header.api_version,
            &response,
        )?)
    }

    fn handle_produce(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        let response = produce::handle(header.api_version, body, &self.storage)?;
        Ok(encode_response(
            header.correlation_id,
            header.api_version,
            &response,
        )?)
    }

    fn handle_fetch(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        let response = fetch::handle(header.api_version, body, &self.storage)?;
        Ok(encode_response(
            header.correlation_id,
            header.api_version,
            &response,
        )?)
    }

    fn handle_list_offsets(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        let response = list_offsets::handle(header.api_version, body, &self.storage)?;
        Ok(encode_response(
            header.correlation_id,
            header.api_version,
            &response,
        )?)
    }

    fn handle_create_topics(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        let response = create_topics::handle(header.api_version, body, &self.storage)?;
        Ok(encode_response(
            header.correlation_id,
            header.api_version,
            &response,
        )?)
    }

    fn handle_delete_topics(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        let response = delete_topics::handle(header.api_version, body, &self.storage)?;
        Ok(encode_response(
            header.correlation_id,
            header.api_version,
            &response,
        )?)
    }

    fn handle_find_coordinator(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        let response = find_coordinator::handle(header.api_version, body, &self.config)?;
        Ok(encode_response(
            header.correlation_id,
            header.api_version,
            &response,
        )?)
    }

    fn handle_join_group(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        let response = join_group::handle(header.api_version, body, &self.consumer_groups)?;
        Ok(encode_response(
            header.correlation_id,
            header.api_version,
            &response,
        )?)
    }

    fn handle_sync_group(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        let response = sync_group::handle(header.api_version, body, &self.consumer_groups)?;
        Ok(encode_response(
            header.correlation_id,
            header.api_version,
            &response,
        )?)
    }

    fn handle_heartbeat(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        let response = heartbeat::handle(header.api_version, body, &self.consumer_groups)?;
        Ok(encode_response(
            header.correlation_id,
            header.api_version,
            &response,
        )?)
    }

    fn handle_leave_group(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        let response = leave_group::handle(header.api_version, body, &self.consumer_groups)?;
        Ok(encode_response(
            header.correlation_id,
            header.api_version,
            &response,
        )?)
    }

    fn handle_offset_commit(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        let response = offset_commit::handle(header.api_version, body, &self.consumer_groups)?;
        Ok(encode_response(
            header.correlation_id,
            header.api_version,
            &response,
        )?)
    }

    fn handle_offset_fetch(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        let response = offset_fetch::handle(header.api_version, body, &self.consumer_groups)?;
        Ok(encode_response(
            header.correlation_id,
            header.api_version,
            &response,
        )?)
    }

    fn handle_unsupported(&self, header: &RequestHeader) -> Result<Bytes> {
        // Return an error response
        use kafka_protocol::messages::ApiVersionsResponse;
        let response = ApiVersionsResponse::default();
        Ok(encode_response(
            header.correlation_id,
            0,
            &response,
        )?)
    }
}
