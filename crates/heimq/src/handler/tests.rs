use super::*;
use crate::consumer_group::{GroupState, Member, MemoryOffsetStore};
use crate::producer_state::ProducerStateManager;
use crate::storage::SingleNodeClusterView;
use crate::test_support::{encode_body, encode_record_batch, init_tracing, test_config, test_consumer_groups, test_storage};
use crate::transaction_state::TransactionManager;
use bytes::{BufMut, Bytes, BytesMut};
use kafka_protocol::messages::create_topics_request::{CreatableTopic, CreateTopicsRequest};
use kafka_protocol::messages::delete_topics_request::DeleteTopicsRequest;
use kafka_protocol::messages::describe_topic_partitions_request::{DescribeTopicPartitionsRequest, TopicRequest as DtpTopicRequest};
use kafka_protocol::messages::elect_leaders_request::{ElectLeadersRequest, TopicPartitions as ElectLeadersTopicPartitions};
use kafka_protocol::messages::list_transactions_request::ListTransactionsRequest;
use kafka_protocol::messages::offset_for_leader_epoch_request::{OffsetForLeaderEpochRequest, OffsetForLeaderTopic, OffsetForLeaderPartition};
use kafka_protocol::messages::fetch_request::{FetchPartition, FetchRequest, FetchTopic};
use kafka_protocol::messages::find_coordinator_request::FindCoordinatorRequest;
use kafka_protocol::messages::heartbeat_request::HeartbeatRequest;
use kafka_protocol::messages::join_group_request::{JoinGroupRequest, JoinGroupRequestProtocol};
use kafka_protocol::messages::leave_group_request::{LeaveGroupRequest, MemberIdentity};
use kafka_protocol::messages::list_offsets_request::{ListOffsetsPartition, ListOffsetsRequest, ListOffsetsTopic};
use kafka_protocol::messages::metadata_request::{MetadataRequest, MetadataRequestTopic};
use kafka_protocol::messages::offset_commit_request::{OffsetCommitRequest, OffsetCommitRequestPartition, OffsetCommitRequestTopic};
use kafka_protocol::messages::offset_fetch_request::{OffsetFetchRequest, OffsetFetchRequestGroup, OffsetFetchRequestTopic, OffsetFetchRequestTopics};
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
    let response = metadata::handle(1, &body, &storage, &SingleNodeClusterView::new(&config)).unwrap();
    assert!(response.topics.iter().any(|t| t.name.as_ref().map(|n| n.0.as_str()) == Some("t1")));

    let config_no_auto = test_config(false);
    let storage_no_auto = test_storage(false);
    let mut topic = MetadataRequestTopic::default();
    topic.name = Some(TopicName(StrBytes::from_string("missing".to_string())));
    let mut request = MetadataRequest::default();
    request.topics = Some(vec![topic]);
    let body = encode_body(&request, 4);
    let response = metadata::handle(4, &body, &storage_no_auto, &SingleNodeClusterView::new(&config_no_auto)).unwrap();
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
    let response = metadata::handle(4, &body, &storage, &SingleNodeClusterView::new(&config)).unwrap();
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

    let response = metadata::handle(0, &buf, &storage, &SingleNodeClusterView::new(&config)).unwrap();
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
    buf.put_i32(1000); // timeout_ms
    buf.put_i8(0); // validate_only BOOLEAN (v1+)

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
    let response = produce::handle(2, &body, &storage, &ProducerStateManager::new(), &TransactionManager::new()).unwrap();
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
    let response = produce::handle(2, &body, &storage, &ProducerStateManager::new(), &TransactionManager::new()).unwrap();
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
    let response = produce::handle(2, &body, &storage_no_auto, &ProducerStateManager::new(), &TransactionManager::new()).unwrap();
    assert_eq!(response.responses[0].partition_responses[0].error_code, 3);

    let response = produce::handle(2, &[0, 1, 2], &storage, &ProducerStateManager::new(), &TransactionManager::new()).unwrap();
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
    let response = fetch::handle(3, &body, &storage, &TransactionManager::new()).unwrap();
    assert_eq!(response.responses[0].partitions[0].error_code, 0);

    request.topics[0].partitions[0].fetch_offset = 1;
    let body = encode_body(&request, 3);
    let response = fetch::handle(3, &body, &storage, &TransactionManager::new()).unwrap();
    assert_eq!(response.responses[0].partitions[0].error_code, 0);

    request.topics[0].partitions[0].fetch_offset = -1;
    let body = encode_body(&request, 3);
    let response = fetch::handle(3, &body, &storage, &TransactionManager::new()).unwrap();
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
    let response = fetch::handle(3, &body, &storage, &TransactionManager::new()).unwrap();
    assert_eq!(response.responses[0].partitions[0].error_code, 3);

    let response = fetch::handle(3, &[], &storage, &TransactionManager::new()).unwrap();
    assert!(response.responses.is_empty());
}

