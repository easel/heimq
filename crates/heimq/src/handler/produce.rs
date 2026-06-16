//! Produce request handler (API Key 0)

use crate::error::{ErrorCode, Result};
use crate::producer_state::{ProducerStateManager, SequenceCheck};
use crate::storage::LogBackend;
use crate::transaction_state::TransactionManager;
use bytes::Bytes;
use heimq_broker::produce::{
    append_records, append_records_async, ProduceAppend, ProduceAppendError, ProduceAppendStatus,
    SequenceDecision, SequenceValidator,
};
use kafka_protocol::messages::produce_request::ProduceRequest;
use kafka_protocol::messages::produce_response::{PartitionProduceResponse, TopicProduceResponse};
use kafka_protocol::messages::{ProduceResponse, TopicName};
use kafka_protocol::protocol::{Decodable, StrBytes};
use std::sync::Arc;
use tracing::{debug, warn};

struct ProducerStateSequenceValidator<'a> {
    producer_state: &'a ProducerStateManager,
}

impl SequenceValidator for ProducerStateSequenceValidator<'_> {
    fn validate(
        &self,
        producer_id: i64,
        producer_epoch: i16,
        topic: &str,
        partition: i32,
        base_sequence: i32,
        record_count: i32,
    ) -> SequenceDecision {
        match self.producer_state.validate(
            producer_id,
            producer_epoch,
            topic,
            partition,
            base_sequence,
            record_count,
        ) {
            SequenceCheck::Accept => SequenceDecision::Accept,
            SequenceCheck::Duplicate => SequenceDecision::Duplicate,
            SequenceCheck::OutOfOrder => SequenceDecision::OutOfOrder,
        }
    }
}

