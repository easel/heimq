use super::*;
use crate::consumer_group::{GroupState, Member};
use crate::test_support::{encode_body, encode_record_batch, init_tracing, test_config, test_consumer_groups, test_storage};
use bytes::{BufMut, Bytes, BytesMut};
use kafka_protocol::messages::create_topics_request::{CreatableTopic, CreateTopicsRequest};
use kafka_protocol::messages::delete_topics_request::DeleteTopicsRequest;
use kafka_protocol::messages::fetch_request::{FetchPartition, FetchRequest, FetchTopic};
use kafka_protocol::messages::find_coordinator_request::FindCoordinatorRequest;
use kafka_protocol::messages::heartbeat_request::HeartbeatRequest;
use kafka_protocol::messages::join_group_request::{JoinGroupRequest, JoinGroupRequestProtocol};
use kafka_protocol::messages::leave_group_request::{LeaveGroupRequest, MemberIdentity};
use kafka_protocol::messages::list_offsets_request::{ListOffsetsPartition, ListOffsetsRequest, ListOffsetsTopic};
use kafka_protocol::messages::metadata_request::{MetadataRequest, MetadataRequestTopic};
use kafka_protocol::messages::offset_commit_request::{OffsetCommitRequest, OffsetCommitRequestPartition, OffsetCommitRequestTopic};
use kafka_protocol::messages::offset_fetch_request::{OffsetFetchRequest, OffsetFetchRequestTopic};
use kafka_protocol::messages::produce_request::{PartitionProduceData, ProduceRequest, TopicProduceData};
use kafka_protocol::messages::sync_group_request::{SyncGroupRequest, SyncGroupRequestAssignment};
use kafka_protocol::messages::{BrokerId, GroupId, TopicName};
use kafka_protocol::protocol::StrBytes;
use kafka_protocol::records::Record;
use std::sync::Arc;

fn new_record(offset: i64) -> Record {
    Record {
        transactional: false,
        control: false,
        partition_leader_epoch: 0,
        producer_id: -1,
        producer_epoch: -1,
        timestamp_type: kafka_protocol::records::TimestampType::Creation,
        timestamp: offset,
        sequence: offset as i32,
        offset,
        key: Some(format!("key-{offset}").into()),
        value: Some(format!("value-{offset}").into()),
        headers: Default::default(),
    }
}

fn put_str(buf: &mut BytesMut, value: Option<&str>) {
    match value {
        Some(text) => {
            buf.put_i16(text.len() as i16);
            buf.extend_from_slice(text.as_bytes());
        }
        None => {
            buf.put_i16(-1);
        }
    }
}

#[test]
fn api_versions_lists_supported_apis() {
    let response = api_versions::handle(0, crate::protocol::SUPPORTED_APIS);
    assert_eq!(response.error_code, 0);
    let keys: Vec<i16> = response.api_keys.iter().map(|k| k.api_key).collect();
    assert!(keys.contains(&0));
    assert!(keys.contains(&18));
}

#[test]
fn metadata_all_topics_and_unknown_topic() {
    let config = test_config(true);
    let storage = test_storage(true);
    storage.create_topic("t1", 1).unwrap();

    let body = encode_body(&MetadataRequest::default(), 1);
    let response = metadata::handle(1, &body, &storage, &config).unwrap();
    assert!(response.topics.iter().any(|t| t.name.as_ref().map(|n| n.0.as_str()) == Some("t1")));

    let config_no_auto = test_config(false);
    let storage_no_auto = test_storage(false);
    let mut topic = MetadataRequestTopic::default();
    topic.name = Some(TopicName(StrBytes::from_string("missing".to_string())));
    let mut request = MetadataRequest::default();
    request.topics = Some(vec![topic]);
    let body = encode_body(&request, 4);
    let response = metadata::handle(4, &body, &storage_no_auto, &config_no_auto).unwrap();
    let missing = response.topics.first().unwrap();
    assert_eq!(missing.error_code, 3);
}

#[test]
fn metadata_mixed_existing_and_missing_topics() {
    let config = test_config(true);
    let storage = test_storage(true);
    storage.create_topic("existing", 1).unwrap();

    let mut existing = MetadataRequestTopic::default();
    existing.name = Some(TopicName(StrBytes::from_string("existing".to_string())));
    let mut missing = MetadataRequestTopic::default();
    missing.name = Some(TopicName(StrBytes::from_string("missing".to_string())));

    let mut request = MetadataRequest::default();
    request.topics = Some(vec![existing, missing]);
    let body = encode_body(&request, 4);
    let response = metadata::handle(4, &body, &storage, &config).unwrap();
    assert!(response
        .topics
        .iter()
        .any(|t| t.name.as_ref().map(|n| n.0.as_str()) == Some("existing")));
    assert!(response
        .topics
        .iter()
        .any(|t| t.name.as_ref().map(|n| n.0.as_str()) == Some("missing")));
}

#[test]
fn metadata_version_zero_parses_topics() {
    let storage = test_storage(true);
    storage.create_topic("meta-v0", 1).unwrap();
    let config = test_config(true);

    let mut buf = BytesMut::new();
    buf.put_i32(1);
    put_str(&mut buf, Some("meta-v0"));

    let response = metadata::handle(0, &buf, &storage, &config).unwrap();
    assert_eq!(response.topics.len(), 1);
    assert!(response.cluster_id.is_none());
}

#[test]
fn create_topics_default_partitions_and_duplicate() {
    let storage = test_storage(true);

    let mut topic = CreatableTopic::default();
    topic.name = TopicName(StrBytes::from_string("auto".to_string()));
    topic.num_partitions = -1;
    topic.replication_factor = 1;

    let mut request = CreateTopicsRequest::default();
    request.topics = vec![topic];
    request.timeout_ms = 1000;
    request.validate_only = false;

    let body = encode_body(&request, 1);
    let response = create_topics::handle(1, &body, &storage).unwrap();
    assert_eq!(response.topics[0].error_code, 0);

    let body = encode_body(&request, 1);
    let response = create_topics::handle(1, &body, &storage).unwrap();
    assert_eq!(response.topics[0].error_code, 36);

    let response = create_topics::handle(1, &[], &storage).unwrap();
    assert!(response.topics.is_empty());
}

#[test]
fn create_topics_with_config_values() {
    let storage = test_storage(true);
    let mut buf = BytesMut::new();

    buf.put_i32(1); // topic count
    put_str(&mut buf, Some("cfg-topic"));
    buf.put_i32(1); // num_partitions
    buf.put_i16(1); // replication_factor
    buf.put_i32(0); // assignment_count
    buf.put_i32(1); // config_count
    put_str(&mut buf, Some("cleanup.policy"));
    put_str(&mut buf, Some("compact"));

    let response = create_topics::handle(1, &buf, &storage).unwrap();
    assert_eq!(response.topics.len(), 1);
    assert_eq!(response.topics[0].error_code, 0);
}

