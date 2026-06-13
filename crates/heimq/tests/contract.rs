//! Contract-level protocol tests against heimq.
//!
//! These tests use raw Kafka protocol requests to validate API compliance
//! for supported endpoints.

use bytes::{Buf, BufMut, BytesMut};
use heimq::protocol::SUPPORTED_APIS;
use heimq::test_support::{unique_group, unique_topic, TestServer};
use kafka_protocol::messages::api_versions_request::ApiVersionsRequest;
use kafka_protocol::messages::api_versions_response::ApiVersionsResponse;
use kafka_protocol::messages::create_topics_request::{CreatableTopic, CreateTopicsRequest};
use kafka_protocol::messages::create_topics_response::CreateTopicsResponse;
use kafka_protocol::messages::delete_topics_request::DeleteTopicsRequest;
use kafka_protocol::messages::delete_topics_response::DeleteTopicsResponse;
use kafka_protocol::messages::fetch_request::{FetchPartition, FetchRequest, FetchTopic};
use kafka_protocol::messages::fetch_response::FetchResponse;
use kafka_protocol::messages::find_coordinator_request::FindCoordinatorRequest;
use kafka_protocol::messages::find_coordinator_response::FindCoordinatorResponse;
use kafka_protocol::messages::heartbeat_request::HeartbeatRequest;
use kafka_protocol::messages::heartbeat_response::HeartbeatResponse;
use kafka_protocol::messages::join_group_request::{JoinGroupRequest, JoinGroupRequestProtocol};
use kafka_protocol::messages::join_group_response::JoinGroupResponse;
use kafka_protocol::messages::leave_group_request::LeaveGroupRequest;
use kafka_protocol::messages::leave_group_response::LeaveGroupResponse;
use kafka_protocol::messages::list_offsets_request::{
    ListOffsetsPartition, ListOffsetsRequest, ListOffsetsTopic,
};
use kafka_protocol::messages::list_offsets_response::ListOffsetsResponse;
use kafka_protocol::messages::metadata_request::{MetadataRequest, MetadataRequestTopic};
use kafka_protocol::messages::metadata_response::MetadataResponse;
use kafka_protocol::messages::offset_commit_request::{
    OffsetCommitRequest, OffsetCommitRequestPartition, OffsetCommitRequestTopic,
};
use kafka_protocol::messages::offset_commit_response::OffsetCommitResponse;
use kafka_protocol::messages::offset_fetch_request::{OffsetFetchRequest, OffsetFetchRequestTopic};
use kafka_protocol::messages::offset_fetch_response::OffsetFetchResponse;
use kafka_protocol::messages::produce_request::{
    PartitionProduceData, ProduceRequest, TopicProduceData,
};
use kafka_protocol::messages::produce_response::ProduceResponse;
use kafka_protocol::messages::sync_group_request::{SyncGroupRequest, SyncGroupRequestAssignment};
use kafka_protocol::messages::sync_group_response::SyncGroupResponse;
use kafka_protocol::messages::{BrokerId, GroupId, TopicName};
use kafka_protocol::protocol::{Decodable, Encodable, StrBytes};
use kafka_protocol::records::{
    Compression, Record, RecordBatchDecoder, RecordBatchEncoder, RecordEncodeOptions,
    TimestampType,
};
use std::io::{Read, Write};
use std::time::Duration;

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

// @covers US-001-AC1 US-013-AC2
#[test]
fn contract_api_versions_matches_supported_range() {
    let server = TestServer::start();
    let response: ApiVersionsResponse = send_request(&server, 18, 0, &ApiVersionsRequest::default());

    assert_eq!(response.error_code, 0);

    // Derive the expected set from SUPPORTED_APIS so this test cannot
    // silently desync when the flexible-version policy changes.
    // TestServer uses the memory backend, which has full capabilities, so
    // every entry in SUPPORTED_APIS should be advertised.
    let mut advertised: Vec<(i16, i16, i16)> = response
        .api_keys
        .iter()
        .map(|api| (api.api_key, api.min_version, api.max_version))
        .collect();
    advertised.sort_by_key(|(k, _, _)| *k);

    let mut expected: Vec<(i16, i16, i16)> = SUPPORTED_APIS.to_vec();
    expected.sort_by_key(|(k, _, _)| *k);

    assert_eq!(advertised, expected);
}

// @covers US-001-AC1
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

// @covers US-001-AC1 US-001-AC2 US-013-AC3
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

// @covers US-013-AC3
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

// @covers US-002-AC4
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

// ============================================================================
// Consumer Group Contract Tests (Phase 2)
// ============================================================================

// @covers US-002-AC1
#[test]
fn contract_find_coordinator_returns_self() {
    let server = TestServer::start();
    let group = unique_group("contract-coordinator");

    let mut request = FindCoordinatorRequest::default();
    request.key = StrBytes::from_string(group);

    let response: FindCoordinatorResponse = send_request(&server, 10, 0, &request);
    assert_eq!(response.error_code, 0, "expected no error");
    assert_eq!(response.node_id.0, 0, "node_id should be 0 for single broker");
    assert!(
        response.host.as_str().contains("localhost") || response.host.as_str().contains("127.0.0.1"),
        "host should contain localhost or 127.0.0.1, got: {}",
        response.host.as_str()
    );
}

// @covers US-002-AC3
#[test]
fn contract_join_group_new_member() {
    let server = TestServer::start();
    let group = unique_group("contract-join");

    // Create a protocol
    let mut protocol = JoinGroupRequestProtocol::default();
    protocol.name = StrBytes::from_string("range".to_string());
    protocol.metadata = bytes::Bytes::from(vec![0, 1, 0, 0, 0, 1, 0, 4, 116, 101, 115, 116]);

    // First JoinGroup with empty member_id -> should get MEMBER_ID_REQUIRED (79)
    let mut request = JoinGroupRequest::default();
    request.group_id = GroupId(StrBytes::from_string(group.clone()));
    request.session_timeout_ms = 30000;
    request.rebalance_timeout_ms = 30000;
    request.member_id = StrBytes::from_string(String::new());
    request.protocol_type = StrBytes::from_string("consumer".to_string());
    request.protocols = vec![protocol.clone()];

    let response: JoinGroupResponse = send_request(&server, 11, 1, &request);
    assert_eq!(response.error_code, 79, "first join should return MEMBER_ID_REQUIRED");
    assert!(!response.member_id.is_empty(), "should be assigned a member_id");

    let assigned_member_id = response.member_id.clone();

    // Second JoinGroup with assigned member_id -> should succeed and become leader
    let mut request2 = JoinGroupRequest::default();
    request2.group_id = GroupId(StrBytes::from_string(group));
    request2.session_timeout_ms = 30000;
    request2.rebalance_timeout_ms = 30000;
    request2.member_id = assigned_member_id.clone();
    request2.protocol_type = StrBytes::from_string("consumer".to_string());
    request2.protocols = vec![protocol];

    let response2: JoinGroupResponse = send_request(&server, 11, 1, &request2);
    assert_eq!(response2.error_code, 0, "second join should succeed");
    assert_eq!(response2.leader, assigned_member_id, "member should become leader");
    assert!(response2.generation_id > 0, "generation_id should be positive");
}

