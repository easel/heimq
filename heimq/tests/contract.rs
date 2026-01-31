//! Contract-level protocol tests against heimq.
//!
//! These tests use raw Kafka protocol requests to validate API compliance
//! for supported endpoints.

use bytes::{Buf, BufMut, BytesMut};
use kafka_protocol::messages::api_versions_request::ApiVersionsRequest;
use kafka_protocol::messages::api_versions_response::ApiVersionsResponse;
use kafka_protocol::messages::create_topics_request::{CreatableTopic, CreateTopicsRequest};
use kafka_protocol::messages::create_topics_response::CreateTopicsResponse;
use kafka_protocol::messages::delete_topics_request::DeleteTopicsRequest;
use kafka_protocol::messages::delete_topics_response::DeleteTopicsResponse;
use kafka_protocol::messages::list_offsets_request::{ListOffsetsPartition, ListOffsetsRequest, ListOffsetsTopic};
use kafka_protocol::messages::list_offsets_response::ListOffsetsResponse;
use kafka_protocol::messages::offset_commit_request::{OffsetCommitRequest, OffsetCommitRequestPartition, OffsetCommitRequestTopic};
use kafka_protocol::messages::offset_commit_response::OffsetCommitResponse;
use kafka_protocol::messages::offset_fetch_request::{OffsetFetchRequest, OffsetFetchRequestTopic};
use kafka_protocol::messages::offset_fetch_response::OffsetFetchResponse;
use kafka_protocol::messages::{BrokerId, GroupId, TopicName};
use kafka_protocol::protocol::{Decodable, Encodable, StrBytes};
use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU16, AtomicUsize, Ordering};
use std::time::Duration;

static PORT_COUNTER: AtomicU16 = AtomicU16::new(20092);
static TOPIC_COUNTER: AtomicUsize = AtomicUsize::new(1);
static GROUP_COUNTER: AtomicUsize = AtomicUsize::new(1);

fn next_port() -> u16 {
    PORT_COUNTER.fetch_add(1, Ordering::SeqCst)
}

fn unique_topic(prefix: &str) -> String {
    let id = TOPIC_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("{}-{}", prefix, id)
}

fn unique_group(prefix: &str) -> String {
    let id = GROUP_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("{}-{}", prefix, id)
}

struct TestServer {
    child: Child,
    port: u16,
}

