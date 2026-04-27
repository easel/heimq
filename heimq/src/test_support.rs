//! Shared helpers for unit and integration tests.

use crate::config::Config;
use crate::consumer_group::ConsumerGroupManager;
use crate::storage::{LogBackend, MemoryLog};
use bytes::{Bytes, BytesMut};
use kafka_protocol::protocol::Encodable;
use kafka_protocol::records::{Compression, Record, RecordBatchEncoder, RecordEncodeOptions};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU16, AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Once;
use std::time::Duration;

// Port counter for test isolation - starts high to avoid conflicts
static PORT_COUNTER: AtomicU16 = AtomicU16::new(19092);

// Topic counter for unique topic names
static TOPIC_COUNTER: AtomicUsize = AtomicUsize::new(1);

// Group counter for unique consumer group names
static GROUP_COUNTER: AtomicUsize = AtomicUsize::new(1);

/// Get the next available port for test isolation
pub fn next_port() -> u16 {
    PORT_COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// Generate a unique topic name with the given prefix
pub fn unique_topic(prefix: &str) -> String {
    let id = TOPIC_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("{}-{}", prefix, id)
}

/// Generate a unique consumer group name with the given prefix
pub fn unique_group(prefix: &str) -> String {
    let id = GROUP_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("{}-{}", prefix, id)
}

/// Test server wrapper that manages a heimq process lifecycle
pub struct TestServer {
    pub child: Child,
    pub port: u16,
}

impl TestServer {
    /// Start a test server with default settings (auto_create_topics = true)
    pub fn start() -> Self {
        Self::start_with_options(true, 1)
    }

    /// Start a test server with configurable auto_create_topics setting
    pub fn start_with_auto_create(auto_create: bool) -> Self {
        Self::start_with_options(auto_create, 1)
    }

    /// Start a test server with a custom default_partitions for auto-created topics
    pub fn start_with_partitions(auto_create: bool, default_partitions: i32) -> Self {
        Self::start_with_options(auto_create, default_partitions)
    }

    fn start_with_options(auto_create: bool, default_partitions: i32) -> Self {
        let port = next_port();
        let auto_value = if auto_create { "true" } else { "false" };

        // Prefer CARGO_BIN_EXE_heimq (set by cargo test), fall back to target paths
        let binary = std::env::var("CARGO_BIN_EXE_heimq")
            .ok()
            .or_else(|| {
                let debug = std::path::Path::new("./target/debug/heimq");
                let release = std::path::Path::new("./target/release/heimq");
                if debug.exists() {
                    Some(debug.to_string_lossy().to_string())
                } else if release.exists() {
                    Some(release.to_string_lossy().to_string())
                } else {
                    None
                }
            })
            .expect("heimq binary not found - run 'cargo build' first");

        let child = Command::new(&binary)
            .args(["--port", &port.to_string(), "--memory-only"])
            .env("HEIMQ_AUTO_CREATE_TOPICS", auto_value)
            .env("HEIMQ_DEFAULT_PARTITIONS", default_partitions.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to start heimq");

        let server = Self { child, port };
        server.wait_for_ready();
        server
    }

    fn wait_for_ready(&self) {
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while std::time::Instant::now() < deadline {
            if TcpStream::connect(("127.0.0.1", self.port)).is_ok() {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        panic!(
            "heimq server failed to start within 10 seconds on port {}",
            self.port
        );
    }

    /// Get the bootstrap servers string for Kafka clients
    pub fn bootstrap_servers(&self) -> String {
        format!("127.0.0.1:{}", self.port)  // Use IPv4 to avoid IPv6 resolution issues
    }

    /// Get the hosts list for legacy Kafka clients
    pub fn hosts(&self) -> Vec<String> {
        vec![self.bootstrap_servers()]
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

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
        create_topics: Vec::new(),
        storage_log: "memory://".to_string(),
        storage_offsets: "memory://".to_string(),
        storage_groups: "memory://".to_string(),
    })
}

pub fn test_storage(auto_create: bool) -> Arc<dyn LogBackend> {
    Arc::new(MemoryLog::new(test_config(auto_create)))
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

/// One diverging field between heimq and Redpanda for a given workload step.
/// Serializes to JSONL; written to target/parity/<timestamp>-<workload>.jsonl.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct DiffRecord {
    pub workload: String,
    pub step: u32,
    pub field: String,
    pub heimq_value: serde_json::Value,
    pub redpanda_value: serde_json::Value,
    /// "value_mismatch" | "missing_in_heimq" | "extra_in_heimq"
    pub divergence: String,
    /// exemption id string, or null
    pub exemption: Option<String>,
}

#[test]
fn diff_record_round_trips_through_json() {
    let d = DiffRecord {
        workload: "test".to_string(),
        step: 0,
        field: "record.value".to_string(),
        heimq_value: serde_json::json!(null),
        redpanda_value: serde_json::json!(null),
        divergence: "value_mismatch".to_string(),
        exemption: None,
    };
    let s = serde_json::to_string(&d).expect("DiffRecord must be JSON-serializable");
    let back: DiffRecord =
        serde_json::from_str(&s).expect("DiffRecord must round-trip from JSON");
    assert_eq!(back.divergence, "value_mismatch");
    assert_eq!(back.field, "record.value");
}
