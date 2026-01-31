//! Request routing

use crate::config::Config;
use crate::consumer_group::ConsumerGroupManager;
use crate::error::Result;
use crate::handler::*;
use crate::protocol::{decode_request, encode_response, RequestHeader};
use crate::storage::Storage;
use bytes::{Bytes, BytesMut};
use std::sync::Arc;
use tracing::{debug, error, warn};

/// Routes requests to appropriate handlers
pub struct Router {
    storage: Arc<Storage>,
    consumer_groups: Arc<ConsumerGroupManager>,
    config: Arc<Config>,
}

impl Router {
    pub fn new(
        storage: Arc<Storage>,
        consumer_groups: Arc<ConsumerGroupManager>,
        config: Arc<Config>,
    ) -> Self {
        Self {
            storage,
            consumer_groups,
            config,
        }
    }

    /// Route a request and return the response
    pub fn route(&self, data: &[u8]) -> Result<Bytes> {
        let (header, body) = decode_request(data)?;

        debug!(
            api_key = header.api_key,
            api_version = header.api_version,
            correlation_id = header.correlation_id,
            "Routing request"
        );

        let response = match header.api_key {
            0 => self.handle_produce(&header, &body),
            1 => self.handle_fetch(&header, &body),
            2 => self.handle_list_offsets(&header, &body),
            3 => self.handle_metadata(&header, &body),
            8 => self.handle_offset_commit(&header, &body),
            9 => self.handle_offset_fetch(&header, &body),
            10 => self.handle_find_coordinator(&header, &body),
            11 => self.handle_join_group(&header, &body),
            12 => self.handle_heartbeat(&header, &body),
            13 => self.handle_leave_group(&header, &body),
            14 => self.handle_sync_group(&header, &body),
            18 => self.handle_api_versions(&header, &body),
            19 => self.handle_create_topics(&header, &body),
            20 => self.handle_delete_topics(&header, &body),
            _ => {
                warn!(api_key = header.api_key, "Unsupported API");
                self.handle_unsupported(&header)
            }
        };

        response
    }

    fn handle_api_versions(&self, header: &RequestHeader, _body: &[u8]) -> Result<Bytes> {
        let response = api_versions::handle(header.api_version);
        Ok(encode_response(header.correlation_id, header.api_version, &response)?)
    }

    fn handle_metadata(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        let response = metadata::handle(header.api_version, body, &self.storage, &self.config)?;
        Ok(encode_response(header.correlation_id, header.api_version, &response)?)
    }

    fn handle_produce(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        let response = produce::handle(header.api_version, body, &self.storage)?;
        Ok(encode_response(header.correlation_id, header.api_version, &response)?)
    }

    fn handle_fetch(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        let response = fetch::handle(header.api_version, body, &self.storage)?;
        Ok(encode_response(header.correlation_id, header.api_version, &response)?)
    }

    fn handle_list_offsets(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        let response = list_offsets::handle(header.api_version, body, &self.storage)?;
        Ok(encode_response(header.correlation_id, header.api_version, &response)?)
    }

    fn handle_create_topics(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        let response = create_topics::handle(header.api_version, body, &self.storage)?;
        Ok(encode_response(header.correlation_id, header.api_version, &response)?)
    }

    fn handle_delete_topics(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        let response = delete_topics::handle(header.api_version, body, &self.storage)?;
        Ok(encode_response(header.correlation_id, header.api_version, &response)?)
    }

    fn handle_find_coordinator(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        let response = find_coordinator::handle(header.api_version, body, &self.config)?;
        Ok(encode_response(header.correlation_id, header.api_version, &response)?)
    }

    fn handle_join_group(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        let response = join_group::handle(header.api_version, body, &self.consumer_groups)?;
        Ok(encode_response(header.correlation_id, header.api_version, &response)?)
    }