#[test]
fn delete_topics_existing_and_missing() {
    let storage = test_storage(true);
    storage.create_topic("delete-me", 1).unwrap();

    let mut request = DeleteTopicsRequest::default();
    request.topic_names = vec![TopicName(StrBytes::from_string("delete-me".to_string()))];
    request.timeout_ms = 1000;
    let body = encode_body(&request, 1);
    let response = delete_topics::handle(1, &body, &storage).unwrap();
    assert_eq!(response.responses[0].error_code, 0);

    let mut missing = DeleteTopicsRequest::default();
    missing.topic_names = vec![TopicName(StrBytes::from_string("missing".to_string()))];
    missing.timeout_ms = 1000;
    let body = encode_body(&missing, 1);
    let response = delete_topics::handle(1, &body, &storage).unwrap();
    assert_eq!(response.responses[0].error_code, 3);

    let response = delete_topics::handle(1, &[], &storage).unwrap();
    assert!(response.responses.is_empty());
}

#[test]
fn list_offsets_latest_earliest_and_timestamp() {
    let storage = test_storage(true);
    storage.create_topic("offsets", 1).unwrap();

    let records = vec![new_record(0), new_record(1)];
    let batch = encode_record_batch(&records);
    storage.append("offsets", 0, &batch).unwrap();

    let mut partition = ListOffsetsPartition::default();
    partition.partition_index = 0;
    partition.timestamp = -1;
    partition.max_num_offsets = 1;

    let mut topic = ListOffsetsTopic::default();
    topic.name = TopicName(StrBytes::from_string("offsets".to_string()));
    topic.partitions = vec![partition];

    let mut request = ListOffsetsRequest::default();
    request.replica_id = BrokerId(-1);
    request.topics = vec![topic];

    let body = encode_body(&request, 1);
    let response = list_offsets::handle(1, &body, &storage).unwrap();
    assert_eq!(response.topics[0].partitions[0].offset, 2);

    request.topics[0].partitions[0].timestamp = -2;
    let body = encode_body(&request, 1);
    let response = list_offsets::handle(1, &body, &storage).unwrap();
    assert_eq!(response.topics[0].partitions[0].offset, 0);

    request.topics[0].partitions[0].timestamp = 1234;
    let body = encode_body(&request, 1);
    let response = list_offsets::handle(1, &body, &storage).unwrap();
    assert_eq!(response.topics[0].partitions[0].offset, 0);

    let response = list_offsets::handle(1, &[], &storage).unwrap();
    assert!(response.topics.is_empty());
}

#[test]
fn produce_empty_null_and_unknown_topic() {
    let storage = test_storage(true);
    storage.create_topic("produce", 1).unwrap();

    let mut partition = PartitionProduceData::default();
    partition.index = 0;
    partition.records = Some(Bytes::new());

    let mut topic = TopicProduceData::default();
    topic.name = TopicName(StrBytes::from_string("produce".to_string()));
    topic.partition_data = vec![partition];

    let mut request = ProduceRequest::default();
    request.acks = 1;
    request.timeout_ms = 1000;
    request.topic_data = vec![topic];

    let body = encode_body(&request, 2);
    let response = produce::handle(2, &body, &storage).unwrap();
    assert_eq!(response.responses[0].partition_responses[0].error_code, 0);

    let mut null_partition = PartitionProduceData::default();
    null_partition.index = 0;
    null_partition.records = None;
    let mut null_topic = TopicProduceData::default();
    null_topic.name = TopicName(StrBytes::from_string("produce".to_string()));
    null_topic.partition_data = vec![null_partition];
    let mut null_request = ProduceRequest::default();
    null_request.acks = 1;
    null_request.timeout_ms = 1000;
    null_request.topic_data = vec![null_topic];

    let body = encode_body(&null_request, 2);
    let response = produce::handle(2, &body, &storage).unwrap();
    assert_eq!(response.responses[0].partition_responses[0].error_code, 87);

    let storage_no_auto = test_storage(false);
    let mut bad_topic = TopicProduceData::default();
    bad_topic.name = TopicName(StrBytes::from_string("missing".to_string()));
    let mut bad_partition = PartitionProduceData::default();
    bad_partition.index = 0;
    bad_partition.records = Some(Bytes::from(vec![1, 2, 3]));
    bad_topic.partition_data = vec![bad_partition];
    let mut bad_request = ProduceRequest::default();
    bad_request.acks = 1;
    bad_request.timeout_ms = 1000;
    bad_request.topic_data = vec![bad_topic];

    let body = encode_body(&bad_request, 2);
    let response = produce::handle(2, &body, &storage_no_auto).unwrap();
    assert_eq!(response.responses[0].partition_responses[0].error_code, 3);

    let response = produce::handle(2, &[0, 1, 2], &storage).unwrap();
    assert!(response.responses.is_empty());
}

#[test]
fn fetch_offsets_and_errors() {
    let storage = test_storage(true);
    storage.create_topic("fetch", 1).unwrap();

    let batch = encode_record_batch(&[new_record(0)]);
    storage.append("fetch", 0, &batch).unwrap();

    let mut partition = FetchPartition::default();
    partition.partition = 0;
    partition.fetch_offset = 0;
    partition.partition_max_bytes = 1024;

    let mut topic = FetchTopic::default();
    topic.topic = TopicName(StrBytes::from_string("fetch".to_string()));
    topic.partitions = vec![partition];

    let mut request = FetchRequest::default();
    request.replica_id = BrokerId(-1);
    request.max_wait_ms = 1000;
    request.min_bytes = 1;
    request.max_bytes = 1024;
    request.topics = vec![topic];

    let body = encode_body(&request, 3);
    let response = fetch::handle(3, &body, &storage).unwrap();
    assert_eq!(response.responses[0].partitions[0].error_code, 0);

    request.topics[0].partitions[0].fetch_offset = 1;
    let body = encode_body(&request, 3);
    let response = fetch::handle(3, &body, &storage).unwrap();
    assert_eq!(response.responses[0].partitions[0].error_code, 0);

    request.topics[0].partitions[0].fetch_offset = -1;
    let body = encode_body(&request, 3);
    let response = fetch::handle(3, &body, &storage).unwrap();
    assert_eq!(response.responses[0].partitions[0].error_code, 1);

    let mut missing_topic = FetchTopic::default();
    missing_topic.topic = TopicName(StrBytes::from_string("missing".to_string()));
    missing_topic.partitions = vec![FetchPartition::default()];
    let mut request_missing = FetchRequest::default();
    request_missing.replica_id = BrokerId(-1);
    request_missing.max_wait_ms = 1000;
    request_missing.min_bytes = 1;
    request_missing.max_bytes = 1024;
    request_missing.topics = vec![missing_topic];

    let body = encode_body(&request_missing, 3);
    let response = fetch::handle(3, &body, &storage).unwrap();
    assert_eq!(response.responses[0].partitions[0].error_code, 3);

    let response = fetch::handle(3, &[], &storage).unwrap();
    assert!(response.responses.is_empty());
}