// @covers US-002-AC1
#[test]
fn contract_sync_group_leader() {
    let server = TestServer::start();
    let group = unique_group("contract-sync");

    // Join the group first
    let mut protocol = JoinGroupRequestProtocol::default();
    protocol.name = StrBytes::from_string("range".to_string());
    protocol.metadata = bytes::Bytes::from(vec![0, 1, 0, 0, 0, 1, 0, 4, 116, 101, 115, 116]);

    // First join - get member_id
    let mut join_request = JoinGroupRequest::default();
    join_request.group_id = GroupId(StrBytes::from_string(group.clone()));
    join_request.session_timeout_ms = 30000;
    join_request.rebalance_timeout_ms = 30000;
    join_request.member_id = StrBytes::from_string(String::new());
    join_request.protocol_type = StrBytes::from_string("consumer".to_string());
    join_request.protocols = vec![protocol.clone()];

    let join_response: JoinGroupResponse = send_request(&server, 11, 1, &join_request);
    let member_id = join_response.member_id.clone();

    // Second join - become leader
    join_request.member_id = member_id.clone();
    let join_response: JoinGroupResponse = send_request(&server, 11, 1, &join_request);
    assert_eq!(join_response.error_code, 0);
    let generation_id = join_response.generation_id;

    // Now send SyncGroup as leader with assignment
    let mut assignment = SyncGroupRequestAssignment::default();
    assignment.member_id = member_id.clone();
    assignment.assignment = bytes::Bytes::from(vec![0, 0, 0, 1, 0, 4, 116, 101, 115, 116]);

    let mut sync_request = SyncGroupRequest::default();
    sync_request.group_id = GroupId(StrBytes::from_string(group));
    sync_request.generation_id = generation_id;
    sync_request.member_id = member_id;
    sync_request.assignments = vec![assignment];

    let sync_response: SyncGroupResponse = send_request(&server, 14, 1, &sync_request);
    assert_eq!(sync_response.error_code, 0, "sync_group should succeed");
}

// @covers US-002-AC1
#[test]
fn contract_heartbeat_active_member() {
    let server = TestServer::start();
    let group = unique_group("contract-heartbeat");

    // Join the group first
    let mut protocol = JoinGroupRequestProtocol::default();
    protocol.name = StrBytes::from_string("range".to_string());
    protocol.metadata = bytes::Bytes::from(vec![0, 1, 0, 0, 0, 1, 0, 4, 116, 101, 115, 116]);

    // First join - get member_id
    let mut join_request = JoinGroupRequest::default();
    join_request.group_id = GroupId(StrBytes::from_string(group.clone()));
    join_request.session_timeout_ms = 30000;
    join_request.rebalance_timeout_ms = 30000;
    join_request.member_id = StrBytes::from_string(String::new());
    join_request.protocol_type = StrBytes::from_string("consumer".to_string());
    join_request.protocols = vec![protocol.clone()];

    let join_response: JoinGroupResponse = send_request(&server, 11, 1, &join_request);
    let member_id = join_response.member_id.clone();

    // Second join - become leader
    join_request.member_id = member_id.clone();
    let join_response: JoinGroupResponse = send_request(&server, 11, 1, &join_request);
    assert_eq!(join_response.error_code, 0);
    let generation_id = join_response.generation_id;

    // Send heartbeat
    let mut heartbeat_request = HeartbeatRequest::default();
    heartbeat_request.group_id = GroupId(StrBytes::from_string(group));
    heartbeat_request.generation_id = generation_id;
    heartbeat_request.member_id = member_id;

    let heartbeat_response: HeartbeatResponse = send_request(&server, 12, 1, &heartbeat_request);
    assert_eq!(heartbeat_response.error_code, 0, "heartbeat should succeed for active member");
}

// @covers US-002-AC2
#[test]
fn contract_leave_group_success() {
    let server = TestServer::start();
    let group = unique_group("contract-leave");

    // Join the group first
    let mut protocol = JoinGroupRequestProtocol::default();
    protocol.name = StrBytes::from_string("range".to_string());
    protocol.metadata = bytes::Bytes::from(vec![0, 1, 0, 0, 0, 1, 0, 4, 116, 101, 115, 116]);

    // First join - get member_id
    let mut join_request = JoinGroupRequest::default();
    join_request.group_id = GroupId(StrBytes::from_string(group.clone()));
    join_request.session_timeout_ms = 30000;
    join_request.rebalance_timeout_ms = 30000;
    join_request.member_id = StrBytes::from_string(String::new());
    join_request.protocol_type = StrBytes::from_string("consumer".to_string());
    join_request.protocols = vec![protocol.clone()];

    let join_response: JoinGroupResponse = send_request(&server, 11, 1, &join_request);
    let member_id = join_response.member_id.clone();

    // Second join - become member
    join_request.member_id = member_id.clone();
    let join_response: JoinGroupResponse = send_request(&server, 11, 1, &join_request);
    assert_eq!(join_response.error_code, 0);

    // Leave the group
    let mut leave_request = LeaveGroupRequest::default();
    leave_request.group_id = GroupId(StrBytes::from_string(group));
    leave_request.member_id = member_id;

    let leave_response: LeaveGroupResponse = send_request(&server, 13, 1, &leave_request);
    assert_eq!(leave_response.error_code, 0, "leave_group should succeed");
}

