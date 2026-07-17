//! Version-neutral produce append decisions.
//!
//! This module intentionally stays below Kafka request/response encoding. It
//! takes raw record-batch bytes plus storage and idempotence policy hooks, then
//! returns append outcomes that protocol handlers can map to wire responses.

use crate::error::HeimqError;
use crate::storage::{LogBackend, RecordBatchHeader, RetentionPolicy};
use crate::RequestContext;

/// Terminal outcome for a provisional sequence reservation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceCompletion {
    /// Storage completed the append successfully.
    Commit,
    /// Storage returned a definite error without committing the append.
    Rollback,
    /// The append future was dropped with an unknown storage outcome.
    Abandon,
}

/// A provisional sequence reservation that is committed only after storage
/// accepts the corresponding append.
///
/// Dropping an unfinished reservation abandons it rather than rolling it back:
/// a generic storage future may have committed before cancellation. Callers of
/// [`append_records_async`] must drive the future to terminal completion unless
/// their backend provides a stronger cancellation contract.
#[must_use = "a sequence reservation must be completed after the append terminates"]
pub struct SequenceReservation<'a> {
    completion: Option<Box<dyn FnOnce(SequenceCompletion) + Send + 'a>>,
}

impl<'a> SequenceReservation<'a> {
    /// Create a reservation whose callback receives its terminal outcome.
    pub fn new(completion: impl FnOnce(SequenceCompletion) + Send + 'a) -> Self {
        Self {
            completion: Some(Box::new(completion)),
        }
    }

    /// Commit the reserved sequence after the corresponding storage append
    /// has completed successfully.
    pub fn commit(mut self) {
        if let Some(completion) = self.completion.take() {
            completion(SequenceCompletion::Commit);
        }
    }

    /// Roll back the reserved sequence after storage returns a definite error.
    pub fn rollback(mut self) {
        if let Some(completion) = self.completion.take() {
            completion(SequenceCompletion::Rollback);
        }
    }
}

impl Drop for SequenceReservation<'_> {
    fn drop(&mut self) {
        if let Some(completion) = self.completion.take() {
            completion(SequenceCompletion::Abandon);
        }
    }
}

/// Result of an idempotent sequence validation check.
pub enum SequenceDecision<'a> {
    /// Sequence is in-order and reserved while the append is in flight.
    Accept(SequenceReservation<'a>),
    /// Sequence repeats a previously accepted batch; report success without append.
    Duplicate,
    /// Sequence has a gap or otherwise violates ordering.
    OutOfOrder,
    /// Another append for this producer-partition has not completed yet.
    InFlight,
}

/// Hook used by protocol/front-end crates to supply idempotent producer state.
pub trait SequenceValidator: Sync {
    fn validate<'a>(
        &'a self,
        producer_id: i64,
        producer_epoch: i16,
        topic: &str,
        partition: i32,
        base_sequence: i32,
        record_count: i32,
    ) -> SequenceDecision<'a>;
}

/// Default validator for callers that do not track producer sequence state.
#[derive(Debug, Default)]
pub struct AcceptAllSequenceValidator;

impl SequenceValidator for AcceptAllSequenceValidator {
    fn validate<'a>(
        &'a self,
        _producer_id: i64,
        _producer_epoch: i16,
        _topic: &str,
        _partition: i32,
        _base_sequence: i32,
        _record_count: i32,
    ) -> SequenceDecision<'a> {
        SequenceDecision::Accept(SequenceReservation::new(|_| {}))
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

enum AppendPlan<'a> {
    Append {
        header: Option<RecordBatchHeader>,
        sequence_reservation: Option<SequenceReservation<'a>>,
    },
    Complete(ProduceAppendOutcome),
}

/// Rejections that a protocol handler can map to its wire-level error codes.
#[derive(Debug)]
pub enum ProduceAppendError {
    TransactionsUnsupported,
    MessageTooLarge,
    OutOfOrderSequenceNumber,
    SequenceInFlight,
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
    let (header, sequence_reservation) = match plan_append(&cmd)? {
        AppendPlan::Append {
            header,
            sequence_reservation,
        } => (header, sequence_reservation),
        AppendPlan::Complete(outcome) => return Ok(outcome),
    };

    match cmd.storage.append_with_context_and_retention_policy(
        cmd.ctx,
        cmd.topic,
        cmd.partition,
        cmd.records,
        cmd.retention_policy,
    ) {
        Ok((base_offset, high_watermark)) => {
            if let Some(reservation) = sequence_reservation {
                reservation.commit();
            }
            Ok(ProduceAppendOutcome {
                status: ProduceAppendStatus::Appended {
                    base_offset,
                    high_watermark,
                },
                header,
            })
        }
        Err(err) => {
            if let Some(reservation) = sequence_reservation {
                reservation.rollback();
            }
            Err(ProduceAppendError::Storage(err))
        }
    }
}