#[test]
fn find_coordinator_single_node() {
    let mut config = (*test_config(true)).clone();
    config.host = "0.0.0.0".to_string();
    let config = Arc::new(config);
    let response = find_coordinator::handle(0, &[], &SingleNodeClusterView::new(&config)).unwrap();
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
fn sync_group_non_leader_before_leader_syncs_returns_rebalance_in_progress() {
    // Non-leader SyncGroup before the leader has assigned must return
    // REBALANCE_IN_PROGRESS (22) so the follower retries rather than
    // stabilising with an empty assignment.
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
    assert_eq!(response.error_code, 27); // REBALANCE_IN_PROGRESS (22 is ILLEGAL_GENERATION) — must retry without resetting member_id
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
    assert_eq!(response.error_code, 0);
}

#[test]
fn find_coordinator_request_ignored() {
    let config = test_config(true);
    let mut request = FindCoordinatorRequest::default();
    request.key = StrBytes::from_string("group".to_string());
    let body = encode_body(&request, 1);
    let response = find_coordinator::handle(1, &body, &SingleNodeClusterView::new(&config)).unwrap();
    assert_eq!(response.error_code, 0);
}

#[test]
fn find_coordinator_v4_populates_coordinators_array() {
    use kafka_protocol::messages::find_coordinator_request::FindCoordinatorRequest;
    let config = test_config(true);
    let mut request = FindCoordinatorRequest::default();
    request.coordinator_keys = vec![StrBytes::from_string("my-group".to_string())];
    let body = encode_body(&request, 4);
    let response = find_coordinator::handle(4, &body, &SingleNodeClusterView::new(&config)).unwrap();
    // v4: coordinators array must be populated; legacy fields are absent.
    assert_eq!(response.coordinators.len(), 1, "v4 must return one coordinator entry");
    let coord = &response.coordinators[0];
    assert_eq!(coord.error_code, 0);
    assert_eq!(coord.node_id.0, config.broker_id);
    assert!(!coord.host.is_empty(), "coordinator host must be non-empty");
    assert!(coord.port > 0, "coordinator port must be positive");
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
    // All malformed / truncated inputs must not panic — they return empty responses.
    let response = fetch::handle(3, &[], &storage, &TransactionManager::new()).unwrap();
    assert!(response.responses.is_empty());
    let response = fetch::handle(3, &[0xFF, 0x00], &storage, &TransactionManager::new()).unwrap();
    assert!(response.responses.is_empty());
}

#[test]
fn fetch_optional_fields_and_records() {
    init_tracing();
    let storage = test_storage(true);
    storage.create_topic("fetch-opt", 1).unwrap();
    let batch = encode_record_batch(&[new_record(0)]);
    storage.append("fetch-opt", 0, &batch).unwrap();

    // v7: session_id + session_epoch added. Use encode_body for version-correct encoding.
    let mut fp = FetchPartition::default();
    fp.partition = 0;
    fp.fetch_offset = 0;
    fp.partition_max_bytes = 65536;

    let mut ft = FetchTopic::default();
    ft.topic = TopicName(StrBytes::from_string("fetch-opt".to_string()));
    ft.partitions = vec![fp];

    let mut req = FetchRequest::default();
    req.replica_id = BrokerId(-1);
    req.max_wait_ms = 0;
    req.min_bytes = 1;
    req.max_bytes = 65536;
    req.session_id = 0;
    req.session_epoch = -1;
    req.topics = vec![ft];

    let body = encode_body(&req, 7);
    let response = fetch::handle(7, &body, &storage, &TransactionManager::new()).unwrap();
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

    let response = fetch::handle(5, &buf, &storage, &TransactionManager::new()).unwrap();
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
    assert!(response.topics.is_empty()); // partition data truncated → decode fails

    let mut buf = BytesMut::new();
    buf.put_i32(-1);
    buf.put_i32(1);
    put_str(&mut buf, Some("topic"));
    buf.put_i32(1);
    buf.put_i32(0);
    let response = list_offsets::handle(1, &buf, &storage).unwrap();
    assert!(response.topics.is_empty()); // timestamp INT64 missing → decode fails

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
    let response = metadata::handle(1, &body, &storage, &SingleNodeClusterView::new(&config)).unwrap();
    assert!(response.topics.iter().any(|t| t.name.as_ref().map(|n| n.0.as_str()) == Some("newtopic")));

    let response = metadata::handle(1, &[], &storage, &SingleNodeClusterView::new(&config)).unwrap();
    assert!(!response.brokers.is_empty());

    let response = metadata::handle(1, &[0], &storage, &SingleNodeClusterView::new(&config)).unwrap();
    assert!(!response.topics.is_empty());

    let mut buf = BytesMut::new();
    buf.put_i32(0);
    let response = metadata::handle(1, &buf, &storage, &SingleNodeClusterView::new(&config)).unwrap();
    assert!(!response.topics.is_empty());

    let mut buf = BytesMut::new();
    buf.put_i32(1);
    let response = metadata::handle(1, &buf, &storage, &SingleNodeClusterView::new(&config)).unwrap();
    assert!(!response.topics.is_empty());

    let mut buf = BytesMut::new();
    buf.put_i32(1);
    buf.put_i16(4);
    buf.extend_from_slice(b"ab");
    let response = metadata::handle(1, &buf, &storage, &SingleNodeClusterView::new(&config)).unwrap();
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
    assert!(response.topics.is_empty()); // partition_index + offset + metadata missing → decode fails

    let mut buf = BytesMut::new();
    put_str(&mut buf, Some("group"));
    buf.put_i32(1);
    put_str(&mut buf, Some("member"));
    buf.put_i32(1);
    put_str(&mut buf, Some("topic"));
    buf.put_i32(1);
    buf.put_i32(0);
    let response = offset_commit::handle(1, &buf, consumer_groups.offset_store()).unwrap();
    assert!(response.topics.is_empty()); // committed_offset + metadata missing → decode fails

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
    assert!(response.topics.is_empty()); // partition_index INT32 missing → decode fails

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
fn offset_fetch_v8_groups_response_path() {
    use crate::handler::offset_commit;
    use kafka_protocol::messages::offset_commit_request::{OffsetCommitRequestPartition, OffsetCommitRequestTopic};

    let config = test_config(true);
    let consumer_groups = test_consumer_groups(config);

    // Commit an offset at v1 so we have something to fetch.
    let mut commit_part = OffsetCommitRequestPartition::default();
    commit_part.partition_index = 0;
    commit_part.committed_offset = 42;

    let mut commit_topic = OffsetCommitRequestTopic::default();
    commit_topic.name = TopicName(StrBytes::from_string("t".to_string()));
    commit_topic.partitions = vec![commit_part];

    let mut commit_req = OffsetCommitRequest::default();
    commit_req.group_id = GroupId(StrBytes::from_string("g".to_string()));
    commit_req.topics = vec![commit_topic];
    let body = encode_body(&commit_req, 1);
    offset_commit::handle(1, &body, consumer_groups.offset_store()).unwrap();

    // v8 request: uses groups, not group_id + topics.
    let mut fetch_topics = OffsetFetchRequestTopics::default();
    fetch_topics.name = TopicName(StrBytes::from_string("t".to_string()));
    fetch_topics.partition_indexes = vec![0];

    let mut grp = OffsetFetchRequestGroup::default();
    grp.group_id = GroupId(StrBytes::from_string("g".to_string()));
    grp.topics = Some(vec![fetch_topics]);

    let mut req = OffsetFetchRequest::default();
    req.groups = vec![grp];

    let body = encode_body(&req, 8);
    let response = offset_fetch::handle(8, &body, consumer_groups.offset_store()).unwrap();
    assert_eq!(response.groups.len(), 1);
    assert!(response.topics.is_empty()); // v8+ uses groups, not topics
    let g = &response.groups[0];
    assert_eq!(g.topics[0].partitions[0].committed_offset, 42);

    // v8 with topics=None fetches all offsets for the group.
    let mut grp_all = OffsetFetchRequestGroup::default();
    grp_all.group_id = GroupId(StrBytes::from_string("g".to_string()));
    grp_all.topics = None;

    let mut req_all = OffsetFetchRequest::default();
    req_all.groups = vec![grp_all];

    let body = encode_body(&req_all, 8);
    let response = offset_fetch::handle(8, &body, consumer_groups.offset_store()).unwrap();
    assert_eq!(response.groups[0].topics[0].partitions[0].committed_offset, 42);

    // Truncated v8 body → default (empty groups).
    let response = offset_fetch::handle(8, &[], consumer_groups.offset_store()).unwrap();
    assert!(response.groups.is_empty());
}

#[test]
fn produce_oversize_batch_rejected_with_message_too_large() {
    use crate::storage::{BackendCapabilities, LogBackend, MemoryLog};
    use kafka_protocol::messages::TransactionalId;

    let caps = BackendCapabilities {
        name: "in-memory",
        max_message_bytes: 1024,
        max_batch_bytes: 1024,
        ..BackendCapabilities::minimal()
    };
    let storage: Arc<dyn LogBackend> =
        Arc::new(MemoryLog::with_capabilities(test_config(true), caps));
    storage.create_topic("oversize", 1).unwrap();

    let big_value: Vec<u8> = vec![b'x'; 2048];
    let mut record = new_record(0);
    record.value = Some(big_value.into());
    let batch = encode_record_batch(&[record]);
    assert!(batch.len() > 1024, "expected batch > 1KB, got {}", batch.len());

    let mut partition = PartitionProduceData::default();
    partition.index = 0;
    partition.records = Some(batch);

    let mut topic = TopicProduceData::default();
    topic.name = TopicName(StrBytes::from_string("oversize".to_string()));
    topic.partition_data = vec![partition];

    let mut request = ProduceRequest::default();
    request.acks = 1;
    request.timeout_ms = 1000;
    request.topic_data = vec![topic];

    let body = encode_body(&request, 2);
    let response = produce::handle(2, &body, &storage, &ProducerStateManager::new(), &TransactionManager::new()).unwrap();
    assert_eq!(
        response.responses[0].partition_responses[0].error_code,
        10,
        "expected MESSAGE_TOO_LARGE (10)"
    );
    assert_eq!(response.responses[0].partition_responses[0].base_offset, -1);

    // Transactional produce against a backend with transactions=false is rejected.
    let mut tx_partition = PartitionProduceData::default();
    tx_partition.index = 0;
    tx_partition.records = Some(encode_record_batch(&[new_record(0)]));
    let mut tx_topic = TopicProduceData::default();
    tx_topic.name = TopicName(StrBytes::from_string("oversize".to_string()));
    tx_topic.partition_data = vec![tx_partition];
    let mut tx_request = ProduceRequest::default();
    tx_request.transactional_id =
        Some(TransactionalId(StrBytes::from_string("txn-1".to_string())));
    tx_request.acks = -1;
    tx_request.timeout_ms = 1000;
    tx_request.topic_data = vec![tx_topic];
    let body = encode_body(&tx_request, 3);
    let response = produce::handle(3, &body, &storage, &ProducerStateManager::new(), &TransactionManager::new()).unwrap();
    assert_eq!(
        response.responses[0].partition_responses[0].error_code,
        48,
        "expected INVALID_TXN_STATE (48) for transactional produce against non-tx backend"
    );
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
    let response = produce::handle(2, &body, &storage, &ProducerStateManager::new(), &TransactionManager::new()).unwrap();
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
    assert_eq!(response.error_code, 35);

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
    buf.put_i32(0); // empty metadata bytes for protocol
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
    assert_eq!(response.error_code, 35); // protocol name + metadata bytes missing
}

#[test]
fn join_group_read_string_edges() {
    init_tracing();
    let config = test_config(true);
    let consumer_groups = test_consumer_groups(config);

    let mut buf = BytesMut::new();
    buf.put_i16(0); // empty group_id (length=0, not null)
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
    assert_eq!(response.error_code, 35); // v5 flexible requires compact encoding, legacy format fails

    let mut buf = BytesMut::new();
    put_str(&mut buf, Some("missing"));
    buf.put_i32(1);
    put_str(&mut buf, Some("member"));
    buf.put_i32(1);
    put_str(&mut buf, Some("member"));
    let response = sync_group::handle(0, &buf, consumer_groups.as_ref()).unwrap();
    assert_eq!(response.error_code, 35); // assignment BYTES missing → decode fails

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

#[test]
fn list_transactions_returns_empty() {
    let req = ListTransactionsRequest::default();
    let body = encode_body(&req, 0);
    let response = list_transactions::handle(0, &body).unwrap();
    assert_eq!(response.error_code, 0);
    assert!(response.transaction_states.is_empty());
}

#[test]
fn elect_leaders_always_succeeds() {
    let mut req = ElectLeadersRequest::default();
    let mut tp = ElectLeadersTopicPartitions::default();
    tp.topic = TopicName(StrBytes::from_static_str("my-topic"));
    tp.partitions = vec![0, 1, 2];
    req.topic_partitions = Some(vec![tp]);
    let body = encode_body(&req, 0);
    let response = elect_leaders::handle(0, &body).unwrap();
    assert_eq!(response.error_code, 0);
    assert_eq!(response.replica_election_results.len(), 1);
    let result = &response.replica_election_results[0];
    for p in &result.partition_result {
        assert_eq!(p.error_code, 0, "partition {} should succeed", p.partition_id);
    }
}

#[test]
fn elect_leaders_no_topics_returns_empty() {
    let mut req = ElectLeadersRequest::default();
    req.topic_partitions = None;
    let body = encode_body(&req, 0);
    let response = elect_leaders::handle(0, &body).unwrap();
    assert_eq!(response.error_code, 0);
    assert!(response.replica_election_results.is_empty());
}

#[test]
fn offset_for_leader_epoch_returns_hwm() {
    let storage = test_storage(false);
    storage.create_topic("ep-test", 1).unwrap();

    let mut req = OffsetForLeaderEpochRequest::default();
    let mut topic = OffsetForLeaderTopic::default();
    topic.topic = TopicName(StrBytes::from_static_str("ep-test"));
    let mut part = OffsetForLeaderPartition::default();
    part.partition = 0;
    part.leader_epoch = 0;
    part.current_leader_epoch = -1;
    topic.partitions = vec![part];
    req.topics = vec![topic];
    let body = encode_body(&req, 0);
    let response = offset_for_leader_epoch::handle(0, &body, &storage).unwrap();
    assert_eq!(response.topics.len(), 1);
    let tp = &response.topics[0].partitions[0];
    assert_eq!(tp.error_code, 0);
    assert!(tp.end_offset >= 0);
}

#[test]
fn offset_for_leader_epoch_unknown_topic_returns_error3() {
    let storage = test_storage(false);

    let mut req = OffsetForLeaderEpochRequest::default();
    let mut topic = OffsetForLeaderTopic::default();
    topic.topic = TopicName(StrBytes::from_static_str("nonexistent"));
    let mut part = OffsetForLeaderPartition::default();
    part.partition = 0;
    part.leader_epoch = 0;
    part.current_leader_epoch = -1;
    topic.partitions = vec![part];
    req.topics = vec![topic];
    let body = encode_body(&req, 0);
    let response = offset_for_leader_epoch::handle(0, &body, &storage).unwrap();
    let tp = &response.topics[0].partitions[0];
    assert_eq!(tp.error_code, 3);
}

#[test]
fn describe_topic_partitions_known_topic() {
    let config = test_config(false);
    let storage = test_storage(false);
    let cluster_view = SingleNodeClusterView::new(&config);
    storage.create_topic("dtp-test", 2).unwrap();

    let mut req = DescribeTopicPartitionsRequest::default();
    let mut topic_req = DtpTopicRequest::default();
    topic_req.name = TopicName(StrBytes::from_static_str("dtp-test"));
    req.topics = vec![topic_req];
    let body = encode_body(&req, 0);
    let response = describe_topic_partitions::handle(0, &body, &storage, &cluster_view).unwrap();
    assert_eq!(response.topics.len(), 1);
    let t = &response.topics[0];
    assert_eq!(t.error_code, 0);
    assert_eq!(t.partitions.len(), 2);
    for p in &t.partitions {
        assert_eq!(p.error_code, 0);
        assert!(p.leader_id.0 >= 0);
    }
}

#[test]
fn describe_topic_partitions_unknown_topic() {
    let config = test_config(false);
    let storage = test_storage(false);
    let cluster_view = SingleNodeClusterView::new(&config);

    let mut req = DescribeTopicPartitionsRequest::default();
    let mut topic_req = DtpTopicRequest::default();
    topic_req.name = TopicName(StrBytes::from_static_str("no-such-topic"));
    req.topics = vec![topic_req];
    let body = encode_body(&req, 0);
    let response = describe_topic_partitions::handle(0, &body, &storage, &cluster_view).unwrap();
    assert_eq!(response.topics.len(), 1);
    assert_eq!(response.topics[0].error_code, 3);
}

#[test]
fn init_producer_id_assigns_id() {
    use kafka_protocol::messages::init_producer_id_request::InitProducerIdRequest;
    let txn_mgr = TransactionManager::new();
    let req = InitProducerIdRequest::default();
    let body = encode_body(&req, 0);
    let response = init_producer_id::handle(0, &body, &txn_mgr).unwrap();
    assert_eq!(response.error_code, 0);
    assert!(response.producer_id.0 >= 0, "producer_id must be non-negative");
}

#[test]
fn init_producer_id_transactional_assigns_id() {
    use kafka_protocol::messages::init_producer_id_request::InitProducerIdRequest;
    use kafka_protocol::messages::TransactionalId;
    let txn_mgr = TransactionManager::new();
    let mut req = InitProducerIdRequest::default();
    req.transactional_id = Some(TransactionalId(StrBytes::from_static_str("my-txn")));
    req.transaction_timeout_ms = 60000;
    let body = encode_body(&req, 0);
    let response = init_producer_id::handle(0, &body, &txn_mgr).unwrap();
    assert_eq!(response.error_code, 0);
    assert!(response.producer_id.0 >= 0, "transactional producer_id must be non-negative");
}

#[test]
fn describe_cluster_returns_broker_list() {
    use kafka_protocol::messages::describe_cluster_request::DescribeClusterRequest;
    let config = test_config(false);
    let cluster_view = SingleNodeClusterView::new(&config);
    let req = DescribeClusterRequest::default();
    let body = encode_body(&req, 0);
    let response = describe_cluster::handle(0, &body, &cluster_view).unwrap();
    assert_eq!(response.error_code, 0);
    assert!(!response.brokers.is_empty(), "must return at least one broker");
    assert!(!response.cluster_id.is_empty(), "cluster_id must be non-empty");
}

#[test]
fn describe_configs_returns_entries() {
    use kafka_protocol::messages::describe_configs_request::{DescribeConfigsRequest, DescribeConfigsResource};
    let mut req = DescribeConfigsRequest::default();
    let mut resource = DescribeConfigsResource::default();
    resource.resource_type = 2; // TOPIC
    resource.resource_name = StrBytes::from_static_str("my-topic");
    req.resources = vec![resource];
    let body = encode_body(&req, 1);
    let store = crate::config_store::ConfigStore::new();
    let response = describe_configs::handle(1, &body, &store).unwrap();
    assert_eq!(response.results.len(), 1);
    let result = &response.results[0];
    assert_eq!(result.error_code, 0);
    assert!(!result.configs.is_empty(), "must return at least one config entry");
}

/// Helper: DescribeConfigs a topic and return the value reported for `key`.
#[cfg(test)]
fn describe_config_value(store: &crate::config_store::ConfigStore, topic: &str, key: &str) -> Option<String> {
    use kafka_protocol::messages::describe_configs_request::{DescribeConfigsRequest, DescribeConfigsResource};
    let mut req = DescribeConfigsRequest::default();
    let mut resource = DescribeConfigsResource::default();
    resource.resource_type = 2;
    resource.resource_name = StrBytes::from_string(topic.to_string());
    req.resources = vec![resource];
    let body = encode_body(&req, 1);
    let resp = describe_configs::handle(1, &body, store).unwrap();
    resp.results[0]
        .configs
        .iter()
        .find(|c| c.name.as_str() == key)
        .and_then(|c| c.value.as_ref().map(|v| v.to_string()))
}

fn alter_configs_body(topic: &str, key: &str, value: &str) -> Vec<u8> {
    use kafka_protocol::messages::alter_configs_request::{AlterConfigsRequest, AlterConfigsResource, AlterableConfig};
    let mut req = AlterConfigsRequest::default();
    let mut resource = AlterConfigsResource::default();
    resource.resource_type = 2; // TOPIC
    resource.resource_name = StrBytes::from_string(topic.to_string());
    let mut entry = AlterableConfig::default();
    entry.name = StrBytes::from_string(key.to_string());
    entry.value = Some(StrBytes::from_string(value.to_string()));
    resource.configs = vec![entry];
    req.resources = vec![resource];
    encode_body(&req, 0)
}

#[test]
fn alter_configs_round_trips_and_rejects_unknown() {
    let store = crate::config_store::ConfigStore::new();

    // Supported key: stored and reflected by DescribeConfigs (round-trip).
    let body = alter_configs_body("my-topic", "retention.ms", "86400000");
    let response = alter_configs::handle(0, &body, &store).unwrap();
    assert_eq!(response.responses.len(), 1);
    assert_eq!(response.responses[0].error_code, 0);
    assert_eq!(
        describe_config_value(&store, "my-topic", "retention.ms").as_deref(),
        Some("86400000"),
        "AlterConfigs value must round-trip through DescribeConfigs"
    );

    // Unsupported key: rejected with INVALID_CONFIG (40), not silent success.
    let body = alter_configs_body("my-topic", "bogus.unknown.key", "1");
    let response = alter_configs::handle(0, &body, &store).unwrap();
    assert_eq!(response.responses[0].error_code, 40, "unknown key must be INVALID_CONFIG");
}

fn incremental_alter_body(topic: &str, key: &str, op: i8, value: Option<&str>) -> Vec<u8> {
    use kafka_protocol::messages::incremental_alter_configs_request::{
        IncrementalAlterConfigsRequest, AlterConfigsResource, AlterableConfig,
    };
    let mut req = IncrementalAlterConfigsRequest::default();
    let mut resource = AlterConfigsResource::default();
    resource.resource_type = 2; // TOPIC
    resource.resource_name = StrBytes::from_string(topic.to_string());
    let mut entry = AlterableConfig::default();
    entry.name = StrBytes::from_string(key.to_string());
    entry.value = value.map(|v| StrBytes::from_string(v.to_string()));
    entry.config_operation = op;
    resource.configs = vec![entry];
    req.resources = vec![resource];
    encode_body(&req, 0)
}

#[test]
fn incremental_alter_configs_set_then_delete_round_trips() {
    let store = crate::config_store::ConfigStore::new();

    // SET retention.ms -> reflected by DescribeConfigs.
    let body = incremental_alter_body("my-topic", "retention.ms", 0, Some("3600000"));
    let response = incremental_alter_configs::handle(0, &body, &store).unwrap();
    assert_eq!(response.responses[0].error_code, 0);
    assert_eq!(
        describe_config_value(&store, "my-topic", "retention.ms").as_deref(),
        Some("3600000")
    );

    // DELETE retention.ms -> reverts to default.
    let body = incremental_alter_body("my-topic", "retention.ms", 1, None);
    let response = incremental_alter_configs::handle(0, &body, &store).unwrap();
    assert_eq!(response.responses[0].error_code, 0);
    assert_eq!(
        describe_config_value(&store, "my-topic", "retention.ms").as_deref(),
        Some("604800000"),
        "DELETE must revert to the default value"
    );
}

#[test]
fn list_groups_empty() {
    let config = test_config(false);
    let coordinator = test_consumer_groups(config);
    use kafka_protocol::messages::list_groups_request::ListGroupsRequest;
    let req = ListGroupsRequest::default();
    let body = encode_body(&req, 0);
    let response = list_groups::handle(0, &body, coordinator.as_ref()).unwrap();
    assert_eq!(response.error_code, 0);
    assert!(response.groups.is_empty(), "fresh coordinator must have no groups");
}

#[test]
fn describe_groups_unknown_returns_error() {
    let config = test_config(false);
    let coordinator = test_consumer_groups(config);
    use kafka_protocol::messages::describe_groups_request::DescribeGroupsRequest;
    use kafka_protocol::messages::GroupId;
    let mut req = DescribeGroupsRequest::default();
    req.groups = vec![GroupId(StrBytes::from_static_str("no-such-group"))];
    let body = encode_body(&req, 0);
    let response = describe_groups::handle(0, &body, coordinator.as_ref()).unwrap();
    assert_eq!(response.groups.len(), 1);
    assert_ne!(response.groups[0].error_code, 0, "unknown group must return non-zero error_code");
}

#[test]
fn describe_log_dirs_known_topic() {
    use kafka_protocol::messages::describe_log_dirs_request::DescribeLogDirsRequest;
    let storage = test_storage(false);
    storage.create_topic("logdirs-test", 1).unwrap();
    // topics = None means "all topics"; the default gives Some([]) so set it explicitly.
    let mut req = DescribeLogDirsRequest::default();
    req.topics = None;
    let body = encode_body(&req, 0);
    let response = describe_log_dirs::handle(0, &body, &storage).unwrap();
    assert_eq!(response.results.len(), 1);
    assert_eq!(response.results[0].error_code, 0);
    assert!(!response.results[0].topics.is_empty(), "must return at least one topic entry");
}

#[test]
fn create_partitions_expands_topic() {
    use kafka_protocol::messages::create_partitions_request::{CreatePartitionsRequest, CreatePartitionsTopic};
    let storage = test_storage(false);
    storage.create_topic("expand-me", 1).unwrap();
    let mut req = CreatePartitionsRequest::default();
    let mut topic = CreatePartitionsTopic::default();
    topic.name = TopicName(StrBytes::from_static_str("expand-me"));
    topic.count = 3;
    req.topics = vec![topic];
    let body = encode_body(&req, 0);
    let response = create_partitions::handle(0, &body, &storage).unwrap();
    assert_eq!(response.results.len(), 1);
    assert_eq!(response.results[0].error_code, 0, "expand from 1→3 must succeed");
    assert_eq!(storage.topic("expand-me").unwrap().num_partitions(), 3);
}

#[test]
fn delete_records_updates_low_watermark() {
    use kafka_protocol::messages::delete_records_request::{DeleteRecordsRequest, DeleteRecordsTopic, DeleteRecordsPartition};
    let storage = test_storage(false);
    storage.create_topic("del-rec-test", 1).unwrap();
    let batch = encode_record_batch(&[new_record(0), new_record(1), new_record(2), new_record(3)]);
    storage.append("del-rec-test", 0, &batch).unwrap();

    let mut req = DeleteRecordsRequest::default();
    let mut topic = DeleteRecordsTopic::default();
    topic.name = TopicName(StrBytes::from_static_str("del-rec-test"));
    let mut part = DeleteRecordsPartition::default();
    part.partition_index = 0;
    part.offset = 2;
    topic.partitions = vec![part];
    req.topics = vec![topic];
    let body = encode_body(&req, 0);
    let response = delete_records::handle(0, &body, &storage).unwrap();
    assert_eq!(response.topics.len(), 1);
    let pr = &response.topics[0].partitions[0];
    assert_eq!(pr.error_code, 0);
    assert_eq!(pr.low_watermark, 2, "low watermark must advance to the requested offset");
}

#[test]
fn end_txn_unknown_txn_returns_invalid_epoch() {
    use kafka_protocol::messages::end_txn_request::EndTxnRequest;
    use kafka_protocol::messages::{ProducerId, TransactionalId};
    let storage = test_storage(false);
    let offset_store: Arc<dyn crate::storage::OffsetStore> = Arc::new(MemoryOffsetStore::new());
    let txn_mgr = TransactionManager::new();
    let mut req = EndTxnRequest::default();
    req.transactional_id = TransactionalId(StrBytes::from_static_str("nonexistent-txn"));
    req.producer_id = ProducerId(42);
    req.producer_epoch = 0;
    req.committed = true;
    let body = encode_body(&req, 0);
    let response = end_txn::handle(0, &body, &storage, &offset_store, &txn_mgr).unwrap();
    assert_ne!(response.error_code, 0, "unknown transaction must return non-zero error_code");
}

#[test]
fn add_partitions_to_txn_unknown_returns_error() {
    use kafka_protocol::messages::add_partitions_to_txn_request::{AddPartitionsToTxnRequest, AddPartitionsToTxnTopic};
    use kafka_protocol::messages::{ProducerId, TransactionalId};
    let txn_mgr = TransactionManager::new();
    let mut req = AddPartitionsToTxnRequest::default();
    req.v3_and_below_transactional_id = TransactionalId(StrBytes::from_static_str("no-such-txn"));
    req.v3_and_below_producer_id = ProducerId(1);
    req.v3_and_below_producer_epoch = 0;
    let mut topic = AddPartitionsToTxnTopic::default();
    topic.name = TopicName(StrBytes::from_static_str("test-topic"));
    topic.partitions = vec![0];
    req.v3_and_below_topics = vec![topic];
    let body = encode_body(&req, 0);
    let response = add_partitions_to_txn::handle(0, &body, &txn_mgr).unwrap();
    assert_eq!(response.results_by_topic_v3_and_below.len(), 1);
    let err = response.results_by_topic_v3_and_below[0].results_by_partition[0].partition_error_code;
    assert_ne!(err, 0, "unknown txn must return non-zero error on partition");
}

#[test]
fn add_partitions_to_txn_valid_txn_succeeds() {
    use kafka_protocol::messages::add_partitions_to_txn_request::{AddPartitionsToTxnRequest, AddPartitionsToTxnTopic};
    use kafka_protocol::messages::{ProducerId, TransactionalId};
    let txn_mgr = TransactionManager::new();
    // Initialise a real transaction first.
    let (pid, epoch) = txn_mgr.init_transactional_producer("txn-apts");
    let mut req = AddPartitionsToTxnRequest::default();
    req.v3_and_below_transactional_id = TransactionalId(StrBytes::from_static_str("txn-apts"));
    req.v3_and_below_producer_id = ProducerId(pid);
    req.v3_and_below_producer_epoch = epoch;
    let mut topic = AddPartitionsToTxnTopic::default();
    topic.name = TopicName(StrBytes::from_static_str("test-topic"));
    topic.partitions = vec![0];
    req.v3_and_below_topics = vec![topic];
    let body = encode_body(&req, 0);
    let response = add_partitions_to_txn::handle(0, &body, &txn_mgr).unwrap();
    assert_eq!(response.results_by_topic_v3_and_below.len(), 1);
    let err = response.results_by_topic_v3_and_below[0].results_by_partition[0].partition_error_code;
    assert_eq!(err, 0, "valid txn must return 0 error on partition");
}

#[test]
fn add_offsets_to_txn_unknown_returns_error() {
    use kafka_protocol::messages::add_offsets_to_txn_request::AddOffsetsToTxnRequest;
    use kafka_protocol::messages::{GroupId, ProducerId, TransactionalId};
    let txn_mgr = TransactionManager::new();
    let mut req = AddOffsetsToTxnRequest::default();
    req.transactional_id = TransactionalId(StrBytes::from_static_str("no-such-txn"));
    req.producer_id = ProducerId(1);
    req.producer_epoch = 0;
    req.group_id = GroupId(StrBytes::from_static_str("my-group"));
    let body = encode_body(&req, 0);
    let response = add_offsets_to_txn::handle(0, &body, &txn_mgr).unwrap();
    assert_ne!(response.error_code, 0, "unknown txn must return non-zero error");
}

#[test]
fn add_offsets_to_txn_valid_txn_succeeds() {
    use kafka_protocol::messages::add_offsets_to_txn_request::AddOffsetsToTxnRequest;
    use kafka_protocol::messages::{GroupId, ProducerId, TransactionalId};
    let txn_mgr = TransactionManager::new();
    let (pid, epoch) = txn_mgr.init_transactional_producer("txn-aotxn");
    let mut req = AddOffsetsToTxnRequest::default();
    req.transactional_id = TransactionalId(StrBytes::from_static_str("txn-aotxn"));
    req.producer_id = ProducerId(pid);
    req.producer_epoch = epoch;
    req.group_id = GroupId(StrBytes::from_static_str("my-group"));
    let body = encode_body(&req, 0);
    let response = add_offsets_to_txn::handle(0, &body, &txn_mgr).unwrap();
    assert_eq!(response.error_code, 0, "valid txn must return 0 error");
}

#[test]
fn txn_offset_commit_unknown_txn_returns_error() {
    use kafka_protocol::messages::txn_offset_commit_request::{TxnOffsetCommitRequest, TxnOffsetCommitRequestPartition, TxnOffsetCommitRequestTopic};
    use kafka_protocol::messages::{GroupId, ProducerId, TransactionalId};
    let txn_mgr = TransactionManager::new();
    let mut req = TxnOffsetCommitRequest::default();
    req.transactional_id = TransactionalId(StrBytes::from_static_str("no-such-txn"));
    req.group_id = GroupId(StrBytes::from_static_str("my-group"));
    req.producer_id = ProducerId(1);
    req.producer_epoch = 0;
    let mut topic = TxnOffsetCommitRequestTopic::default();
    topic.name = TopicName(StrBytes::from_static_str("test-topic"));
    let mut part = TxnOffsetCommitRequestPartition::default();
    part.partition_index = 0;
    part.committed_offset = 5;
    topic.partitions = vec![part];
    req.topics = vec![topic];
    let body = encode_body(&req, 0);
    let response = txn_offset_commit::handle(0, &body, &txn_mgr).unwrap();
    assert_eq!(response.topics.len(), 1);
    let err = response.topics[0].partitions[0].error_code;
    assert_ne!(err, 0, "unknown txn offset commit must return non-zero error");
}

#[test]
fn write_txn_markers_appends_control_record() {
    use kafka_protocol::messages::write_txn_markers_request::{WriteTxnMarkersRequest, WritableTxnMarker, WritableTxnMarkerTopic};
    use kafka_protocol::messages::ProducerId;
    let storage = test_storage(false);
    storage.create_topic("markers-test", 1).unwrap();
    let txn_mgr = TransactionManager::new();
    let mut marker = WritableTxnMarker::default();
    marker.producer_id = ProducerId(42);
    marker.producer_epoch = 0;
    marker.transaction_result = true;
    let mut topic = WritableTxnMarkerTopic::default();
    topic.name = TopicName(StrBytes::from_static_str("markers-test"));
    topic.partition_indexes = vec![0];
    marker.topics = vec![topic];
    let mut req = WriteTxnMarkersRequest::default();
    req.markers = vec![marker];
    let body = encode_body(&req, 0);
    let response = write_txn_markers::handle(0, &body, &storage, &txn_mgr).unwrap();
    assert_eq!(response.markers.len(), 1);
    let err = response.markers[0].topics[0].partitions[0].error_code;
    assert_eq!(err, 0, "write_txn_markers on existing topic must succeed");
}

#[test]
fn offset_delete_removes_group_offsets() {
    use kafka_protocol::messages::offset_delete_request::{OffsetDeleteRequest, OffsetDeleteRequestTopic, OffsetDeleteRequestPartition};
    let offset_store: Arc<dyn crate::storage::OffsetStore> = Arc::new(MemoryOffsetStore::new());
    let mut req = OffsetDeleteRequest::default();
    req.group_id = GroupId(StrBytes::from_static_str("del-off-grp"));
    let mut topic = OffsetDeleteRequestTopic::default();
    topic.name = TopicName(StrBytes::from_static_str("del-off-test"));
    let mut part = OffsetDeleteRequestPartition::default();
    part.partition_index = 0;
    topic.partitions = vec![part];
    req.topics = vec![topic];
    let body = encode_body(&req, 0);
    let response = offset_delete::handle(0, &body, &offset_store).unwrap();
    assert_eq!(response.error_code, 0);
    assert_eq!(response.topics.len(), 1);
    assert_eq!(response.topics[0].partitions[0].error_code, 0);
}

#[test]
fn delete_groups_removes_empty_group() {
    use kafka_protocol::messages::delete_groups_request::DeleteGroupsRequest;
    let config = test_config(false);
    let coordinator = test_consumer_groups(config);
    let mut req = DeleteGroupsRequest::default();
    req.groups_names = vec![GroupId(StrBytes::from_static_str("nonexistent-group"))];
    let body = encode_body(&req, 0);
    let response = delete_groups::handle(0, &body, coordinator.as_ref()).unwrap();
    assert_eq!(response.results.len(), 1);
    // Deleting a non-existent group returns 0 (already gone).
    assert_eq!(response.results[0].error_code, 0);
}