/// Handle Produce request
pub fn handle(
    api_version: i16,
    body: &[u8],
    storage: &Arc<dyn LogBackend>,
    producer_state: &Arc<ProducerStateManager>,
    transaction_manager: &Arc<TransactionManager>,
) -> Result<ProduceResponse> {
    debug!(
        api_version = api_version,
        body_len = body.len(),
        "Handling produce request"
    );

    // Use kafka-protocol's decoder for proper version-aware parsing
    let mut buf = Bytes::copy_from_slice(body);
    let request = match ProduceRequest::decode(&mut buf, api_version) {
        Ok(req) => req,
        Err(e) => {
            warn!(error = %e, "Failed to decode produce request");
            return Ok(ProduceResponse::default());
        }
    };

    let mut response = ProduceResponse::default();

    let caps = storage.capabilities();
    let txn_id: Option<String> = request
        .transactional_id
        .as_ref()
        .map(|id| id.0.to_string())
        .filter(|s| !s.is_empty());
    let transactional_attempted = txn_id.is_some();
    let max_message_bytes = caps.max_message_bytes;
    let max_batch_bytes = caps.max_batch_bytes;

    let sequence_validator = ProducerStateSequenceValidator {
        producer_state: producer_state.as_ref(),
    };

    for topic_data in request.topic_data {
        let topic_name = topic_data.name.0.to_string();
        debug!(topic = %topic_name, partitions = topic_data.partition_data.len(), "Processing topic");

        let mut topic_response = TopicProduceResponse::default();
        topic_response.name = TopicName(StrBytes::from_string(topic_name.clone()));

        for partition_data in topic_data.partition_data {
            let partition = partition_data.index;
            let mut partition_response = PartitionProduceResponse::default();
            partition_response.index = partition;

            if let Some(records) = partition_data.records {
                match append_records(ProduceAppend {
                    storage: storage.as_ref(),
                    topic: &topic_name,
                    partition,
                    records: records.as_ref(),
                    transactional_attempted,
                    sequence_validator: &sequence_validator,
                }) {
                    Ok(outcome) => {
                        if matches!(outcome.status, ProduceAppendStatus::Duplicate { .. }) {
                            if let Some(hdr) = outcome.header {
                                debug!(
                                    topic = %topic_name,
                                    partition,
                                    producer_id = hdr.producer_id,
                                    base_seq = hdr.base_sequence,
                                    "Duplicate sequence; returning successful de-duplication"
                                );
                            }
                        } else if matches!(outcome.status, ProduceAppendStatus::Appended { .. }) {
                            debug!(
                                topic = %topic_name,
                                partition,
                                base_offset = outcome.status.base_offset(),
                                "Produced records"
                            );
                        }

                        if let Some(hdr) = outcome.header {
                            if hdr.producer_id != -1
                                && hdr.is_transactional
                                && matches!(outcome.status, ProduceAppendStatus::Appended { .. })
                            {
                                transaction_manager.record_produce(
                                    txn_id.as_deref(),
                                    &topic_name,
                                    partition,
                                    outcome.status.base_offset(),
                                    hdr.producer_id,
                                );
                            }
                        }

                        partition_response.base_offset = outcome.status.base_offset();
                        partition_response.error_code = 0;
                        if !matches!(outcome.status, ProduceAppendStatus::Empty { .. }) {
                            partition_response.log_append_time_ms =
                                chrono::Utc::now().timestamp_millis();
                        }
                    }
                    Err(ProduceAppendError::TransactionsUnsupported) => {
                        warn!(topic = %topic_name, partition, "Rejecting transactional produce: backend does not support transactions");
                        partition_response.error_code = 48; // INVALID_TXN_STATE
                        partition_response.base_offset = -1;
                    }
                    Err(ProduceAppendError::MessageTooLarge) => {
                        warn!(
                            topic = %topic_name,
                            partition,
                            len = records.len(),
                            max_message_bytes,
                            max_batch_bytes,
                            "Rejecting oversized produce batch"
                        );
                        partition_response.error_code = 10; // MESSAGE_TOO_LARGE
                        partition_response.base_offset = -1;
                    }
                    Err(ProduceAppendError::OutOfOrderSequenceNumber) => {
                        warn!(
                            topic = %topic_name,
                            partition,
                            "Out-of-order sequence"
                        );
                        partition_response.error_code = 45; // OUT_OF_ORDER_SEQUENCE_NUMBER
                        partition_response.base_offset = -1;
                    }
                    Err(err @ ProduceAppendError::Storage(_)) => {
                        if let Some(storage_err) = err.storage_error() {
                            warn!(error = %storage_err, topic = %topic_name, partition, "Failed to append records");
                            partition_response.error_code = storage_err.to_error_code();
                        } else {
                            partition_response.error_code = -1;
                        }
                        partition_response.base_offset = -1;
                    }
                }
            } else {
                // Null records
                partition_response.error_code = 87; // INVALID_RECORD
                partition_response.base_offset = -1;
            }

            topic_response.partition_responses.push(partition_response);
        }

        response.responses.push(topic_response);
    }

    Ok(response)
}

