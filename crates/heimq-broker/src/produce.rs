//! Version-neutral produce append decisions.
//!
//! This module intentionally stays below Kafka request/response encoding. It
//! takes raw record-batch bytes plus storage and idempotence policy hooks, then
//! returns append outcomes that protocol handlers can map to wire responses.

use crate::error::HeimqError;
use crate::storage::{LogBackend, RecordBatchHeader, RetentionPolicy};
use crate::RequestContext;

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
pub trait SequenceValidator: Sync {
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
    pub ctx: &'a RequestContext,
    pub storage: &'a dyn LogBackend,
    pub topic: &'a str,
    pub partition: i32,
    pub records: &'a [u8],
    pub transactional_attempted: bool,
    pub sequence_validator: &'a dyn SequenceValidator,
    pub retention_policy: Option<RetentionPolicy>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppendPlan {
    Append { header: Option<RecordBatchHeader> },
    Complete(ProduceAppendOutcome),
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
    let header = match plan_append(&cmd)? {
        AppendPlan::Append { header } => header,
        AppendPlan::Complete(outcome) => return Ok(outcome),
    };

    match cmd.storage.append_with_context_and_retention_policy(
        cmd.ctx,
        cmd.topic,
        cmd.partition,
        cmd.records,
        cmd.retention_policy,
    ) {
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

/// Async variant of [`append_records`] for storage backends that can defer
/// commit completion without blocking the caller's worker thread.
pub async fn append_records_async(
    cmd: ProduceAppend<'_>,
) -> Result<ProduceAppendOutcome, ProduceAppendError> {
    let header = match plan_append(&cmd)? {
        AppendPlan::Append { header } => header,
        AppendPlan::Complete(outcome) => return Ok(outcome),
    };

    match cmd
        .storage
        .append_async_with_context_and_retention_policy(
            cmd.ctx,
            cmd.topic,
            cmd.partition,
            cmd.records,
            cmd.retention_policy,
        )
        .await
    {
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

fn plan_append(cmd: &ProduceAppend<'_>) -> Result<AppendPlan, ProduceAppendError> {
    let caps = cmd.storage.capabilities();
    let header = RecordBatchHeader::peek(cmd.records);

    if cmd.transactional_attempted && !caps.transactions {
        return Err(ProduceAppendError::TransactionsUnsupported);
    }

    if cmd.records.is_empty() {
        return Ok(AppendPlan::Complete(ProduceAppendOutcome {
            status: ProduceAppendStatus::Empty {
                high_watermark: cmd
                    .storage
                    .high_watermark_with_context(cmd.ctx, cmd.topic, cmd.partition)
                    .unwrap_or(0),
            },
            header,
        }));
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
                        .high_watermark_with_context(cmd.ctx, cmd.topic, cmd.partition)
                        .unwrap_or(0);
                    return Ok(AppendPlan::Complete(ProduceAppendOutcome {
                        status: ProduceAppendStatus::Duplicate {
                            base_offset: high_watermark.saturating_sub(i64::from(hdr.record_count)),
                            high_watermark,
                        },
                        header,
                    }));
                }
                SequenceDecision::OutOfOrder => {
                    return Err(ProduceAppendError::OutOfOrderSequenceNumber);
                }
            }
        }
    }

    Ok(AppendPlan::Append { header })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Result;
    use crate::storage::{AppendFuture, BackendCapabilities, TopicLog};
    use parking_lot::Mutex;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

    struct TestStorage {
        caps: BackendCapabilities,
        high_watermark: Mutex<i64>,
        appended: Mutex<Vec<Vec<u8>>>,
        seen_contexts: Mutex<Vec<RequestContext>>,
        fail_append: bool,
    }

    struct DeferredAppendState {
        complete: AtomicBool,
        sync_append_count: AtomicUsize,
        async_poll_count: AtomicUsize,
    }

    impl DeferredAppendState {
        fn new() -> Self {
            Self {
                complete: AtomicBool::new(false),
                sync_append_count: AtomicUsize::new(0),
                async_poll_count: AtomicUsize::new(0),
            }
        }
    }

    struct DeferredStorage {
        caps: BackendCapabilities,
        high_watermark: Arc<Mutex<i64>>,
        state: Arc<DeferredAppendState>,
    }

    impl DeferredStorage {
        fn new(state: Arc<DeferredAppendState>) -> Self {
            Self {
                caps: BackendCapabilities::default(),
                high_watermark: Arc::new(Mutex::new(0)),
                state,
            }
        }
    }

    struct DeferredAppendFuture {
        state: Arc<DeferredAppendState>,
        high_watermark: Arc<Mutex<i64>>,
    }

    impl Future for DeferredAppendFuture {
        type Output = Result<(i64, i64)>;

        fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
            self.state.async_poll_count.fetch_add(1, Ordering::SeqCst);
            if !self.state.complete.load(Ordering::SeqCst) {
                return Poll::Pending;
            }
            let mut high_watermark = self.high_watermark.lock();
            let base_offset = *high_watermark;
            *high_watermark += 1;
            Poll::Ready(Ok((base_offset, *high_watermark)))
        }
    }

    impl LogBackend for DeferredStorage {
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

        fn append(
            &self,
            _topic_name: &str,
            _partition: i32,
            _records: &[u8],
        ) -> Result<(i64, i64)> {
            self.state.sync_append_count.fetch_add(1, Ordering::SeqCst);
            Err(HeimqError::Storage("sync append should not be used".into()))
        }

        fn append_async<'a>(
            &'a self,
            _topic_name: &'a str,
            _partition: i32,
            _records: &'a [u8],
        ) -> AppendFuture<'a> {
            Box::pin(DeferredAppendFuture {
                state: self.state.clone(),
                high_watermark: self.high_watermark.clone(),
            })
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

    impl TestStorage {
        fn new() -> Self {
            Self {
                caps: BackendCapabilities::default(),
                high_watermark: Mutex::new(0),
                appended: Mutex::new(Vec::new()),
                seen_contexts: Mutex::new(Vec::new()),
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

        fn seen_contexts(&self) -> Vec<RequestContext> {
            self.seen_contexts.lock().clone()
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
            self.append_with_context(
                &RequestContext::anonymous(),
                _topic_name,
                _partition,
                records,
            )
        }

        fn append_with_context(
            &self,
            ctx: &RequestContext,
            _topic_name: &str,
            _partition: i32,
            records: &[u8],
        ) -> Result<(i64, i64)> {
            if self.fail_append {
                return Err(HeimqError::Storage("append failed".into()));
            }
            let mut high_watermark = self.high_watermark.lock();
            let base_offset = *high_watermark;
            *high_watermark += 1;
            self.seen_contexts.lock().push(ctx.clone());
            self.appended.lock().push(records.to_vec());
            Ok((base_offset, *high_watermark))
        }

        fn append_with_context_and_retention_policy(
            &self,
            ctx: &RequestContext,
            topic_name: &str,
            partition: i32,
            records: &[u8],
            _retention: Option<RetentionPolicy>,
        ) -> Result<(i64, i64)> {
            self.append_with_context(ctx, topic_name, partition, records)
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
            ctx: &RequestContext::ANONYMOUS,
            storage,
            topic: "events",
            partition: 0,
            records,
            transactional_attempted: false,
            sequence_validator,
            retention_policy: None,
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

    fn noop_waker() -> Waker {
        unsafe fn clone(_: *const ()) -> RawWaker {
            noop_raw_waker()
        }
        unsafe fn wake(_: *const ()) {}
        unsafe fn wake_by_ref(_: *const ()) {}
        unsafe fn drop(_: *const ()) {}
        fn noop_raw_waker() -> RawWaker {
            RawWaker::new(
                std::ptr::null(),
                &RawWakerVTable::new(clone, wake, wake_by_ref, drop),
            )
        }
        unsafe { Waker::from_raw(noop_raw_waker()) }
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
    // @covers US-016-AC4
    // @covers US-017-AC1
    fn produce_append_core_threads_request_context_to_backend() {
        let storage = TestStorage::new();
        let records = record_batch_header_bytes(1, -1, 0);
        let ctx = RequestContext::new(
            Some("principal-a".to_string()),
            Some("tenant-a".to_string()),
            Some("client-a".to_string()),
        );

        let outcome = append_records(ProduceAppend {
            ctx: &ctx,
            ..append(&storage, &records, &AcceptAllSequenceValidator)
        })
        .unwrap();

        assert!(matches!(
            outcome.status,
            ProduceAppendStatus::Appended { .. }
        ));
        assert_eq!(storage.seen_contexts(), vec![ctx]);
    }

    #[test]
    // @covers US-017-AC2
    fn produce_append_async_waits_on_deferred_backend_without_sync_append() {
        let state = Arc::new(DeferredAppendState::new());
        let storage = DeferredStorage::new(state.clone());
        let records = record_batch_header_bytes(1, -1, 0);
        let mut future = Box::pin(append_records_async(append(
            &storage,
            &records,
            &AcceptAllSequenceValidator,
        )));
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);

        assert!(matches!(future.as_mut().poll(&mut cx), Poll::Pending));
        assert_eq!(state.sync_append_count.load(Ordering::SeqCst), 0);
        assert_eq!(state.async_poll_count.load(Ordering::SeqCst), 1);

        state.complete.store(true, Ordering::SeqCst);
        let outcome = match future.as_mut().poll(&mut cx) {
            Poll::Ready(Ok(outcome)) => outcome,
            other => panic!("expected ready outcome, got {other:?}"),
        };

        assert_eq!(
            outcome.status,
            ProduceAppendStatus::Appended {
                base_offset: 0,
                high_watermark: 1
            }
        );
        assert_eq!(state.sync_append_count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn produce_append_core_rejects_batches_over_backend_limits() {
        let caps = BackendCapabilities {
            max_message_bytes: 16,
            max_batch_bytes: 16,
            ..BackendCapabilities::default()
        };
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