#[test]
fn find_coordinator_single_node() {
    let mut config = (*test_config(true)).clone();
    config.host = "0.0.0.0".to_string();
    let config = Arc::new(config);
    let response = find_coordinator::handle(0, &[], &config).unwrap();
    assert_eq!(response.error_code, 0);
    assert_eq!(response.node_id.0, config.broker_id);
    assert_eq!(response.host.to_string(), "127.0.0.1");
}

#[test]
fn offset_commit_and_fetch_roundtrip() {
    let config = test_config(true);
    let consumer_groups = test_consumer_groups(config);

    let mut partition = OffsetCommitRequestPartition::default();
    partition.partition_index = 0;
    partition.committed_offset = 5;
    partition.commit_timestamp = 0;

    let mut topic = OffsetCommitRequestTopic::default();
    topic.name = TopicName(StrBytes::from_string("topic".to_string()));
    topic.partitions = vec![partition];

    let mut request = OffsetCommitRequest::default();
    request.group_id = GroupId(StrBytes::from_string("group".to_string()));
    request.generation_id_or_member_epoch = 1;
    request.member_id = StrBytes::from_string("member".to_string());
    request.topics = vec![topic];

    let body = encode_body(&request, 1);
    let response = offset_commit::handle(1, &body, consumer_groups.offset_store()).unwrap();
    assert_eq!(response.topics[0].partitions[0].error_code, 0);

    let mut fetch_topic = OffsetFetchRequestTopic::default();
    fetch_topic.name = TopicName(StrBytes::from_string("topic".to_string()));
    fetch_topic.partition_indexes = vec![0];

    let mut fetch_request = OffsetFetchRequest::default();
    fetch_request.group_id = GroupId(StrBytes::from_string("group".to_string()));
    fetch_request.topics = Some(vec![fetch_topic]);

    let body = encode_body(&fetch_request, 1);
    let response = offset_fetch::handle(1, &body, consumer_groups.offset_store()).unwrap();
    assert_eq!(response.topics[0].partitions[0].committed_offset, 5);

    let mut fetch_all = OffsetFetchRequest::default();
    fetch_all.group_id = GroupId(StrBytes::from_string("group".to_string()));
    fetch_all.topics = None;
    let body = encode_body(&fetch_all, 1);
    let response = offset_fetch::handle(1, &body, consumer_groups.offset_store()).unwrap();
    assert_eq!(response.topics[0].partitions[0].committed_offset, 5);

    let response = offset_commit::handle(1, &[], consumer_groups.offset_store()).unwrap();
    assert!(response.topics.is_empty());
}

#[test]
fn join_sync_heartbeat_leave_group_flow() {
    let config = test_config(true);
    let consumer_groups = test_consumer_groups(config);

    let mut protocol = JoinGroupRequestProtocol::default();
    protocol.name = StrBytes::from_string("range".to_string());
    protocol.metadata = Bytes::from(vec![1, 2, 3]);

    let mut request = JoinGroupRequest::default();
    request.group_id = GroupId(StrBytes::from_string("group".to_string()));
    request.session_timeout_ms = 30000;
    request.rebalance_timeout_ms = 30000;
    request.member_id = StrBytes::from_string(String::new());
    request.protocol_type = StrBytes::from_string("consumer".to_string());
    request.protocols = vec![protocol];

    let body = encode_body(&request, 1);
    let response = join_group::handle(1, &body, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 79);
    assert!(!response.member_id.is_empty());

    request.member_id = response.member_id.clone();
    let body = encode_body(&request, 1);
    let response = join_group::handle(1, &body, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 0);
    assert_eq!(response.leader, response.member_id);

    let mut assignment = SyncGroupRequestAssignment::default();
    assignment.member_id = response.member_id.clone();
    assignment.assignment = Bytes::from(vec![9, 9, 9]);

    let member_id = response.member_id.clone();
    let generation_id = response.generation_id;

    let mut sync = SyncGroupRequest::default();
    sync.group_id = GroupId(StrBytes::from_string("group".to_string()));
    sync.generation_id = generation_id;
    sync.member_id = member_id.clone();
    sync.assignments = vec![assignment];

    let body = encode_body(&sync, 0);
    let response = sync_group::handle(0, &body, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 0);

    let mut heartbeat = HeartbeatRequest::default();
    heartbeat.group_id = GroupId(StrBytes::from_string("group".to_string()));
    heartbeat.generation_id = generation_id;
    heartbeat.member_id = member_id.clone();
    let body = encode_body(&heartbeat, 0);
    let response = heartbeat::handle(0, &body, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 0);

    let mut leave = LeaveGroupRequest::default();
    leave.group_id = GroupId(StrBytes::from_string("group".to_string()));
    leave.member_id = StrBytes::from_string(String::new());
    let mut member = MemberIdentity::default();
    member.member_id = member_id;
    leave.members = vec![member];
    let body = encode_body(&leave, 3);
    let response = leave_group::handle(3, &body, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 0);
}

#[test]
fn sync_group_leader_applies_assignments_and_stabilizes() {
    let config = test_config(true);
    let consumer_groups = test_consumer_groups(config);
    let group = consumer_groups.get_or_create_group("sync-group");

    let member = Member::new(
        "leader".to_string(),
        "client".to_string(),
        "127.0.0.1".to_string(),
        30000,
        30000,
        "consumer".to_string(),
        vec![("range".to_string(), vec![1, 2])],
    );
    let generation_id = group.add_member(member);

    let mut buf = BytesMut::new();
    put_str(&mut buf, Some("sync-group"));
    buf.put_i32(generation_id);
    put_str(&mut buf, Some("leader"));
    buf.put_i32(1);
    put_str(&mut buf, Some("leader"));
    buf.put_i32(3);
    buf.extend_from_slice(&[1, 2, 3]);

    let response = sync_group::handle(0, &buf, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 0);

    let group = consumer_groups.get_group("sync-group").unwrap();
    assert_eq!(group.state(), GroupState::Stable);
    assert_eq!(group.get_assignment("leader"), Some(vec![1, 2, 3]));
}