/// Async Produce handler that awaits the backend append future instead of
/// blocking the caller's async worker thread inside the storage implementation.
pub async fn handle_async(
    api_version: i16,
    body: &[u8],
    storage: &Arc<dyn LogBackend>,
    producer_state: &Arc<ProducerStateManager>,
    transaction_manager: &Arc<TransactionManager>,
) -> Result<ProduceResponse> {
    debug!(
        api_version = api_version,
        body_len = body.len(),
        "Handling async produce request"
    );

    let mut buf = Bytes::copy_from_slice(body);
    let request = match ProduceRequest::decode(&mut buf, api_version) {
        Ok(req) => req,
        Err(e) => {
            warn!(error = %e, "Failed to decode produce request");
            return Ok(ProduceResponse::default());
        }
    };

    let mut response = ProduceResponse::default();

    let caps = storage.capabilities();
    let txn_id: Option<String> = request
        .transactional_id
        .as_ref()
        .map(|id| id.0.to_string())
        .filter(|s| !s.is_empty());
    let transactional_attempted = txn_id.is_some();
    let max_message_bytes = caps.max_message_bytes;
    let max_batch_bytes = caps.max_batch_bytes;

    let sequence_validator = ProducerStateSequenceValidator {
        producer_state: producer_state.as_ref(),
    };

    for topic_data in request.topic_data {
        let topic_name = topic_data.name.0.to_string();
        debug!(topic = %topic_name, partitions = topic_data.partition_data.len(), "Processing topic");

        let mut topic_response = TopicProduceResponse::default();
        topic_response.name = TopicName(StrBytes::from_string(topic_name.clone()));

        for partition_data in topic_data.partition_data {
            let partition = partition_data.index;
            let mut partition_response = PartitionProduceResponse::default();
            partition_response.index = partition;

            if let Some(records) = partition_data.records {
                match append_records_async(ProduceAppend {
                    storage: storage.as_ref(),
                    topic: &topic_name,
                    partition,
                    records: records.as_ref(),
                    transactional_attempted,
                    sequence_validator: &sequence_validator,
                })
                .await
                {
                    Ok(outcome) => {
                        if matches!(outcome.status, ProduceAppendStatus::Duplicate { .. }) {
                            if let Some(hdr) = outcome.header {
                                debug!(
                                    topic = %topic_name,
                                    partition,
                                    producer_id = hdr.producer_id,
                                    base_seq = hdr.base_sequence,
                                    "Duplicate sequence; returning successful de-duplication"
                                );
                            }
                        } else if matches!(outcome.status, ProduceAppendStatus::Appended { .. }) {
                            debug!(
                                topic = %topic_name,
                                partition,
                                base_offset = outcome.status.base_offset(),
                                "Produced records"
                            );
                        }

                        if let Some(hdr) = outcome.header {
                            if hdr.producer_id != -1
                                && hdr.is_transactional
                                && matches!(outcome.status, ProduceAppendStatus::Appended { .. })
                            {
                                transaction_manager.record_produce(
                                    txn_id.as_deref(),
                                    &topic_name,
                                    partition,
                                    outcome.status.base_offset(),
                                    hdr.producer_id,
                                );
                            }
                        }

                        partition_response.base_offset = outcome.status.base_offset();
                        partition_response.error_code = 0;
                        if !matches!(outcome.status, ProduceAppendStatus::Empty { .. }) {
                            partition_response.log_append_time_ms =
                                chrono::Utc::now().timestamp_millis();
                        }
                    }
                    Err(ProduceAppendError::TransactionsUnsupported) => {
                        warn!(topic = %topic_name, partition, "Rejecting transactional produce: backend does not support transactions");
                        partition_response.error_code = 48;
                        partition_response.base_offset = -1;
                    }
                    Err(ProduceAppendError::MessageTooLarge) => {
                        warn!(
                            topic = %topic_name,
                            partition,
                            len = records.len(),
                            max_message_bytes,
                            max_batch_bytes,
                            "Rejecting oversized produce batch"
                        );
                        partition_response.error_code = 10;
                        partition_response.base_offset = -1;
                    }
                    Err(ProduceAppendError::OutOfOrderSequenceNumber) => {
                        warn!(
                            topic = %topic_name,
                            partition,
                            "Out-of-order sequence"
                        );
                        partition_response.error_code = 45;
                        partition_response.base_offset = -1;
                    }
                    Err(err @ ProduceAppendError::Storage(_)) => {
                        if let Some(storage_err) = err.storage_error() {
                            warn!(error = %storage_err, topic = %topic_name, partition, "Failed to append records");
                            partition_response.error_code = storage_err.to_error_code();
                        } else {
                            partition_response.error_code = -1;
                        }
                        partition_response.base_offset = -1;
                    }
                }
            } else {
                partition_response.error_code = 87;
                partition_response.base_offset = -1;
            }

            topic_response.partition_responses.push(partition_response);
        }

        response.responses.push(topic_response);
    }

    Ok(response)
}
