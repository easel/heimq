//! Shared helpers for unit tests.

use crate::config::Config;
use crate::consumer_group::ConsumerGroupManager;
use crate::storage::Storage;
use bytes::{Bytes, BytesMut};
use kafka_protocol::protocol::Encodable;
use kafka_protocol::records::{Compression, Record, RecordBatchEncoder, RecordEncodeOptions};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Once;

pub fn test_config(auto_create: bool) -> Arc<Config> {
    Arc::new(Config {
        host: "127.0.0.1".to_string(),
        port: 9092,
        data_dir: PathBuf::from("/tmp/heimq-test"),
        memory_only: true,
        segment_size: 1024 * 1024,
        retention_ms: 60_000,
        default_partitions: 1,
        auto_create_topics: auto_create,
        broker_id: 0,
        cluster_id: "test-cluster".to_string(),
        metrics: false,
        metrics_port: 9093,
    })
}

pub fn test_storage(auto_create: bool) -> Arc<Storage> {
    Arc::new(Storage::new(test_config(auto_create)))
}

pub fn test_consumer_groups(config: Arc<Config>) -> Arc<ConsumerGroupManager> {
    Arc::new(ConsumerGroupManager::new(config))
}

pub fn encode_body<R: Encodable>(request: &R, api_version: i16) -> Vec<u8> {
    let mut buf = BytesMut::new();
    request.encode(&mut buf, api_version).expect("encode request body");
    buf.to_vec()
}

pub fn encode_record_batch(records: &[Record]) -> Bytes {
    let mut encoded = BytesMut::new();
    RecordBatchEncoder::encode(
        &mut encoded,
        records,
        &RecordEncodeOptions {
            version: 2,
            compression: Compression::None,
        },
    )
    .expect("encode record batch");
    encoded.freeze()
}

pub fn init_tracing() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let _ = tracing_subscriber::fmt()
            .with_env_filter("heimq=trace")
            .with_test_writer()
            .try_init();
    });
}
