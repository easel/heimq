//! Integration test for the Postgres-backed offset store.
//!
//! Runs only when `HEIMQ_PG_TEST_URL` is set in the environment. The test
//! starts a heimq server with `--storage-offsets <url>`, commits an offset
//! through the Kafka `OffsetCommit` API, restarts the server, and verifies
//! the offset survives via `OffsetFetch`.
//!
//! Compiled only with `--features backend-postgres`. Without the feature the
//! file is empty so `cargo test` still succeeds without a Postgres backend.

#![cfg(feature = "backend-postgres")]

use bytes::{Buf, BufMut, BytesMut};
use kafka_protocol::messages::create_topics_request::{CreatableTopic, CreateTopicsRequest};
use kafka_protocol::messages::create_topics_response::CreateTopicsResponse;
use kafka_protocol::messages::offset_commit_request::{
    OffsetCommitRequest, OffsetCommitRequestPartition, OffsetCommitRequestTopic,
};
use kafka_protocol::messages::offset_commit_response::OffsetCommitResponse;
use kafka_protocol::messages::offset_fetch_request::{
    OffsetFetchRequest, OffsetFetchRequestTopic,
};
use kafka_protocol::messages::offset_fetch_response::OffsetFetchResponse;
use kafka_protocol::messages::{GroupId, TopicName};
use kafka_protocol::protocol::{Decodable, Encodable, StrBytes};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::{Duration, Instant};

static PORT: AtomicU16 = AtomicU16::new(29092);

fn next_port() -> u16 {
    PORT.fetch_add(1, Ordering::SeqCst)
}

fn binary_path() -> Option<String> {
    std::env::var("CARGO_BIN_EXE_heimq").ok()
}

struct PgServer {
    child: Child,
    port: u16,
}

impl PgServer {
    fn start(storage_offsets_url: &str) -> Self {
        let bin = binary_path().expect("CARGO_BIN_EXE_heimq must be set under cargo test");
        let port = next_port();
        let child = Command::new(&bin)
            .args([
                "--host",
                "127.0.0.1",
                "--port",
                &port.to_string(),
                "--memory-only",
                "--storage-offsets",
                storage_offsets_url,
            ])
            .env("HEIMQ_AUTO_CREATE_TOPICS", "true")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn heimq");
        let server = PgServer { child, port };
        server.wait_for_ready();
        server
    }

    fn wait_for_ready(&self) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if TcpStream::connect(("127.0.0.1", self.port)).is_ok() {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        panic!("heimq server failed to start on port {}", self.port);
    }
}

impl Drop for PgServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn encode_request<R: Encodable>(api_key: i16, api_version: i16, correlation_id: i32, request: &R) -> Vec<u8> {
    let mut body = BytesMut::new();
    body.put_i16(api_key);
    body.put_i16(api_version);
    body.put_i32(correlation_id);
    body.put_i16(-1);
    request.encode(&mut body, api_version).expect("encode");
    let mut framed = BytesMut::new();
    framed.put_i32(body.len() as i32);
    framed.extend_from_slice(&body);
    framed.to_vec()
}

fn send_request<R: Encodable, S: Decodable>(
    port: u16,
    api_key: i16,
    api_version: i16,
    request: &R,
) -> S {
    let correlation_id = 17;
    let payload = encode_request(api_key, api_version, correlation_id, request);
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    stream.set_write_timeout(Some(Duration::from_secs(5))).unwrap();
    stream.write_all(&payload).unwrap();
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).unwrap();
    let len = i32::from_be_bytes(len_buf) as usize;
    let mut resp_buf = vec![0u8; len];
    stream.read_exact(&mut resp_buf).unwrap();
    let mut cursor = std::io::Cursor::new(resp_buf);
    let response_correlation_id = cursor.get_i32();
    assert_eq!(response_correlation_id, correlation_id);
    S::decode(&mut cursor, api_version).expect("decode")
}

fn create_topic(port: u16, topic: &str) {
    let mut creatable = CreatableTopic::default();
    creatable.name = TopicName(StrBytes::from_string(topic.to_string()));
    creatable.num_partitions = 1;
    creatable.replication_factor = 1;
    let mut request = CreateTopicsRequest::default();
    request.topics = vec![creatable];
    request.timeout_ms = 1000;
    let _resp: CreateTopicsResponse = send_request(port, 19, 1, &request);
}

fn unique_suffix() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{}_{}", std::process::id(), nanos)
}

#[test]
fn postgres_offset_store_survives_restart() {
    let base_url = match std::env::var("HEIMQ_PG_TEST_URL") {
        Ok(v) if !v.is_empty() => v,
        _ => {
            eprintln!("skipping postgres_offset_store_survives_restart: HEIMQ_PG_TEST_URL not set");
            return;
        }
    };

    // Use a unique schema per test run so concurrent runs don't collide and
    // each run starts from an empty table.
    let schema = format!("heimq_test_{}", unique_suffix());
    let separator = if base_url.contains('?') { "&" } else { "?" };
    let url = format!("{base}{sep}schema={schema}", base = base_url, sep = separator);

    let topic = format!("heimq-pg-{}", unique_suffix());
    let group = format!("heimq-pg-grp-{}", unique_suffix());

    // Phase 1: commit an offset through the OffsetCommit handler.
    {
        let server = PgServer::start(&url);
        create_topic(server.port, &topic);

        let mut partition = OffsetCommitRequestPartition::default();
        partition.partition_index = 0;
        partition.committed_offset = 1234;
        partition.committed_metadata = Some(StrBytes::from_string("durable".to_string()));
        partition.commit_timestamp = 0;

        let mut topic_req = OffsetCommitRequestTopic::default();
        topic_req.name = TopicName(StrBytes::from_string(topic.clone()));
        topic_req.partitions = vec![partition];

        let mut request = OffsetCommitRequest::default();
        request.group_id = GroupId(StrBytes::from_string(group.clone()));
        request.generation_id_or_member_epoch = 1;
        request.member_id = StrBytes::from_string("durable-member".to_string());
        request.topics = vec![topic_req];

        let response: OffsetCommitResponse = send_request(server.port, 8, 1, &request);
        assert_eq!(
            response.topics[0].partitions[0].error_code, 0,
            "OffsetCommit must succeed"
        );
    } // server dropped/killed

    // Phase 2: restart against the same Postgres+schema; the offset must come back.
    {
        let server = PgServer::start(&url);

        let mut fetch_topic = OffsetFetchRequestTopic::default();
        fetch_topic.name = TopicName(StrBytes::from_string(topic));
        fetch_topic.partition_indexes = vec![0];

        let mut fetch_request = OffsetFetchRequest::default();
        fetch_request.group_id = GroupId(StrBytes::from_string(group));
        fetch_request.topics = Some(vec![fetch_topic]);

        let response: OffsetFetchResponse = send_request(server.port, 9, 1, &fetch_request);
        let partition = &response.topics[0].partitions[0];
        assert_eq!(partition.error_code, 0, "OffsetFetch must succeed");
        assert_eq!(
            partition.committed_offset, 1234,
            "committed offset must survive restart"
        );
    }
}
