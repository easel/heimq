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
use kafka_protocol::messages::fetch_request::{FetchPartition, FetchRequest, FetchTopic};
use kafka_protocol::messages::fetch_response::FetchResponse;
use kafka_protocol::messages::list_offsets_request::{ListOffsetsPartition, ListOffsetsRequest, ListOffsetsTopic};
use kafka_protocol::messages::list_offsets_response::ListOffsetsResponse;
use kafka_protocol::messages::metadata_request::{MetadataRequest, MetadataRequestTopic};
use kafka_protocol::messages::metadata_response::MetadataResponse;
use kafka_protocol::messages::offset_commit_request::{OffsetCommitRequest, OffsetCommitRequestPartition, OffsetCommitRequestTopic};
use kafka_protocol::messages::offset_commit_response::OffsetCommitResponse;
use kafka_protocol::messages::offset_fetch_request::{OffsetFetchRequest, OffsetFetchRequestTopic};
use kafka_protocol::messages::offset_fetch_response::OffsetFetchResponse;
use kafka_protocol::messages::produce_request::{PartitionProduceData, ProduceRequest, TopicProduceData};
use kafka_protocol::messages::produce_response::ProduceResponse;
use kafka_protocol::messages::{BrokerId, GroupId, TopicName};
use kafka_protocol::protocol::{Decodable, Encodable, StrBytes};
use kafka_protocol::records::{Compression, Record, RecordBatchDecoder, RecordBatchEncoder, RecordEncodeOptions, TimestampType};
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
        Self::start_with_auto_create(true)
    }

    fn start_with_auto_create(auto_create: bool) -> Self {
        let port = next_port();
        let auto_value = if auto_create { "true" } else { "false" };
        let child = Command::new("./target/debug/heimq")
            .args(["--port", &port.to_string(), "--memory-only"])
            .env("HEIMQ_AUTO_CREATE_TOPICS", auto_value)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .or_else(|_| {
                Command::new("./target/release/heimq")
                    .args(["--port", &port.to_string(), "--memory-only"])
                    .env("HEIMQ_AUTO_CREATE_TOPICS", auto_value)
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

fn new_record(offset: i64) -> Record {
    Record {
        transactional: false,
        control: false,
        partition_leader_epoch: 0,
        producer_id: -1,
        producer_epoch: -1,
        timestamp_type: TimestampType::Creation,
        timestamp: offset,
        sequence: offset as i32,
        offset,
        key: Some(format!("key-{offset}").into()),
        value: Some(format!("value-{offset}").into()),
        headers: Default::default(),
    }
}

fn encode_record_batch(records: &[Record]) -> bytes::Bytes {
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

fn produce_batch(server: &TestServer, topic: &str, batch: bytes::Bytes) -> ProduceResponse {
    let mut partition = PartitionProduceData::default();
    partition.index = 0;
    partition.records = Some(batch);

    let mut topic_data = TopicProduceData::default();
    topic_data.name = TopicName(StrBytes::from_string(topic.to_string()));
    topic_data.partition_data = vec![partition];

    let mut produce_request = ProduceRequest::default();
    produce_request.acks = 1;
    produce_request.timeout_ms = 1000;
    produce_request.topic_data = vec![topic_data];

    send_request(server, 0, 2, &produce_request)
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
fn contract_metadata_auto_creates_topic() {
    let server = TestServer::start();
    let topic = unique_topic("contract-metadata");

    let mut topic_req = MetadataRequestTopic::default();
    topic_req.name = Some(TopicName(StrBytes::from_string(topic.clone())));

    let mut request = MetadataRequest::default();
    request.topics = Some(vec![topic_req]);
    request.allow_auto_topic_creation = true;

    let response: MetadataResponse = send_request(&server, 3, 4, &request);
    assert!(!response.brokers.is_empty(), "expected broker metadata");
    assert!(response.cluster_id.is_some(), "cluster id should be set for v4");

    let topic_response = response
        .topics
        .iter()
        .find(|t| t.name.as_ref().map(|n| n.0.as_str()) == Some(topic.as_str()))
        .expect("topic metadata");

    assert_eq!(topic_response.error_code, 0);
    assert!(!topic_response.partitions.is_empty(), "expected partitions");
}

#[test]
fn contract_metadata_unknown_topic_no_autocreate() {
    let server = TestServer::start_with_auto_create(false);
    let topic = unique_topic("contract-metadata-no-auto");

    let mut topic_req = MetadataRequestTopic::default();
    topic_req.name = Some(TopicName(StrBytes::from_string(topic.clone())));

    let mut request = MetadataRequest::default();
    request.topics = Some(vec![topic_req]);
    request.allow_auto_topic_creation = true;

    let response: MetadataResponse = send_request(&server, 3, 4, &request);
    let topic_response = response
        .topics
        .iter()
        .find(|t| t.name.as_ref().map(|n| n.0.as_str()) == Some(topic.as_str()))
        .expect("topic metadata");

    assert_eq!(topic_response.error_code, 3);
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
fn contract_produce_fetch_roundtrip() {
    let server = TestServer::start();
    let topic = unique_topic("contract-produce");

    let response = create_topic(&server, &topic, 1);
    let result = response.topics.first().expect("topic result");
    assert_eq!(result.error_code, 0);

    let records = vec![new_record(0)];
    let encoded = encode_record_batch(&records);
    let produce_response = produce_batch(&server, &topic, encoded);
    let produce_partition = &produce_response.responses[0].partition_responses[0];
    assert_eq!(produce_partition.error_code, 0);

    let mut fetch_partition = FetchPartition::default();
    fetch_partition.partition = 0;
    fetch_partition.fetch_offset = 0;
    fetch_partition.partition_max_bytes = 1024 * 1024;

    let mut fetch_topic = FetchTopic::default();
    fetch_topic.topic = TopicName(StrBytes::from_string(topic));
    fetch_topic.partitions = vec![fetch_partition];

    let mut fetch_request = FetchRequest::default();
    fetch_request.replica_id = BrokerId(-1);
    fetch_request.max_wait_ms = 1000;
    fetch_request.min_bytes = 1;
    fetch_request.max_bytes = 1024 * 1024;
    fetch_request.topics = vec![fetch_topic];

    let fetch_response: FetchResponse = send_request(&server, 1, 3, &fetch_request);
    let partition_response = &fetch_response.responses[0].partitions[0];
    assert_eq!(partition_response.error_code, 0);
    let mut fetched = partition_response.records.clone().expect("records present");
    let decoded = RecordBatchDecoder::decode_all(&mut fetched).expect("decode fetched");
    let decoded_records: Vec<Record> = decoded.into_iter().flat_map(|r| r.records).collect();
    assert_eq!(decoded_records, records);
}

#[test]
fn contract_fetch_respects_max_bytes() {
    let server = TestServer::start();
    let topic = unique_topic("contract-fetch-max-bytes");

    let response = create_topic(&server, &topic, 1);
    let result = response.topics.first().expect("topic result");
    assert_eq!(result.error_code, 0);

    let records_first = vec![new_record(0)];
    let records_second = vec![new_record(1)];
    let encoded_first = encode_record_batch(&records_first);
    let encoded_second = encode_record_batch(&records_second);

    let produce_response = produce_batch(&server, &topic, encoded_first.clone());
    assert_eq!(produce_response.responses[0].partition_responses[0].error_code, 0);
    let produce_response = produce_batch(&server, &topic, encoded_second);
    assert_eq!(produce_response.responses[0].partition_responses[0].error_code, 0);

    let mut fetch_partition = FetchPartition::default();
    fetch_partition.partition = 0;
    fetch_partition.fetch_offset = 0;
    fetch_partition.partition_max_bytes = 1024 * 1024;

    let mut fetch_topic = FetchTopic::default();
    fetch_topic.topic = TopicName(StrBytes::from_string(topic));
    fetch_topic.partitions = vec![fetch_partition];

    let mut fetch_request = FetchRequest::default();
    fetch_request.replica_id = BrokerId(-1);
    fetch_request.max_wait_ms = 1000;
    fetch_request.min_bytes = 1;
    fetch_request.max_bytes = (encoded_first.len() as i32) + 1;
    fetch_request.topics = vec![fetch_topic];

    let fetch_response: FetchResponse = send_request(&server, 1, 3, &fetch_request);
    let partition_response = &fetch_response.responses[0].partitions[0];
    assert_eq!(partition_response.error_code, 0);

    let mut fetched = partition_response.records.clone().expect("records present");
    let decoded = RecordBatchDecoder::decode_all(&mut fetched).expect("decode fetched");
    let decoded_records: Vec<Record> = decoded.into_iter().flat_map(|r| r.records).collect();
    assert_eq!(decoded_records, records_first);
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
