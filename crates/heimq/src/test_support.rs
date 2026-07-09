//! Shared helpers for unit and integration tests.

use crate::config::Config;
use crate::consumer_group::ConsumerGroupManager;
use crate::storage::{LogBackend, MemoryLog};
use bytes::{Bytes, BytesMut};
use heimq_protocol::protocol::Encodable;
use heimq_protocol::records::{Compression, Record, RecordBatchEncoder, RecordEncodeOptions};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Once;
use std::time::Duration;

// Topic counter for unique topic names
static TOPIC_COUNTER: AtomicUsize = AtomicUsize::new(1);

// Group counter for unique consumer group names
static GROUP_COUNTER: AtomicUsize = AtomicUsize::new(1);

/// Bind to port 0 and let the OS pick an available port, then immediately
/// release the socket so the child process can bind to the same port.
/// This avoids TOCTOU races, but the window is small and far better than
/// a fixed-counter scheme that re-uses ports from lingering processes.
pub fn next_port() -> u16 {
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind to port 0");
    listener.local_addr().expect("no local address").port()
}

/// Generate a unique topic name with the given prefix
pub fn unique_topic(prefix: &str) -> String {
    let id = TOPIC_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("{prefix}-{id}")
}

/// Generate a unique consumer group name with the given prefix
pub fn unique_group(prefix: &str) -> String {
    let id = GROUP_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("{prefix}-{id}")
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

    /// Start a test server with bounded in-memory retention settings.
    pub fn start_with_memory_bounds(retention_ms: u64, max_memory_bytes: u64) -> Self {
        Self::start_with_runtime_options(true, 1, retention_ms, max_memory_bytes)
    }

    fn start_with_options(auto_create: bool, default_partitions: i32) -> Self {
        Self::start_with_runtime_options(auto_create, default_partitions, 60_000, 0)
    }

    fn start_with_runtime_options(
        auto_create: bool,
        default_partitions: i32,
        retention_ms: u64,
        max_memory_bytes: u64,
    ) -> Self {
        let port = next_port();
        let auto_value = if auto_create { "true" } else { "false" };

        // Prefer CARGO_BIN_EXE_heimq (set by cargo test), fall back to target paths.
        // When the binary env var is not set (can happen in OrbStack environments where
        // macOS/Linux path mappings differ), use CARGO_MANIFEST_DIR to find the workspace
        // root (this crate lives at <workspace>/crates/heimq, so go up two levels).
        let binary = std::env::var("CARGO_BIN_EXE_heimq")
            .ok()
            .or_else(|| {
                let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").ok()?;
                let workspace_root = std::path::Path::new(&manifest_dir)
                    .join("../..")
                    .canonicalize()
                    .ok()?;
                let debug = workspace_root.join("target/debug/heimq");
                let release = workspace_root.join("target/release/heimq");
                if debug.exists() {
                    Some(debug.to_string_lossy().to_string())
                } else if release.exists() {
                    Some(release.to_string_lossy().to_string())
                } else {
                    None
                }
            })
            .expect("heimq binary not found - run 'cargo build' first");

        let log_output = if std::env::var("HEIMQ_LOG").as_deref() == Ok("1") {
            Stdio::inherit()
        } else {
            Stdio::null()
        };
        let child = Command::new(&binary)
            .args(["--port", &port.to_string(), "--memory-only"])
            .env("HEIMQ_AUTO_CREATE_TOPICS", auto_value)
            .env("HEIMQ_DEFAULT_PARTITIONS", default_partitions.to_string())
            .env("HEIMQ_RETENTION_MS", retention_ms.to_string())
            .env("HEIMQ_MAX_MEMORY_BYTES", max_memory_bytes.to_string())
            .env("RUST_LOG", "heimq=debug")
            .stdout(Stdio::null())
            .stderr(log_output)
            .spawn()
            .expect("Failed to start heimq");

        let server = Self { child, port };
        server.wait_for_ready();
        server
    }

    fn wait_for_ready(&self) {
        // Probe with a real ApiVersions request so we know the server is
        // accepting AND handling Kafka protocol traffic, not just accepting
        // TCP connections during early startup.
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while std::time::Instant::now() < deadline {
            if self.probe_api_versions() {
                return;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        panic!(
            "heimq server failed to start within 10 seconds on port {}",
            self.port
        );
    }

    /// Send a minimal ApiVersions request and return true if the server replies.
    fn probe_api_versions(&self) -> bool {
        use std::io::{Read as _, Write as _};

        let mut stream = match TcpStream::connect(("127.0.0.1", self.port)) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
        let _ = stream.set_write_timeout(Some(Duration::from_millis(200)));

        // ApiVersions v0: header only (body is empty for v0).
        // Frame: [length(4)] [api_key=18(2)] [api_version=0(2)] [correlation_id=1(4)] [client_id=null(-1 as i16)(2)]
        let body: [u8; 10] = [
            0, 18, // api_key = 18
            0, 0, // api_version = 0
            0, 0, 0, 1, // correlation_id = 1
            0xFF, 0xFF, // client_id = null
        ];
        let len_prefix = (body.len() as i32).to_be_bytes();
        if stream.write_all(&len_prefix).is_err() {
            return false;
        }
        if stream.write_all(&body).is_err() {
            return false;
        }

        // A valid response starts with a 4-byte length — that's enough to confirm readiness.
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).is_ok()
    }

    /// Get the bootstrap servers string for Kafka clients
    pub fn bootstrap_servers(&self) -> String {
        format!("127.0.0.1:{}", self.port) // Use IPv4 to avoid IPv6 resolution issues
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
        max_memory_bytes: 0,
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
        advertised_host: None,
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
    request
        .encode(&mut buf, api_version)
        .expect("encode request body");
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

/// One diverging field between heimq and a reference broker for a given workload step.
/// Serializes to JSONL; written to target/parity/<timestamp>-<oracle>-<workload>.jsonl.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct DiffRecord {
    pub workload: String,
    /// Reference broker this diff was computed against ("redpanda" | "kafka").
    pub oracle: String,
    pub step: u32,
    pub field: String,
    pub heimq_value: serde_json::Value,
    /// The reference broker's value for this field (the `oracle` named above).
    pub oracle_value: serde_json::Value,
    /// "value_mismatch" | "missing_in_heimq" | "extra_in_heimq"
    pub divergence: String,
    /// exemption id string, or null
    pub exemption: Option<String>,
}

#[test]
fn diff_record_round_trips_through_json() {
    let d = DiffRecord {
        workload: "test".to_string(),
        oracle: "kafka".to_string(),
        step: 0,
        field: "record.value".to_string(),
        heimq_value: serde_json::json!(null),
        oracle_value: serde_json::json!(null),
        divergence: "value_mismatch".to_string(),
        exemption: None,
    };
    let s = serde_json::to_string(&d).expect("DiffRecord must be JSON-serializable");
    let back: DiffRecord = serde_json::from_str(&s).expect("DiffRecord must round-trip from JSON");
    assert_eq!(back.divergence, "value_mismatch");
    assert_eq!(back.field, "record.value");
}