// @covers US-002-AC1 US-002-AC2
#[test]
fn contract_consumer_group_lifecycle() {
    let server = TestServer::start();
    let group = unique_group("contract-lifecycle");
    let topic = unique_topic("contract-lifecycle");

    // Create topic for offset commit/fetch
    let response = create_topic(&server, &topic, 1);
    assert_eq!(response.topics[0].error_code, 0);

    // 1. FindCoordinator
    let mut find_request = FindCoordinatorRequest::default();
    find_request.key = StrBytes::from_string(group.clone());
    let find_response: FindCoordinatorResponse = send_request(&server, 10, 0, &find_request);
    assert_eq!(find_response.error_code, 0, "FindCoordinator should succeed");

    // 2. JoinGroup (first call - get member_id)
    let mut protocol = JoinGroupRequestProtocol::default();
    protocol.name = StrBytes::from_string("range".to_string());
    protocol.metadata = bytes::Bytes::from(vec![0, 1, 0, 0, 0, 1, 0, 4, 116, 101, 115, 116]);

    let mut join_request = JoinGroupRequest::default();
    join_request.group_id = GroupId(StrBytes::from_string(group.clone()));
    join_request.session_timeout_ms = 30000;
    join_request.rebalance_timeout_ms = 30000;
    join_request.member_id = StrBytes::from_string(String::new());
    join_request.protocol_type = StrBytes::from_string("consumer".to_string());
    join_request.protocols = vec![protocol.clone()];

    let join_response: JoinGroupResponse = send_request(&server, 11, 1, &join_request);
    assert_eq!(join_response.error_code, 79, "First JoinGroup should return MEMBER_ID_REQUIRED");
    let member_id = join_response.member_id.clone();

    // 3. JoinGroup (second call - become leader)
    join_request.member_id = member_id.clone();
    let join_response: JoinGroupResponse = send_request(&server, 11, 1, &join_request);
    assert_eq!(join_response.error_code, 0, "Second JoinGroup should succeed");
    let generation_id = join_response.generation_id;
    assert_eq!(join_response.leader, member_id, "Member should be leader");

    // 4. SyncGroup
    let mut assignment = SyncGroupRequestAssignment::default();
    assignment.member_id = member_id.clone();
    assignment.assignment = bytes::Bytes::from(vec![0, 0, 0, 1, 0, 4, 116, 101, 115, 116]);

    let mut sync_request = SyncGroupRequest::default();
    sync_request.group_id = GroupId(StrBytes::from_string(group.clone()));
    sync_request.generation_id = generation_id;
    sync_request.member_id = member_id.clone();
    sync_request.assignments = vec![assignment];

    let sync_response: SyncGroupResponse = send_request(&server, 14, 1, &sync_request);
    assert_eq!(sync_response.error_code, 0, "SyncGroup should succeed");

    // 5. Heartbeat
    let mut heartbeat_request = HeartbeatRequest::default();
    heartbeat_request.group_id = GroupId(StrBytes::from_string(group.clone()));
    heartbeat_request.generation_id = generation_id;
    heartbeat_request.member_id = member_id.clone();

    let heartbeat_response: HeartbeatResponse = send_request(&server, 12, 1, &heartbeat_request);
    assert_eq!(heartbeat_response.error_code, 0, "Heartbeat should succeed");

    // 6. OffsetCommit
    let mut commit_partition = OffsetCommitRequestPartition::default();
    commit_partition.partition_index = 0;
    commit_partition.committed_offset = 100;
    commit_partition.commit_timestamp = 0;

    let mut commit_topic = OffsetCommitRequestTopic::default();
    commit_topic.name = TopicName(StrBytes::from_string(topic.clone()));
    commit_topic.partitions = vec![commit_partition];

    let mut commit_request = OffsetCommitRequest::default();
    commit_request.group_id = GroupId(StrBytes::from_string(group.clone()));
    commit_request.generation_id_or_member_epoch = generation_id;
    commit_request.member_id = member_id.clone();
    commit_request.topics = vec![commit_topic];

    let commit_response: OffsetCommitResponse = send_request(&server, 8, 1, &commit_request);
    assert_eq!(commit_response.topics[0].partitions[0].error_code, 0, "OffsetCommit should succeed");

    // 7. OffsetFetch
    let mut fetch_topic = OffsetFetchRequestTopic::default();
    fetch_topic.name = TopicName(StrBytes::from_string(topic));
    fetch_topic.partition_indexes = vec![0];

    let mut fetch_request = OffsetFetchRequest::default();
    fetch_request.group_id = GroupId(StrBytes::from_string(group.clone()));
    fetch_request.topics = Some(vec![fetch_topic]);

    let fetch_response: OffsetFetchResponse = send_request(&server, 9, 1, &fetch_request);
    assert_eq!(fetch_response.topics[0].partitions[0].error_code, 0, "OffsetFetch should succeed");
    assert_eq!(fetch_response.topics[0].partitions[0].committed_offset, 100, "Committed offset should match");

    // 8. LeaveGroup
    let mut leave_request = LeaveGroupRequest::default();
    leave_request.group_id = GroupId(StrBytes::from_string(group));
    leave_request.member_id = member_id;

    let leave_response: LeaveGroupResponse = send_request(&server, 13, 1, &leave_request);
    assert_eq!(leave_response.error_code, 0, "LeaveGroup should succeed");
}

fn new_idempotent_record(offset: i64, producer_id: i64, producer_epoch: i16) -> Record {
    Record {
        transactional: false,
        control: false,
        partition_leader_epoch: 0,
        producer_id,
        producer_epoch,
        timestamp_type: TimestampType::Creation,
        timestamp: offset,
        sequence: offset as i32,
        offset,
        key: Some(format!("key-{offset}").into()),
        value: Some(format!("val-{offset}").into()),
        headers: Default::default(),
    }
}

// @covers US-003-AC1
#[test]
fn contract_init_producer_id_returns_valid_id() {
    use kafka_protocol::messages::init_producer_id_request::InitProducerIdRequest;
    use kafka_protocol::messages::init_producer_id_response::InitProducerIdResponse;

    let server = TestServer::start();
    let request = InitProducerIdRequest::default();
    let response: InitProducerIdResponse = send_request(&server, 22, 0, &request);

    assert_eq!(response.error_code, 0, "InitProducerId should succeed");
    assert!(response.producer_id.0 >= 0, "producer_id must be non-negative");
    assert_eq!(response.producer_epoch, 0, "initial epoch must be 0");

    // Second call returns a different producer_id.
    let response2: InitProducerIdResponse = send_request(&server, 22, 0, &request);
    assert_eq!(response2.error_code, 0);
    assert_ne!(
        response.producer_id.0, response2.producer_id.0,
        "each InitProducerId call must return a unique producer_id"
    );
}

// @covers US-003-AC4
#[test]
fn contract_out_of_order_sequence_returns_error() {
    let server = TestServer::start();
    let topic = unique_topic("contract-idempotent-ooo");

    use kafka_protocol::messages::init_producer_id_request::InitProducerIdRequest;
    use kafka_protocol::messages::init_producer_id_response::InitProducerIdResponse;

    // Get a producer ID.
    let pid_resp: InitProducerIdResponse =
        send_request(&server, 22, 0, &InitProducerIdRequest::default());
    assert_eq!(pid_resp.error_code, 0);
    let producer_id = pid_resp.producer_id.0;

    // First batch: base_sequence=0 (should succeed).
    let batch0 = {
        let mut buf = BytesMut::new();
        RecordBatchEncoder::encode(
            &mut buf,
            &[new_idempotent_record(0, producer_id, 0)],
            &RecordEncodeOptions { version: 2, compression: Compression::None },
        )
        .expect("encode batch 0");
        buf.freeze()
    };
    let resp0 = produce_batch(&server, &topic, batch0);
    assert_eq!(
        resp0.responses[0].partition_responses[0].error_code, 0,
        "first idempotent batch should succeed"
    );

    // Second batch with base_sequence=5 (skips 1..4 → out of order).
    let batch_ooo = {
        let mut buf = BytesMut::new();
        RecordBatchEncoder::encode(
            &mut buf,
            &[new_idempotent_record(5, producer_id, 0)],
            &RecordEncodeOptions { version: 2, compression: Compression::None },
        )
        .expect("encode batch ooo");
        buf.freeze()
    };
    let resp_ooo = produce_batch(&server, &topic, batch_ooo);
    assert_eq!(
        resp_ooo.responses[0].partition_responses[0].error_code,
        45, // OUT_OF_ORDER_SEQUENCE_NUMBER
        "out-of-order sequence must return error 45"
    );
}

