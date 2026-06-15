//! Version-neutral produce append decisions.
//!
//! This module intentionally stays below Kafka request/response encoding. It
//! takes raw record-batch bytes plus storage and idempotence policy hooks, then
//! returns append outcomes that protocol handlers can map to wire responses.

use crate::error::HeimqError;
use crate::storage::{LogBackend, RecordBatchHeader};

/// Result of an idempotent sequence validation check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceDecision {
    /// Sequence is in-order and the append should proceed.
    Accept,
    /// Sequence repeats a previously accepted batch; report success without append.
    Duplicate,
    /// Sequence has a gap or otherwise violates ordering.
    OutOfOrder,
}

/// Hook used by protocol/front-end crates to supply idempotent producer state.
pub trait SequenceValidator {
    fn validate(
        &self,
        producer_id: i64,
        producer_epoch: i16,
        topic: &str,
        partition: i32,
        base_sequence: i32,
        record_count: i32,
    ) -> SequenceDecision;
}

/// Default validator for callers that do not track producer sequence state.
#[derive(Debug, Default)]
pub struct AcceptAllSequenceValidator;

impl SequenceValidator for AcceptAllSequenceValidator {
    fn validate(
        &self,
        _producer_id: i64,
        _producer_epoch: i16,
        _topic: &str,
        _partition: i32,
        _base_sequence: i32,
        _record_count: i32,
    ) -> SequenceDecision {
        SequenceDecision::Accept
    }
}

/// Inputs needed to decide and perform one partition append.
pub struct ProduceAppend<'a> {
    pub storage: &'a dyn LogBackend,
    pub topic: &'a str,
    pub partition: i32,
    pub records: &'a [u8],
    pub transactional_attempted: bool,
    pub sequence_validator: &'a dyn SequenceValidator,
}

/// Successful append decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProduceAppendOutcome {
    pub status: ProduceAppendStatus,
    pub header: Option<RecordBatchHeader>,
}

/// Produce append status. All statuses map to successful protocol responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProduceAppendStatus {
    Appended {
        base_offset: i64,
        high_watermark: i64,
    },
    Duplicate {
        base_offset: i64,
        high_watermark: i64,
    },
    Empty {
        high_watermark: i64,
    },
}

impl ProduceAppendStatus {
    pub fn base_offset(self) -> i64 {
        match self {
            ProduceAppendStatus::Appended { base_offset, .. }
            | ProduceAppendStatus::Duplicate { base_offset, .. } => base_offset,
            ProduceAppendStatus::Empty { high_watermark } => high_watermark,
        }
    }
}

/// Rejections that a protocol handler can map to its wire-level error codes.
#[derive(Debug)]
pub enum ProduceAppendError {
    TransactionsUnsupported,
    MessageTooLarge,
    OutOfOrderSequenceNumber,
    Storage(HeimqError),
}

impl ProduceAppendError {
    pub fn storage_error(&self) -> Option<&HeimqError> {
        match self {
            ProduceAppendError::Storage(err) => Some(err),
            _ => None,
        }
    }
}

