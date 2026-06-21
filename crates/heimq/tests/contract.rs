//! Contract-level protocol tests against heimq.
//!
//! These tests use raw Kafka protocol requests to validate API compliance
//! for supported endpoints.

#![allow(clippy::uninlined_format_args)]

use bytes::{Buf, BufMut, BytesMut};
use heimq::protocol::{is_flexible, SUPPORTED_APIS};
use heimq::test_support::{unique_group, unique_topic, TestServer};
use kafka_protocol::messages::api_versions_request::ApiVersionsRequest;
use kafka_protocol::messages::api_versions_response::ApiVersionsResponse;
use kafka_protocol::messages::create_topics_request::{CreatableTopic, CreateTopicsRequest};
use kafka_protocol::messages::create_topics_response::CreateTopicsResponse;
use kafka_protocol::messages::delete_groups_request::DeleteGroupsRequest;
use kafka_protocol::messages::delete_groups_response::DeleteGroupsResponse;
use kafka_protocol::messages::delete_topics_request::DeleteTopicsRequest;
use kafka_protocol::messages::delete_topics_response::DeleteTopicsResponse;
use kafka_protocol::messages::describe_cluster_request::DescribeClusterRequest;
use kafka_protocol::messages::describe_cluster_response::DescribeClusterResponse;
use kafka_protocol::messages::describe_topic_partitions_request::{
    DescribeTopicPartitionsRequest, TopicRequest,
};
use kafka_protocol::messages::describe_topic_partitions_response::DescribeTopicPartitionsResponse;
use kafka_protocol::messages::elect_leaders_request::{
    ElectLeadersRequest, TopicPartitions as ElectLeadersTopicPartitions,
};
use kafka_protocol::messages::elect_leaders_response::ElectLeadersResponse;
use kafka_protocol::messages::fetch_request::{FetchPartition, FetchRequest, FetchTopic};
use kafka_protocol::messages::fetch_response::FetchResponse;
use kafka_protocol::messages::find_coordinator_request::FindCoordinatorRequest;
use kafka_protocol::messages::find_coordinator_response::FindCoordinatorResponse;
use kafka_protocol::messages::heartbeat_request::HeartbeatRequest;
use kafka_protocol::messages::heartbeat_response::HeartbeatResponse;
use kafka_protocol::messages::incremental_alter_configs_request::{
    AlterConfigsResource, AlterableConfig, IncrementalAlterConfigsRequest,
};
use kafka_protocol::messages::incremental_alter_configs_response::IncrementalAlterConfigsResponse;
use kafka_protocol::messages::join_group_request::{JoinGroupRequest, JoinGroupRequestProtocol};
use kafka_protocol::messages::join_group_response::JoinGroupResponse;
use kafka_protocol::messages::leave_group_request::LeaveGroupRequest;
use kafka_protocol::messages::leave_group_response::LeaveGroupResponse;
use kafka_protocol::messages::list_groups_request::ListGroupsRequest;
use kafka_protocol::messages::list_groups_response::ListGroupsResponse;
use kafka_protocol::messages::list_offsets_request::{
    ListOffsetsPartition, ListOffsetsRequest, ListOffsetsTopic,
};
use kafka_protocol::messages::list_offsets_response::ListOffsetsResponse;
use kafka_protocol::messages::list_transactions_request::ListTransactionsRequest;
use kafka_protocol::messages::list_transactions_response::ListTransactionsResponse;
use kafka_protocol::messages::metadata_request::{MetadataRequest, MetadataRequestTopic};
use kafka_protocol::messages::metadata_response::MetadataResponse;
use kafka_protocol::messages::offset_commit_request::{
    OffsetCommitRequest, OffsetCommitRequestPartition, OffsetCommitRequestTopic,
};
use kafka_protocol::messages::offset_commit_response::OffsetCommitResponse;
use kafka_protocol::messages::offset_fetch_request::{OffsetFetchRequest, OffsetFetchRequestTopic};
use kafka_protocol::messages::offset_fetch_response::OffsetFetchResponse;
use kafka_protocol::messages::offset_for_leader_epoch_request::{
    OffsetForLeaderEpochRequest, OffsetForLeaderPartition, OffsetForLeaderTopic,
};
use kafka_protocol::messages::offset_for_leader_epoch_response::OffsetForLeaderEpochResponse;
use kafka_protocol::messages::produce_request::{
    PartitionProduceData, ProduceRequest, TopicProduceData,
};
use kafka_protocol::messages::produce_response::ProduceResponse;
use kafka_protocol::messages::sync_group_request::{SyncGroupRequest, SyncGroupRequestAssignment};
use kafka_protocol::messages::sync_group_response::SyncGroupResponse;
use kafka_protocol::messages::{BrokerId, GroupId, TopicName};
use kafka_protocol::protocol::{Decodable, Encodable, StrBytes};
use kafka_protocol::records::{
    Compression, Record, RecordBatchDecoder, RecordBatchEncoder, RecordEncodeOptions, TimestampType,
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

    // Flexible request headers (v2) require an empty tagged-fields block after client_id.
    if is_flexible(api_key, api_version) {
        body.put_u8(0x00);
    }

    request
        .encode(&mut body, api_version)
        .expect("encode request");

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

    let mut stream =
        std::net::TcpStream::connect(("127.0.0.1", server.port)).expect("connect to heimq");
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
    assert_eq!(
        response_correlation_id, correlation_id,
        "correlation id mismatch"
    );

    // Flexible response header v1 has an empty tagged-fields block after correlation_id.
    // ApiVersions always uses header v0 (no tag buffer) even at flexible versions.
    if api_key != 18 && is_flexible(api_key, api_version) {
        cursor.advance(1); // skip the 0x00 empty tag buffer byte
    }

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

fn new_record_with_payload(offset: i64, timestamp_ms: i64, payload: &str) -> Record {
    Record {
        transactional: false,
        control: false,
        partition_leader_epoch: 0,
        producer_id: -1,
        producer_epoch: -1,
        timestamp_type: TimestampType::Creation,
        timestamp: timestamp_ms,
        sequence: offset as i32,
        offset,
        key: Some(format!("key-{offset}").into()),
        value: Some(payload.to_string().into()),
        headers: Default::default(),
    }
}

fn produce_records(server: &TestServer, topic: &str, records: &[Record]) -> ProduceResponse {
    produce_batch(server, topic, encode_record_batch(records))
}

fn fetch_records_from(server: &TestServer, topic: &str, offset: i64) -> (i16, Vec<Record>, i64) {
    let mut fetch_partition = FetchPartition::default();
    fetch_partition.partition = 0;
    fetch_partition.fetch_offset = offset;
    fetch_partition.partition_max_bytes = 1024 * 1024;

    let mut fetch_topic = FetchTopic::default();
    fetch_topic.topic = TopicName(StrBytes::from_string(topic.to_string()));
    fetch_topic.partitions = vec![fetch_partition];

    let mut fetch_request = FetchRequest::default();
    fetch_request.replica_id = BrokerId(-1);
    fetch_request.max_wait_ms = 1000;
    fetch_request.min_bytes = 1;
    fetch_request.max_bytes = 1024 * 1024;
    fetch_request.topics = vec![fetch_topic];

    let fetch_response: FetchResponse = send_request(server, 1, 3, &fetch_request);
    let partition_response = &fetch_response.responses[0].partitions[0];
    let mut records = Vec::new();
    if let Some(mut raw) = partition_response.records.clone() {
        if !raw.is_empty() {
            let batches = RecordBatchDecoder::decode_all(&mut raw).expect("decode fetched");
            records = batches.into_iter().flat_map(|b| b.records).collect();
        }
    }
    (
        partition_response.error_code,
        records,
        partition_response.high_watermark,
    )
}

fn list_offset(server: &TestServer, topic: &str, timestamp: i64) -> i64 {
    let mut partition = ListOffsetsPartition::default();
    partition.partition_index = 0;
    partition.timestamp = timestamp;
    partition.max_num_offsets = 1;

    let mut topic_req = ListOffsetsTopic::default();
    topic_req.name = TopicName(StrBytes::from_string(topic.to_string()));
    topic_req.partitions = vec![partition];

    let mut request = ListOffsetsRequest::default();
    request.replica_id = BrokerId(-1);
    request.topics = vec![topic_req];

    let response: ListOffsetsResponse = send_request(server, 2, 1, &request);
    let partition_response = &response.topics[0].partitions[0];
    assert_eq!(partition_response.error_code, 0);
    partition_response.offset
}

fn set_topic_config(server: &TestServer, topic: &str, key: &str, value: &str) -> i16 {
    let mut config = AlterableConfig::default();
    config.name = StrBytes::from_string(key.to_string());
    config.config_operation = 0; // SET
    config.value = Some(StrBytes::from_string(value.to_string()));

    let mut resource = AlterConfigsResource::default();
    resource.resource_type = 2; // TOPIC
    resource.resource_name = StrBytes::from_string(topic.to_string());
    resource.configs = vec![config];

    let mut request = IncrementalAlterConfigsRequest::default();
    request.resources = vec![resource];
    request.validate_only = false;

    let response: IncrementalAlterConfigsResponse = send_request(server, 44, 0, &request);
    response.responses[0].error_code
}

// @covers US-001-AC1 US-013-AC2
#[test]
fn contract_api_versions_matches_supported_range() {
    let server = TestServer::start();
    let response: ApiVersionsResponse =
        send_request(&server, 18, 0, &ApiVersionsRequest::default());

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
    assert!(
        response.cluster_id.is_some(),
        "cluster id should be set for v4"
    );

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
    assert_eq!(
        produce_response.responses[0].partition_responses[0].error_code,
        0
    );
    let produce_response = produce_batch(&server, &topic, encoded_second);
    assert_eq!(
        produce_response.responses[0].partition_responses[0].error_code,
        0
    );

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
fn memory_bound_unexpired_data_backpressures_produce() {
    let now_ms = chrono::Utc::now().timestamp_millis();
    let first_records = vec![new_record_with_payload(0, now_ms, &"a".repeat(4096))];
    let first_batch = encode_record_batch(&first_records);
    let cap = first_batch.len() as u64 + 64;

    let server = TestServer::start_with_memory_bounds(60_000, cap);
    let topic = unique_topic("memory-bound-full");
    assert_eq!(create_topic(&server, &topic, 1).topics[0].error_code, 0);

    let first_response = produce_batch(&server, &topic, first_batch);
    assert_eq!(
        first_response.responses[0].partition_responses[0].error_code,
        0
    );

    let second_records = vec![new_record_with_payload(1, now_ms + 1, &"b".repeat(4096))];
    let second_response = produce_records(&server, &topic, &second_records);
    assert_eq!(
        second_response.responses[0].partition_responses[0].error_code, 56,
        "unexpired data must not be evicted to satisfy the memory cap"
    );

    let (error_code, fetched, high_watermark) = fetch_records_from(&server, &topic, 0);
    assert_eq!(error_code, 0);
    assert_eq!(high_watermark, 1);
    assert_eq!(fetched, first_records);
}

#[test]
fn memory_bound_expired_data_is_reclaimed_for_later_produce_fetch_flow() {
    let now_ms = chrono::Utc::now().timestamp_millis();
    let first_records = vec![new_record_with_payload(0, now_ms, &"expired".repeat(512))];
    let first_batch = encode_record_batch(&first_records);
    let cap = first_batch.len() as u64 + 64;

    let server = TestServer::start_with_memory_bounds(100, cap);
    let topic = unique_topic("memory-bound-expire");
    assert_eq!(create_topic(&server, &topic, 1).topics[0].error_code, 0);

    let first_response = produce_batch(&server, &topic, first_batch);
    assert_eq!(
        first_response.responses[0].partition_responses[0].error_code,
        0
    );

    std::thread::sleep(std::time::Duration::from_millis(800));

    let second_records = vec![new_record_with_payload(
        1,
        chrono::Utc::now().timestamp_millis(),
        &"current".repeat(512),
    )];
    let second_response = produce_records(&server, &topic, &second_records);
    assert_eq!(
        second_response.responses[0].partition_responses[0].error_code,
        0
    );

    let earliest = list_offset(&server, &topic, -2);
    assert_eq!(
        earliest, 1,
        "retention should advance the earliest readable offset"
    );
    let (error_code, fetched, high_watermark) = fetch_records_from(&server, &topic, earliest);
    assert_eq!(error_code, 0);
    assert_eq!(high_watermark, 2);
    assert_eq!(fetched, second_records);
}

#[test]
fn memory_bound_per_topic_retention_bytes_trims_consumer_visible_log() {
    let now_ms = chrono::Utc::now().timestamp_millis();
    let first_records = vec![new_record_with_payload(0, now_ms, &"topic-a".repeat(512))];
    let second_records = vec![new_record_with_payload(
        1,
        now_ms + 1,
        &"topic-b".repeat(512),
    )];
    let first_batch = encode_record_batch(&first_records);
    let second_batch = encode_record_batch(&second_records);
    let cap = (first_batch.len() + second_batch.len()) as u64 + 1024;

    let server = TestServer::start_with_memory_bounds(60_000, cap);
    let topic = unique_topic("memory-bound-topic-bytes");
    assert_eq!(create_topic(&server, &topic, 1).topics[0].error_code, 0);
    assert_eq!(
        set_topic_config(
            &server,
            &topic,
            "retention.bytes",
            &(first_batch.len() + 16).to_string()
        ),
        0
    );

    assert_eq!(
        produce_batch(&server, &topic, first_batch).responses[0].partition_responses[0].error_code,
        0
    );
    assert_eq!(
        produce_batch(&server, &topic, second_batch).responses[0].partition_responses[0].error_code,
        0
    );

    std::thread::sleep(std::time::Duration::from_millis(800));

    let earliest = list_offset(&server, &topic, -2);
    assert_eq!(earliest, 1, "retention.bytes should trim the oldest batch");
    assert_eq!(list_offset(&server, &topic, -1), 2);

    let (error_code, fetched, high_watermark) = fetch_records_from(&server, &topic, earliest);
    assert_eq!(error_code, 0);
    assert_eq!(high_watermark, 2);
    assert_eq!(fetched, second_records);
}

#[test]
fn memory_bound_retention_bytes_trims_before_storage_full() {
    let now_ms = chrono::Utc::now().timestamp_millis();
    let first_records = vec![new_record_with_payload(0, now_ms, &"first".repeat(512))];
    let second_records = vec![new_record_with_payload(
        1,
        now_ms + 1,
        &"second".repeat(512),
    )];
    let first_batch = encode_record_batch(&first_records);
    let second_batch = encode_record_batch(&second_records);
    let cap = second_batch.len() as u64 + 128;

    let server = TestServer::start_with_memory_bounds(60_000, cap);
    let topic = unique_topic("memory-bound-append-retention-bytes");
    assert_eq!(create_topic(&server, &topic, 1).topics[0].error_code, 0);
    assert_eq!(
        set_topic_config(
            &server,
            &topic,
            "retention.bytes",
            &(second_batch.len() + 16).to_string()
        ),
        0
    );

    assert_eq!(
        produce_batch(&server, &topic, first_batch).responses[0].partition_responses[0].error_code,
        0
    );
    assert_eq!(
        produce_batch(&server, &topic, second_batch).responses[0].partition_responses[0].error_code,
        0,
        "retention.bytes should trim before append admission returns storage-full"
    );

    let earliest = list_offset(&server, &topic, -2);
    assert_eq!(earliest, 1);
    let (error_code, fetched, high_watermark) = fetch_records_from(&server, &topic, earliest);
    assert_eq!(error_code, 0);
    assert_eq!(high_watermark, 2);
    assert_eq!(fetched, second_records);
}

#[test]
fn memory_bound_per_topic_retention_ms_protects_data_from_low_broker_default() {
    let now_ms = chrono::Utc::now().timestamp_millis();
    let first_records = vec![new_record_with_payload(0, now_ms, &"protected".repeat(512))];
    let second_records = vec![new_record_with_payload(
        1,
        now_ms + 2_000,
        &"blocked".repeat(512),
    )];
    let first_batch = encode_record_batch(&first_records);
    let cap = first_batch.len() as u64 + 64;

    let server = TestServer::start_with_memory_bounds(100, cap);
    let topic = unique_topic("memory-bound-topic-retention-ms");
    assert_eq!(create_topic(&server, &topic, 1).topics[0].error_code, 0);
    assert_eq!(
        set_topic_config(&server, &topic, "retention.ms", "60000"),
        0
    );

    assert_eq!(
        produce_batch(&server, &topic, first_batch).responses[0].partition_responses[0].error_code,
        0
    );

    std::thread::sleep(std::time::Duration::from_millis(250));

    let second_response = produce_records(&server, &topic, &second_records);
    assert_eq!(
        second_response.responses[0].partition_responses[0].error_code, 56,
        "topic retention.ms must keep the first batch protected even when broker default is low"
    );

    let earliest = list_offset(&server, &topic, -2);
    assert_eq!(earliest, 0);
    let (error_code, fetched, high_watermark) = fetch_records_from(&server, &topic, 0);
    assert_eq!(error_code, 0);
    assert_eq!(high_watermark, 1);
    assert_eq!(fetched, first_records);
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
    assert_eq!(
        response.node_id.0, 0,
        "node_id should be 0 for single broker"
    );
    assert!(
        response.host.as_str().contains("localhost")
            || response.host.as_str().contains("127.0.0.1"),
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
    assert_eq!(
        response.error_code, 79,
        "first join should return MEMBER_ID_REQUIRED"
    );
    assert!(
        !response.member_id.is_empty(),
        "should be assigned a member_id"
    );

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
    assert_eq!(
        response2.leader, assigned_member_id,
        "member should become leader"
    );
    assert!(
        response2.generation_id > 0,
        "generation_id should be positive"
    );
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

    // Complete the rebalance as leader so the member becomes active (group Stable).
    // A heartbeat *before* SyncGroup correctly returns REBALANCE_IN_PROGRESS (27),
    // so an "active member" must have finished syncing.
    let mut assignment = SyncGroupRequestAssignment::default();
    assignment.member_id = member_id.clone();
    assignment.assignment = bytes::Bytes::from(vec![0, 0, 0, 1, 0, 4, 116, 101, 115, 116]);
    let mut sync_request = SyncGroupRequest::default();
    sync_request.group_id = GroupId(StrBytes::from_string(group.clone()));
    sync_request.generation_id = generation_id;
    sync_request.member_id = member_id.clone();
    sync_request.assignments = vec![assignment];
    let sync_response: SyncGroupResponse = send_request(&server, 14, 1, &sync_request);
    assert_eq!(
        sync_response.error_code, 0,
        "leader sync should complete rebalance"
    );

    // Send heartbeat
    let mut heartbeat_request = HeartbeatRequest::default();
    heartbeat_request.group_id = GroupId(StrBytes::from_string(group));
    heartbeat_request.generation_id = generation_id;
    heartbeat_request.member_id = member_id;

    let heartbeat_response: HeartbeatResponse = send_request(&server, 12, 1, &heartbeat_request);
    assert_eq!(
        heartbeat_response.error_code, 0,
        "heartbeat should succeed for active member"
    );
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
    assert_eq!(
        find_response.error_code, 0,
        "FindCoordinator should succeed"
    );

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
    assert_eq!(
        join_response.error_code, 79,
        "First JoinGroup should return MEMBER_ID_REQUIRED"
    );
    let member_id = join_response.member_id.clone();

    // 3. JoinGroup (second call - become leader)
    join_request.member_id = member_id.clone();
    let join_response: JoinGroupResponse = send_request(&server, 11, 1, &join_request);
    assert_eq!(
        join_response.error_code, 0,
        "Second JoinGroup should succeed"
    );
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
    assert_eq!(
        commit_response.topics[0].partitions[0].error_code, 0,
        "OffsetCommit should succeed"
    );

    // 7. OffsetFetch
    let mut fetch_topic = OffsetFetchRequestTopic::default();
    fetch_topic.name = TopicName(StrBytes::from_string(topic));
    fetch_topic.partition_indexes = vec![0];

    let mut fetch_request = OffsetFetchRequest::default();
    fetch_request.group_id = GroupId(StrBytes::from_string(group.clone()));
    fetch_request.topics = Some(vec![fetch_topic]);

    let fetch_response: OffsetFetchResponse = send_request(&server, 9, 1, &fetch_request);
    assert_eq!(
        fetch_response.topics[0].partitions[0].error_code, 0,
        "OffsetFetch should succeed"
    );
    assert_eq!(
        fetch_response.topics[0].partitions[0].committed_offset, 100,
        "Committed offset should match"
    );

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
    assert!(
        response.producer_id.0 >= 0,
        "producer_id must be non-negative"
    );
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
            &RecordEncodeOptions {
                version: 2,
                compression: Compression::None,
            },
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
            &RecordEncodeOptions {
                version: 2,
                compression: Compression::None,
            },
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
    assert_eq!(
        response.error_code, 0,
        "InitProducerId with transactional_id should succeed"
    );
    assert!(
        response.producer_id.0 >= 0,
        "producer_id must be non-negative"
    );
    assert_eq!(response.producer_epoch, 0, "initial epoch must be 0");

    // Second call with same transactional_id should bump the epoch
    let response2: InitProducerIdResponse = send_request(&server, 22, 0, &request);
    assert_eq!(
        response2.error_code, 0,
        "Re-init with same transactional_id should succeed"
    );
    assert_eq!(
        response.producer_id.0, response2.producer_id.0,
        "same producer_id for same transactional_id"
    );
    assert_eq!(
        response2.producer_epoch, 1,
        "epoch should be bumped on re-init"
    );
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
    add_req.v3_and_below_transactional_id = kafka_protocol::messages::TransactionalId(
        StrBytes::from_string("txn-add-parts".to_string()),
    );
    add_req.v3_and_below_producer_id = producer_id;
    add_req.v3_and_below_producer_epoch = epoch;
    add_req.v3_and_below_topics = vec![add_topic];

    let add_resp: AddPartitionsToTxnResponse = send_request(&server, 24, 0, &add_req);
    assert_eq!(
        add_resp.results_by_topic_v3_and_below[0].results_by_partition[0].partition_error_code, 0,
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
    end_req.transactional_id = kafka_protocol::messages::TransactionalId(StrBytes::from_string(
        "txn-end-basic".to_string(),
    ));
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
    req.transactional_id = kafka_protocol::messages::TransactionalId(StrBytes::from_string(
        "txn-add-offsets".to_string(),
    ));
    req.producer_id = init_resp.producer_id;
    req.producer_epoch = init_resp.producer_epoch;
    req.group_id =
        kafka_protocol::messages::GroupId(StrBytes::from_string("test-group".to_string()));

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
    add_offsets_req.transactional_id = kafka_protocol::messages::TransactionalId(
        StrBytes::from_string("txn-offset-commit".to_string()),
    );
    add_offsets_req.producer_id = init_resp.producer_id;
    add_offsets_req.producer_epoch = init_resp.producer_epoch;
    add_offsets_req.group_id =
        kafka_protocol::messages::GroupId(StrBytes::from_string("test-group".to_string()));
    let add_offsets_resp: AddOffsetsToTxnResponse = send_request(&server, 25, 0, &add_offsets_req);
    assert_eq!(
        add_offsets_resp.error_code, 0,
        "AddOffsetsToTxn must succeed"
    );

    let mut part = TxnOffsetCommitRequestPartition::default();
    part.partition_index = 0;
    part.committed_offset = 10;

    let mut txn_topic = TxnOffsetCommitRequestTopic::default();
    txn_topic.name = TopicName(StrBytes::from_string(topic.clone()));
    txn_topic.partitions = vec![part];

    let mut req = TxnOffsetCommitRequest::default();
    req.transactional_id = kafka_protocol::messages::TransactionalId(StrBytes::from_string(
        "txn-offset-commit".to_string(),
    ));
    req.group_id =
        kafka_protocol::messages::GroupId(StrBytes::from_string("test-group".to_string()));
    req.producer_id = init_resp.producer_id;
    req.producer_epoch = init_resp.producer_epoch;
    req.topics = vec![txn_topic];

    let resp: TxnOffsetCommitResponse = send_request(&server, 28, 0, &req);
    assert_eq!(
        resp.topics[0].partitions[0].error_code, 0,
        "TxnOffsetCommit should succeed"
    );
}

// @covers US-004 WriteTxnMarkers basic flow
#[test]
fn contract_write_txn_markers_basic() {
    use kafka_protocol::messages::init_producer_id_request::InitProducerIdRequest;
    use kafka_protocol::messages::init_producer_id_response::InitProducerIdResponse;
    use kafka_protocol::messages::write_txn_markers_request::{
        WritableTxnMarker, WritableTxnMarkerTopic, WriteTxnMarkersRequest,
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
        resp.markers[0].topics[0].partitions[0].error_code, 0,
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
        &RecordEncodeOptions {
            version: 2,
            compression: Compression::None,
        },
    )
    .expect("encode txn batch");

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
    assert_eq!(
        produce_resp.responses[0].partition_responses[0].error_code, 0,
        "transactional produce should succeed"
    );

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
    assert!(
        !records.is_empty(),
        "committed transactional records must be fetchable"
    );
    assert_eq!(records[0].value, Some(bytes::Bytes::from("txn-val")));
}

// @covers US-003-AC3 (duplicate retry is de-duplicated)
#[test]
fn contract_duplicate_sequence_is_deduplicated() {
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
            &RecordEncodeOptions {
                version: 2,
                compression: Compression::None,
            },
        )
        .expect("encode batch");
        buf.freeze()
    };

    // First produce at sequence 0: should succeed.
    let resp0 = produce_batch(&server, &topic, encode_batch(0));
    assert_eq!(
        resp0.responses[0].partition_responses[0].error_code, 0,
        "first batch should succeed"
    );

    // Retry with same sequence 0: Kafka/Redpanda return success while
    // de-duplicating the batch, so consumers must still see only one record.
    let resp_dup = produce_batch(&server, &topic, encode_batch(0));
    assert_eq!(
        resp_dup.responses[0].partition_responses[0].error_code, 0,
        "duplicate retry should return success"
    );

    let (fetch_error, records, high_watermark) = fetch_records_from(&server, &topic, 0);
    assert_eq!(fetch_error, 0, "fetch should succeed");
    assert_eq!(
        high_watermark, 1,
        "duplicate retry must not advance high watermark"
    );
    assert_eq!(
        records.len(),
        1,
        "duplicate retry must not append a second record"
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
    add_req.v3_and_below_transactional_id = kafka_protocol::messages::TransactionalId(
        StrBytes::from_string("txn-stale-epoch".to_string()),
    );
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
    add_req.v3_and_below_transactional_id = kafka_protocol::messages::TransactionalId(
        StrBytes::from_string("txn-abort-test".to_string()),
    );
    add_req.v3_and_below_producer_id = init_resp.producer_id;
    add_req.v3_and_below_producer_epoch = epoch;
    add_req.v3_and_below_topics = vec![add_topic];
    let add_resp: AddPartitionsToTxnResponse = send_request(&server, 24, 0, &add_req);
    assert_eq!(
        add_resp.results_by_topic_v3_and_below[0].results_by_partition[0].partition_error_code,
        0
    );

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
        &RecordEncodeOptions {
            version: 2,
            compression: Compression::None,
        },
    )
    .expect("encode txn batch");

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
    assert_eq!(
        produce_resp.responses[0].partition_responses[0].error_code,
        0
    );

    // EndTxn with committed=false (ABORT)
    let mut end_req = EndTxnRequest::default();
    end_req.transactional_id = kafka_protocol::messages::TransactionalId(StrBytes::from_string(
        "txn-abort-test".to_string(),
    ));
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
    add_req.v3_and_below_transactional_id = kafka_protocol::messages::TransactionalId(
        StrBytes::from_string("txn-uncommitted-test".to_string()),
    );
    add_req.v3_and_below_producer_id = init_resp.producer_id;
    add_req.v3_and_below_producer_epoch = epoch;
    add_req.v3_and_below_topics = vec![add_topic];
    let add_resp: AddPartitionsToTxnResponse = send_request(&server, 24, 0, &add_req);
    assert_eq!(
        add_resp.results_by_topic_v3_and_below[0].results_by_partition[0].partition_error_code,
        0
    );

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
        &RecordEncodeOptions {
            version: 2,
            compression: Compression::None,
        },
    )
    .expect("encode txn batch");

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
    assert_eq!(
        produce_resp.responses[0].partition_responses[0].error_code,
        0
    );

    // EndTxn ABORT
    let mut end_req = EndTxnRequest::default();
    end_req.transactional_id = kafka_protocol::messages::TransactionalId(StrBytes::from_string(
        "txn-uncommitted-test".to_string(),
    ));
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

    // Complete the rebalance as leader so the member is active (Stable); only then
    // does a heartbeat return 0 (before SyncGroup it is REBALANCE_IN_PROGRESS).
    let mut sg_assignment = SyncGroupRequestAssignment::default();
    sg_assignment.member_id = member_id.clone();
    sg_assignment.assignment = bytes::Bytes::from(vec![0, 0, 0, 0, 0, 0]);
    let mut sync_request = SyncGroupRequest::default();
    sync_request.group_id = GroupId(StrBytes::from_string(group.clone()));
    sync_request.generation_id = generation_id;
    sync_request.member_id = member_id.clone();
    sync_request.assignments = vec![sg_assignment];
    let sync_resp: SyncGroupResponse = send_request(&server, 14, 1, &sync_request);
    assert_eq!(
        sync_resp.error_code, 0,
        "leader sync should complete rebalance"
    );

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

/// Verify that Fetch with max_wait_ms > 0 on an empty partition actually
/// waits at least half that duration before returning an empty response.
#[test]
fn contract_fetch_long_poll_waits_on_empty_partition() {
    use kafka_protocol::messages::fetch_request::{FetchPartition, FetchRequest, FetchTopic};
    use kafka_protocol::messages::fetch_response::FetchResponse;

    let server = TestServer::start();
    let topic = unique_topic("contract-long-poll");

    // Create a topic so the partition exists.
    let create_resp = create_topic(&server, &topic, 1);
    assert_eq!(create_resp.topics[0].error_code, 0);

    // Build a Fetch request for offset 0 with 300 ms max_wait_ms.
    let mut fetch_part = FetchPartition::default();
    fetch_part.partition = 0;
    fetch_part.fetch_offset = 0;
    fetch_part.partition_max_bytes = 1024 * 1024;

    let mut fetch_topic_req = FetchTopic::default();
    fetch_topic_req.topic = TopicName(StrBytes::from_string(topic.clone()));
    fetch_topic_req.partitions = vec![fetch_part];

    let mut fetch_req = FetchRequest::default();
    fetch_req.replica_id = BrokerId(-1);
    fetch_req.max_wait_ms = 300;
    fetch_req.min_bytes = 1;
    fetch_req.max_bytes = 1024 * 1024;
    fetch_req.isolation_level = 0;
    fetch_req.topics = vec![fetch_topic_req];

    let start = std::time::Instant::now();
    let fetch_resp: FetchResponse = send_request(&server, 1, 4, &fetch_req);
    let elapsed = start.elapsed();

    assert_eq!(fetch_resp.responses[0].partitions[0].error_code, 0);
    // Records should be absent (partition is empty).
    let has_records = fetch_resp.responses[0].partitions[0]
        .records
        .as_ref()
        .is_some_and(|r| !r.is_empty());
    assert!(!has_records, "expected no records on empty partition");

    // Server should have waited at least 150 ms (half of max_wait_ms).
    assert!(
        elapsed >= std::time::Duration::from_millis(150),
        "long-poll should wait at least 150 ms; elapsed = {:?}",
        elapsed
    );
}

/// Verify that DeleteRecords advances the log start offset and that fetching
/// before the new LSO returns OFFSET_OUT_OF_RANGE (error code 1).
#[test]
fn contract_delete_records_advances_lso() {
    use kafka_protocol::messages::delete_records_request::{
        DeleteRecordsPartition, DeleteRecordsRequest, DeleteRecordsTopic,
    };
    use kafka_protocol::messages::delete_records_response::DeleteRecordsResponse;

    let server = TestServer::start();
    let topic = unique_topic("contract-delete-records");

    let create_resp = create_topic(&server, &topic, 1);
    assert_eq!(create_resp.topics[0].error_code, 0, "create failed");

    // Produce 5 records.
    for i in 0..5i64 {
        let record = new_record(i);
        let batch = encode_record_batch(&[record]);
        let produce_resp = produce_batch(&server, &topic, batch);
        assert_eq!(
            produce_resp.responses[0].partition_responses[0].error_code, 0,
            "produce {i} failed"
        );
    }

    // DeleteRecords: truncate before offset 3 (keep offsets 3, 4).
    let mut partition = DeleteRecordsPartition::default();
    partition.partition_index = 0;
    partition.offset = 3;

    let mut del_topic = DeleteRecordsTopic::default();
    del_topic.name = TopicName(StrBytes::from_string(topic.clone()));
    del_topic.partitions = vec![partition];

    let mut del_req = DeleteRecordsRequest::default();
    del_req.topics = vec![del_topic];
    del_req.timeout_ms = 5000;

    let del_resp: DeleteRecordsResponse = send_request(&server, 21, 1, &del_req);
    assert_eq!(
        del_resp.topics[0].partitions[0].error_code, 0,
        "DeleteRecords error"
    );
    assert_eq!(
        del_resp.topics[0].partitions[0].low_watermark, 3,
        "low watermark should be 3 after deleting before offset 3"
    );

    // Fetch from offset 0 — must return OFFSET_OUT_OF_RANGE (1).
    let mut fetch_part = FetchPartition::default();
    fetch_part.partition = 0;
    fetch_part.fetch_offset = 0;
    fetch_part.partition_max_bytes = 1024 * 1024;

    let mut fetch_topic = FetchTopic::default();
    fetch_topic.topic = TopicName(StrBytes::from_string(topic.clone()));
    fetch_topic.partitions = vec![fetch_part];

    let mut fetch_req = FetchRequest::default();
    fetch_req.replica_id = BrokerId(-1);
    fetch_req.max_wait_ms = 0;
    fetch_req.min_bytes = 0;
    fetch_req.max_bytes = 1024 * 1024;
    fetch_req.isolation_level = 0;
    fetch_req.topics = vec![fetch_topic];

    let fetch_resp: FetchResponse = send_request(&server, 1, 4, &fetch_req);
    assert_eq!(
        fetch_resp.responses[0].partitions[0].error_code, 1,
        "fetch from before LSO must return OFFSET_OUT_OF_RANGE"
    );

    // Fetch from offset 3 — must succeed and return records 3 and 4.
    let mut fetch_part2 = FetchPartition::default();
    fetch_part2.partition = 0;
    fetch_part2.fetch_offset = 3;
    fetch_part2.partition_max_bytes = 1024 * 1024;

    let mut fetch_topic2 = FetchTopic::default();
    fetch_topic2.topic = TopicName(StrBytes::from_string(topic.clone()));
    fetch_topic2.partitions = vec![fetch_part2];

    let mut fetch_req2 = FetchRequest::default();
    fetch_req2.replica_id = BrokerId(-1);
    fetch_req2.max_wait_ms = 0;
    fetch_req2.min_bytes = 0;
    fetch_req2.max_bytes = 1024 * 1024;
    fetch_req2.isolation_level = 0;
    fetch_req2.topics = vec![fetch_topic2];

    let fetch_resp2: FetchResponse = send_request(&server, 1, 4, &fetch_req2);
    assert_eq!(
        fetch_resp2.responses[0].partitions[0].error_code, 0,
        "fetch from LSO must succeed"
    );
    let records_bytes = fetch_resp2.responses[0].partitions[0]
        .records
        .as_ref()
        .expect("should have record data");
    assert!(
        !records_bytes.is_empty(),
        "should have records starting at offset 3"
    );
}

/// Verify that DescribeLogDirs returns the memory:// synthetic log dir
/// with the correct topic/partition structure.
#[test]
fn contract_describe_log_dirs_returns_memory_dir() {
    use kafka_protocol::messages::describe_log_dirs_request::{
        DescribableLogDirTopic as ReqTopic, DescribeLogDirsRequest,
    };
    use kafka_protocol::messages::describe_log_dirs_response::DescribeLogDirsResponse;

    let server = TestServer::start();
    let topic = unique_topic("contract-logdirs");

    let create_resp = create_topic(&server, &topic, 2);
    assert_eq!(create_resp.topics[0].error_code, 0, "create failed");

    // Request log dirs for the specific topic.
    let mut req_topic = ReqTopic::default();
    req_topic.topic = TopicName(StrBytes::from_string(topic.clone()));
    req_topic.partitions = vec![0, 1];

    let mut req = DescribeLogDirsRequest::default();
    req.topics = Some(vec![req_topic]);

    let resp: DescribeLogDirsResponse = send_request(&server, 35, 1, &req);

    assert_eq!(resp.results.len(), 1, "should have one log dir result");
    let dir = &resp.results[0];
    assert_eq!(dir.error_code, 0, "DescribeLogDirs error");
    assert_eq!(
        dir.log_dir.as_str(),
        "memory://",
        "log dir must be memory://"
    );
    assert_eq!(dir.topics.len(), 1, "should have one topic entry");
    assert_eq!(
        dir.topics[0].partitions.len(),
        2,
        "should describe both partitions"
    );
}

/// Verify that CreatePartitions adds partitions to an existing topic and
/// that the new count is visible in Metadata.
#[test]
fn contract_create_partitions_increases_count() {
    use kafka_protocol::messages::create_partitions_request::{
        CreatePartitionsRequest, CreatePartitionsTopic,
    };
    use kafka_protocol::messages::create_partitions_response::CreatePartitionsResponse;

    let server = TestServer::start();
    let topic = unique_topic("contract-create-parts");

    // Create with 1 partition.
    let create_resp = create_topic(&server, &topic, 1);
    assert_eq!(create_resp.topics[0].error_code, 0, "create failed");

    // Expand to 3 partitions.
    let mut cp_topic = CreatePartitionsTopic::default();
    cp_topic.name = TopicName(StrBytes::from_string(topic.clone()));
    cp_topic.count = 3;

    let mut cp_req = CreatePartitionsRequest::default();
    cp_req.topics = vec![cp_topic];
    cp_req.timeout_ms = 5000;
    cp_req.validate_only = false;

    let cp_resp: CreatePartitionsResponse = send_request(&server, 37, 1, &cp_req);
    assert_eq!(
        cp_resp.results[0].error_code,
        0,
        "CreatePartitions must succeed: {}",
        cp_resp.results[0]
            .error_message
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("(none)")
    );

    // Verify via Metadata that partition count is now 3.
    let mut meta_req_topic = MetadataRequestTopic::default();
    meta_req_topic.name = Some(TopicName(StrBytes::from_string(topic.clone())));

    let mut meta_req = MetadataRequest::default();
    meta_req.topics = Some(vec![meta_req_topic]);
    meta_req.allow_auto_topic_creation = false;

    let meta_resp: MetadataResponse = send_request(&server, 3, 4, &meta_req);
    assert_eq!(meta_resp.topics.len(), 1, "metadata must return topic");
    assert_eq!(
        meta_resp.topics[0].partitions.len(),
        3,
        "topic should now have 3 partitions after CreatePartitions"
    );
}

/// Verify that OffsetDelete removes a committed consumer-group offset and that
/// a subsequent OffsetFetch returns offset -1 (no committed offset).
#[test]
fn contract_offset_delete_clears_committed_offset() {
    use kafka_protocol::messages::offset_delete_request::{
        OffsetDeleteRequest, OffsetDeleteRequestPartition, OffsetDeleteRequestTopic,
    };
    use kafka_protocol::messages::offset_delete_response::OffsetDeleteResponse;

    let server = TestServer::start();
    let topic = unique_topic("contract-offset-del");
    let group = unique_group("contract-offset-del-group");

    let create_resp = create_topic(&server, &topic, 1);
    assert_eq!(create_resp.topics[0].error_code, 0, "create failed");

    // Commit offset 10 for the group.
    let mut commit_part = OffsetCommitRequestPartition::default();
    commit_part.partition_index = 0;
    commit_part.committed_offset = 10;
    commit_part.commit_timestamp = 0;

    let mut commit_topic = OffsetCommitRequestTopic::default();
    commit_topic.name = TopicName(StrBytes::from_string(topic.clone()));
    commit_topic.partitions = vec![commit_part];

    let mut commit_req = OffsetCommitRequest::default();
    commit_req.group_id = GroupId(StrBytes::from_string(group.clone()));
    commit_req.generation_id_or_member_epoch = 1;
    commit_req.member_id = StrBytes::from_string("m1".to_string());
    commit_req.topics = vec![commit_topic];

    let commit_resp: OffsetCommitResponse = send_request(&server, 8, 1, &commit_req);
    assert_eq!(
        commit_resp.topics[0].partitions[0].error_code, 0,
        "commit error"
    );

    // Verify it's committed (offset = 10).
    let mut fetch_topic = OffsetFetchRequestTopic::default();
    fetch_topic.name = TopicName(StrBytes::from_string(topic.clone()));
    fetch_topic.partition_indexes = vec![0];

    let mut fetch_req = OffsetFetchRequest::default();
    fetch_req.group_id = GroupId(StrBytes::from_string(group.clone()));
    fetch_req.topics = Some(vec![fetch_topic.clone()]);

    let fetch_resp: OffsetFetchResponse = send_request(&server, 9, 1, &fetch_req);
    assert_eq!(
        fetch_resp.topics[0].partitions[0].committed_offset, 10,
        "offset should be 10 before deletion"
    );

    // OffsetDelete: delete the committed offset.
    let mut del_part = OffsetDeleteRequestPartition::default();
    del_part.partition_index = 0;

    let mut del_topic = OffsetDeleteRequestTopic::default();
    del_topic.name = TopicName(StrBytes::from_string(topic.clone()));
    del_topic.partitions = vec![del_part];

    let mut del_req = OffsetDeleteRequest::default();
    del_req.group_id = GroupId(StrBytes::from_string(group.clone()));
    del_req.topics = vec![del_topic];

    let del_resp: OffsetDeleteResponse = send_request(&server, 47, 0, &del_req);
    assert_eq!(del_resp.error_code, 0, "OffsetDelete top-level error");
    assert_eq!(
        del_resp.topics[0].partitions[0].error_code, 0,
        "OffsetDelete partition error"
    );

    // After deletion, OffsetFetch must return -1 (no committed offset).
    let mut fetch_req2 = OffsetFetchRequest::default();
    fetch_req2.group_id = GroupId(StrBytes::from_string(group.clone()));
    fetch_req2.topics = Some(vec![fetch_topic]);

    let fetch_resp2: OffsetFetchResponse = send_request(&server, 9, 1, &fetch_req2);
    assert_eq!(
        fetch_resp2.topics[0].partitions[0].committed_offset, -1,
        "OffsetFetch after OffsetDelete must return -1 (no committed offset)"
    );
}

/// Verify that acks=0 produce (fire-and-forget) elicits NO response from the
/// broker, and that a subsequent request on the same connection receives the
/// CORRECT response — not a spurious produce reply that would corrupt the
/// protocol stream.
#[test]
fn contract_produce_acks0_no_response_sent() {
    use std::io::{Read as _, Write as _};

    let server = TestServer::start();
    let topic = unique_topic("contract-acks0");

    let create_resp = create_topic(&server, &topic, 1);
    assert_eq!(create_resp.topics[0].error_code, 0, "create failed");

    let mut stream = std::net::TcpStream::connect(("127.0.0.1", server.port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .unwrap();

    // Build a Produce request with acks=0.
    let record = new_record(0);
    let batch = encode_record_batch(&[record]);

    let mut partition = PartitionProduceData::default();
    partition.index = 0;
    partition.records = Some(batch);

    let mut topic_data = TopicProduceData::default();
    topic_data.name = TopicName(StrBytes::from_string(topic.clone()));
    topic_data.partition_data = vec![partition];

    let mut produce_req = ProduceRequest::default();
    produce_req.acks = 0; // fire-and-forget
    produce_req.timeout_ms = 1000;
    produce_req.topic_data = vec![topic_data];

    let produce_frame = encode_request(0, 2, 100, None, &produce_req);
    stream.write_all(&produce_frame).expect("write produce");

    // Immediately send a ListOffsets request with a different correlation_id.
    let mut lo_part = ListOffsetsPartition::default();
    lo_part.partition_index = 0;
    lo_part.timestamp = -1; // latest

    let mut lo_topic = ListOffsetsTopic::default();
    lo_topic.name = TopicName(StrBytes::from_string(topic.clone()));
    lo_topic.partitions = vec![lo_part];

    let mut lo_req = ListOffsetsRequest::default();
    lo_req.replica_id = BrokerId(-1);
    lo_req.topics = vec![lo_topic];

    let lo_frame = encode_request(2, 1, 999, None, &lo_req);
    stream.write_all(&lo_frame).expect("write list_offsets");

    // Read the next response. It MUST be the ListOffsets response (correlation_id=999).
    // If the server wrongly sent a produce response first, the correlation_id
    // check here would catch the mismatch (we'd see 100 instead of 999).
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).expect("read length");
    let len = i32::from_be_bytes(len_buf) as usize;
    let mut resp_buf = vec![0u8; len];
    stream.read_exact(&mut resp_buf).expect("read response");

    let mut cursor = std::io::Cursor::new(resp_buf);
    let correlation_id = cursor.get_i32();
    assert_eq!(
        correlation_id, 999,
        "first response must be the ListOffsets reply (correlation_id=999), \
         not a spurious acks=0 produce response (would be 100)"
    );

    // Decode as ListOffsetsResponse to confirm it's valid.
    let lo_resp = ListOffsetsResponse::decode(&mut cursor, 1).expect("decode list_offsets");
    assert_eq!(
        lo_resp.topics[0].partitions[0].error_code, 0,
        "ListOffsets error"
    );
    // The produce with acks=0 MUST have been stored: HWM should be ≥ 1.
    assert!(
        lo_resp.topics[0].partitions[0].offset >= 1,
        "acks=0 produce must be stored; HWM = {}",
        lo_resp.topics[0].partitions[0].offset
    );
}

/// Verify that DescribeConfigs returns at least one named config entry for a
/// topic resource and that the response carries no error code.
#[test]
fn contract_describe_configs_returns_topic_configs() {
    use kafka_protocol::messages::describe_configs_request::{
        DescribeConfigsRequest, DescribeConfigsResource,
    };
    use kafka_protocol::messages::describe_configs_response::DescribeConfigsResponse;

    let server = TestServer::start();
    let topic = unique_topic("contract-describe-cfg");

    let create_resp = create_topic(&server, &topic, 1);
    assert_eq!(create_resp.topics[0].error_code, 0, "create failed");

    // Resource type 2 = TOPIC.
    let mut resource = DescribeConfigsResource::default();
    resource.resource_type = 2;
    resource.resource_name = StrBytes::from_string(topic.clone());

    let mut req = DescribeConfigsRequest::default();
    req.resources = vec![resource];
    req.include_synonyms = false;

    // Version 1: non-flexible, supports config_source (v1+).
    let resp: DescribeConfigsResponse = send_request(&server, 32, 1, &req);
    assert_eq!(resp.results.len(), 1, "should have one result");
    let result = &resp.results[0];
    assert_eq!(result.error_code, 0, "DescribeConfigs error");
    assert!(
        !result.configs.is_empty(),
        "DescribeConfigs must return at least one config entry for the topic"
    );
    // Verify at least cleanup.policy is present.
    let has_cleanup = result
        .configs
        .iter()
        .any(|c| c.name.as_str() == "cleanup.policy");
    assert!(
        has_cleanup,
        "cleanup.policy must be in the DescribeConfigs response"
    );
}

#[test]
fn contract_list_groups_returns_joined_group() {
    use kafka_protocol::messages::list_groups_request::ListGroupsRequest;
    use kafka_protocol::messages::list_groups_response::ListGroupsResponse;

    let server = TestServer::start();
    let group = unique_group("contract-list-groups");

    // Join a group first.
    let mut protocol = JoinGroupRequestProtocol::default();
    protocol.name = StrBytes::from_string("range".to_string());
    protocol.metadata = bytes::Bytes::from(vec![0, 1, 0, 0, 0, 1, 0, 4, 116, 101, 115, 116]);

    let mut join = JoinGroupRequest::default();
    join.group_id = GroupId(StrBytes::from_string(group.clone()));
    join.session_timeout_ms = 30000;
    join.rebalance_timeout_ms = 30000;
    join.member_id = StrBytes::from_string(String::new());
    join.protocol_type = StrBytes::from_string("consumer".to_string());
    join.protocols = vec![protocol.clone()];

    let r1: JoinGroupResponse = send_request(&server, 11, 1, &join);
    assert_eq!(r1.error_code, 79);
    join.member_id = r1.member_id.clone();
    let r2: JoinGroupResponse = send_request(&server, 11, 1, &join);
    assert_eq!(r2.error_code, 0, "JoinGroup should succeed");

    // ListGroups v0 (non-flexible).
    let req = ListGroupsRequest::default();
    let resp: ListGroupsResponse = send_request(&server, 16, 0, &req);
    assert_eq!(resp.error_code, 0, "ListGroups error");
    let found = resp
        .groups
        .iter()
        .any(|g| g.group_id.0.as_str() == group.as_str());
    assert!(found, "ListGroups must return the joined group '{group}'");
}

#[test]
fn contract_describe_groups_active_member() {
    use kafka_protocol::messages::describe_groups_request::DescribeGroupsRequest;
    use kafka_protocol::messages::describe_groups_response::DescribeGroupsResponse;

    let server = TestServer::start();
    let group = unique_group("contract-describe-groups");

    // Join and sync to get the group into Stable state.
    let mut protocol = JoinGroupRequestProtocol::default();
    protocol.name = StrBytes::from_string("range".to_string());
    protocol.metadata = bytes::Bytes::from(vec![0, 1, 0, 0, 0, 1, 0, 4, 116, 101, 115, 116]);

    let mut join = JoinGroupRequest::default();
    join.group_id = GroupId(StrBytes::from_string(group.clone()));
    join.session_timeout_ms = 30000;
    join.rebalance_timeout_ms = 30000;
    join.member_id = StrBytes::from_string(String::new());
    join.protocol_type = StrBytes::from_string("consumer".to_string());
    join.protocols = vec![protocol.clone()];

    let r1: JoinGroupResponse = send_request(&server, 11, 1, &join);
    assert_eq!(r1.error_code, 79);
    join.member_id = r1.member_id.clone();
    let r2: JoinGroupResponse = send_request(&server, 11, 1, &join);
    assert_eq!(r2.error_code, 0);
    let generation_id = r2.generation_id;
    let member_id = r2.member_id.clone();

    let mut assignment = SyncGroupRequestAssignment::default();
    assignment.member_id = member_id.clone();
    assignment.assignment = bytes::Bytes::from(vec![0, 0, 0, 1, 0, 4, 116, 101, 115, 116]);
    let mut sync = SyncGroupRequest::default();
    sync.group_id = GroupId(StrBytes::from_string(group.clone()));
    sync.generation_id = generation_id;
    sync.member_id = member_id.clone();
    sync.assignments = vec![assignment];
    let sync_r: SyncGroupResponse = send_request(&server, 14, 1, &sync);
    assert_eq!(sync_r.error_code, 0, "SyncGroup should succeed");

    // DescribeGroups v0 (non-flexible).
    let mut req = DescribeGroupsRequest::default();
    req.groups = vec![GroupId(StrBytes::from_string(group.clone()))];
    let resp: DescribeGroupsResponse = send_request(&server, 15, 0, &req);

    assert_eq!(resp.groups.len(), 1, "should describe one group");
    let g = &resp.groups[0];
    assert_eq!(g.error_code, 0, "DescribeGroups error");
    assert_eq!(g.group_id.0.as_str(), group.as_str());
    assert_eq!(g.protocol_type.as_str(), "consumer");
    assert_eq!(
        g.group_state.as_str(),
        "Stable",
        "group should be Stable after SyncGroup"
    );
    assert_eq!(g.members.len(), 1, "should have exactly one member");
    assert_eq!(g.members[0].member_id, member_id);
}

#[test]
fn contract_alter_configs_no_op_success() {
    use kafka_protocol::messages::alter_configs_request::{
        AlterConfigsRequest, AlterConfigsResource, AlterableConfig,
    };
    use kafka_protocol::messages::alter_configs_response::AlterConfigsResponse;

    let server = TestServer::start();
    let topic = unique_topic("contract-alter-cfg");
    let create_resp = create_topic(&server, &topic, 1);
    assert_eq!(create_resp.topics[0].error_code, 0, "create failed");

    // resource_type 2 = TOPIC; config key retention.ms.
    let mut cfg = AlterableConfig::default();
    cfg.name = StrBytes::from_string("retention.ms".to_string());
    cfg.value = Some(StrBytes::from_string("86400000".to_string()));

    let mut resource = AlterConfigsResource::default();
    resource.resource_type = 2;
    resource.resource_name = StrBytes::from_string(topic.clone());
    resource.configs = vec![cfg];

    let mut req = AlterConfigsRequest::default();
    req.resources = vec![resource];
    req.validate_only = false;

    // Version 0 (non-flexible).
    let resp: AlterConfigsResponse = send_request(&server, 33, 0, &req);
    assert_eq!(resp.responses.len(), 1);
    assert_eq!(
        resp.responses[0].error_code, 0,
        "AlterConfigs must return success"
    );
    assert_eq!(resp.responses[0].resource_name.as_str(), topic.as_str());
}

#[test]
fn contract_list_offsets_timestamp_returns_offset() {
    let server = TestServer::start();
    let topic = unique_topic("contract-list-offsets-ts");
    let create_resp = create_topic(&server, &topic, 1);
    assert_eq!(create_resp.topics[0].error_code, 0, "create failed");

    // Produce 3 records.
    let records = vec![new_record(0), new_record(1), new_record(2)];
    let batch = encode_record_batch(&records);
    let produce_resp = produce_batch(&server, &topic, batch);
    assert_eq!(
        produce_resp.responses[0].partition_responses[0].error_code,
        0
    );

    // ListOffsets with timestamp -1 (latest).
    let mut lp_latest = ListOffsetsPartition::default();
    lp_latest.partition_index = 0;
    lp_latest.timestamp = -1;

    let mut lt = ListOffsetsTopic::default();
    lt.name = TopicName(StrBytes::from_string(topic.clone()));
    lt.partitions = vec![lp_latest];

    let mut req = ListOffsetsRequest::default();
    req.replica_id = BrokerId(-1);
    req.topics = vec![lt];
    let resp: ListOffsetsResponse = send_request(&server, 2, 1, &req);
    assert_eq!(resp.topics[0].partitions[0].error_code, 0);
    assert_eq!(
        resp.topics[0].partitions[0].offset, 3,
        "latest offset should be 3 after producing 3 records"
    );

    // ListOffsets with timestamp -2 (earliest).
    let mut lp_earliest = ListOffsetsPartition::default();
    lp_earliest.partition_index = 0;
    lp_earliest.timestamp = -2;

    let mut lt2 = ListOffsetsTopic::default();
    lt2.name = TopicName(StrBytes::from_string(topic.clone()));
    lt2.partitions = vec![lp_earliest];

    let mut req2 = ListOffsetsRequest::default();
    req2.replica_id = BrokerId(-1);
    req2.topics = vec![lt2];
    let resp2: ListOffsetsResponse = send_request(&server, 2, 1, &req2);
    assert_eq!(resp2.topics[0].partitions[0].error_code, 0);
    assert_eq!(
        resp2.topics[0].partitions[0].offset, 0,
        "earliest offset should be 0"
    );

    // ListOffsets with a specific timestamp (>0) — should return a valid offset.
    let mut lp_ts = ListOffsetsPartition::default();
    lp_ts.partition_index = 0;
    lp_ts.timestamp = 1; // arbitrary positive timestamp

    let mut lt3 = ListOffsetsTopic::default();
    lt3.name = TopicName(StrBytes::from_string(topic));
    lt3.partitions = vec![lp_ts];

    let mut req3 = ListOffsetsRequest::default();
    req3.replica_id = BrokerId(-1);
    req3.topics = vec![lt3];
    let resp3: ListOffsetsResponse = send_request(&server, 2, 1, &req3);
    assert_eq!(
        resp3.topics[0].partitions[0].error_code, 0,
        "timestamp ListOffsets must not error"
    );
    // offset >= 0 is all we guarantee for in-memory storage without a timestamp index.
    assert!(
        resp3.topics[0].partitions[0].offset >= 0,
        "timestamp ListOffsets must return a non-negative offset"
    );
}

#[test]
fn contract_fetch_long_poll_wakes_on_produce() {
    use kafka_protocol::messages::fetch_request::{FetchPartition, FetchRequest, FetchTopic};
    use kafka_protocol::messages::fetch_response::FetchResponse;
    use std::sync::{Arc, Barrier};

    let server = Arc::new(TestServer::start());
    let topic = unique_topic("contract-lp-wake");

    let create_resp = create_topic(&server, &topic, 1);
    assert_eq!(create_resp.topics[0].error_code, 0);

    // Start a Fetch long-poll (max_wait=2000ms) in a background thread before producing.
    let server2 = server.clone();
    let topic2 = topic.clone();
    let barrier = Arc::new(Barrier::new(2));
    let barrier2 = barrier.clone();

    let fetch_handle = std::thread::spawn(move || {
        let mut fetch_part = FetchPartition::default();
        fetch_part.partition = 0;
        fetch_part.fetch_offset = 0;
        fetch_part.partition_max_bytes = 1024 * 1024;
        let mut fetch_topic_req = FetchTopic::default();
        fetch_topic_req.topic = TopicName(StrBytes::from_string(topic2));
        fetch_topic_req.partitions = vec![fetch_part];
        let mut fetch_req = FetchRequest::default();
        fetch_req.replica_id = BrokerId(-1);
        fetch_req.max_wait_ms = 2000;
        fetch_req.min_bytes = 1;
        fetch_req.max_bytes = 1024 * 1024;
        fetch_req.topics = vec![fetch_topic_req];

        // Signal that we're about to send the Fetch, then start timing.
        barrier2.wait();
        let start = std::time::Instant::now();
        let resp: FetchResponse = send_request(&server2, 1, 4, &fetch_req);
        (resp, start.elapsed())
    });

    // Wait until the fetch thread is about to start, then produce ~50ms later.
    barrier.wait();
    std::thread::sleep(std::time::Duration::from_millis(50));
    let batch = encode_record_batch(&[new_record(0)]);
    let produce_resp = produce_batch(&server, &topic, batch);
    assert_eq!(
        produce_resp.responses[0].partition_responses[0].error_code,
        0
    );

    let (fetch_resp, elapsed) = fetch_handle.join().expect("fetch thread panicked");
    assert_eq!(fetch_resp.responses[0].partitions[0].error_code, 0);

    // The long-poll should have returned with data, not waited the full 2000ms.
    assert!(
        elapsed < std::time::Duration::from_millis(500),
        "long-poll should wake immediately on produce; elapsed = {:?}",
        elapsed
    );

    // Verify data was received.
    let has_records = fetch_resp.responses[0].partitions[0]
        .records
        .as_ref()
        .is_some_and(|r| !r.is_empty());
    assert!(
        has_records,
        "long-poll response must contain the produced records"
    );
}

#[test]
fn contract_fetch_beyond_hwm_returns_empty() {
    use kafka_protocol::messages::fetch_request::{FetchPartition, FetchRequest, FetchTopic};
    use kafka_protocol::messages::fetch_response::FetchResponse;

    let server = TestServer::start();
    let topic = unique_topic("contract-fetch-beyond-hwm");

    let create_resp = create_topic(&server, &topic, 1);
    assert_eq!(create_resp.topics[0].error_code, 0);

    // Produce 2 records, HWM = 2.
    let batch = encode_record_batch(&[new_record(0), new_record(1)]);
    let produce_resp = produce_batch(&server, &topic, batch);
    assert_eq!(
        produce_resp.responses[0].partition_responses[0].error_code,
        0
    );

    // Fetch at offset 999 (well beyond HWM=2) — Kafka returns empty response, no error.
    // This is the normal case for a consumer parked at the tail.
    let mut fetch_part = FetchPartition::default();
    fetch_part.partition = 0;
    fetch_part.fetch_offset = 999;
    fetch_part.partition_max_bytes = 1024 * 1024;
    let mut fetch_topic_req = FetchTopic::default();
    fetch_topic_req.topic = TopicName(StrBytes::from_string(topic));
    fetch_topic_req.partitions = vec![fetch_part];
    let mut fetch_req = FetchRequest::default();
    fetch_req.replica_id = BrokerId(-1);
    fetch_req.max_wait_ms = 0;
    fetch_req.min_bytes = 0;
    fetch_req.max_bytes = 1024 * 1024;
    fetch_req.topics = vec![fetch_topic_req];

    let fetch_resp: FetchResponse = send_request(&server, 1, 3, &fetch_req);
    let partition = &fetch_resp.responses[0].partitions[0];
    assert_eq!(
        partition.error_code, 0,
        "fetch beyond HWM should return no error"
    );
    let has_records = partition.records.as_ref().is_some_and(|r| !r.is_empty());
    assert!(!has_records, "fetch beyond HWM must return empty records");
    assert_eq!(
        partition.high_watermark, 2,
        "high_watermark should reflect current HWM"
    );
}

#[test]
fn contract_offset_fetch_unknown_group_returns_minus_one() {
    let server = TestServer::start();
    let topic = unique_topic("contract-offset-fetch-unknown");
    let group = unique_group("contract-unk-group");

    let create_resp = create_topic(&server, &topic, 1);
    assert_eq!(create_resp.topics[0].error_code, 0);

    // OffsetFetch for a group that has never committed offsets.
    let mut fetch_topic = OffsetFetchRequestTopic::default();
    fetch_topic.name = TopicName(StrBytes::from_string(topic));
    fetch_topic.partition_indexes = vec![0];

    let mut req = OffsetFetchRequest::default();
    req.group_id = GroupId(StrBytes::from_string(group));
    req.topics = Some(vec![fetch_topic]);

    let resp: OffsetFetchResponse = send_request(&server, 9, 1, &req);
    assert_eq!(
        resp.topics[0].partitions[0].error_code, 0,
        "unknown group must not error"
    );
    assert_eq!(
        resp.topics[0].partitions[0].committed_offset, -1,
        "unknown group offset must be -1 (no committed offset)"
    );
}

#[test]
fn contract_delete_groups_removes_group() {
    let server = TestServer::start();
    let group = unique_group("contract-del-group");

    // Join a group and reach Stable state.
    let mut protocol = JoinGroupRequestProtocol::default();
    protocol.name = StrBytes::from_string("range".to_string());
    protocol.metadata = bytes::Bytes::from(vec![0, 1, 0, 0, 0, 1, 0, 4, 116, 101, 115, 116]);

    let mut join = JoinGroupRequest::default();
    join.group_id = GroupId(StrBytes::from_string(group.clone()));
    join.session_timeout_ms = 30000;
    join.rebalance_timeout_ms = 30000;
    join.member_id = StrBytes::from_string(String::new());
    join.protocol_type = StrBytes::from_string("consumer".to_string());
    join.protocols = vec![protocol.clone()];

    let r1: JoinGroupResponse = send_request(&server, 11, 1, &join);
    let member_id = r1.member_id.clone();
    join.member_id = member_id.clone();
    let r2: JoinGroupResponse = send_request(&server, 11, 1, &join);
    assert_eq!(r2.error_code, 0);

    // Sync to enter Stable state.
    let mut assignment = SyncGroupRequestAssignment::default();
    assignment.member_id = member_id.clone();
    assignment.assignment = bytes::Bytes::from(vec![0, 0, 0, 1, 0, 4, 116, 101, 115, 116]);
    let mut sync = SyncGroupRequest::default();
    sync.group_id = GroupId(StrBytes::from_string(group.clone()));
    sync.generation_id = r2.generation_id;
    sync.member_id = member_id.clone();
    sync.assignments = vec![assignment];
    let sync_resp: SyncGroupResponse = send_request(&server, 14, 1, &sync);
    assert_eq!(sync_resp.error_code, 0);

    // Leave the group — now the group is Empty.
    let mut leave = LeaveGroupRequest::default();
    leave.group_id = GroupId(StrBytes::from_string(group.clone()));
    leave.member_id = member_id;
    let leave_resp: LeaveGroupResponse = send_request(&server, 13, 0, &leave);
    assert_eq!(leave_resp.error_code, 0);

    // DeleteGroups v0 (non-flexible) — should return error_code=0.
    let mut del_req = DeleteGroupsRequest::default();
    del_req.groups_names = vec![kafka_protocol::messages::GroupId(StrBytes::from_string(
        group.clone(),
    ))];
    let del_resp: DeleteGroupsResponse = send_request(&server, 42, 0, &del_req);
    assert_eq!(
        del_resp.results[0].error_code, 0,
        "delete empty group must succeed"
    );

    // ListGroups — the deleted group must not appear.
    let list_req = ListGroupsRequest::default();
    let list_resp: ListGroupsResponse = send_request(&server, 16, 0, &list_req);
    let found = list_resp
        .groups
        .iter()
        .any(|g| g.group_id.0.as_str() == group.as_str());
    assert!(!found, "deleted group must not appear in ListGroups");
}

#[test]
fn contract_incremental_alter_configs_succeeds() {
    let server = TestServer::start();
    let topic = unique_topic("contract-incr-alter");

    let create_resp = create_topic(&server, &topic, 1);
    assert_eq!(create_resp.topics[0].error_code, 0);

    // IncrementalAlterConfigs v0 (non-flexible: header_version=1).
    // Set retention.ms to 86400000 (1 day) for the topic.
    let mut config = AlterableConfig::default();
    config.name = StrBytes::from_string("retention.ms".to_string());
    config.config_operation = 0; // SET
    config.value = Some(StrBytes::from_string("86400000".to_string()));

    let mut resource = AlterConfigsResource::default();
    resource.resource_type = 2; // TOPIC
    resource.resource_name = StrBytes::from_string(topic.clone());
    resource.configs = vec![config];

    let mut req = IncrementalAlterConfigsRequest::default();
    req.resources = vec![resource];
    req.validate_only = false;

    let resp: IncrementalAlterConfigsResponse = send_request(&server, 44, 0, &req);
    assert!(
        !resp.responses.is_empty(),
        "IncrementalAlterConfigs must return at least one response"
    );
    assert_eq!(
        resp.responses[0].error_code, 0,
        "IncrementalAlterConfigs must succeed"
    );
}

#[test]
fn contract_describe_cluster_returns_broker() {
    let server = TestServer::start();

    // DescribeCluster v0 (always flexible: header_version=2).
    let req = DescribeClusterRequest::default();
    let resp: DescribeClusterResponse = send_request(&server, 60, 0, &req);
    assert_eq!(resp.error_code, 0, "DescribeCluster must return no error");
    assert!(
        !resp.brokers.is_empty(),
        "DescribeCluster must return at least one broker"
    );
    let broker = &resp.brokers[0];
    assert_eq!(
        broker.port as u16, server.port,
        "broker port must match server port"
    );
}

#[test]
fn contract_multi_partition_produce_fetch() {
    let server = TestServer::start();
    let topic = unique_topic("contract-multipart");

    // Create a 2-partition topic.
    let create_resp = create_topic(&server, &topic, 2);
    assert_eq!(create_resp.topics[0].error_code, 0);

    // Produce to partition 0.
    let batch0 = encode_record_batch(&[new_record(0)]);
    let mut part0_data = PartitionProduceData::default();
    part0_data.index = 0;
    part0_data.records = Some(batch0);

    // Produce to partition 1.
    let batch1 = encode_record_batch(&[new_record(1)]);
    let mut part1_data = PartitionProduceData::default();
    part1_data.index = 1;
    part1_data.records = Some(batch1);

    let mut topic_data = TopicProduceData::default();
    topic_data.name = TopicName(StrBytes::from_string(topic.clone()));
    topic_data.partition_data = vec![part0_data, part1_data];

    let mut produce_req = ProduceRequest::default();
    produce_req.acks = 1;
    produce_req.timeout_ms = 1000;
    produce_req.topic_data = vec![topic_data];

    let produce_resp: ProduceResponse = send_request(&server, 0, 2, &produce_req);
    assert_eq!(
        produce_resp.responses[0].partition_responses[0].error_code, 0,
        "produce p0"
    );
    assert_eq!(
        produce_resp.responses[0].partition_responses[1].error_code, 0,
        "produce p1"
    );

    // Fetch from both partitions in a single request.
    let mut fp0 = FetchPartition::default();
    fp0.partition = 0;
    fp0.fetch_offset = 0;
    fp0.partition_max_bytes = 1024 * 1024;

    let mut fp1 = FetchPartition::default();
    fp1.partition = 1;
    fp1.fetch_offset = 0;
    fp1.partition_max_bytes = 1024 * 1024;

    let mut ft = FetchTopic::default();
    ft.topic = TopicName(StrBytes::from_string(topic.clone()));
    ft.partitions = vec![fp0, fp1];

    let mut fetch_req = FetchRequest::default();
    fetch_req.replica_id = BrokerId(-1);
    fetch_req.max_wait_ms = 0;
    fetch_req.min_bytes = 0;
    fetch_req.max_bytes = 1024 * 1024;
    fetch_req.topics = vec![ft];

    let fetch_resp: FetchResponse = send_request(&server, 1, 3, &fetch_req);
    let topic_resp = &fetch_resp.responses[0];
    let p0 = topic_resp
        .partitions
        .iter()
        .find(|p| p.partition_index == 0)
        .expect("partition 0 must be in fetch response");
    let p1 = topic_resp
        .partitions
        .iter()
        .find(|p| p.partition_index == 1)
        .expect("partition 1 must be in fetch response");

    assert_eq!(p0.error_code, 0, "fetch p0 must succeed");
    assert_eq!(p1.error_code, 0, "fetch p1 must succeed");
    assert_eq!(p0.high_watermark, 1, "partition 0 HWM must be 1");
    assert_eq!(p1.high_watermark, 1, "partition 1 HWM must be 1");

    // Decode and verify one record from each partition.
    let p0_records = p0.records.as_ref().expect("partition 0 must have records");
    let p1_records = p1.records.as_ref().expect("partition 1 must have records");
    let mut p0_bytes = bytes::Bytes::copy_from_slice(p0_records);
    let mut p1_bytes = bytes::Bytes::copy_from_slice(p1_records);
    let p0_batches = RecordBatchDecoder::decode(&mut p0_bytes).expect("decode p0 batch");
    let p1_batches = RecordBatchDecoder::decode(&mut p1_bytes).expect("decode p1 batch");
    assert_eq!(
        p0_batches.records.len(),
        1,
        "partition 0 must have 1 record"
    );
    assert_eq!(
        p1_batches.records.len(),
        1,
        "partition 1 must have 1 record"
    );
}

#[test]
fn contract_list_transactions_returns_empty() {
    let server = TestServer::start();

    // ListTransactions v0 (always flexible: header_version=2).
    let req = ListTransactionsRequest::default();
    let resp: ListTransactionsResponse = send_request(&server, 66, 0, &req);
    assert_eq!(resp.error_code, 0, "ListTransactions must return no error");
    assert!(
        resp.transaction_states.is_empty(),
        "ListTransactions on idle broker must return empty list"
    );
}

#[test]
fn contract_produce_acks_minus_one_succeeds() {
    let server = TestServer::start();
    let topic = unique_topic("contract-acks-all");

    let create_resp = create_topic(&server, &topic, 1);
    assert_eq!(create_resp.topics[0].error_code, 0);

    let batch = encode_record_batch(&[new_record(0), new_record(1)]);
    let mut partition = PartitionProduceData::default();
    partition.index = 0;
    partition.records = Some(batch);
    let mut topic_data = TopicProduceData::default();
    topic_data.name = TopicName(StrBytes::from_string(topic.clone()));
    topic_data.partition_data = vec![partition];

    let mut produce_req = ProduceRequest::default();
    produce_req.acks = -1; // all in-sync replicas
    produce_req.timeout_ms = 5000;
    produce_req.topic_data = vec![topic_data];

    let produce_resp: ProduceResponse = send_request(&server, 0, 2, &produce_req);
    let part = &produce_resp.responses[0].partition_responses[0];
    assert_eq!(
        part.error_code, 0,
        "acks=-1 must succeed on single-node broker"
    );
    assert_eq!(part.base_offset, 0, "first produce base_offset must be 0");
}

#[test]
fn contract_produce_sequential_base_offsets() {
    let server = TestServer::start();
    let topic = unique_topic("contract-base-offset");

    let create_resp = create_topic(&server, &topic, 1);
    assert_eq!(create_resp.topics[0].error_code, 0);

    // First produce: 3 records → offsets 0, 1, 2 → base_offset=0.
    let batch0 = encode_record_batch(&[new_record(0), new_record(1), new_record(2)]);
    let resp0 = produce_batch(&server, &topic, batch0);
    assert_eq!(resp0.responses[0].partition_responses[0].error_code, 0);
    assert_eq!(resp0.responses[0].partition_responses[0].base_offset, 0);

    // Second produce: 2 records → offsets 3, 4 → base_offset=3.
    let batch1 = encode_record_batch(&[new_record(3), new_record(4)]);
    let resp1 = produce_batch(&server, &topic, batch1);
    assert_eq!(resp1.responses[0].partition_responses[0].error_code, 0);
    assert_eq!(
        resp1.responses[0].partition_responses[0].base_offset, 3,
        "second batch must start at offset 3"
    );
}

#[test]
fn contract_offset_for_leader_epoch_returns_hwm() {
    let server = TestServer::start();
    let topic = unique_topic("contract-ofle");

    let create_resp = create_topic(&server, &topic, 1);
    assert_eq!(create_resp.topics[0].error_code, 0);

    // Produce 4 records (HWM = 4).
    let batch = encode_record_batch(&[new_record(0), new_record(1), new_record(2), new_record(3)]);
    let produce_resp = produce_batch(&server, &topic, batch);
    assert_eq!(
        produce_resp.responses[0].partition_responses[0].error_code,
        0
    );

    // OffsetForLeaderEpoch v0 (non-flexible: header_version=1).
    let mut part_req = OffsetForLeaderPartition::default();
    part_req.partition = 0;
    part_req.leader_epoch = 0;
    part_req.current_leader_epoch = -1;

    let mut topic_req = OffsetForLeaderTopic::default();
    topic_req.topic = TopicName(StrBytes::from_string(topic.clone()));
    topic_req.partitions = vec![part_req];

    let mut req = OffsetForLeaderEpochRequest::default();
    req.topics = vec![topic_req];

    // Use v1 (non-flexible; flexible starts at v4): response includes leader_epoch.
    let resp: OffsetForLeaderEpochResponse = send_request(&server, 23, 1, &req);
    assert_eq!(
        resp.topics[0].partitions[0].error_code, 0,
        "OffsetForLeaderEpoch must succeed"
    );
    assert_eq!(
        resp.topics[0].partitions[0].end_offset, 4,
        "end_offset must equal HWM (4)"
    );
    assert_eq!(
        resp.topics[0].partitions[0].leader_epoch, 0,
        "single-node leader_epoch is always 0"
    );
}

#[test]
fn contract_produce_compressed_batch_roundtrip() {
    let server = TestServer::start();
    let topic = unique_topic("contract-compressed");

    let create_resp = create_topic(&server, &topic, 1);
    assert_eq!(create_resp.topics[0].error_code, 0);

    // Produce a snappy-compressed batch (Compression::Snappy is available in kafka-protocol).
    let records = vec![new_record(0), new_record(1), new_record(2)];
    let mut buf = BytesMut::new();
    RecordBatchEncoder::encode(
        &mut buf,
        &records,
        &RecordEncodeOptions {
            version: 2,
            compression: Compression::Gzip,
        },
    )
    .expect("encode compressed batch");
    let compressed_batch = buf.freeze();

    let produce_resp = produce_batch(&server, &topic, compressed_batch);
    assert_eq!(
        produce_resp.responses[0].partition_responses[0].error_code,
        0
    );

    // Fetch back — the broker returns the compressed bytes intact.
    let mut fp = FetchPartition::default();
    fp.partition = 0;
    fp.fetch_offset = 0;
    fp.partition_max_bytes = 1024 * 1024;

    let mut ft = FetchTopic::default();
    ft.topic = TopicName(StrBytes::from_string(topic));
    ft.partitions = vec![fp];

    let mut fetch_req = FetchRequest::default();
    fetch_req.replica_id = BrokerId(-1);
    fetch_req.max_wait_ms = 0;
    fetch_req.min_bytes = 0;
    fetch_req.max_bytes = 1024 * 1024;
    fetch_req.topics = vec![ft];

    let fetch_resp: FetchResponse = send_request(&server, 1, 3, &fetch_req);
    let part = &fetch_resp.responses[0].partitions[0];
    assert_eq!(part.error_code, 0, "fetch of compressed batch must succeed");
    let raw = part.records.as_ref().expect("must have records");
    // Decode: the kafka-protocol crate decompresses automatically.
    let mut bytes = bytes::Bytes::copy_from_slice(raw);
    let batch = RecordBatchDecoder::decode(&mut bytes).expect("decode compressed batch");
    assert_eq!(
        batch.records.len(),
        3,
        "all 3 compressed records must be decoded"
    );
}

#[test]
fn contract_fetch_from_multiple_topics() {
    let server = TestServer::start();
    let topic_a = unique_topic("contract-multi-topic-a");
    let topic_b = unique_topic("contract-multi-topic-b");

    // Create two topics.
    assert_eq!(create_topic(&server, &topic_a, 1).topics[0].error_code, 0);
    assert_eq!(create_topic(&server, &topic_b, 1).topics[0].error_code, 0);

    // Produce to each.
    let batch_a = encode_record_batch(&[new_record(0)]);
    let batch_b = encode_record_batch(&[new_record(1)]);

    let produce_a = produce_batch(&server, &topic_a, batch_a);
    let produce_b = produce_batch(&server, &topic_b, batch_b);
    assert_eq!(produce_a.responses[0].partition_responses[0].error_code, 0);
    assert_eq!(produce_b.responses[0].partition_responses[0].error_code, 0);

    // Fetch from both topics in one request.
    let mut fp = FetchPartition::default();
    fp.partition = 0;
    fp.fetch_offset = 0;
    fp.partition_max_bytes = 1024 * 1024;

    let mut ft_a = FetchTopic::default();
    ft_a.topic = TopicName(StrBytes::from_string(topic_a.clone()));
    ft_a.partitions = vec![fp.clone()];

    let mut ft_b = FetchTopic::default();
    ft_b.topic = TopicName(StrBytes::from_string(topic_b.clone()));
    ft_b.partitions = vec![fp];

    let mut fetch_req = FetchRequest::default();
    fetch_req.replica_id = BrokerId(-1);
    fetch_req.max_wait_ms = 0;
    fetch_req.min_bytes = 0;
    fetch_req.max_bytes = 1024 * 1024;
    fetch_req.topics = vec![ft_a, ft_b];

    let fetch_resp: FetchResponse = send_request(&server, 1, 3, &fetch_req);
    assert_eq!(
        fetch_resp.responses.len(),
        2,
        "must get responses for both topics"
    );

    let resp_a = fetch_resp
        .responses
        .iter()
        .find(|r| r.topic.0.as_str() == topic_a.as_str())
        .expect("response for topic_a");
    let resp_b = fetch_resp
        .responses
        .iter()
        .find(|r| r.topic.0.as_str() == topic_b.as_str())
        .expect("response for topic_b");

    assert_eq!(resp_a.partitions[0].error_code, 0);
    assert_eq!(resp_b.partitions[0].error_code, 0);
    assert!(
        resp_a.partitions[0].records.is_some(),
        "topic_a must have records"
    );
    assert!(
        resp_b.partitions[0].records.is_some(),
        "topic_b must have records"
    );
}

#[test]
fn contract_describe_topic_partitions_returns_partition_leader() {
    let server = TestServer::start();
    let topic = unique_topic("contract-dtp");

    // Create a 3-partition topic.
    let create_resp = create_topic(&server, &topic, 3);
    assert_eq!(create_resp.topics[0].error_code, 0);

    // DescribeTopicPartitions v0 (always flexible: header_version=2).
    let mut topic_req = TopicRequest::default();
    topic_req.name = TopicName(StrBytes::from_string(topic.clone()));

    let mut req = DescribeTopicPartitionsRequest::default();
    req.topics = vec![topic_req];

    let resp: DescribeTopicPartitionsResponse = send_request(&server, 75, 0, &req);
    let topic_resp = resp
        .topics
        .iter()
        .find(|t| t.name.as_ref().map(|n| n.0.as_str()) == Some(topic.as_str()))
        .expect("topic must be in response");

    assert_eq!(
        topic_resp.error_code, 0,
        "DescribeTopicPartitions must succeed"
    );
    assert_eq!(
        topic_resp.partitions.len(),
        3,
        "3-partition topic must return 3 partitions"
    );

    // All partitions should have the self broker as leader.
    for part in &topic_resp.partitions {
        assert_eq!(
            part.error_code, 0,
            "partition {} must have no error",
            part.partition_index
        );
        assert!(
            part.leader_id.0 >= 0,
            "partition {} must have a valid leader",
            part.partition_index
        );
    }

    // Test unknown topic returns UNKNOWN_TOPIC_OR_PARTITION (error 3).
    let mut req_unknown = DescribeTopicPartitionsRequest::default();
    let mut t_unknown = TopicRequest::default();
    t_unknown.name = TopicName(StrBytes::from_string("no-such-topic".to_string()));
    req_unknown.topics = vec![t_unknown];
    let resp_unknown: DescribeTopicPartitionsResponse = send_request(&server, 75, 0, &req_unknown);
    assert_eq!(
        resp_unknown.topics[0].error_code, 3,
        "unknown topic must return UNKNOWN_TOPIC_OR_PARTITION"
    );
}

/// ElectLeaders (API 43 v0) — single-node broker always succeeds.
///
/// In a single-broker cluster the leader is always the current node, so any
/// preferred-replica election is a no-op and must return error_code=0 for
/// every requested partition. Also verifies the null `topic_partitions` form
/// (elect all) returns a top-level error_code=0 with no per-topic results.
#[test]
fn contract_elect_leaders_succeeds_on_single_node() {
    let server = TestServer::start();
    let topic = unique_topic("contract-elect-leaders");

    let create_resp = create_topic(&server, &topic, 2);
    assert_eq!(create_resp.topics[0].error_code, 0);

    // ElectLeaders v0 is non-flexible (header_version=1 for versions 0-1).
    let mut tp = ElectLeadersTopicPartitions::default();
    tp.topic = TopicName(StrBytes::from_string(topic.clone()));
    tp.partitions = vec![0, 1];

    let mut req = ElectLeadersRequest::default();
    req.topic_partitions = Some(vec![tp]);
    req.timeout_ms = 5000;

    let resp: ElectLeadersResponse = send_request(&server, 43, 0, &req);
    assert_eq!(
        resp.error_code, 0,
        "ElectLeaders must succeed on a single-node broker"
    );
    assert_eq!(
        resp.replica_election_results.len(),
        1,
        "one topic in result"
    );
    let results = &resp.replica_election_results[0];
    for pr in &results.partition_result {
        assert_eq!(
            pr.error_code, 0,
            "partition {} must succeed",
            pr.partition_id
        );
    }

    // Elect all (null topic_partitions) must also return no error.
    let mut req_all = ElectLeadersRequest::default();
    req_all.topic_partitions = None;
    req_all.timeout_ms = 5000;
    let resp_all: ElectLeadersResponse = send_request(&server, 43, 0, &req_all);
    assert_eq!(
        resp_all.error_code, 0,
        "ElectLeaders with null topics must succeed"
    );
}