    fn handle_sync_group(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        let response = sync_group::handle(header.api_version, body, &self.consumer_groups)?;
        Ok(encode_response(header.correlation_id, header.api_version, &response)?)
    }

    fn handle_heartbeat(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        let response = heartbeat::handle(header.api_version, body, &self.consumer_groups)?;
        Ok(encode_response(header.correlation_id, header.api_version, &response)?)
    }

    fn handle_leave_group(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        let response = leave_group::handle(header.api_version, body, &self.consumer_groups)?;
        Ok(encode_response(header.correlation_id, header.api_version, &response)?)
    }

    fn handle_offset_commit(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        let response = offset_commit::handle(header.api_version, body, &self.consumer_groups)?;
        Ok(encode_response(header.correlation_id, header.api_version, &response)?)
    }

    fn handle_offset_fetch(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        let response = offset_fetch::handle(header.api_version, body, &self.consumer_groups)?;
        Ok(encode_response(header.correlation_id, header.api_version, &response)?)
    }

    fn handle_unsupported(&self, header: &RequestHeader) -> Result<Bytes> {
        // Return an error response
        use kafka_protocol::messages::ApiVersionsResponse;
        let response = ApiVersionsResponse::default();
        Ok(encode_response(header.correlation_id, 0, &response)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{encode_body, encode_record_batch, init_tracing, test_config, test_consumer_groups, test_storage};
    use bytes::{Buf, BufMut, BytesMut};
    use kafka_protocol::messages::api_versions_request::ApiVersionsRequest;
    use kafka_protocol::messages::create_topics_request::{CreatableTopic, CreateTopicsRequest};
    use kafka_protocol::messages::delete_topics_request::DeleteTopicsRequest;
    use kafka_protocol::messages::fetch_request::{FetchPartition, FetchRequest, FetchTopic};
    use kafka_protocol::messages::find_coordinator_request::FindCoordinatorRequest;
    use kafka_protocol::messages::heartbeat_request::HeartbeatRequest;
    use kafka_protocol::messages::join_group_request::{JoinGroupRequest, JoinGroupRequestProtocol};
    use kafka_protocol::messages::leave_group_request::LeaveGroupRequest;
    use kafka_protocol::messages::list_offsets_request::{ListOffsetsPartition, ListOffsetsRequest, ListOffsetsTopic};
    use kafka_protocol::messages::metadata_request::MetadataRequest;
    use kafka_protocol::messages::offset_commit_request::{OffsetCommitRequest, OffsetCommitRequestPartition, OffsetCommitRequestTopic};
    use kafka_protocol::messages::offset_fetch_request::OffsetFetchRequest;
    use kafka_protocol::messages::produce_request::{PartitionProduceData, ProduceRequest, TopicProduceData};
    use kafka_protocol::messages::sync_group_request::SyncGroupRequest;
    use kafka_protocol::messages::{BrokerId, GroupId, TopicName};
    use kafka_protocol::protocol::StrBytes;

    fn build_request(api_key: i16, api_version: i16, correlation_id: i32, body: &[u8]) -> Vec<u8> {
        let mut buf = BytesMut::new();
        buf.put_i16(api_key);
        buf.put_i16(api_version);
        buf.put_i32(correlation_id);
        buf.put_i16(-1); // null client id
        buf.extend_from_slice(body);
        buf.to_vec()
    }

    fn response_correlation_id(response: Bytes) -> i32 {
        let mut cursor = std::io::Cursor::new(response);
        let _len = cursor.get_i32();
        cursor.get_i32()
    }

    #[test]
    fn route_supported_apis_and_unsupported() {
        init_tracing();
        let config = test_config(true);
        let storage = test_storage(true);
        let consumer_groups = test_consumer_groups(config.clone());
        let router = Router::new(storage.clone(), consumer_groups.clone(), config.clone());

        let correlation_id = 7;

        let body = encode_body(&ApiVersionsRequest::default(), 0);
        let req = build_request(18, 0, correlation_id, &body);
        let resp = router.route(&req).unwrap();
        assert_eq!(response_correlation_id(resp), correlation_id);

        let body = encode_body(&MetadataRequest::default(), 1);
        let req = build_request(3, 1, correlation_id, &body);
        let resp = router.route(&req).unwrap();
        assert_eq!(response_correlation_id(resp), correlation_id);

        let record = kafka_protocol::records::Record {
            transactional: false,
            control: false,
            partition_leader_epoch: 0,
            producer_id: -1,
            producer_epoch: -1,
            timestamp_type: kafka_protocol::records::TimestampType::Creation,
            timestamp: 0,
            sequence: 0,
            offset: 0,
            key: Some("key".into()),
            value: Some("value".into()),
            headers: Default::default(),
        };
        let batch = encode_record_batch(&[record]);
        let mut partition = PartitionProduceData::default();
        partition.index = 0;
        partition.records = Some(batch);
        let mut topic = TopicProduceData::default();
        topic.name = TopicName(StrBytes::from_string("topic".to_string()));
        topic.partition_data = vec![partition];
        let mut produce = ProduceRequest::default();
        produce.acks = 1;
        produce.timeout_ms = 1000;
        produce.topic_data = vec![topic];
        let body = encode_body(&produce, 2);
        let req = build_request(0, 2, correlation_id, &body);
        let resp = router.route(&req).unwrap();
        assert_eq!(response_correlation_id(resp), correlation_id);

        let mut fetch_partition = FetchPartition::default();
        fetch_partition.partition = 0;
        fetch_partition.fetch_offset = 0;
        fetch_partition.partition_max_bytes = 1024;
        let mut fetch_topic = FetchTopic::default();
        fetch_topic.topic = TopicName(StrBytes::from_string("topic".to_string()));
        fetch_topic.partitions = vec![fetch_partition];
        let mut fetch = FetchRequest::default();
        fetch.replica_id = BrokerId(-1);
        fetch.max_wait_ms = 1000;
        fetch.min_bytes = 1;
        fetch.max_bytes = 1024;
        fetch.topics = vec![fetch_topic];
        let body = encode_body(&fetch, 3);
        let req = build_request(1, 3, correlation_id, &body);
        let resp = router.route(&req).unwrap();
        assert_eq!(response_correlation_id(resp), correlation_id);

        let mut list_partition = ListOffsetsPartition::default();
        list_partition.partition_index = 0;
        list_partition.timestamp = -1;
        list_partition.max_num_offsets = 1;
        let mut list_topic = ListOffsetsTopic::default();
        list_topic.name = TopicName(StrBytes::from_string("topic".to_string()));
        list_topic.partitions = vec![list_partition];
        let mut list = ListOffsetsRequest::default();
        list.replica_id = BrokerId(-1);
        list.topics = vec![list_topic];
        let body = encode_body(&list, 1);
        let req = build_request(2, 1, correlation_id, &body);
        let resp = router.route(&req).unwrap();
        assert_eq!(response_correlation_id(resp), correlation_id);

        let mut creatable = CreatableTopic::default();
        creatable.name = TopicName(StrBytes::from_string("topic2".to_string()));
        creatable.num_partitions = 1;
        creatable.replication_factor = 1;
        let mut create = CreateTopicsRequest::default();
        create.topics = vec![creatable];
        create.timeout_ms = 1000;
        create.validate_only = false;
        let body = encode_body(&create, 1);
        let req = build_request(19, 1, correlation_id, &body);
        let resp = router.route(&req).unwrap();
        assert_eq!(response_correlation_id(resp), correlation_id);

        let mut delete = DeleteTopicsRequest::default();
        delete.topic_names = vec![TopicName(StrBytes::from_string("topic2".to_string()))];
        delete.timeout_ms = 1000;
        let body = encode_body(&delete, 1);
        let req = build_request(20, 1, correlation_id, &body);
        let resp = router.route(&req).unwrap();
        assert_eq!(response_correlation_id(resp), correlation_id);

        let mut offset_partition = OffsetCommitRequestPartition::default();
        offset_partition.partition_index = 0;
        offset_partition.committed_offset = 1;
        offset_partition.commit_timestamp = 0;
        let mut offset_topic = OffsetCommitRequestTopic::default();
        offset_topic.name = TopicName(StrBytes::from_string("topic".to_string()));
        offset_topic.partitions = vec![offset_partition];
        let mut offset_commit = OffsetCommitRequest::default();
        offset_commit.group_id = GroupId(StrBytes::from_string("group".to_string()));
        offset_commit.generation_id_or_member_epoch = 1;
        offset_commit.member_id = StrBytes::from_string("member".to_string());
        offset_commit.topics = vec![offset_topic];
        let body = encode_body(&offset_commit, 1);
        let req = build_request(8, 1, correlation_id, &body);
        let resp = router.route(&req).unwrap();
        assert_eq!(response_correlation_id(resp), correlation_id);

        let offset_fetch = OffsetFetchRequest::default()
            .with_group_id(GroupId(StrBytes::from_string("group".to_string())));
        let body = encode_body(&offset_fetch, 1);
        let req = build_request(9, 1, correlation_id, &body);
        let resp = router.route(&req).unwrap();
        assert_eq!(response_correlation_id(resp), correlation_id);

        let find = FindCoordinatorRequest::default();
        let body = encode_body(&find, 1);
        let req = build_request(10, 1, correlation_id, &body);
        let resp = router.route(&req).unwrap();
        assert_eq!(response_correlation_id(resp), correlation_id);

        let mut join_protocol = JoinGroupRequestProtocol::default();
        join_protocol.name = StrBytes::from_string("range".to_string());
        let mut join = JoinGroupRequest::default();
        join.group_id = GroupId(StrBytes::from_string("group".to_string()));
        join.session_timeout_ms = 30000;
        join.rebalance_timeout_ms = 30000;
        join.member_id = StrBytes::from_string(String::new());
        join.protocol_type = StrBytes::from_string("consumer".to_string());
        join.protocols = vec![join_protocol];
        let body = encode_body(&join, 1);
        let req = build_request(11, 1, correlation_id, &body);
        let resp = router.route(&req).unwrap();
        assert_eq!(response_correlation_id(resp), correlation_id);

        let heartbeat = HeartbeatRequest::default()
            .with_group_id(GroupId(StrBytes::from_string("group".to_string())))
            .with_generation_id(1)
            .with_member_id(StrBytes::from_string("member".to_string()));
        let body = encode_body(&heartbeat, 0);
        let req = build_request(12, 0, correlation_id, &body);
        let resp = router.route(&req).unwrap();
        assert_eq!(response_correlation_id(resp), correlation_id);

        let leave = LeaveGroupRequest::default()
            .with_group_id(GroupId(StrBytes::from_string("group".to_string())))
            .with_member_id(StrBytes::from_string("member".to_string()));
        let body = encode_body(&leave, 0);
        let req = build_request(13, 0, correlation_id, &body);
        let resp = router.route(&req).unwrap();
        assert_eq!(response_correlation_id(resp), correlation_id);

        let sync = SyncGroupRequest::default()
            .with_group_id(GroupId(StrBytes::from_string("group".to_string())))
            .with_generation_id(1)
            .with_member_id(StrBytes::from_string("member".to_string()));
        let body = encode_body(&sync, 0);
        let req = build_request(14, 0, correlation_id, &body);
        let resp = router.route(&req).unwrap();
        assert_eq!(response_correlation_id(resp), correlation_id);

        let unsupported = build_request(999, 0, correlation_id, &[]);
        let resp = router.route(&unsupported).unwrap();
        assert_eq!(response_correlation_id(resp), correlation_id);
    }
}