#[test]
fn sync_group_non_leader_missing_assignment() {
    let config = test_config(true);
    let consumer_groups = test_consumer_groups(config);
    let group = consumer_groups.get_or_create_group("sync-group-2");

    let leader = Member::new(
        "leader".to_string(),
        "client".to_string(),
        "127.0.0.1".to_string(),
        30000,
        30000,
        "consumer".to_string(),
        vec![("range".to_string(), vec![])],
    );
    let follower = Member::new(
        "follower".to_string(),
        "client".to_string(),
        "127.0.0.1".to_string(),
        30000,
        30000,
        "consumer".to_string(),
        vec![("range".to_string(), vec![])],
    );
    group.add_member(leader);
    let generation_id = group.add_member(follower);

    let mut buf = BytesMut::new();
    put_str(&mut buf, Some("sync-group-2"));
    buf.put_i32(generation_id);
    put_str(&mut buf, Some("follower"));
    buf.put_i32(0);

    let response = sync_group::handle(0, &buf, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 0);
    assert!(response.assignment.is_empty());
}

#[test]
fn join_group_leader_includes_metadata() {
    let config = test_config(true);
    let consumer_groups = test_consumer_groups(config);

    let mut protocol = JoinGroupRequestProtocol::default();
    protocol.name = StrBytes::from_string("range".to_string());
    protocol.metadata = Bytes::from(vec![9, 9]);

    let mut request = JoinGroupRequest::default();
    request.group_id = GroupId(StrBytes::from_string("group".to_string()));
    request.session_timeout_ms = 30000;
    request.rebalance_timeout_ms = 30000;
    request.member_id = StrBytes::from_string("leader".to_string());
    request.protocol_type = StrBytes::from_string("consumer".to_string());
    request.protocols = vec![protocol];

    let body = encode_body(&request, 1);
    let response = join_group::handle(1, &body, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 0);
    assert_eq!(response.members.len(), 1);
    assert_eq!(response.members[0].metadata.as_ref(), &[9, 9]);
}

#[test]
fn join_group_select_protocol_fallback_for_non_leader() {
    let config = test_config(true);
    let consumer_groups = test_consumer_groups(config);

    let mut protocol_a = JoinGroupRequestProtocol::default();
    protocol_a.name = StrBytes::from_string("range".to_string());
    protocol_a.metadata = Bytes::from(vec![1]);

    let mut request_a = JoinGroupRequest::default();
    request_a.group_id = GroupId(StrBytes::from_string("group".to_string()));
    request_a.session_timeout_ms = 30000;
    request_a.rebalance_timeout_ms = 30000;
    request_a.member_id = StrBytes::from_string("member-a".to_string());
    request_a.protocol_type = StrBytes::from_string("consumer".to_string());
    request_a.protocols = vec![protocol_a];

    let body = encode_body(&request_a, 1);
    let response_a = join_group::handle(1, &body, consumer_groups.as_ref()).unwrap();
    assert_eq!(response_a.error_code, 0);

    let mut protocol_b = JoinGroupRequestProtocol::default();
    protocol_b.name = StrBytes::from_string("roundrobin".to_string());
    protocol_b.metadata = Bytes::from(vec![2]);

    let mut request_b = JoinGroupRequest::default();
    request_b.group_id = GroupId(StrBytes::from_string("group".to_string()));
    request_b.session_timeout_ms = 30000;
    request_b.rebalance_timeout_ms = 30000;
    request_b.member_id = StrBytes::from_string("member-b".to_string());
    request_b.protocol_type = StrBytes::from_string("consumer".to_string());
    request_b.protocols = vec![protocol_b];

    let body = encode_body(&request_b, 1);
    let response_b = join_group::handle(1, &body, consumer_groups.as_ref()).unwrap();
    assert_eq!(response_b.error_code, 0);
    assert_eq!(response_b.protocol_name.as_ref().unwrap().as_str(), "roundrobin");
    assert!(response_b.members.is_empty());
}

#[test]
fn group_handlers_error_paths() {
    let config = test_config(true);
    let consumer_groups = test_consumer_groups(config);

    let response = join_group::handle(1, &[], consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 35);

    let response = sync_group::handle(0, &[], consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 35);

    let response = heartbeat::handle(0, &[], consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 35);

    let response = leave_group::handle(0, &[], consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 16);
}

#[test]
fn find_coordinator_request_ignored() {
    let config = test_config(true);
    let mut request = FindCoordinatorRequest::default();
    request.key = StrBytes::from_string("group".to_string());
    let body = encode_body(&request, 1);
    let response = find_coordinator::handle(1, &body, &config).unwrap();
    assert_eq!(response.error_code, 0);
}

#[test]
fn create_topics_truncated_inputs() {
    let storage = test_storage(true);
    let mut bodies: Vec<BytesMut> = Vec::new();

    let mut buf = BytesMut::new();
    buf.put_i32(1);
    bodies.push(buf);

    let mut buf = BytesMut::new();
    buf.put_i32(1);
    buf.put_i16(-1);
    bodies.push(buf);

    let mut buf = BytesMut::new();
    buf.put_i32(1);
    put_str(&mut buf, Some("a"));
    bodies.push(buf);

    let mut buf = BytesMut::new();
    buf.put_i32(1);
    put_str(&mut buf, Some("a"));
    buf.put_i32(1);
    bodies.push(buf);

    let mut buf = BytesMut::new();
    buf.put_i32(1);
    put_str(&mut buf, Some("a"));
    buf.put_i32(1);
    buf.put_i16(1);
    bodies.push(buf);

    let mut buf = BytesMut::new();
    buf.put_i32(1);
    put_str(&mut buf, Some("a"));
    buf.put_i32(1);
    buf.put_i16(1);
    buf.put_i32(1);
    bodies.push(buf);

    let mut buf = BytesMut::new();
    buf.put_i32(1);
    put_str(&mut buf, Some("a"));
    buf.put_i32(1);
    buf.put_i16(1);
    buf.put_i32(1);
    buf.put_i32(0);
    bodies.push(buf);

    let mut buf = BytesMut::new();
    buf.put_i32(1);
    put_str(&mut buf, Some("a"));
    buf.put_i32(1);
    buf.put_i16(1);
    buf.put_i32(1);
    buf.put_i32(0);
    buf.put_i32(1);
    bodies.push(buf);

    let mut buf = BytesMut::new();
    buf.put_i32(1);
    put_str(&mut buf, Some("a"));
    buf.put_i32(1);
    buf.put_i16(1);
    buf.put_i32(1);
    buf.put_i32(0);
    buf.put_i32(1);
    buf.put_i32(0);
    bodies.push(buf);

    let mut buf = BytesMut::new();
    buf.put_i32(1);
    put_str(&mut buf, Some("a"));
    buf.put_i32(1);
    buf.put_i16(1);
    buf.put_i32(0);
    bodies.push(buf);

    let mut buf = BytesMut::new();
    buf.put_i32(1);
    put_str(&mut buf, Some("a"));
    buf.put_i32(1);
    buf.put_i16(1);
    buf.put_i32(0);
    buf.put_i32(1);
    bodies.push(buf);

    let mut buf = BytesMut::new();
    buf.put_i32(1);
    put_str(&mut buf, Some("a"));
    buf.put_i32(1);
    buf.put_i16(1);
    buf.put_i32(0);
    buf.put_i32(1);
    buf.put_i16(1);
    buf.extend_from_slice(b"a");
    bodies.push(buf);

    let mut buf = BytesMut::new();
    buf.put_i32(1);
    put_str(&mut buf, Some("a"));
    buf.put_i32(1);
    buf.put_i16(1);
    buf.put_i32(0);
    buf.put_i32(1);
    buf.put_i16(0);
    buf.put_i16(1);
    buf.extend_from_slice(b"b");
    bodies.push(buf);

    for body in bodies {
        let _ = create_topics::handle(1, &body, &storage).unwrap();
    }
}

