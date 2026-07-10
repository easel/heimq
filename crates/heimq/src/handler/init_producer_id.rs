//! InitProducerId handler (API key 22) — US-003-AC1, US-004-AC1

use crate::error::Result;
use crate::producer_state::ProducerStateManager;
use crate::transaction_state::TransactionManager;
use bytes::Bytes;
use heimq_protocol::messages::init_producer_id_request::InitProducerIdRequest;
use heimq_protocol::messages::init_producer_id_response::InitProducerIdResponse;
use heimq_protocol::messages::ProducerId;
use heimq_protocol::protocol::Decodable;
use std::sync::Arc;
use tracing::debug;

// @covers US-003-AC1 US-004-AC1
pub fn handle(
    api_version: i16,
    body: &[u8],
    transaction_manager: &Arc<TransactionManager>,
) -> Result<InitProducerIdResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    let request: InitProducerIdRequest =
        InitProducerIdRequest::decode(&mut buf, api_version).unwrap_or_default();

    // If transactional_id is non-empty, delegate to TransactionManager (US-004).
    // Treat empty string same as null (non-transactional).
    let is_transactional = request
        .transactional_id
        .as_ref()
        .map(|id| !id.0.is_empty())
        .unwrap_or(false);

    if is_transactional {
        let txn_id = request.transactional_id.as_ref().unwrap().0.to_string();
        let mut resp = InitProducerIdResponse::default();

        match transaction_manager.init_transactional_producer(&txn_id) {
            Ok((pid, epoch)) => {
                debug!(producer_id = pid, epoch, txn_id = %txn_id, "Allocated transactional producer ID");
                resp.error_code = 0;
                resp.producer_id = ProducerId(pid);
                resp.producer_epoch = epoch;
            }
            Err(code) => {
                // A transaction for this id is still in flight; it has been aborted
                // and the caller must retry. Kafka reports no id alongside the error.
                debug!(error_code = code, txn_id = %txn_id, "InitProducerId deferred");
                resp.error_code = code;
                resp.producer_id = ProducerId(-1);
                resp.producer_epoch = -1;
            }
        }
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