// @covers US-004-AC1
#[test]
fn contract_init_producer_id_with_transactional_id() {
    use kafka_protocol::messages::init_producer_id_request::InitProducerIdRequest;
    use kafka_protocol::messages::init_producer_id_response::InitProducerIdResponse;
    use kafka_protocol::protocol::StrBytes;

    let server = TestServer::start();

    let mut request = InitProducerIdRequest::default();
    request.transactional_id = Some(kafka_protocol::messages::TransactionalId(
        StrBytes::from_string("test-txn-id-1".to_string()),
    ));
    request.transaction_timeout_ms = 60000;

    let response: InitProducerIdResponse = send_request(&server, 22, 0, &request);
    assert_eq!(response.error_code, 0, "InitProducerId with transactional_id should succeed");
    assert!(response.producer_id.0 >= 0, "producer_id must be non-negative");
    assert_eq!(response.producer_epoch, 0, "initial epoch must be 0");

    // Second call with same transactional_id should bump the epoch
    let response2: InitProducerIdResponse = send_request(&server, 22, 0, &request);
    assert_eq!(response2.error_code, 0, "Re-init with same transactional_id should succeed");
    assert_eq!(
        response.producer_id.0, response2.producer_id.0,
        "same producer_id for same transactional_id"
    );
    assert_eq!(response2.producer_epoch, 1, "epoch should be bumped on re-init");
}

// @covers US-004-AC2 US-004-AC9
#[test]
fn contract_add_partitions_to_txn_basic() {
    use kafka_protocol::messages::add_partitions_to_txn_request::{
        AddPartitionsToTxnRequest, AddPartitionsToTxnTopic,
    };
    use kafka_protocol::messages::add_partitions_to_txn_response::AddPartitionsToTxnResponse;
    use kafka_protocol::messages::init_producer_id_request::InitProducerIdRequest;
    use kafka_protocol::messages::init_producer_id_response::InitProducerIdResponse;
    use kafka_protocol::protocol::StrBytes;

    let server = TestServer::start();
    let topic = unique_topic("contract-txn-partitions");
    create_topic(&server, &topic, 1);

    // Init transactional producer
    let mut init_req = InitProducerIdRequest::default();
    init_req.transactional_id = Some(kafka_protocol::messages::TransactionalId(
        StrBytes::from_string("txn-add-parts".to_string()),
    ));
    init_req.transaction_timeout_ms = 60000;
    let init_resp: InitProducerIdResponse = send_request(&server, 22, 0, &init_req);
    assert_eq!(init_resp.error_code, 0);
    let producer_id = init_resp.producer_id;
    let epoch = init_resp.producer_epoch;

    // AddPartitionsToTxn
    let mut add_topic = AddPartitionsToTxnTopic::default();
    add_topic.name = TopicName(StrBytes::from_string(topic.clone()));
    add_topic.partitions = vec![0];

    let mut add_req = AddPartitionsToTxnRequest::default();
    add_req.v3_and_below_transactional_id =
        kafka_protocol::messages::TransactionalId(StrBytes::from_string("txn-add-parts".to_string()));
    add_req.v3_and_below_producer_id = producer_id;
    add_req.v3_and_below_producer_epoch = epoch;
    add_req.v3_and_below_topics = vec![add_topic];

    let add_resp: AddPartitionsToTxnResponse = send_request(&server, 24, 0, &add_req);
    assert_eq!(
        add_resp.results_by_topic_v3_and_below[0].results_by_partition[0].partition_error_code,
        0,
        "AddPartitionsToTxn should succeed"
    );
}

// @covers US-004 EndTxn basic flow
#[test]
fn contract_end_txn_basic() {
    use kafka_protocol::messages::end_txn_request::EndTxnRequest;
    use kafka_protocol::messages::end_txn_response::EndTxnResponse;
    use kafka_protocol::messages::init_producer_id_request::InitProducerIdRequest;
    use kafka_protocol::messages::init_producer_id_response::InitProducerIdResponse;
    use kafka_protocol::protocol::StrBytes;

    let server = TestServer::start();

    let mut init_req = InitProducerIdRequest::default();
    init_req.transactional_id = Some(kafka_protocol::messages::TransactionalId(
        StrBytes::from_string("txn-end-basic".to_string()),
    ));
    init_req.transaction_timeout_ms = 60000;
    let init_resp: InitProducerIdResponse = send_request(&server, 22, 0, &init_req);
    assert_eq!(init_resp.error_code, 0);

    let mut end_req = EndTxnRequest::default();
    end_req.transactional_id =
        kafka_protocol::messages::TransactionalId(StrBytes::from_string("txn-end-basic".to_string()));
    end_req.producer_id = init_resp.producer_id;
    end_req.producer_epoch = init_resp.producer_epoch;
    end_req.committed = true;

    let end_resp: EndTxnResponse = send_request(&server, 26, 0, &end_req);
    assert_eq!(end_resp.error_code, 0, "EndTxn commit should succeed");
}

// @covers US-004 AddOffsetsToTxn basic flow
#[test]
fn contract_add_offsets_to_txn_basic() {
    use kafka_protocol::messages::add_offsets_to_txn_request::AddOffsetsToTxnRequest;
    use kafka_protocol::messages::add_offsets_to_txn_response::AddOffsetsToTxnResponse;
    use kafka_protocol::messages::init_producer_id_request::InitProducerIdRequest;
    use kafka_protocol::messages::init_producer_id_response::InitProducerIdResponse;
    use kafka_protocol::protocol::StrBytes;

    let server = TestServer::start();

    let mut init_req = InitProducerIdRequest::default();
    init_req.transactional_id = Some(kafka_protocol::messages::TransactionalId(
        StrBytes::from_string("txn-add-offsets".to_string()),
    ));
    init_req.transaction_timeout_ms = 60000;
    let init_resp: InitProducerIdResponse = send_request(&server, 22, 0, &init_req);
    assert_eq!(init_resp.error_code, 0);

    let mut req = AddOffsetsToTxnRequest::default();
    req.transactional_id =
        kafka_protocol::messages::TransactionalId(StrBytes::from_string("txn-add-offsets".to_string()));
    req.producer_id = init_resp.producer_id;
    req.producer_epoch = init_resp.producer_epoch;
    req.group_id = kafka_protocol::messages::GroupId(StrBytes::from_string("test-group".to_string()));

    let resp: AddOffsetsToTxnResponse = send_request(&server, 25, 0, &req);
    assert_eq!(resp.error_code, 0, "AddOffsetsToTxn should succeed");
}