#[test]
fn delete_topics_truncated_inputs() {
    let storage = test_storage(true);

    let mut buf = BytesMut::new();
    buf.put_i32(1);
    let response = delete_topics::handle(1, &buf, &storage).unwrap();
    assert!(response.responses.is_empty());

    let mut buf = BytesMut::new();
    buf.put_i32(1);
    buf.put_i16(-1);
    let response = delete_topics::handle(1, &buf, &storage).unwrap();
    assert!(response.responses.is_empty());

    let mut buf = BytesMut::new();
    buf.put_i32(1);
    buf.put_i16(4);
    buf.extend_from_slice(b"a");
    let response = delete_topics::handle(1, &buf, &storage).unwrap();
    assert!(response.responses.is_empty());
}

#[test]
fn fetch_truncated_inputs() {
    let storage = test_storage(true);

    let mut buf = BytesMut::new();
    buf.put_i32(-1);
    let response = fetch::handle(3, &buf, &storage).unwrap();
    assert!(response.responses.is_empty());

    let mut buf = BytesMut::new();
    buf.put_i32(-1);
    buf.put_i32(0);
    let response = fetch::handle(3, &buf, &storage).unwrap();
    assert!(response.responses.is_empty());

    let mut buf = BytesMut::new();
    buf.put_i32(-1);
    buf.put_i32(0);
    buf.put_i32(0);
    let response = fetch::handle(2, &buf, &storage).unwrap();
    assert!(response.responses.is_empty());

    let mut buf = BytesMut::new();
    buf.put_i32(-1);
    buf.put_i32(0);
    buf.put_i32(0);
    buf.put_i32(1024);
    buf.put_i32(1);
    let response = fetch::handle(3, &buf, &storage).unwrap();
    assert!(response.responses.is_empty());

    let mut buf = BytesMut::new();
    buf.put_i32(-1);
    buf.put_i32(0);
    buf.put_i32(0);
    buf.put_i32(1024);
    buf.put_i32(1);
    buf.put_i16(4);
    buf.extend_from_slice(b"a");
    let response = fetch::handle(3, &buf, &storage).unwrap();
    assert!(response.responses.is_empty());

    let mut buf = BytesMut::new();
    buf.put_i32(-1);
    buf.put_i32(0);
    buf.put_i32(0);
    buf.put_i32(1024);
    buf.put_i32(1);
    put_str(&mut buf, Some("topic"));
    let response = fetch::handle(3, &buf, &storage).unwrap();
    assert!(response.responses.is_empty());

    let mut buf = BytesMut::new();
    buf.put_i32(-1);
    buf.put_i32(0);
    buf.put_i32(0);
    buf.put_i32(1024);
    buf.put_i32(1);
    put_str(&mut buf, Some("topic"));
    buf.put_i32(1);
    let response = fetch::handle(3, &buf, &storage).unwrap();
    assert_eq!(response.responses.len(), 1);

    let mut buf = BytesMut::new();
    buf.put_i32(-1);
    buf.put_i32(0);
    buf.put_i32(0);
    buf.put_i32(1024);
    buf.put_i32(1);
    put_str(&mut buf, Some("topic"));
    buf.put_i32(1);
    buf.put_i32(0);
    let response = fetch::handle(3, &buf, &storage).unwrap();
    assert_eq!(response.responses.len(), 1);

    let mut buf = BytesMut::new();
    buf.put_i32(-1);
    buf.put_i32(0);
    buf.put_i32(0);
    buf.put_i32(1024);
    buf.put_i32(1);
    put_str(&mut buf, Some("topic"));
    buf.put_i32(1);
    buf.put_i32(0);
    buf.put_i64(0);
    let response = fetch::handle(3, &buf, &storage).unwrap();
    assert_eq!(response.responses.len(), 1);
}

#[test]
fn fetch_optional_fields_and_records() {
    init_tracing();
    let storage = test_storage(true);
    storage.create_topic("fetch-opt", 1).unwrap();
    let batch = encode_record_batch(&[new_record(0)]);
    storage.append("fetch-opt", 0, &batch).unwrap();

    let mut buf = BytesMut::new();
    buf.put_i32(-1);
    buf.put_i32(0);
    buf.put_i32(0);
    buf.put_i32(1024);
    buf.put_i8(0);
    buf.put_i32(0);
    buf.put_i32(0);
    buf.put_i32(1);
    put_str(&mut buf, Some("fetch-opt"));
    buf.put_i32(1);
    buf.put_i32(0);
    buf.put_i32(0);
    buf.put_i64(0);
    buf.put_i32(0);
    buf.put_i64(0);
    buf.put_i32(1024);

    let response = fetch::handle(12, &buf, &storage).unwrap();
    assert!(response.responses[0].partitions[0].records.is_some());
}

#[test]
fn fetch_sets_log_start_offset() {
    let storage = test_storage(true);
    storage.create_topic("fetch-log", 1).unwrap();
    let batch = encode_record_batch(&[new_record(0)]);
    storage.append("fetch-log", 0, &batch).unwrap();

    let mut buf = BytesMut::new();
    buf.put_i32(-1); // replica_id
    buf.put_i32(0); // max_wait_ms
    buf.put_i32(0); // min_bytes
    buf.put_i32(1024); // max_bytes
    buf.put_i8(0); // isolation_level (v4+)
    buf.put_i32(1); // topic count
    put_str(&mut buf, Some("fetch-log"));
    buf.put_i32(1); // partition count
    buf.put_i32(0); // partition
    buf.put_i64(0); // fetch_offset
    buf.put_i64(0); // log_start_offset (v5+)
    buf.put_i32(1024); // partition_max_bytes

    let response = fetch::handle(5, &buf, &storage).unwrap();
    assert_eq!(response.responses[0].partitions[0].log_start_offset, 0);
}