/// Async variant of [`append_records`] for storage backends that can defer
/// commit completion without blocking the caller's worker thread.
///
/// # Cancellation
///
/// Once polled, this future must be driven to completion. Dropping it abandons
/// any idempotent sequence reservation because a generic [`LogBackend`] cannot
/// prove whether its append committed before cancellation. The affected
/// producer-partition remains fenced instead of risking a duplicate append.
pub async fn append_records_async(
    cmd: ProduceAppend<'_>,
) -> Result<ProduceAppendOutcome, ProduceAppendError> {
    let (header, sequence_reservation) = match plan_append(&cmd)? {
        AppendPlan::Append {
            header,
            sequence_reservation,
        } => (header, sequence_reservation),
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
        Ok((base_offset, high_watermark)) => {
            if let Some(reservation) = sequence_reservation {
                reservation.commit();
            }
            Ok(ProduceAppendOutcome {
                status: ProduceAppendStatus::Appended {
                    base_offset,
                    high_watermark,
                },
                header,
            })
        }
        Err(err) => {
            if let Some(reservation) = sequence_reservation {
                reservation.rollback();
            }
            Err(ProduceAppendError::Storage(err))
        }
    }
}

fn plan_append<'a>(cmd: &'a ProduceAppend<'a>) -> Result<AppendPlan<'a>, ProduceAppendError> {
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

    let mut sequence_reservation = None;
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
                SequenceDecision::Accept(reservation) => {
                    sequence_reservation = Some(reservation);
                }
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
                SequenceDecision::InFlight => {
                    return Err(ProduceAppendError::SequenceInFlight);
                }
            }
        }
    }

    Ok(AppendPlan::Append {
        header,
        sequence_reservation,
    })
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

    #[derive(Clone, Copy)]
    enum StaticSequenceDecision {
        Duplicate,
        OutOfOrder,
        InFlight,
    }

    struct StaticSequenceValidator(StaticSequenceDecision);

    impl SequenceValidator for StaticSequenceValidator {
        fn validate<'a>(
            &'a self,
            _producer_id: i64,
            _producer_epoch: i16,
            _topic: &str,
            _partition: i32,
            _base_sequence: i32,
            _record_count: i32,
        ) -> SequenceDecision<'a> {
            match self.0 {
                StaticSequenceDecision::Duplicate => SequenceDecision::Duplicate,
                StaticSequenceDecision::OutOfOrder => SequenceDecision::OutOfOrder,
                StaticSequenceDecision::InFlight => SequenceDecision::InFlight,
            }
        }
    }

    struct TrackingSequenceValidator {
        completions: Arc<Mutex<Vec<SequenceCompletion>>>,
    }

    impl SequenceValidator for TrackingSequenceValidator {
        fn validate<'a>(
            &'a self,
            _producer_id: i64,
            _producer_epoch: i16,
            _topic: &str,
            _partition: i32,
            _base_sequence: i32,
            _record_count: i32,
        ) -> SequenceDecision<'a> {
            SequenceDecision::Accept(SequenceReservation::new(move |completion| {
                self.completions.lock().push(completion);
            }))
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
        let validator = StaticSequenceValidator(StaticSequenceDecision::Duplicate);

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
        let validator = StaticSequenceValidator(StaticSequenceDecision::OutOfOrder);

        let err = append_records(append(&storage, &records, &validator)).unwrap_err();

        assert!(matches!(err, ProduceAppendError::OutOfOrderSequenceNumber));
        assert_eq!(storage.append_count(), 0);
    }

    #[test]
    fn produce_append_core_reports_in_flight_sequence_without_appending() {
        let storage = TestStorage::new();
        let records = record_batch_header_bytes(1, 22, 1);
        let validator = StaticSequenceValidator(StaticSequenceDecision::InFlight);

        let err = append_records(append(&storage, &records, &validator)).unwrap_err();

        assert!(matches!(err, ProduceAppendError::SequenceInFlight));
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

    #[test]
    fn produce_append_core_rolls_back_sequence_on_storage_error() {
        let storage = TestStorage::new().with_fail_append();
        let records = record_batch_header_bytes(1, 22, 0);
        let completions = Arc::new(Mutex::new(Vec::new()));
        let validator = TrackingSequenceValidator {
            completions: completions.clone(),
        };

        let err = append_records(append(&storage, &records, &validator)).unwrap_err();

        assert!(matches!(err, ProduceAppendError::Storage(_)));
        assert_eq!(*completions.lock(), vec![SequenceCompletion::Rollback]);
    }

    #[test]
    fn produce_append_core_commits_sequence_after_storage_success() {
        let storage = TestStorage::new();
        let records = record_batch_header_bytes(1, 22, 0);
        let completions = Arc::new(Mutex::new(Vec::new()));
        let validator = TrackingSequenceValidator {
            completions: completions.clone(),
        };

        append_records(append(&storage, &records, &validator)).expect("append succeeds");

        assert_eq!(*completions.lock(), vec![SequenceCompletion::Commit]);
    }

    #[test]
    fn cancelled_async_append_abandons_sequence_with_unknown_outcome() {
        let state = Arc::new(DeferredAppendState::new());
        let storage = DeferredStorage::new(state);
        let records = record_batch_header_bytes(1, 22, 0);
        let completions = Arc::new(Mutex::new(Vec::new()));
        let validator = TrackingSequenceValidator {
            completions: completions.clone(),
        };
        let mut future = Box::pin(append_records_async(append(&storage, &records, &validator)));
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);

        assert!(matches!(future.as_mut().poll(&mut cx), Poll::Pending));
        drop(future);

        assert_eq!(*completions.lock(), vec![SequenceCompletion::Abandon]);
    }
}