// @covers US-004 TxnOffsetCommit basic flow
#[test]
fn contract_txn_offset_commit_basic() {
    use kafka_protocol::messages::add_offsets_to_txn_request::AddOffsetsToTxnRequest;
    use kafka_protocol::messages::add_offsets_to_txn_response::AddOffsetsToTxnResponse;
    use kafka_protocol::messages::init_producer_id_request::InitProducerIdRequest;
    use kafka_protocol::messages::init_producer_id_response::InitProducerIdResponse;
    use kafka_protocol::messages::txn_offset_commit_request::{
        TxnOffsetCommitRequest, TxnOffsetCommitRequestPartition, TxnOffsetCommitRequestTopic,
    };
    use kafka_protocol::messages::txn_offset_commit_response::TxnOffsetCommitResponse;
    use kafka_protocol::protocol::StrBytes;

    let server = TestServer::start();
    let topic = unique_topic("contract-txn-offset");
    create_topic(&server, &topic, 1);

    let mut init_req = InitProducerIdRequest::default();
    init_req.transactional_id = Some(kafka_protocol::messages::TransactionalId(
        StrBytes::from_string("txn-offset-commit".to_string()),
    ));
    init_req.transaction_timeout_ms = 60000;
    let init_resp: InitProducerIdResponse = send_request(&server, 22, 0, &init_req);
    assert_eq!(init_resp.error_code, 0);

    // AddOffsetsToTxn moves the transaction to Ongoing, required before TxnOffsetCommit.
    let mut add_offsets_req = AddOffsetsToTxnRequest::default();
    add_offsets_req.transactional_id =
        kafka_protocol::messages::TransactionalId(StrBytes::from_string("txn-offset-commit".to_string()));
    add_offsets_req.producer_id = init_resp.producer_id;
    add_offsets_req.producer_epoch = init_resp.producer_epoch;
    add_offsets_req.group_id =
        kafka_protocol::messages::GroupId(StrBytes::from_string("test-group".to_string()));
    let add_offsets_resp: AddOffsetsToTxnResponse = send_request(&server, 25, 0, &add_offsets_req);
    assert_eq!(add_offsets_resp.error_code, 0, "AddOffsetsToTxn must succeed");

    let mut part = TxnOffsetCommitRequestPartition::default();
    part.partition_index = 0;
    part.committed_offset = 10;

    let mut txn_topic = TxnOffsetCommitRequestTopic::default();
    txn_topic.name = TopicName(StrBytes::from_string(topic.clone()));
    txn_topic.partitions = vec![part];

    let mut req = TxnOffsetCommitRequest::default();
    req.transactional_id =
        kafka_protocol::messages::TransactionalId(StrBytes::from_string("txn-offset-commit".to_string()));
    req.group_id = kafka_protocol::messages::GroupId(StrBytes::from_string("test-group".to_string()));
    req.producer_id = init_resp.producer_id;
    req.producer_epoch = init_resp.producer_epoch;
    req.topics = vec![txn_topic];

    let resp: TxnOffsetCommitResponse = send_request(&server, 28, 0, &req);
    assert_eq!(
        resp.topics[0].partitions[0].error_code,
        0,
        "TxnOffsetCommit should succeed"
    );
}

// @covers US-004 WriteTxnMarkers basic flow
#[test]
fn contract_write_txn_markers_basic() {
    use kafka_protocol::messages::init_producer_id_request::InitProducerIdRequest;
    use kafka_protocol::messages::init_producer_id_response::InitProducerIdResponse;
    use kafka_protocol::messages::write_txn_markers_request::{
        WriteTxnMarkersRequest, WritableTxnMarker, WritableTxnMarkerTopic,
    };
    use kafka_protocol::messages::write_txn_markers_response::WriteTxnMarkersResponse;
    use kafka_protocol::protocol::StrBytes;

    let server = TestServer::start();
    let topic = unique_topic("contract-txn-markers");
    create_topic(&server, &topic, 1);

    let init_resp: InitProducerIdResponse =
        send_request(&server, 22, 0, &InitProducerIdRequest::default());
    assert_eq!(init_resp.error_code, 0);

    let mut marker_topic = WritableTxnMarkerTopic::default();
    marker_topic.name = TopicName(StrBytes::from_string(topic.clone()));
    marker_topic.partition_indexes = vec![0];

    let mut marker = WritableTxnMarker::default();
    marker.producer_id = init_resp.producer_id;
    marker.producer_epoch = init_resp.producer_epoch;
    marker.transaction_result = true;
    marker.topics = vec![marker_topic];

    let mut req = WriteTxnMarkersRequest::default();
    req.markers = vec![marker];

    let resp: WriteTxnMarkersResponse = send_request(&server, 27, 0, &req);
    assert_eq!(
        resp.markers[0].topics[0].partitions[0].error_code,
        0,
        "WriteTxnMarkers should succeed"
    );
}

// @covers US-004-AC6
#[test]
fn contract_transactional_produce_commit_fetch() {
    use kafka_protocol::messages::add_partitions_to_txn_request::{
        AddPartitionsToTxnRequest, AddPartitionsToTxnTopic,
    };
    use kafka_protocol::messages::add_partitions_to_txn_response::AddPartitionsToTxnResponse;
    use kafka_protocol::messages::end_txn_request::EndTxnRequest;
    use kafka_protocol::messages::end_txn_response::EndTxnResponse;
    use kafka_protocol::messages::init_producer_id_request::InitProducerIdRequest;
    use kafka_protocol::messages::init_producer_id_response::InitProducerIdResponse;
    use kafka_protocol::messages::produce_request::{PartitionProduceData, TopicProduceData};
    use kafka_protocol::messages::ProduceRequest;
    use kafka_protocol::messages::ProduceResponse;
    use kafka_protocol::protocol::StrBytes;

    let server = TestServer::start();
    let topic = unique_topic("contract-txn-e2e");
    create_topic(&server, &topic, 1);

    // Step 1: InitProducerId
    let mut init_req = InitProducerIdRequest::default();
    init_req.transactional_id = Some(kafka_protocol::messages::TransactionalId(
        StrBytes::from_string("txn-e2e".to_string()),
    ));
    init_req.transaction_timeout_ms = 60000;
    let init_resp: InitProducerIdResponse = send_request(&server, 22, 0, &init_req);
    assert_eq!(init_resp.error_code, 0);
    let pid = init_resp.producer_id.0;
    let epoch = init_resp.producer_epoch;

    // Step 2: AddPartitionsToTxn
    let mut add_topic = AddPartitionsToTxnTopic::default();
    add_topic.name = TopicName(StrBytes::from_string(topic.clone()));
    add_topic.partitions = vec![0];

    let mut add_req = AddPartitionsToTxnRequest::default();
    add_req.v3_and_below_transactional_id =
        kafka_protocol::messages::TransactionalId(StrBytes::from_string("txn-e2e".to_string()));
    add_req.v3_and_below_producer_id = init_resp.producer_id;
    add_req.v3_and_below_producer_epoch = epoch;
    add_req.v3_and_below_topics = vec![add_topic];

    let add_resp: AddPartitionsToTxnResponse = send_request(&server, 24, 0, &add_req);
    assert_eq!(
        add_resp.results_by_topic_v3_and_below[0].results_by_partition[0].partition_error_code,
        0
    );

    // Step 3: Transactional produce
    let txn_record = Record {
        transactional: true,
        control: false,
        partition_leader_epoch: 0,
        producer_id: pid,
        producer_epoch: epoch,
        timestamp_type: TimestampType::Creation,
        timestamp: 0,
        sequence: 0,
        offset: 0,
        key: Some("txn-key".into()),
        value: Some("txn-val".into()),
        headers: Default::default(),
    };

    let mut txn_batch_buf = BytesMut::new();
    RecordBatchEncoder::encode(
        &mut txn_batch_buf,
        &[txn_record],
        &RecordEncodeOptions { version: 2, compression: Compression::None },
    ).expect("encode txn batch");

    let mut partition_data = PartitionProduceData::default();
    partition_data.index = 0;
    partition_data.records = Some(txn_batch_buf.freeze());

    let mut topic_data = TopicProduceData::default();
    topic_data.name = TopicName(StrBytes::from_string(topic.clone()));
    topic_data.partition_data = vec![partition_data];

    let mut produce_req = ProduceRequest::default();
    produce_req.acks = 1;
    produce_req.timeout_ms = 1000;
    produce_req.transactional_id = Some(kafka_protocol::messages::TransactionalId(
        StrBytes::from_string("txn-e2e".to_string()),
    ));
    produce_req.topic_data = vec![topic_data];

    // transactional_id requires Produce v3+
    let produce_resp: ProduceResponse = send_request(&server, 0, 3, &produce_req);
    assert_eq!(produce_resp.responses[0].partition_responses[0].error_code, 0, "transactional produce should succeed");

    // Step 4: EndTxn (commit)
    let mut end_req = EndTxnRequest::default();
    end_req.transactional_id =
        kafka_protocol::messages::TransactionalId(StrBytes::from_string("txn-e2e".to_string()));
    end_req.producer_id = init_resp.producer_id;
    end_req.producer_epoch = epoch;
    end_req.committed = true;
    let end_resp: EndTxnResponse = send_request(&server, 26, 0, &end_req);
    assert_eq!(end_resp.error_code, 0, "EndTxn commit should succeed");

    // Step 5: Fetch and verify committed record is readable
    let mut fetch_partition = FetchPartition::default();
    fetch_partition.partition = 0;
    fetch_partition.fetch_offset = 0;
    fetch_partition.partition_max_bytes = 1024 * 1024;

    let mut fetch_topic = FetchTopic::default();
    fetch_topic.topic = TopicName(StrBytes::from_string(topic.clone()));
    fetch_topic.partitions = vec![fetch_partition];

    let mut fetch_req = FetchRequest::default();
    fetch_req.replica_id = BrokerId(-1);
    fetch_req.max_wait_ms = 0;
    fetch_req.min_bytes = 1;
    fetch_req.max_bytes = 1024 * 1024;
    fetch_req.topics = vec![fetch_topic];

    let fetch_resp: FetchResponse = send_request(&server, 1, 3, &fetch_req);
    let part = &fetch_resp.responses[0].partitions[0];
    assert_eq!(part.error_code, 0, "fetch should succeed");
    let mut raw = part.records.clone().expect("records present");
    let batches = RecordBatchDecoder::decode_all(&mut raw).expect("decode batches");
    let records: Vec<Record> = batches.into_iter().flat_map(|b| b.records).collect();
    assert!(!records.is_empty(), "committed transactional records must be fetchable");
    assert_eq!(records[0].value, Some(bytes::Bytes::from("txn-val")));
}