#[test]
fn list_offsets_truncated_and_errors() {
    let storage = test_storage(true);

    let mut buf = BytesMut::new();
    buf.put_i32(-1);
    let response = list_offsets::handle(1, &buf, &storage).unwrap();
    assert!(response.topics.is_empty());

    let mut buf = BytesMut::new();
    buf.put_i32(-1);
    buf.put_i8(0);
    buf.put_i32(0);
    let response = list_offsets::handle(2, &buf, &storage).unwrap();
    assert!(response.topics.is_empty());

    let mut buf = BytesMut::new();
    buf.put_i32(-1);
    buf.put_i32(1);
    let response = list_offsets::handle(1, &buf, &storage).unwrap();
    assert!(response.topics.is_empty());

    let mut buf = BytesMut::new();
    buf.put_i32(-1);
    buf.put_i32(1);
    buf.put_i16(4);
    buf.extend_from_slice(b"a");
    let response = list_offsets::handle(1, &buf, &storage).unwrap();
    assert!(response.topics.is_empty());

    let mut buf = BytesMut::new();
    buf.put_i32(-1);
    buf.put_i32(1);
    put_str(&mut buf, Some("topic"));
    let response = list_offsets::handle(1, &buf, &storage).unwrap();
    assert!(response.topics.is_empty());

    let mut buf = BytesMut::new();
    buf.put_i32(-1);
    buf.put_i32(1);
    put_str(&mut buf, Some("topic"));
    buf.put_i32(1);
    let response = list_offsets::handle(1, &buf, &storage).unwrap();
    assert_eq!(response.topics.len(), 1);

    let mut buf = BytesMut::new();
    buf.put_i32(-1);
    buf.put_i32(1);
    put_str(&mut buf, Some("topic"));
    buf.put_i32(1);
    buf.put_i32(0);
    let response = list_offsets::handle(1, &buf, &storage).unwrap();
    assert_eq!(response.topics.len(), 1);

    let mut buf = BytesMut::new();
    buf.put_i32(-1);
    buf.put_i8(0);
    buf.put_i32(1);
    put_str(&mut buf, Some("topic"));
    buf.put_i32(1);
    buf.put_i32(0);
    buf.put_i32(0);
    buf.put_i64(-1);
    let response = list_offsets::handle(4, &buf, &storage).unwrap();
    assert_eq!(response.topics.len(), 1);
}

#[test]
fn list_offsets_missing_topics_error_codes() {
    let storage = test_storage(true);

    let mut partition = ListOffsetsPartition::default();
    partition.partition_index = 0;
    partition.timestamp = -1;
    partition.max_num_offsets = 1;
    let mut topic = ListOffsetsTopic::default();
    topic.name = TopicName(StrBytes::from_string("missing".to_string()));
    topic.partitions = vec![partition];
    let mut request = ListOffsetsRequest::default();
    request.replica_id = BrokerId(-1);
    request.topics = vec![topic];
    let body = encode_body(&request, 1);
    let response = list_offsets::handle(1, &body, &storage).unwrap();
    assert_ne!(response.topics[0].partitions[0].error_code, 0);

    let mut request = ListOffsetsRequest::default();
    let mut partition = ListOffsetsPartition::default();
    partition.partition_index = 0;
    partition.timestamp = -2;
    partition.max_num_offsets = 1;
    let mut topic = ListOffsetsTopic::default();
    topic.name = TopicName(StrBytes::from_string("missing".to_string()));
    topic.partitions = vec![partition];
    request.replica_id = BrokerId(-1);
    request.topics = vec![topic];
    let body = encode_body(&request, 1);
    let response = list_offsets::handle(1, &body, &storage).unwrap();
    assert_ne!(response.topics[0].partitions[0].error_code, 0);

    let mut request = ListOffsetsRequest::default();
    let mut partition = ListOffsetsPartition::default();
    partition.partition_index = 0;
    partition.timestamp = 123;
    partition.max_num_offsets = 1;
    let mut topic = ListOffsetsTopic::default();
    topic.name = TopicName(StrBytes::from_string("missing".to_string()));
    topic.partitions = vec![partition];
    request.replica_id = BrokerId(-1);
    request.topics = vec![topic];
    let body = encode_body(&request, 1);
    let response = list_offsets::handle(1, &body, &storage).unwrap();
    assert_ne!(response.topics[0].partitions[0].error_code, 0);
}

#[test]
fn metadata_parsing_edges() {
    let mut config = (*test_config(true)).clone();
    config.host = "0.0.0.0".to_string();
    let config = Arc::new(config);
    let storage = test_storage(true);

    let mut body = BytesMut::new();
    body.put_i32(1);
    put_str(&mut body, Some("newtopic"));
    let response = metadata::handle(1, &body, &storage, &config).unwrap();
    assert!(response.topics.iter().any(|t| t.name.as_ref().map(|n| n.0.as_str()) == Some("newtopic")));

    let response = metadata::handle(1, &[], &storage, &config).unwrap();
    assert!(!response.brokers.is_empty());

    let response = metadata::handle(1, &[0], &storage, &config).unwrap();
    assert!(!response.topics.is_empty());

    let mut buf = BytesMut::new();
    buf.put_i32(0);
    let response = metadata::handle(1, &buf, &storage, &config).unwrap();
    assert!(!response.topics.is_empty());

    let mut buf = BytesMut::new();
    buf.put_i32(1);
    let response = metadata::handle(1, &buf, &storage, &config).unwrap();
    assert!(!response.topics.is_empty());

    let mut buf = BytesMut::new();
    buf.put_i32(1);
    buf.put_i16(4);
    buf.extend_from_slice(b"ab");
    let response = metadata::handle(1, &buf, &storage, &config).unwrap();
    assert!(!response.topics.is_empty());
}

