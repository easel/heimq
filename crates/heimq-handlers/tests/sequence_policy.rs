use bytes::BytesMut;
use heimq_broker::produce::AcceptAllSequenceValidator;
use heimq_broker::storage::{BackendCapabilities, LogBackend, TopicLog};
use heimq_broker::{HeimqError, RequestContext, Result as BrokerResult};
use heimq_handlers::config_store::ConfigStore;
use heimq_handlers::produce;
use heimq_handlers::producer_state::ProducerStateManager;
use heimq_handlers::transaction_state::TransactionManager;
use heimq_protocol::messages::produce_request::{
    PartitionProduceData, ProduceRequest, TopicProduceData,
};
use heimq_protocol::messages::{ProduceResponse, TopicName};
use heimq_protocol::protocol::{Encodable, StrBytes};
use heimq_protocol::records::{
    Compression, Record, RecordBatchEncoder, RecordEncodeOptions, TimestampType,
};
use parking_lot::Mutex;
use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

struct RecordingBackend {
    capabilities: BackendCapabilities,
    batches: Mutex<Vec<Vec<u8>>>,
}

impl RecordingBackend {
    fn new() -> Self {
        Self {
            capabilities: BackendCapabilities::default(),
            batches: Mutex::new(Vec::new()),
        }
    }

    fn append_count(&self) -> usize {
        self.batches.lock().len()
    }
}

impl LogBackend for RecordingBackend {
    fn create_topic(&self, _name: &str, _num_partitions: i32) -> BrokerResult<Arc<dyn TopicLog>> {
        Err(HeimqError::Storage(
            "not needed by sequence policy tests".into(),
        ))
    }

    fn delete_topic(&self, _name: &str) -> BrokerResult<()> {
        Ok(())
    }

    fn list_topics(&self) -> Vec<String> {
        Vec::new()
    }

    fn topic(&self, _name: &str) -> Option<Arc<dyn TopicLog>> {
        None
    }

    fn capabilities(&self) -> &BackendCapabilities {
        &self.capabilities
    }

    fn get_or_create_topic(&self, _name: &str, _num_partitions: i32) -> Arc<dyn TopicLog> {
        panic!("not needed by sequence policy tests")
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
        records: &[u8],
    ) -> BrokerResult<(i64, i64)> {
        let mut batches = self.batches.lock();
        let base_offset = i64::try_from(batches.len()).expect("test append count fits i64");
        batches.push(records.to_vec());
        Ok((base_offset, base_offset + 1))
    }

    fn fetch(
        &self,
        _topic_name: &str,
        _partition: i32,
        _offset: i64,
        _max_bytes: i32,
    ) -> BrokerResult<(Vec<u8>, i64)> {
        Ok((Vec::new(), self.high_watermark("events", 0)?))
    }

    fn high_watermark(&self, _topic_name: &str, _partition: i32) -> BrokerResult<i64> {
        i64::try_from(self.append_count())
            .map_err(|error| HeimqError::Storage(format!("append count overflow: {error}")))
    }

    fn log_start_offset(&self, _topic_name: &str, _partition: i32) -> BrokerResult<i64> {
        Ok(0)
    }
}

fn idempotent_batch(sequence: i32, producer_epoch: i16) -> bytes::Bytes {
    let offset = i64::from(sequence);
    let record = Record {
        transactional: false,
        control: false,
        partition_leader_epoch: 0,
        producer_id: 41,
        producer_epoch,
        timestamp_type: TimestampType::Creation,
        timestamp: offset,
        sequence,
        offset,
        key: Some(format!("key-{sequence}").into()),
        value: Some(format!("value-{sequence}").into()),
        headers: Default::default(),
    };
    let mut encoded = BytesMut::new();
    RecordBatchEncoder::encode(
        &mut encoded,
        &[record],
        &RecordEncodeOptions {
            version: 2,
            compression: Compression::None,
        },
    )
    .expect("encode idempotent record batch");
    encoded.freeze()
}

fn request_body(sequence: i32, producer_epoch: i16) -> Vec<u8> {
    let partition = PartitionProduceData::default()
        .with_index(0)
        .with_records(Some(idempotent_batch(sequence, producer_epoch)));
    let topic = TopicProduceData::default()
        .with_name(TopicName(StrBytes::from_static_str("events")))
        .with_partition_data(vec![partition]);
    let request = ProduceRequest::default()
        .with_acks(1)
        .with_timeout_ms(1_000)
        .with_topic_data(vec![topic]);
    let mut body = BytesMut::new();
    request.encode(&mut body, 8).expect("encode Produce body");
    body.to_vec()
}

fn run_ready<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("recording backend must complete synchronously"),
    }
}

fn injected_produce(
    body: &[u8],
    storage: &Arc<dyn LogBackend>,
    transaction_manager: &Arc<TransactionManager>,
    config_store: &Arc<ConfigStore>,
) -> ProduceResponse {
    run_ready(
        produce::handle_async_with_context_and_config_store_and_sequence_validator(
            8,
            body,
            storage,
            &AcceptAllSequenceValidator,
            transaction_manager,
            &RequestContext::ANONYMOUS,
            config_store,
            60_000,
        ),
    )
    .expect("injected Produce handler succeeds")
}

fn native_produce(
    body: &[u8],
    storage: &Arc<dyn LogBackend>,
    producer_state: &Arc<ProducerStateManager>,
    transaction_manager: &Arc<TransactionManager>,
    config_store: &Arc<ConfigStore>,
) -> ProduceResponse {
    run_ready(produce::handle_async_with_context_and_config_store(
        8,
        body,
        storage,
        producer_state,
        transaction_manager,
        &RequestContext::ANONYMOUS,
        config_store,
        60_000,
    ))
    .expect("native Produce handler succeeds")
}

fn error_code(response: &ProduceResponse) -> i16 {
    response.responses[0].partition_responses[0].error_code
}

#[test]
fn injected_accept_all_appends_duplicate_gap_and_out_of_order_batches() {
    let backend = Arc::new(RecordingBackend::new());
    let storage: Arc<dyn LogBackend> = backend.clone();
    let transactions = TransactionManager::new();
    let configs = Arc::new(ConfigStore::new());

    for sequence in [0, 0, 5, 2] {
        let response = injected_produce(
            &request_body(sequence, 3),
            &storage,
            &transactions,
            &configs,
        );
        assert_eq!(error_code(&response), 0, "sequence {sequence} must append");
    }

    assert_eq!(backend.append_count(), 4);
}

#[test]
fn native_entrypoint_still_deduplicates_and_fences_sequences() {
    let backend = Arc::new(RecordingBackend::new());
    let storage: Arc<dyn LogBackend> = backend.clone();
    let producer_state = ProducerStateManager::new();
    let transactions = TransactionManager::new();
    let configs = Arc::new(ConfigStore::new());

    assert_eq!(
        error_code(&native_produce(
            &request_body(0, 3),
            &storage,
            &producer_state,
            &transactions,
            &configs,
        )),
        0
    );
    assert_eq!(backend.append_count(), 1);

    assert_eq!(
        error_code(&native_produce(
            &request_body(0, 3),
            &storage,
            &producer_state,
            &transactions,
            &configs,
        )),
        0,
        "a committed duplicate remains a successful no-op"
    );
    assert_eq!(backend.append_count(), 1);

    for body in [request_body(5, 3), request_body(0, 2)] {
        assert_eq!(
            error_code(&native_produce(
                &body,
                &storage,
                &producer_state,
                &transactions,
                &configs,
            )),
            45,
            "a sequence gap or fenced epoch must be rejected"
        );
    }
    assert_eq!(backend.append_count(), 1);
}