// @covers US-003-AC4 (duplicate sequence)
#[test]
fn contract_duplicate_sequence_returns_error() {
    use kafka_protocol::messages::init_producer_id_request::InitProducerIdRequest;
    use kafka_protocol::messages::init_producer_id_response::InitProducerIdResponse;

    let server = TestServer::start();
    let topic = unique_topic("contract-idempotent-dup");

    let pid_resp: InitProducerIdResponse =
        send_request(&server, 22, 0, &InitProducerIdRequest::default());
    assert_eq!(pid_resp.error_code, 0);
    let producer_id = pid_resp.producer_id.0;

    let encode_batch = |seq: i64| {
        let mut buf = BytesMut::new();
        RecordBatchEncoder::encode(
            &mut buf,
            &[new_idempotent_record(seq, producer_id, 0)],
            &RecordEncodeOptions { version: 2, compression: Compression::None },
        )
        .expect("encode batch");
        buf.freeze()
    };

    // First produce at sequence 0: should succeed.
    let resp0 = produce_batch(&server, &topic, encode_batch(0));
    assert_eq!(resp0.responses[0].partition_responses[0].error_code, 0, "first batch should succeed");

    // Retry with same sequence 0: should return DUPLICATE_SEQUENCE_NUMBER.
    let resp_dup = produce_batch(&server, &topic, encode_batch(0));
    assert_eq!(
        resp_dup.responses[0].partition_responses[0].error_code,
        46, // DUPLICATE_SEQUENCE_NUMBER
        "duplicate sequence must return error 46"
    );
}

// @covers US-004 (stale producer epoch fencing)
#[test]
fn contract_stale_epoch_returns_invalid_producer_epoch() {
    use kafka_protocol::messages::add_partitions_to_txn_request::{
        AddPartitionsToTxnRequest, AddPartitionsToTxnTopic,
    };
    use kafka_protocol::messages::add_partitions_to_txn_response::AddPartitionsToTxnResponse;
    use kafka_protocol::messages::init_producer_id_request::InitProducerIdRequest;
    use kafka_protocol::messages::init_producer_id_response::InitProducerIdResponse;
    use kafka_protocol::protocol::StrBytes;

    let server = TestServer::start();
    let topic = unique_topic("contract-stale-epoch");
    create_topic(&server, &topic, 1);

    let mut init_req = InitProducerIdRequest::default();
    init_req.transactional_id = Some(kafka_protocol::messages::TransactionalId(
        StrBytes::from_string("txn-stale-epoch".to_string()),
    ));
    init_req.transaction_timeout_ms = 60000;

    // First init: epoch=0
    let resp0: InitProducerIdResponse = send_request(&server, 22, 0, &init_req);
    assert_eq!(resp0.error_code, 0);
    let stale_epoch = resp0.producer_epoch;
    assert_eq!(stale_epoch, 0);

    // Re-init: epoch bumped to 1 (fences the old epoch).
    let resp1: InitProducerIdResponse = send_request(&server, 22, 0, &init_req);
    assert_eq!(resp1.error_code, 0);
    assert_eq!(resp1.producer_epoch, 1, "epoch must be bumped on re-init");

    // Try AddPartitionsToTxn with the stale epoch: must return INVALID_PRODUCER_EPOCH.
    let mut add_topic = AddPartitionsToTxnTopic::default();
    add_topic.name = TopicName(StrBytes::from_string(topic.clone()));
    add_topic.partitions = vec![0];

    let mut add_req = AddPartitionsToTxnRequest::default();
    add_req.v3_and_below_transactional_id =
        kafka_protocol::messages::TransactionalId(StrBytes::from_string("txn-stale-epoch".to_string()));
    add_req.v3_and_below_producer_id = resp0.producer_id;
    add_req.v3_and_below_producer_epoch = stale_epoch; // stale: 0 instead of 1
    add_req.v3_and_below_topics = vec![add_topic];

    let add_resp: AddPartitionsToTxnResponse = send_request(&server, 24, 0, &add_req);
    assert_eq!(
        add_resp.results_by_topic_v3_and_below[0].results_by_partition[0].partition_error_code,
        47, // INVALID_PRODUCER_EPOCH
        "stale epoch must be rejected with error 47"
    );
}