#[test]
fn offset_commit_truncated_and_optional_fields() {
    init_tracing();
    let config = test_config(true);
    let consumer_groups = test_consumer_groups(config);

    let mut buf = BytesMut::new();
    put_str(&mut buf, Some("group"));
    buf.put_i32(1);
    put_str(&mut buf, Some("member"));
    buf.put_i32(1);
    put_str(&mut buf, Some("topic"));
    buf.put_i32(1);
    buf.put_i32(0);
    buf.put_i64(5);
    buf.put_i32(2);
    buf.put_i16(-1);
    let response = offset_commit::handle(6, &buf, consumer_groups.offset_store()).unwrap();
    assert_eq!(response.topics[0].partitions[0].error_code, 0);

    let mut buf = BytesMut::new();
    put_str(&mut buf, Some("group"));
    buf.put_i32(1);
    put_str(&mut buf, Some("member"));
    put_str(&mut buf, None);
    buf.put_i32(0);
    let response = offset_commit::handle(7, &buf, consumer_groups.offset_store()).unwrap();
    assert!(response.topics.is_empty());

    let mut buf = BytesMut::new();
    put_str(&mut buf, Some("group"));
    buf.put_i32(1);
    put_str(&mut buf, Some("member"));
    buf.put_i64(0);
    buf.put_i32(0);
    let response = offset_commit::handle(2, &buf, consumer_groups.offset_store()).unwrap();
    assert!(response.topics.is_empty());

    let mut buf = BytesMut::new();
    put_str(&mut buf, Some("group"));
    buf.put_i32(1);
    put_str(&mut buf, Some("member"));
    buf.put_i32(1);
    put_str(&mut buf, Some("topic"));
    let response = offset_commit::handle(1, &buf, consumer_groups.offset_store()).unwrap();
    assert!(response.topics.is_empty());

    let mut buf = BytesMut::new();
    put_str(&mut buf, Some("group"));
    buf.put_i32(1);
    put_str(&mut buf, Some("member"));
    buf.put_i32(1);
    put_str(&mut buf, Some("topic"));
    buf.put_i32(1);
    let response = offset_commit::handle(1, &buf, consumer_groups.offset_store()).unwrap();
    assert_eq!(response.topics.len(), 1);

    let mut buf = BytesMut::new();
    put_str(&mut buf, Some("group"));
    buf.put_i32(1);
    put_str(&mut buf, Some("member"));
    buf.put_i32(1);
    put_str(&mut buf, Some("topic"));
    buf.put_i32(1);
    buf.put_i32(0);
    let response = offset_commit::handle(1, &buf, consumer_groups.offset_store()).unwrap();
    assert_eq!(response.topics.len(), 1);

    let mut buf = BytesMut::new();
    buf.put_i16(-1);
    let response = offset_commit::handle(1, &buf, consumer_groups.offset_store()).unwrap();
    assert!(response.topics.is_empty());

    let mut buf = BytesMut::new();
    buf.put_i16(4);
    buf.extend_from_slice(b"a");
    let response = offset_commit::handle(1, &buf, consumer_groups.offset_store()).unwrap();
    assert!(response.topics.is_empty());

    let mut buf = BytesMut::new();
    put_str(&mut buf, Some("group"));
    buf.put_i32(1);
    put_str(&mut buf, Some("topic"));
    buf.put_i32(1);
    buf.put_i32(0);
    buf.put_i64(5);
    buf.put_i16(-1);
    let response = offset_commit::handle(0, &buf, consumer_groups.offset_store()).unwrap();
    assert!(response.topics[0].partitions[0].error_code == 0);
}

#[test]
fn offset_fetch_truncated_and_missing_offsets() {
    let config = test_config(true);
    let consumer_groups = test_consumer_groups(config);

    let buf = BytesMut::new();
    let response = offset_fetch::handle(1, &buf, consumer_groups.offset_store()).unwrap();
    assert!(response.topics.is_empty());

    let mut buf = BytesMut::new();
    buf.put_i16(-1);
    let response = offset_fetch::handle(1, &buf, consumer_groups.offset_store()).unwrap();
    assert!(response.topics.is_empty());

    let mut buf = BytesMut::new();
    buf.put_i16(4);
    buf.extend_from_slice(b"a");
    let response = offset_fetch::handle(1, &buf, consumer_groups.offset_store()).unwrap();
    assert!(response.topics.is_empty());

    let mut buf = BytesMut::new();
    put_str(&mut buf, Some("group"));
    buf.put_i32(1);
    put_str(&mut buf, Some("topic"));
    let response = offset_fetch::handle(1, &buf, consumer_groups.offset_store()).unwrap();
    assert!(response.topics.is_empty());

    let mut buf = BytesMut::new();
    put_str(&mut buf, Some("group"));
    buf.put_i32(1);
    put_str(&mut buf, Some("topic"));
    buf.put_i32(1);
    let response = offset_fetch::handle(1, &buf, consumer_groups.offset_store()).unwrap();
    assert_eq!(response.topics.len(), 1);

    let mut buf = BytesMut::new();
    put_str(&mut buf, Some("group"));
    buf.put_i32(1);
    put_str(&mut buf, Some("topic"));
    buf.put_i32(1);
    buf.put_i32(0);
    let response = offset_fetch::handle(1, &buf, consumer_groups.offset_store()).unwrap();
    assert_eq!(response.topics[0].partitions[0].committed_offset, -1);
}

#[test]
fn produce_appends_records() {
    init_tracing();
    let storage = test_storage(true);
    storage.create_topic("produce", 1).unwrap();

    let batch = encode_record_batch(&[new_record(0)]);
    let mut partition = PartitionProduceData::default();
    partition.index = 0;
    partition.records = Some(batch);

    let mut topic = TopicProduceData::default();
    topic.name = TopicName(StrBytes::from_string("produce".to_string()));
    topic.partition_data = vec![partition];

    let mut request = ProduceRequest::default();
    request.acks = 1;
    request.timeout_ms = 1000;
    request.topic_data = vec![topic];

    let body = encode_body(&request, 2);
    let response = produce::handle(2, &body, &storage).unwrap();
    assert_eq!(response.responses[0].partition_responses[0].error_code, 0);
}

#[test]
fn heartbeat_error_and_read_string_cases() {
    let config = test_config(true);
    let consumer_groups = test_consumer_groups(config);

    let mut buf = BytesMut::new();
    put_str(&mut buf, Some("missing"));
    buf.put_i32(1);
    put_str(&mut buf, Some("member"));
    let response = heartbeat::handle(0, &buf, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 16);

    let group = consumer_groups.get_or_create_group("group");
    let member = crate::consumer_group::Member::new(
        "member-1".to_string(),
        "client".to_string(),
        "127.0.0.1".to_string(),
        30000,
        30000,
        "consumer".to_string(),
        vec![("range".to_string(), vec![])],
    );
    group.add_member(member);

    let mut buf = BytesMut::new();
    put_str(&mut buf, Some("group"));
    buf.put_i32(group.generation_id());
    put_str(&mut buf, Some("missing-member"));
    let response = heartbeat::handle(0, &buf, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 25);

    let mut buf = BytesMut::new();
    buf.put_i16(-1);
    buf.put_i32(0);
    let response = heartbeat::handle(0, &buf, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 16);

    let mut buf = BytesMut::new();
    buf.put_i16(4);
    buf.extend_from_slice(b"a");
    let response = heartbeat::handle(0, &buf, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 35);
}