/// Apply shared produce append semantics without depending on a concrete Kafka
/// request/response crate version.
pub fn append_records(cmd: ProduceAppend<'_>) -> Result<ProduceAppendOutcome, ProduceAppendError> {
    let caps = cmd.storage.capabilities();
    let header = RecordBatchHeader::peek(cmd.records);

    if cmd.transactional_attempted && !caps.transactions {
        return Err(ProduceAppendError::TransactionsUnsupported);
    }

    if cmd.records.is_empty() {
        return Ok(ProduceAppendOutcome {
            status: ProduceAppendStatus::Empty {
                high_watermark: cmd
                    .storage
                    .high_watermark(cmd.topic, cmd.partition)
                    .unwrap_or(0),
            },
            header,
        });
    }

    if cmd.records.len() > caps.max_batch_bytes || cmd.records.len() > caps.max_message_bytes {
        return Err(ProduceAppendError::MessageTooLarge);
    }

    if let Some(hdr) = header {
        if hdr.producer_id != -1 {
            match cmd.sequence_validator.validate(
                hdr.producer_id,
                hdr.producer_epoch,
                cmd.topic,
                cmd.partition,
                hdr.base_sequence,
                hdr.record_count,
            ) {
                SequenceDecision::Accept => {}
                SequenceDecision::Duplicate => {
                    let high_watermark = cmd
                        .storage
                        .high_watermark(cmd.topic, cmd.partition)
                        .unwrap_or(0);
                    return Ok(ProduceAppendOutcome {
                        status: ProduceAppendStatus::Duplicate {
                            base_offset: high_watermark.saturating_sub(i64::from(hdr.record_count)),
                            high_watermark,
                        },
                        header,
                    });
                }
                SequenceDecision::OutOfOrder => {
                    return Err(ProduceAppendError::OutOfOrderSequenceNumber);
                }
            }
        }
    }

    match cmd.storage.append(cmd.topic, cmd.partition, cmd.records) {
        Ok((base_offset, high_watermark)) => Ok(ProduceAppendOutcome {
            status: ProduceAppendStatus::Appended {
                base_offset,
                high_watermark,
            },
            header,
        }),
        Err(err) => Err(ProduceAppendError::Storage(err)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Result;
    use crate::storage::{BackendCapabilities, TopicLog};
    use parking_lot::Mutex;
    use std::sync::Arc;

    struct TestStorage {
        caps: BackendCapabilities,
        high_watermark: Mutex<i64>,
        appended: Mutex<Vec<Vec<u8>>>,
        fail_append: bool,
    }

    impl TestStorage {
        fn new() -> Self {
            Self {
                caps: BackendCapabilities::default(),
                high_watermark: Mutex::new(0),
                appended: Mutex::new(Vec::new()),
                fail_append: false,
            }
        }

        fn with_caps(mut self, caps: BackendCapabilities) -> Self {
            self.caps = caps;
            self
        }

        fn with_high_watermark(self, high_watermark: i64) -> Self {
            *self.high_watermark.lock() = high_watermark;
            self
        }

        fn with_fail_append(mut self) -> Self {
            self.fail_append = true;
            self
        }

        fn append_count(&self) -> usize {
            self.appended.lock().len()
        }
    }

    impl LogBackend for TestStorage {
        fn create_topic(&self, _name: &str, _num_partitions: i32) -> Result<Arc<dyn TopicLog>> {
            Err(HeimqError::Storage("not implemented".into()))
        }

        fn delete_topic(&self, _name: &str) -> Result<()> {
            Ok(())
        }

        fn list_topics(&self) -> Vec<String> {
            Vec::new()
        }

        fn topic(&self, _name: &str) -> Option<Arc<dyn TopicLog>> {
            None
        }

        fn capabilities(&self) -> &BackendCapabilities {
            &self.caps
        }

        fn get_or_create_topic(&self, _name: &str, _num_partitions: i32) -> Arc<dyn TopicLog> {
            panic!("not needed for produce core tests")
        }

        fn get_all_topic_metadata(&self) -> Vec<(String, i32)> {
            Vec::new()
        }

        fn default_num_partitions(&self) -> i32 {
            1
        }

        fn auto_create_topics(&self) -> bool {
            true
        }

        fn append(&self, _topic_name: &str, _partition: i32, records: &[u8]) -> Result<(i64, i64)> {
            if self.fail_append {
                return Err(HeimqError::Storage("append failed".into()));
            }
            let mut high_watermark = self.high_watermark.lock();
            let base_offset = *high_watermark;
            *high_watermark += 1;
            self.appended.lock().push(records.to_vec());
            Ok((base_offset, *high_watermark))
        }

        fn fetch(
            &self,
            _topic_name: &str,
            _partition: i32,
            _offset: i64,
            _max_bytes: i32,
        ) -> Result<(Vec<u8>, i64)> {
            Ok((Vec::new(), *self.high_watermark.lock()))
        }

        fn high_watermark(&self, _topic_name: &str, _partition: i32) -> Result<i64> {
            Ok(*self.high_watermark.lock())
        }

        fn log_start_offset(&self, _topic_name: &str, _partition: i32) -> Result<i64> {
            Ok(0)
        }
    }

    struct StaticSequenceValidator(SequenceDecision);

    impl SequenceValidator for StaticSequenceValidator {
        fn validate(
            &self,
            _producer_id: i64,
            _producer_epoch: i16,
            _topic: &str,
            _partition: i32,
            _base_sequence: i32,
            _record_count: i32,
        ) -> SequenceDecision {
            self.0
        }
    }

    fn append<'a>(
        storage: &'a dyn LogBackend,
        records: &'a [u8],
        sequence_validator: &'a dyn SequenceValidator,
    ) -> ProduceAppend<'a> {
        ProduceAppend {
            storage,
            topic: "events",
            partition: 0,
            records,
            transactional_attempted: false,
            sequence_validator,
        }
    }

    fn record_batch_header_bytes(
        record_count: i32,
        producer_id: i64,
        base_sequence: i32,
    ) -> Vec<u8> {
        let mut bytes = vec![0; 61];
        bytes[16] = 2;
        bytes[43..51].copy_from_slice(&producer_id.to_be_bytes());
        bytes[51..53].copy_from_slice(&0_i16.to_be_bytes());
        bytes[53..57].copy_from_slice(&base_sequence.to_be_bytes());
        bytes[57..61].copy_from_slice(&record_count.to_be_bytes());
        bytes
    }

    #[test]
    fn produce_append_core_appends_accepted_records() {
        let storage = TestStorage::new();
        let records = record_batch_header_bytes(2, -1, 0);
        let outcome =
            append_records(append(&storage, &records, &AcceptAllSequenceValidator)).unwrap();

        assert_eq!(
            outcome.status,
            ProduceAppendStatus::Appended {
                base_offset: 0,
                high_watermark: 1
            }
        );
        assert_eq!(storage.append_count(), 1);
        assert_eq!(outcome.header.unwrap().record_count, 2);
    }

    #[test]
    fn produce_append_core_rejects_batches_over_backend_limits() {
        let mut caps = BackendCapabilities::default();
        caps.max_message_bytes = 16;
        caps.max_batch_bytes = 16;
        let storage = TestStorage::new().with_caps(caps);
        let records = record_batch_header_bytes(1, -1, 0);

        let err =
            append_records(append(&storage, &records, &AcceptAllSequenceValidator)).unwrap_err();

        assert!(matches!(err, ProduceAppendError::MessageTooLarge));
        assert_eq!(storage.append_count(), 0);
    }

    #[test]
    fn produce_append_core_reports_duplicate_without_appending() {
        let storage = TestStorage::new().with_high_watermark(7);
        let records = record_batch_header_bytes(3, 22, 0);
        let validator = StaticSequenceValidator(SequenceDecision::Duplicate);

        let outcome = append_records(append(&storage, &records, &validator)).unwrap();

        assert_eq!(
            outcome.status,
            ProduceAppendStatus::Duplicate {
                base_offset: 4,
                high_watermark: 7
            }
        );
        assert_eq!(storage.append_count(), 0);
    }

    #[test]
    fn produce_append_core_rejects_out_of_order_sequence() {
        let storage = TestStorage::new();
        let records = record_batch_header_bytes(1, 22, 2);
        let validator = StaticSequenceValidator(SequenceDecision::OutOfOrder);

        let err = append_records(append(&storage, &records, &validator)).unwrap_err();

        assert!(matches!(err, ProduceAppendError::OutOfOrderSequenceNumber));
        assert_eq!(storage.append_count(), 0);
    }

    #[test]
    fn produce_append_core_rejects_transactional_write_when_backend_lacks_support() {
        let storage = TestStorage::new();
        let records = record_batch_header_bytes(1, -1, 0);
        let err = append_records(ProduceAppend {
            transactional_attempted: true,
            ..append(&storage, &records, &AcceptAllSequenceValidator)
        })
        .unwrap_err();

        assert!(matches!(err, ProduceAppendError::TransactionsUnsupported));
        assert_eq!(storage.append_count(), 0);
    }

    #[test]
    fn produce_append_core_returns_high_watermark_for_empty_records() {
        let storage = TestStorage::new().with_high_watermark(9);

        let outcome = append_records(append(&storage, &[], &AcceptAllSequenceValidator)).unwrap();

        assert_eq!(
            outcome.status,
            ProduceAppendStatus::Empty { high_watermark: 9 }
        );
        assert_eq!(storage.append_count(), 0);
    }

    #[test]
    fn produce_append_core_surfaces_storage_errors() {
        let storage = TestStorage::new().with_fail_append();
        let records = record_batch_header_bytes(1, -1, 0);

        let err =
            append_records(append(&storage, &records, &AcceptAllSequenceValidator)).unwrap_err();

        assert!(matches!(err, ProduceAppendError::Storage(_)));
    }
}