// @covers US-004 (aborted transaction invisible to read_committed)
#[test]
fn contract_aborted_transaction_invisible_to_read_committed() {
    use kafka_protocol::messages::add_partitions_to_txn_request::{
        AddPartitionsToTxnRequest, AddPartitionsToTxnTopic,
    };
    use kafka_protocol::messages::add_partitions_to_txn_response::AddPartitionsToTxnResponse;
    use kafka_protocol::messages::end_txn_request::EndTxnRequest;
    use kafka_protocol::messages::end_txn_response::EndTxnResponse;
    use kafka_protocol::messages::init_producer_id_request::InitProducerIdRequest;
    use kafka_protocol::messages::init_producer_id_response::InitProducerIdResponse;
    use kafka_protocol::messages::produce_request::{PartitionProduceData, TopicProduceData};
    use kafka_protocol::protocol::StrBytes;

    let server = TestServer::start();
    let topic = unique_topic("contract-txn-abort");
    create_topic(&server, &topic, 1);

    // InitProducerId
    let mut init_req = InitProducerIdRequest::default();
    init_req.transactional_id = Some(kafka_protocol::messages::TransactionalId(
        StrBytes::from_string("txn-abort-test".to_string()),
    ));
    init_req.transaction_timeout_ms = 60000;
    let init_resp: InitProducerIdResponse = send_request(&server, 22, 0, &init_req);
    assert_eq!(init_resp.error_code, 0);
    let pid = init_resp.producer_id.0;
    let epoch = init_resp.producer_epoch;

    // AddPartitionsToTxn
    let mut add_topic = AddPartitionsToTxnTopic::default();
    add_topic.name = TopicName(StrBytes::from_string(topic.clone()));
    add_topic.partitions = vec![0];
    let mut add_req = AddPartitionsToTxnRequest::default();
    add_req.v3_and_below_transactional_id =
        kafka_protocol::messages::TransactionalId(StrBytes::from_string("txn-abort-test".to_string()));
    add_req.v3_and_below_producer_id = init_resp.producer_id;
    add_req.v3_and_below_producer_epoch = epoch;
    add_req.v3_and_below_topics = vec![add_topic];
    let add_resp: AddPartitionsToTxnResponse = send_request(&server, 24, 0, &add_req);
    assert_eq!(add_resp.results_by_topic_v3_and_below[0].results_by_partition[0].partition_error_code, 0);

    // Transactional produce
    let txn_record = Record {
        transactional: true,
        control: false,
        partition_leader_epoch: 0,
        producer_id: pid,
        producer_epoch: epoch,
        timestamp_type: TimestampType::Creation,
        timestamp: 0,
        sequence: 0,
        offset: 0,
        key: Some("abort-key".into()),
        value: Some("abort-val".into()),
        headers: Default::default(),
    };
    let mut txn_buf = BytesMut::new();
    RecordBatchEncoder::encode(
        &mut txn_buf,
        &[txn_record],
        &RecordEncodeOptions { version: 2, compression: Compression::None },
    ).expect("encode txn batch");

    let mut pdata = PartitionProduceData::default();
    pdata.index = 0;
    pdata.records = Some(txn_buf.freeze());
    let mut tdata = TopicProduceData::default();
    tdata.name = TopicName(StrBytes::from_string(topic.clone()));
    tdata.partition_data = vec![pdata];
    let mut produce_req = ProduceRequest::default();
    produce_req.acks = 1;
    produce_req.timeout_ms = 1000;
    produce_req.transactional_id = Some(kafka_protocol::messages::TransactionalId(
        StrBytes::from_string("txn-abort-test".to_string()),
    ));
    produce_req.topic_data = vec![tdata];
    let produce_resp: ProduceResponse = send_request(&server, 0, 3, &produce_req);
    assert_eq!(produce_resp.responses[0].partition_responses[0].error_code, 0);

    // EndTxn with committed=false (ABORT)
    let mut end_req = EndTxnRequest::default();
    end_req.transactional_id =
        kafka_protocol::messages::TransactionalId(StrBytes::from_string("txn-abort-test".to_string()));
    end_req.producer_id = init_resp.producer_id;
    end_req.producer_epoch = epoch;
    end_req.committed = false;
    let end_resp: EndTxnResponse = send_request(&server, 26, 0, &end_req);
    assert_eq!(end_resp.error_code, 0, "EndTxn abort should succeed");

    // Fetch with isolation_level=1 (READ_COMMITTED): aborted record must be invisible.
    let mut fetch_partition = FetchPartition::default();
    fetch_partition.partition = 0;
    fetch_partition.fetch_offset = 0;
    fetch_partition.partition_max_bytes = 1024 * 1024;
    let mut fetch_topic = FetchTopic::default();
    fetch_topic.topic = TopicName(StrBytes::from_string(topic.clone()));
    fetch_topic.partitions = vec![fetch_partition];
    let mut fetch_req = FetchRequest::default();
    fetch_req.replica_id = BrokerId(-1);
    fetch_req.max_wait_ms = 0;
    fetch_req.min_bytes = 0;
    fetch_req.max_bytes = 1024 * 1024;
    fetch_req.isolation_level = 1; // READ_COMMITTED
    fetch_req.topics = vec![fetch_topic];

    let fetch_resp: FetchResponse = send_request(&server, 1, 4, &fetch_req);
    let part = &fetch_resp.responses[0].partitions[0];
    assert_eq!(part.error_code, 0);
    // For read_committed, the server populates `aborted_transactions` so
    // Kafka consumer libraries can filter aborted records client-side.
    // The raw record bytes are still returned (the protocol contract);
    // what must be non-null is the aborted_transactions list.
    let aborted = part.aborted_transactions.as_deref().unwrap_or_default();
    assert!(
        aborted.iter().any(|a| a.producer_id.0 == pid),
        "read_committed response must include aborted_transactions entry for producer_id={}; got {:?}",
        pid, aborted.iter().map(|a| a.producer_id.0).collect::<Vec<_>>()
    );
}

