//! AddOffsetsToTxn handler (API Key 25) — US-004

use crate::error::Result;
use crate::transaction_state::TransactionManager;
use bytes::Bytes;
use kafka_protocol::messages::add_offsets_to_txn_request::AddOffsetsToTxnRequest;
use kafka_protocol::messages::add_offsets_to_txn_response::AddOffsetsToTxnResponse;
use kafka_protocol::protocol::Decodable;
use std::sync::Arc;
use tracing::debug;

pub fn handle(
    api_version: i16,
    body: &[u8],
    transaction_manager: &Arc<TransactionManager>,
) -> Result<AddOffsetsToTxnResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    let request = match AddOffsetsToTxnRequest::decode(&mut buf, api_version) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to decode AddOffsetsToTxn request");
            return Ok(AddOffsetsToTxnResponse::default());
        }
    };

    let txn_id = request.transactional_id.0.to_string();
    let producer_id = request.producer_id.0;
    let epoch = request.producer_epoch;
    let group_id = request.group_id.0.to_string();

    debug!(
        txn_id = %txn_id,
        producer_id,
        epoch,
        group_id = %group_id,
        "AddOffsetsToTxn"
    );

    let error_code = transaction_manager.add_offsets(&txn_id, producer_id, epoch, &group_id);

    let mut response = AddOffsetsToTxnResponse::default();
    response.error_code = error_code;
    Ok(response)
}
