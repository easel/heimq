use bytes::Bytes;
use heimq_broker::produce::AcceptAllSequenceValidator;
use heimq_broker::storage::{ClusterView, LogBackend, RecordBatchView};
use heimq_broker::RequestContext;
use heimq_handlers::config_store::ConfigStore;
use heimq_handlers::producer_state::ProducerStateManager;
use heimq_handlers::transaction_state::TransactionManager;
use heimq_handlers::{api_versions, codec, init_producer_id, metadata, produce};
use heimq_protocol::messages::{ApiVersionsResponse, InitProducerIdResponse, MetadataResponse};
use std::sync::Arc;

type MetadataHandler = fn(
    i16,
    &[u8],
    &Arc<dyn LogBackend>,
    &dyn ClusterView,
) -> heimq_handlers::error::Result<MetadataResponse>;
type ApiVersionsHandler = fn(i16, &[(i16, i16, i16)]) -> ApiVersionsResponse;
type InitProducerIdHandler = fn(
    i16,
    &[u8],
    &Arc<TransactionManager>,
) -> heimq_handlers::error::Result<InitProducerIdResponse>;

#[test]
fn codec_embedder_entrypoints_decode_and_encode() {
    let request = [
        0x00, 0x12, // api_key = ApiVersions
        0x00, 0x00, // api_version = 0
        0x00, 0x00, 0x00, 0x2A, // correlation_id = 42
        0xFF, 0xFF, // client_id = null
    ];

    let (header, body) = codec::decode_request(&request).expect("decode borrowed request");
    assert_eq!(header.api_key, 18);
    assert_eq!(header.correlation_id, 42);
    assert!(body.is_empty());

    let (owned_header, owned_body) = codec::decode_request_bytes(Bytes::copy_from_slice(&request))
        .expect("decode owned request");
    assert_eq!(owned_header.api_version, 0);
    assert!(owned_body.is_empty());

    let response = ApiVersionsResponse::default();
    let framed = codec::encode_response(42, 18, 0, &response).expect("encode framed response");
    assert!(framed.len() >= 8);

    let body = codec::encode_response_body(42, 0, &response).expect("encode response body");
    assert!(body.len() >= 4);
}

#[test]
fn typed_handler_entrypoints_keep_their_embedder_signatures() {
    let _: MetadataHandler = metadata::handle;
    let _: ApiVersionsHandler = api_versions::handle;
    let _: InitProducerIdHandler = init_producer_id::handle;
}

#[test]
fn record_batch_decode_all_embedder_entrypoint_accepts_empty_input() {
    let decoded = RecordBatchView::decode_all(&[]).expect("empty record set decodes");
    assert!(decoded.is_empty());
}

#[allow(dead_code)]
fn compile_against_async_produce_embedder_signature(storage: Option<Arc<dyn LogBackend>>) {
    if let Some(storage) = storage {
        let producer_state = ProducerStateManager::new();
        let transaction_manager = TransactionManager::new();
        let ctx = RequestContext::ANONYMOUS;
        let config_store = Arc::new(ConfigStore::new());

        let _future = produce::handle_async_with_context_and_config_store(
            11,
            &[],
            &storage,
            &producer_state,
            &transaction_manager,
            &ctx,
            &config_store,
            604_800_000,
        );
        let _injected_future =
            produce::handle_async_with_context_and_config_store_and_sequence_validator(
                11,
                &[],
                &storage,
                &AcceptAllSequenceValidator,
                &transaction_manager,
                &ctx,
                &config_store,
                604_800_000,
            );
    }
}
