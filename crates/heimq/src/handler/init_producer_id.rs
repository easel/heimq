//! InitProducerId handler (API key 22) — US-003-AC1

use crate::error::Result;
use crate::producer_state::ProducerStateManager;
use bytes::Bytes;
use kafka_protocol::messages::init_producer_id_request::InitProducerIdRequest;
use kafka_protocol::messages::init_producer_id_response::InitProducerIdResponse;
use kafka_protocol::messages::ProducerId;
use kafka_protocol::protocol::Decodable;
use tracing::debug;

// @covers US-003-AC1
pub fn handle(api_version: i16, body: &[u8]) -> Result<InitProducerIdResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    let request = match InitProducerIdRequest::decode(&mut buf, api_version) {
        Ok(r) => r,
        Err(_) => InitProducerIdRequest::default(),
    };

    // Transactions (transactional_id) are US-004; reject here with
    // TRANSACTIONAL_ID_AUTHORIZATION_FAILED (53) to tell the client this
    // broker does not support that path yet.
    // Treat empty string the same as null (non-transactional) — the Kafka
    // default struct uses Some("") but a null wire value is more correct.
    let is_transactional = request
        .transactional_id
        .as_ref()
        .map(|id| !id.0.is_empty())
        .unwrap_or(false);
    if is_transactional {
        debug!("Rejecting transactional InitProducerId (US-004 not yet implemented)");
        let mut resp = InitProducerIdResponse::default();
        resp.error_code = 53;
        resp.producer_id = ProducerId(-1);
        resp.producer_epoch = -1;
        return Ok(resp);
    }

    let pid = ProducerStateManager::allocate_producer_id();
    debug!(producer_id = pid, "Allocated producer ID");

    let mut resp = InitProducerIdResponse::default();
    resp.error_code = 0;
    resp.producer_id = ProducerId(pid);
    resp.producer_epoch = 0;
    Ok(resp)
}