impl TestServer {
    fn start() -> Self {
        let port = next_port();
        let child = Command::new("./target/debug/heimq")
            .args(["--port", &port.to_string(), "--memory-only"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .or_else(|_| {
                Command::new("./target/release/heimq")
                    .args(["--port", &port.to_string(), "--memory-only"])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
            })
            .expect("Failed to start heimq - run 'cargo build' first");

        std::thread::sleep(Duration::from_millis(300));
        Self { child, port }
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn encode_request<R: Encodable>(
    api_key: i16,
    api_version: i16,
    correlation_id: i32,
    client_id: Option<&str>,
    request: &R,
) -> Vec<u8> {
    let mut body = BytesMut::new();
    body.put_i16(api_key);
    body.put_i16(api_version);
    body.put_i32(correlation_id);

    match client_id {
        Some(id) => {
            let len = id.len() as i16;
            body.put_i16(len);
            body.put_slice(id.as_bytes());
        }
        None => body.put_i16(-1),
    }

    request.encode(&mut body, api_version).expect("encode request");

    let mut framed = BytesMut::new();
    framed.put_i32(body.len() as i32);
    framed.extend_from_slice(&body);
    framed.to_vec()
}

fn send_request<R: Encodable, S: Decodable>(
    server: &TestServer,
    api_key: i16,
    api_version: i16,
    request: &R,
) -> S {
    let correlation_id = 42;
    let payload = encode_request(api_key, api_version, correlation_id, None, request);

    let mut stream = std::net::TcpStream::connect(("127.0.0.1", server.port))
        .expect("connect to heimq");
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .expect("set read timeout");
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .expect("set write timeout");

    stream.write_all(&payload).expect("write request");

    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).expect("read length");
    let len = i32::from_be_bytes(len_buf) as usize;
    let mut resp_buf = vec![0u8; len];
    stream.read_exact(&mut resp_buf).expect("read response");

    let mut cursor = std::io::Cursor::new(resp_buf);
    let response_correlation_id = cursor.get_i32();
    assert_eq!(response_correlation_id, correlation_id, "correlation id mismatch");

    S::decode(&mut cursor, api_version).expect("decode response")
}

fn create_topic(server: &TestServer, topic: &str, partitions: i32) -> CreateTopicsResponse {
    let mut creatable = CreatableTopic::default();
    creatable.name = TopicName(StrBytes::from_string(topic.to_string()));
    creatable.num_partitions = partitions;
    creatable.replication_factor = 1;

    let mut request = CreateTopicsRequest::default();
    request.topics = vec![creatable];
    request.timeout_ms = 1000;
    request.validate_only = false;

    send_request(server, 19, 1, &request)
}

#[test]
fn contract_api_versions_matches_supported_range() {
    let server = TestServer::start();
    let response: ApiVersionsResponse = send_request(&server, 18, 0, &ApiVersionsRequest::default());

    assert_eq!(response.error_code, 0);

    let expected = vec![
        (0, 0, 8),
        (1, 0, 11),
        (2, 0, 7),
        (3, 0, 8),
        (8, 0, 7),
        (9, 0, 7),
        (10, 0, 3),
        (11, 0, 8),
        (12, 0, 4),
        (13, 0, 4),
        (14, 0, 4),
        (18, 0, 3),
        (19, 0, 6),
        (20, 0, 5),
    ];

    for (api_key, min_version, max_version) in expected {
        let entry = response
            .api_keys
            .iter()
            .find(|api| api.api_key == api_key)
            .unwrap_or_else(|| panic!("missing api key {}", api_key));

        assert_eq!(entry.min_version, min_version, "api {} min_version mismatch", api_key);
        assert_eq!(entry.max_version, max_version, "api {} max_version mismatch", api_key);
    }
}

#[test]
fn contract_create_and_delete_topics() {
    let server = TestServer::start();
    let topic = unique_topic("contract-topic");

    let response = create_topic(&server, &topic, 1);
    let result = response.topics.first().expect("topic result");
    assert_eq!(result.error_code, 0);

    let mut delete_request = DeleteTopicsRequest::default();
    delete_request.topic_names = vec![TopicName(StrBytes::from_string(topic.clone()))];
    delete_request.timeout_ms = 1000;

    let delete_response: DeleteTopicsResponse = send_request(&server, 20, 1, &delete_request);
    let delete_result = delete_response.responses.first().expect("delete result");
    assert_eq!(delete_result.error_code, 0);
}

#[test]
fn contract_list_offsets_earliest_latest() {
    let server = TestServer::start();
    let topic = unique_topic("contract-offsets");

    let response = create_topic(&server, &topic, 1);
    let result = response.topics.first().expect("topic result");
    assert_eq!(result.error_code, 0);

    let mut partition = ListOffsetsPartition::default();
    partition.partition_index = 0;
    partition.timestamp = -2; // earliest
    partition.max_num_offsets = 1;

    let mut topic_req = ListOffsetsTopic::default();
    topic_req.name = TopicName(StrBytes::from_string(topic.clone()));
    topic_req.partitions = vec![partition];

    let mut request = ListOffsetsRequest::default();
    request.replica_id = BrokerId(-1);
    request.topics = vec![topic_req];

    let response: ListOffsetsResponse = send_request(&server, 2, 1, &request);
    let partition_response = &response.topics[0].partitions[0];
    assert_eq!(partition_response.error_code, 0);
    assert_eq!(partition_response.offset, 0);

    let mut partition_latest = ListOffsetsPartition::default();
    partition_latest.partition_index = 0;
    partition_latest.timestamp = -1; // latest
    partition_latest.max_num_offsets = 1;

    let mut topic_req_latest = ListOffsetsTopic::default();
    topic_req_latest.name = TopicName(StrBytes::from_string(topic));
    topic_req_latest.partitions = vec![partition_latest];

    let mut request_latest = ListOffsetsRequest::default();
    request_latest.replica_id = BrokerId(-1);
    request_latest.topics = vec![topic_req_latest];

    let response_latest: ListOffsetsResponse = send_request(&server, 2, 1, &request_latest);
    let partition_response_latest = &response_latest.topics[0].partitions[0];
    assert_eq!(partition_response_latest.error_code, 0);
    assert_eq!(partition_response_latest.offset, 0);
}

#[test]
fn contract_offset_commit_and_fetch() {
    let server = TestServer::start();
    let topic = unique_topic("contract-commit");
    let group = unique_group("contract-group");

    let response = create_topic(&server, &topic, 1);
    let result = response.topics.first().expect("topic result");
    assert_eq!(result.error_code, 0);

    let mut partition = OffsetCommitRequestPartition::default();
    partition.partition_index = 0;
    partition.committed_offset = 42;
    partition.commit_timestamp = 0;

    let mut topic_req = OffsetCommitRequestTopic::default();
    topic_req.name = TopicName(StrBytes::from_string(topic.clone()));
    topic_req.partitions = vec![partition];

    let mut request = OffsetCommitRequest::default();
    request.group_id = GroupId(StrBytes::from_string(group.clone()));
    request.generation_id_or_member_epoch = 1;
    request.member_id = StrBytes::from_string("member".to_string());
    request.topics = vec![topic_req];

    let response: OffsetCommitResponse = send_request(&server, 8, 1, &request);
    let commit_partition = &response.topics[0].partitions[0];
    assert_eq!(commit_partition.error_code, 0);

    let mut fetch_topic = OffsetFetchRequestTopic::default();
    fetch_topic.name = TopicName(StrBytes::from_string(topic));
    fetch_topic.partition_indexes = vec![0];

    let mut fetch_request = OffsetFetchRequest::default();
    fetch_request.group_id = GroupId(StrBytes::from_string(group));
    fetch_request.topics = Some(vec![fetch_topic]);

    let fetch_response: OffsetFetchResponse = send_request(&server, 9, 1, &fetch_request);
    let fetch_partition = &fetch_response.topics[0].partitions[0];
    assert_eq!(fetch_partition.error_code, 0);
    assert_eq!(fetch_partition.committed_offset, 42);
}