#[test]
fn join_group_additional_error_paths() {
    init_tracing();
    let config = test_config(true);
    let consumer_groups = test_consumer_groups(config);

    let mut buf = BytesMut::new();
    put_str(&mut buf, Some("group"));
    let response = join_group::handle(1, &buf, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 35);

    let mut buf = BytesMut::new();
    put_str(&mut buf, Some("group"));
    buf.put_i32(1);
    let response = join_group::handle(1, &buf, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 35);

    let mut buf = BytesMut::new();
    put_str(&mut buf, Some("group"));
    buf.put_i32(1);
    buf.put_i32(1);
    put_str(&mut buf, Some("member"));
    let response = join_group::handle(1, &buf, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 35);

    let mut buf = BytesMut::new();
    put_str(&mut buf, Some("group"));
    buf.put_i32(1);
    buf.put_i32(1);
    put_str(&mut buf, Some("member"));
    put_str(&mut buf, Some("consumer"));
    let response = join_group::handle(1, &buf, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 35);

    let mut buf = BytesMut::new();
    put_str(&mut buf, Some("group"));
    buf.put_i32(1);
    put_str(&mut buf, Some("member"));
    put_str(&mut buf, Some("consumer"));
    buf.put_i32(0);
    let response = join_group::handle(0, &buf, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 0);

    let mut buf = BytesMut::new();
    put_str(&mut buf, Some("group"));
    buf.put_i32(1);
    buf.put_i32(1);
    put_str(&mut buf, Some("member"));
    put_str(&mut buf, Some("consumer"));
    buf.put_i32(1);
    put_str(&mut buf, Some("range"));
    let response = join_group::handle(1, &buf, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 0);

    let mut buf = BytesMut::new();
    put_str(&mut buf, Some("group"));
    buf.put_i32(1);
    buf.put_i32(1);
    put_str(&mut buf, Some("member"));
    put_str(&mut buf, None);
    let response = join_group::handle(5, &buf, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 35);

    let mut buf = BytesMut::new();
    put_str(&mut buf, Some("group"));
    buf.put_i32(1);
    buf.put_i32(1);
    put_str(&mut buf, Some("member"));
    put_str(&mut buf, Some("consumer"));
    buf.put_i32(1);
    let response = join_group::handle(1, &buf, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 0);
}

#[test]
fn join_group_read_string_edges() {
    init_tracing();
    let config = test_config(true);
    let consumer_groups = test_consumer_groups(config);

    let mut buf = BytesMut::new();
    buf.put_i16(-1);
    buf.put_i32(1);
    put_str(&mut buf, Some("member"));
    put_str(&mut buf, Some("consumer"));
    buf.put_i32(0);
    let response = join_group::handle(0, &buf, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 0);

    let mut buf = BytesMut::new();
    buf.put_i16(4);
    buf.extend_from_slice(b"a");
    let response = join_group::handle(0, &buf, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 35);
}

#[test]
fn leave_group_parses_member_list() {
    init_tracing();
    let config = test_config(true);
    let consumer_groups = test_consumer_groups(config);
    let group = consumer_groups.get_or_create_group("group");
    let member = Member::new(
        "member-1".to_string(),
        "client".to_string(),
        "127.0.0.1".to_string(),
        30000,
        30000,
        "consumer".to_string(),
        vec![("range".to_string(), vec![])],
    );
    group.add_member(member);

    let mut buf = BytesMut::new();
    put_str(&mut buf, Some("group"));
    buf.put_i32(1);
    put_str(&mut buf, Some("member-1"));
    put_str(&mut buf, None);
    let response = leave_group::handle(3, &buf, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 0);
}

#[test]
fn leave_group_missing_member_is_ignored() {
    let config = test_config(true);
    let consumer_groups = test_consumer_groups(config);
    consumer_groups.get_or_create_group("group");

    let mut buf = BytesMut::new();
    put_str(&mut buf, Some("group"));
    buf.put_i32(1);
    put_str(&mut buf, Some("missing"));
    put_str(&mut buf, None);
    let response = leave_group::handle(3, &buf, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 0);
}

#[test]
fn leave_group_truncated_inputs() {
    let config = test_config(true);
    let consumer_groups = test_consumer_groups(config);
    consumer_groups.get_or_create_group("group");

    let mut buf = BytesMut::new();
    put_str(&mut buf, Some("group"));
    let response = leave_group::handle(3, &buf, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 0);

    let mut buf = BytesMut::new();
    put_str(&mut buf, Some("group"));
    buf.put_i16(4);
    buf.extend_from_slice(b"a");
    let response = leave_group::handle(0, &buf, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 0);
}

#[test]
fn sync_group_optional_and_errors() {
    init_tracing();
    let config = test_config(true);
    let consumer_groups = test_consumer_groups(config);

    let mut buf = BytesMut::new();
    put_str(&mut buf, Some("missing"));
    buf.put_i32(1);
    let response = sync_group::handle(0, &buf, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 35);

    let mut buf = BytesMut::new();
    put_str(&mut buf, Some("missing"));
    buf.put_i32(1);
    put_str(&mut buf, Some("member"));
    put_str(&mut buf, Some("instance"));
    put_str(&mut buf, Some("consumer"));
    put_str(&mut buf, Some("range"));
    buf.put_i32(0);
    let response = sync_group::handle(5, &buf, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 16);

    let mut buf = BytesMut::new();
    put_str(&mut buf, Some("missing"));
    buf.put_i32(1);
    put_str(&mut buf, Some("member"));
    buf.put_i32(1);
    put_str(&mut buf, Some("member"));
    let response = sync_group::handle(0, &buf, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 16);

    let group = consumer_groups.get_or_create_group("group");
    let member = crate::consumer_group::Member::new(
        "member-1".to_string(),
        "client".to_string(),
        "127.0.0.1".to_string(),
        30000,
        30000,
        "consumer".to_string(),
        vec![("range".to_string(), vec![])],
    );
    let generation = group.add_member(member);

    let mut buf = BytesMut::new();
    put_str(&mut buf, Some("group"));
    buf.put_i32(generation);
    put_str(&mut buf, Some("missing-member"));
    buf.put_i32(0);
    let response = sync_group::handle(0, &buf, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 25);

    let mut buf = BytesMut::new();
    put_str(&mut buf, Some("group"));
    buf.put_i32(generation);
    put_str(&mut buf, Some("member-1"));
    buf.put_i32(1);
    put_str(&mut buf, Some("member-1"));
    buf.put_i32(0);
    let response = sync_group::handle(0, &buf, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 0);
}

#[test]
fn sync_group_read_string_edges() {
    let config = test_config(true);
    let consumer_groups = test_consumer_groups(config);

    let mut buf = BytesMut::new();
    buf.put_i16(-1);
    buf.put_i32(1);
    buf.put_i32(0);
    let response = sync_group::handle(0, &buf, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 35);

    let mut buf = BytesMut::new();
    buf.put_i16(4);
    buf.extend_from_slice(b"a");
    let response = sync_group::handle(0, &buf, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 35);
}