// @covers US-004
#[test]
fn contract_read_uncommitted_observes_aborted_records() {
    use kafka_protocol::messages::add_partitions_to_txn_request::{
        AddPartitionsToTxnRequest, AddPartitionsToTxnTopic,
    };
    use kafka_protocol::messages::add_partitions_to_txn_response::AddPartitionsToTxnResponse;
    use kafka_protocol::messages::end_txn_request::EndTxnRequest;
    use kafka_protocol::messages::end_txn_response::EndTxnResponse;
    use kafka_protocol::messages::init_producer_id_request::InitProducerIdRequest;
    use kafka_protocol::messages::init_producer_id_response::InitProducerIdResponse;
    use kafka_protocol::messages::produce_request::{PartitionProduceData, TopicProduceData};
    use kafka_protocol::protocol::StrBytes;

    let server = TestServer::start();
    let topic = unique_topic("contract-txn-read-uncommitted");
    create_topic(&server, &topic, 1);

    // InitProducerId
    let mut init_req = InitProducerIdRequest::default();
    init_req.transactional_id = Some(kafka_protocol::messages::TransactionalId(
        StrBytes::from_string("txn-uncommitted-test".to_string()),
    ));
    init_req.transaction_timeout_ms = 60000;
    let init_resp: InitProducerIdResponse = send_request(&server, 22, 0, &init_req);
    assert_eq!(init_resp.error_code, 0);
    let pid = init_resp.producer_id.0;
    let epoch = init_resp.producer_epoch;

    // AddPartitionsToTxn
    let mut add_topic = AddPartitionsToTxnTopic::default();
    add_topic.name = TopicName(StrBytes::from_string(topic.clone()));
    add_topic.partitions = vec![0];
    let mut add_req = AddPartitionsToTxnRequest::default();
    add_req.v3_and_below_transactional_id =
        kafka_protocol::messages::TransactionalId(StrBytes::from_string("txn-uncommitted-test".to_string()));
    add_req.v3_and_below_producer_id = init_resp.producer_id;
    add_req.v3_and_below_producer_epoch = epoch;
    add_req.v3_and_below_topics = vec![add_topic];
    let add_resp: AddPartitionsToTxnResponse = send_request(&server, 24, 0, &add_req);
    assert_eq!(add_resp.results_by_topic_v3_and_below[0].results_by_partition[0].partition_error_code, 0);

    // Transactional produce
    let txn_record = Record {
        transactional: true,
        control: false,
        partition_leader_epoch: 0,
        producer_id: pid,
        producer_epoch: epoch,
        timestamp_type: TimestampType::Creation,
        timestamp: 0,
        sequence: 0,
        offset: 0,
        key: Some("uncommit-key".into()),
        value: Some("uncommit-val".into()),
        headers: Default::default(),
    };
    let mut txn_buf = BytesMut::new();
    RecordBatchEncoder::encode(
        &mut txn_buf,
        &[txn_record],
        &RecordEncodeOptions { version: 2, compression: Compression::None },
    ).expect("encode txn batch");

    let mut pdata = PartitionProduceData::default();
    pdata.index = 0;
    pdata.records = Some(txn_buf.freeze());
    let mut tdata = TopicProduceData::default();
    tdata.name = TopicName(StrBytes::from_string(topic.clone()));
    tdata.partition_data = vec![pdata];
    let mut produce_req = ProduceRequest::default();
    produce_req.acks = 1;
    produce_req.timeout_ms = 1000;
    produce_req.transactional_id = Some(kafka_protocol::messages::TransactionalId(
        StrBytes::from_string("txn-uncommitted-test".to_string()),
    ));
    produce_req.topic_data = vec![tdata];
    let produce_resp: ProduceResponse = send_request(&server, 0, 3, &produce_req);
    assert_eq!(produce_resp.responses[0].partition_responses[0].error_code, 0);

    // EndTxn ABORT
    let mut end_req = EndTxnRequest::default();
    end_req.transactional_id =
        kafka_protocol::messages::TransactionalId(StrBytes::from_string("txn-uncommitted-test".to_string()));
    end_req.producer_id = init_resp.producer_id;
    end_req.producer_epoch = epoch;
    end_req.committed = false;
    let end_resp: EndTxnResponse = send_request(&server, 26, 0, &end_req);
    assert_eq!(end_resp.error_code, 0);

    // Fetch with isolation_level=0 (READ_UNCOMMITTED): records are visible,
    // aborted_transactions list is empty (no filtering hint needed).
    let mut fetch_partition = FetchPartition::default();
    fetch_partition.partition = 0;
    fetch_partition.fetch_offset = 0;
    fetch_partition.partition_max_bytes = 1024 * 1024;
    let mut fetch_topic = FetchTopic::default();
    fetch_topic.topic = TopicName(StrBytes::from_string(topic.clone()));
    fetch_topic.partitions = vec![fetch_partition];
    let mut fetch_req = FetchRequest::default();
    fetch_req.replica_id = BrokerId(-1);
    fetch_req.max_wait_ms = 0;
    fetch_req.min_bytes = 0;
    fetch_req.max_bytes = 1024 * 1024;
    fetch_req.isolation_level = 0; // READ_UNCOMMITTED
    fetch_req.topics = vec![fetch_topic];

    let fetch_resp: FetchResponse = send_request(&server, 1, 4, &fetch_req);
    let part = &fetch_resp.responses[0].partitions[0];
    assert_eq!(part.error_code, 0);
    // READ_UNCOMMITTED: aborted_transactions must be empty (no filtering hints).
    let aborted = part.aborted_transactions.as_deref().unwrap_or_default();
    assert!(
        aborted.is_empty(),
        "read_uncommitted response must have empty aborted_transactions; got {:?}",
        aborted.iter().map(|a| a.producer_id.0).collect::<Vec<_>>()
    );
}

/// Verify that a member whose session_timeout_ms expires is evicted by the
/// background eviction task and subsequent heartbeats return an error.
#[test]
fn contract_session_timeout_evicts_member() {
    let server = TestServer::start();
    let group = unique_group("contract-session-timeout");

    // Join group with a very short session timeout (800 ms)
    let mut protocol = JoinGroupRequestProtocol::default();
    protocol.name = StrBytes::from_string("range".to_string());
    protocol.metadata = bytes::Bytes::from(vec![0, 1, 0, 0, 0, 0]);

    let mut join_request = JoinGroupRequest::default();
    join_request.group_id = GroupId(StrBytes::from_string(group.clone()));
    join_request.session_timeout_ms = 800;
    join_request.rebalance_timeout_ms = 800;
    join_request.member_id = StrBytes::from_string(String::new());
    join_request.protocol_type = StrBytes::from_string("consumer".to_string());
    join_request.protocols = vec![protocol.clone()];

    // First join: get member_id
    let join_resp: JoinGroupResponse = send_request(&server, 11, 1, &join_request);
    assert_eq!(join_resp.error_code, 79, "expected MEMBER_ID_REQUIRED");
    let member_id = join_resp.member_id.clone();

    // Second join: actual join
    join_request.member_id = member_id.clone();
    let join_resp: JoinGroupResponse = send_request(&server, 11, 1, &join_request);
    assert_eq!(join_resp.error_code, 0, "JoinGroup should succeed");
    let generation_id = join_resp.generation_id;

    // Immediately heartbeat — should succeed
    let mut hb = HeartbeatRequest::default();
    hb.group_id = GroupId(StrBytes::from_string(group.clone()));
    hb.generation_id = generation_id;
    hb.member_id = member_id.clone();
    let hb_resp: HeartbeatResponse = send_request(&server, 12, 1, &hb);
    assert_eq!(hb_resp.error_code, 0, "immediate heartbeat should succeed");

    // Wait for session_timeout to elapse (1.5× the timeout to account for
    // the background-eviction task's 500 ms tick interval)
    std::thread::sleep(std::time::Duration::from_millis(1500));

    // Heartbeat after expiry — should fail with a non-zero error code
    let hb_resp2: HeartbeatResponse = send_request(&server, 12, 1, &hb);
    assert_ne!(
        hb_resp2.error_code, 0,
        "heartbeat after session timeout must fail (member should have been evicted)"
    );
}
