//! WriteTxnMarkers handler (API Key 27) — US-004

use crate::error::Result;
use crate::storage::LogBackend;
use crate::transaction_state::TransactionManager;
use bytes::{Bytes, BytesMut};
use heimq_protocol::messages::write_txn_markers_request::WriteTxnMarkersRequest;
use heimq_protocol::messages::write_txn_markers_response::{
    WritableTxnMarkerPartitionResult, WritableTxnMarkerResult, WritableTxnMarkerTopicResult,
    WriteTxnMarkersResponse,
};
use heimq_protocol::messages::{ProducerId, TopicName};
use heimq_protocol::protocol::{Decodable, StrBytes};
use heimq_protocol::records::{
    Compression, Record, RecordBatchEncoder, RecordEncodeOptions, TimestampType,
};
use std::sync::Arc;
use tracing::{debug, warn};

pub fn handle(
    api_version: i16,
    body: &[u8],
    storage: &Arc<dyn LogBackend>,
    _transaction_manager: &Arc<TransactionManager>,
) -> Result<WriteTxnMarkersResponse> {
    let mut buf = Bytes::copy_from_slice(body);
    let request = match WriteTxnMarkersRequest::decode(&mut buf, api_version) {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, "Failed to decode WriteTxnMarkers request");
            return Ok(WriteTxnMarkersResponse::default());
        }
    };

    let mut response = WriteTxnMarkersResponse::default();

    for marker in &request.markers {
        let producer_id = marker.producer_id.0;
        let epoch = marker.producer_epoch;
        let committed = marker.transaction_result;

        debug!(producer_id, epoch, committed, "WriteTxnMarker");

        let mut marker_result = WritableTxnMarkerResult::default();
        marker_result.producer_id = ProducerId(producer_id);

        for topic in &marker.topics {
            let topic_name = topic.name.0.to_string();
            let mut topic_result = WritableTxnMarkerTopicResult::default();
            topic_result.name = TopicName(StrBytes::from_string(topic_name.clone()));

            for &partition in &topic.partition_indexes {
                let control_record = make_control_record(producer_id, epoch, committed);
                let mut buf = BytesMut::new();
                let error_code = match RecordBatchEncoder::encode(
                    &mut buf,
                    &[control_record],
                    &RecordEncodeOptions {
                        version: 2,
                        compression: Compression::None,
                    },
                ) {
                    Err(e) => {
                        warn!(error = %e, "Failed to encode control batch in WriteTxnMarkers");
                        5 // LEADER_NOT_AVAILABLE as generic error
                    }
                    Ok(_) => match storage.append(&topic_name, partition, &buf.freeze()) {
                        Ok(_) => 0,
                        Err(e) => {
                            warn!(error = %e, "Failed to append control batch in WriteTxnMarkers");
                            5
                        }
                    },
                };

                let mut partition_result = WritableTxnMarkerPartitionResult::default();
                partition_result.partition_index = partition;
                partition_result.error_code = error_code;
                topic_result.partitions.push(partition_result);
            }

            marker_result.topics.push(topic_result);
        }

        response.markers.push(marker_result);
    }

    Ok(response)
}

fn make_control_record(producer_id: i64, epoch: i16, committed: bool) -> Record {
    let key = if committed {
        vec![0x00, 0x01, 0x00, 0x00]
    } else {
        vec![0x00, 0x00, 0x00, 0x00]
    };
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
