//! EndTxn handler (API Key 26) — US-004

use crate::error::Result;
use crate::storage::{LogBackend, OffsetStore};
use crate::transaction_state::TransactionManager;
use bytes::{Bytes, BytesMut};
use kafka_protocol::messages::end_txn_request::EndTxnRequest;
use kafka_protocol::messages::end_txn_response::EndTxnResponse;
use kafka_protocol::protocol::Decodable;
use kafka_protocol::records::{
    Compression, Record, RecordBatchEncoder, RecordEncodeOptions, TimestampType,
};
use std::sync::Arc;
use tracing::{debug, warn};

pub fn handle(
    api_version: i16,
    body: &[u8],
    storage: &Arc<dyn LogBackend>,
    offset_store: &Arc<dyn OffsetStore>,
    transaction_manager: &Arc<TransactionManager>,
) -> Result<EndTxnResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    let request = match EndTxnRequest::decode(&mut buf, api_version) {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, "Failed to decode EndTxn request");
            return Ok(EndTxnResponse::default());
        }
    };

    let txn_id = request.transactional_id.0.to_string();
    let producer_id = request.producer_id.0;
    let epoch = request.producer_epoch;
    let committed = request.committed;

    debug!(
        txn_id = %txn_id,
        producer_id,
        epoch,
        committed,
        "EndTxn"
    );

    let (error_code, affected, pending_offsets) =
        transaction_manager.end_txn_with_offsets(&txn_id, producer_id, epoch, committed);

    if error_code != 0 {
        let mut response = EndTxnResponse::default();
        response.error_code = error_code;
        return Ok(response);
    }

    // Write control batches to the log for each affected partition
    for (topic, partition, _first_offset, pid) in &affected {
        let control_record = make_control_record(*pid, epoch, committed);
        let mut buf = BytesMut::new();
        if let Err(e) = RecordBatchEncoder::encode(
            &mut buf,
            &[control_record],
            &RecordEncodeOptions {
                version: 2,
                compression: Compression::None,
            },
        ) {
            warn!(error = %e, topic = %topic, partition, "Failed to encode control batch");
            continue;
        }
        if let Err(e) = storage.append(topic, *partition, &buf.freeze()) {
            warn!(error = %e, topic = %topic, partition, "Failed to append control batch");
        }
    }

    // If committed, apply pending transactional offsets to offset store
    if committed {
        for ((topic, partition, group_id), (offset, metadata)) in pending_offsets {
            if let Err(e) = offset_store.commit(&group_id, &topic, partition, offset, -1, metadata)
            {
                warn!(
                    error = %e,
                    group_id = %group_id,
                    topic = %topic,
                    partition,
                    "Failed to apply transactional offset"
                );
            }
        }
    }

    let mut response = EndTxnResponse::default();
    response.error_code = 0;
    Ok(response)
}

fn make_control_record(producer_id: i64, epoch: i16, committed: bool) -> Record {
    // Control batch key: [version(2)] [type(2)] where type=1 for commit, 0 for abort
    let key = if committed {
        vec![0x00, 0x01, 0x00, 0x00]
    } else {
        vec![0x00, 0x00, 0x00, 0x00]
    };
    // Control batch value: [version(2)] [coordinator_epoch(4)]
    let value = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

    Record {
        transactional: true,
        control: true,
        partition_leader_epoch: -1,
        producer_id,
        producer_epoch: epoch,
        timestamp_type: TimestampType::Creation,
        timestamp: chrono::Utc::now().timestamp_millis(),
        sequence: -1,
        offset: 0,
        key: Some(Bytes::from(key)),
        value: Some(Bytes::from(value)),
        headers: Default::default(),
    }
}
