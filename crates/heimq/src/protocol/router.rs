//! Request routing

use crate::consumer_group::GroupCoordinatorBackend;
use crate::error::Result;
use crate::handler::*;
use crate::protocol::{decode_request, encode_response, RequestHeader};
use crate::storage::{ClusterView, LogBackend};
use bytes::Bytes;
use kafka_protocol::protocol::Encodable;
use std::sync::Arc;
use tracing::{debug, warn};

/// Routes requests to appropriate handlers
pub struct Router {
    storage: Arc<dyn LogBackend>,
    consumer_groups: Arc<dyn GroupCoordinatorBackend>,
    cluster_view: Arc<dyn ClusterView>,
    /// Effective set of `(api_key, min, max)` advertised by ApiVersions,
    /// computed at startup from each backend's capability descriptor.
    advertised_apis: Arc<Vec<(i16, i16, i16)>>,
}

impl Router {
    pub fn new(
        storage: Arc<dyn LogBackend>,
        consumer_groups: Arc<dyn GroupCoordinatorBackend>,
        cluster_view: Arc<dyn ClusterView>,
    ) -> Self {
        let advertised_apis = Arc::new(crate::protocol::compute_supported_apis(
            storage.capabilities(),
            consumer_groups.offset_store().capabilities(),
            consumer_groups.capabilities(),
        ));
        Self::with_advertised_apis(storage, consumer_groups, cluster_view, advertised_apis)
    }

    pub fn with_advertised_apis(
        storage: Arc<dyn LogBackend>,
        consumer_groups: Arc<dyn GroupCoordinatorBackend>,
        cluster_view: Arc<dyn ClusterView>,
        advertised_apis: Arc<Vec<(i16, i16, i16)>>,
    ) -> Self {
        Self {
            storage,
            consumer_groups,
            cluster_view,
            advertised_apis,
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

    fn encode_response_bytes<R: Encodable>(
        &self,
        header: &RequestHeader,
        response: &R,
    ) -> Result<Bytes> {
        encode_response(header.correlation_id, header.api_key, header.api_version, response)
            .map_err(Into::into)
    }

    fn handle_and_encode<R>(
        &self,
        header: &RequestHeader,
        handler: Box<dyn FnOnce() -> Result<R> + '_>,
    ) -> Result<Bytes>
    where
        R: Encodable,
    {
        let response = handler()?;
        self.encode_response_bytes(header, &response)
    }

    fn handle_api_versions(&self, header: &RequestHeader, _body: &[u8]) -> Result<Bytes> {
        let response = api_versions::handle(header.api_version, &self.advertised_apis);
        self.encode_response_bytes(header, &response)
    }

    fn handle_metadata(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        self.handle_and_encode(
            header,
            Box::new(|| metadata::handle(header.api_version, body, &self.storage, self.cluster_view.as_ref())),
        )
    }

    fn handle_produce(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        self.handle_and_encode(
            header,
            Box::new(|| produce::handle(header.api_version, body, &self.storage)),
        )
    }

    fn handle_fetch(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        self.handle_and_encode(
            header,
            Box::new(|| fetch::handle(header.api_version, body, &self.storage)),
        )
    }

    fn handle_list_offsets(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        self.handle_and_encode(
            header,
            Box::new(|| list_offsets::handle(header.api_version, body, &self.storage)),
        )
    }

    fn handle_create_topics(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        self.handle_and_encode(
            header,
            Box::new(|| create_topics::handle(header.api_version, body, &self.storage)),
        )
    }

    fn handle_delete_topics(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        self.handle_and_encode(
            header,
            Box::new(|| delete_topics::handle(header.api_version, body, &self.storage)),
        )
    }

    fn handle_find_coordinator(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        self.handle_and_encode(
            header,
            Box::new(|| find_coordinator::handle(header.api_version, body, self.cluster_view.as_ref())),
        )
    }

    fn handle_join_group(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        self.handle_and_encode(
            header,
            Box::new(|| join_group::handle(header.api_version, body, self.consumer_groups.as_ref())),
        )
    }

    fn handle_sync_group(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        self.handle_and_encode(
            header,
            Box::new(|| sync_group::handle(header.api_version, body, self.consumer_groups.as_ref())),
        )
    }

    fn handle_heartbeat(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        self.handle_and_encode(
            header,
            Box::new(|| heartbeat::handle(header.api_version, body, self.consumer_groups.as_ref())),
        )
    }

    fn handle_leave_group(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        self.handle_and_encode(
            header,
            Box::new(|| leave_group::handle(header.api_version, body, self.consumer_groups.as_ref())),
        )
    }

    fn handle_offset_commit(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        self.handle_and_encode(
            header,
            Box::new(|| {
                let store = self.consumer_groups.offset_store();
                offset_commit::handle(header.api_version, body, &store)
            }),
        )
    }

    fn handle_offset_fetch(&self, header: &RequestHeader, body: &[u8]) -> Result<Bytes> {
        self.handle_and_encode(
            header,
            Box::new(|| {
                let store = self.consumer_groups.offset_store();
                offset_fetch::handle(header.api_version, body, &store)
            }),
        )
    }

    fn handle_unsupported(&self, header: &RequestHeader) -> Result<Bytes> {
        // Return an error response
        use kafka_protocol::messages::ApiVersionsResponse;
        let response = ApiVersionsResponse::default();
        self.encode_response_bytes(header, &response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::SingleNodeClusterView;
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
    use kafka_protocol::messages::api_versions_response::ApiVersionsResponse;
    use kafka_protocol::messages::create_topics_response::CreateTopicsResponse;
    use kafka_protocol::messages::delete_topics_response::DeleteTopicsResponse;
    use kafka_protocol::messages::fetch_response::FetchResponse;
    use kafka_protocol::messages::find_coordinator_response::FindCoordinatorResponse;
    use kafka_protocol::messages::heartbeat_response::HeartbeatResponse;
    use kafka_protocol::messages::join_group_response::JoinGroupResponse;
    use kafka_protocol::messages::leave_group_response::LeaveGroupResponse;
    use kafka_protocol::messages::list_offsets_response::ListOffsetsResponse;
    use kafka_protocol::messages::metadata_response::MetadataResponse;
    use kafka_protocol::messages::offset_commit_response::OffsetCommitResponse;
    use kafka_protocol::messages::offset_fetch_response::OffsetFetchResponse;
    use kafka_protocol::messages::produce_response::ProduceResponse;
    use kafka_protocol::messages::sync_group_response::SyncGroupResponse;
    use kafka_protocol::messages::{BrokerId, GroupId, TopicName};
    use anyhow::anyhow;
    use kafka_protocol::protocol::{Encodable, StrBytes};

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

    struct FailingEncode;

    impl Encodable for FailingEncode {
        fn encode<B: kafka_protocol::protocol::buf::ByteBufMut>(
            &self,
            _buf: &mut B,
            _version: i16,
        ) -> anyhow::Result<()> {
            Err(anyhow!("boom"))
        }

        fn compute_size(&self, _version: i16) -> anyhow::Result<usize> {
            Ok(0)
        }
    }

    #[test]
    fn route_supported_apis_and_unsupported() {
        init_tracing();
        let config = test_config(true);
        let storage = test_storage(true);
        let consumer_groups = test_consumer_groups(config.clone());
        let cluster_view = SingleNodeClusterView::arc_from_config(&config);
        let router = Router::new(storage.clone(), consumer_groups.clone(), cluster_view);

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

    #[test]
    fn encode_response_maps_errors() {
        let config = test_config(true);
        let storage = test_storage(true);
        let consumer_groups = test_consumer_groups(config.clone());
        let cluster_view = SingleNodeClusterView::arc_from_config(&config);
        let router = Router::new(storage, consumer_groups, cluster_view);

        let header = RequestHeader {
            api_key: 18,
            api_version: 0,
            correlation_id: 1,
            client_id: None,
        };

        let result = router.encode_response_bytes(&header, &FailingEncode);
        assert!(result.is_err());

        let failing = FailingEncode;
        assert_eq!(failing.compute_size(0).unwrap(), 0);

        let err = router.handle_and_encode::<ApiVersionsResponse>(
            &header,
            Box::new(|| Err(crate::error::HeimqError::Protocol("handler-fail".to_string()))),
        );
        assert!(err.is_err());
    }

    #[test]
    fn handle_and_encode_errors_cover_all_responses() {
        let config = test_config(true);
        let storage = test_storage(true);
        let consumer_groups = test_consumer_groups(config.clone());
        let cluster_view = SingleNodeClusterView::arc_from_config(&config);
        let router = Router::new(storage, consumer_groups, cluster_view);

        let header = RequestHeader {
            api_key: 18,
            api_version: 0,
            correlation_id: 1,
            client_id: None,
        };

        fn assert_handler_error<R: Encodable>(router: &Router, header: &RequestHeader) {
            let err = router.handle_and_encode::<R>(
                header,
                Box::new(|| Err(crate::error::HeimqError::Protocol("handler-fail".to_string()))),
            );
            assert!(err.is_err());
        }

        assert_handler_error::<ApiVersionsResponse>(&router, &header);
        assert_handler_error::<MetadataResponse>(&router, &header);
        assert_handler_error::<ProduceResponse>(&router, &header);
        assert_handler_error::<FetchResponse>(&router, &header);
        assert_handler_error::<ListOffsetsResponse>(&router, &header);
        assert_handler_error::<CreateTopicsResponse>(&router, &header);
        assert_handler_error::<DeleteTopicsResponse>(&router, &header);
        assert_handler_error::<FindCoordinatorResponse>(&router, &header);
        assert_handler_error::<JoinGroupResponse>(&router, &header);
        assert_handler_error::<HeartbeatResponse>(&router, &header);
        assert_handler_error::<LeaveGroupResponse>(&router, &header);
        assert_handler_error::<SyncGroupResponse>(&router, &header);
        assert_handler_error::<OffsetCommitResponse>(&router, &header);
        assert_handler_error::<OffsetFetchResponse>(&router, &header);
    }

    // WIRE-001 §6: when the SASL capability gate is OFF (heimq default),
    // SaslHandshake (17) and SaslAuthenticate (36) must not appear in the
    // ApiVersions response.
    #[test]
    fn test_sasl_gate_off_no_advertise() {
        use kafka_protocol::protocol::Decodable;
        let config = test_config(true);
        let storage = test_storage(true);
        let consumer_groups = test_consumer_groups(config.clone());
        let cluster_view = SingleNodeClusterView::arc_from_config(&config);
        let router = Router::new(storage, consumer_groups, cluster_view);

        let body = encode_body(&ApiVersionsRequest::default(), 0);
        let req = build_request(18, 0, 1, &body);
        let resp = router.route(&req).unwrap();

        // Skip 4-byte length prefix + 4-byte correlation_id to reach the body
        let mut body_bytes = bytes::Bytes::copy_from_slice(&resp[8..]);
        let api_versions = ApiVersionsResponse::decode(&mut body_bytes, 0).unwrap();

        let sasl_keys: Vec<i16> = api_versions.api_keys.iter()
            .map(|a| a.api_key)
            .filter(|k| *k == 17 || *k == 36)
            .collect();
        assert!(sasl_keys.is_empty(),
            "SASL gate is OFF: api_keys 17 (SaslHandshake) and 36 (SaslAuthenticate) must not be advertised; found: {:?}",
            sasl_keys);
    }

    // TRAIT-001 adapter: niflheim shape — WAL-style TopicLog that records every
    // appended batch before acking (simulating WAL durability guarantee).
    // Verifies the LogBackend abstraction accommodates this shape.
    #[test]
    fn test_niflheim_shape_wal_adapter() {
        use crate::storage::{BackendCapabilities, TopicLog};
        use std::sync::Mutex;

        struct WalShapeBackend {
            inner: Arc<dyn crate::storage::LogBackend>,
            caps: BackendCapabilities,
            wal: Arc<Mutex<Vec<(String, i32, Vec<u8>)>>>,
        }
        impl crate::storage::LogBackend for WalShapeBackend {
            fn create_topic(&self, n: &str, p: i32) -> crate::error::Result<Arc<dyn TopicLog>> {
                self.inner.create_topic(n, p)
            }
            fn delete_topic(&self, n: &str) -> crate::error::Result<()> { self.inner.delete_topic(n) }
            fn list_topics(&self) -> Vec<String> { self.inner.list_topics() }
            fn topic(&self, n: &str) -> Option<Arc<dyn TopicLog>> { self.inner.topic(n) }
            fn capabilities(&self) -> &BackendCapabilities { &self.caps }
            fn get_or_create_topic(&self, n: &str, p: i32) -> Arc<dyn TopicLog> {
                self.inner.get_or_create_topic(n, p)
            }
            fn get_all_topic_metadata(&self) -> Vec<(String, i32)> { self.inner.get_all_topic_metadata() }
            fn default_num_partitions(&self) -> i32 { 1 }
            fn auto_create_topics(&self) -> bool { true }
            fn append(&self, topic: &str, partition: i32, records: &[u8]) -> crate::error::Result<(i64, i64)> {
                // WAL write BEFORE ack (simulating niflheim durability guarantee)
                self.wal.lock().unwrap().push((topic.to_string(), partition, records.to_vec()));
                self.inner.append(topic, partition, records)
            }
            fn fetch(&self, t: &str, p: i32, o: i64, max: i32) -> crate::error::Result<(Vec<u8>, i64)> {
                self.inner.fetch(t, p, o, max)
            }
            fn high_watermark(&self, t: &str, p: i32) -> crate::error::Result<i64> { self.inner.high_watermark(t, p) }
            fn log_start_offset(&self, t: &str, p: i32) -> crate::error::Result<i64> { self.inner.log_start_offset(t, p) }
        }

        let config = test_config(true);
        let inner = test_storage(true);
        let wal = Arc::new(Mutex::new(Vec::new()));
        let backend: Arc<dyn crate::storage::LogBackend> = Arc::new(WalShapeBackend {
            caps: inner.capabilities().clone(),
            wal: wal.clone(),
            inner,
        });
        let consumer_groups = test_consumer_groups(config.clone());
        let cluster_view = SingleNodeClusterView::arc_from_config(&config);
        let router = Router::new(backend, consumer_groups, cluster_view);

        let batch = encode_record_batch(&[kafka_protocol::records::Record {
            transactional: false, control: false, partition_leader_epoch: 0,
            producer_id: -1, producer_epoch: -1,
            timestamp_type: kafka_protocol::records::TimestampType::Creation,
            timestamp: 0, sequence: 0, offset: 0,
            key: Some("k".into()), value: Some("v".into()), headers: Default::default(),
        }]);
        let mut partition = PartitionProduceData::default();
        partition.index = 0;
        partition.records = Some(batch);
        let mut topic_data = TopicProduceData::default();
        topic_data.name = TopicName(StrBytes::from_string("wal-test".to_string()));
        topic_data.partition_data = vec![partition];
        let mut produce = ProduceRequest::default();
        produce.acks = 1;
        produce.timeout_ms = 1000;
        produce.topic_data = vec![topic_data];
        let body = encode_body(&produce, 2);
        let req = build_request(0, 2, 42, &body);

        let resp = router.route(&req).unwrap();
        assert_eq!(response_correlation_id(resp), 42);

        let guard = wal.lock().unwrap();
        assert_eq!(guard.len(), 1, "WAL must have exactly one entry after one produce");
        assert_eq!(guard[0].0, "wal-test");
        assert_eq!(guard[0].1, 0);
    }

    // TRAIT-001 adapter: pqueue shape — produce-sink backend that enqueues record
    // batches to an mpsc channel instead of a log (simulating queue-enqueue pattern).
    // Verifies the LogBackend abstraction accommodates this shape.
    #[test]
    fn test_pqueue_shape_producer_adapter() {
        use crate::storage::{BackendCapabilities, TopicLog};
        use std::sync::mpsc as std_mpsc;

        struct QueueSinkBackend {
            caps: BackendCapabilities,
            sink: std_mpsc::SyncSender<Vec<u8>>,
        }
        impl crate::storage::LogBackend for QueueSinkBackend {
            fn create_topic(&self, _n: &str, _p: i32) -> crate::error::Result<Arc<dyn TopicLog>> {
                Err(crate::error::HeimqError::Protocol("pqueue-sink: no topics".into()).into())
            }
            fn delete_topic(&self, _n: &str) -> crate::error::Result<()> { Ok(()) }
            fn list_topics(&self) -> Vec<String> { vec![] }
            fn topic(&self, _n: &str) -> Option<Arc<dyn TopicLog>> { None }
            fn capabilities(&self) -> &BackendCapabilities { &self.caps }
            fn get_or_create_topic(&self, n: &str, _p: i32) -> Arc<dyn TopicLog> {
                // Minimal stub to satisfy the produce handler's auto-create path
                Arc::new(crate::storage::MemoryTopicLog::new(n.to_string(), 1))
            }
            fn get_all_topic_metadata(&self) -> Vec<(String, i32)> { vec![] }
            fn default_num_partitions(&self) -> i32 { 1 }
            fn auto_create_topics(&self) -> bool { true }
            fn append(&self, _topic: &str, _partition: i32, records: &[u8]) -> crate::error::Result<(i64, i64)> {
                // Enqueue batch to processing sink (pqueue pattern)
                let _ = self.sink.try_send(records.to_vec());
                Ok((0, 1))
            }
            fn fetch(&self, _t: &str, _p: i32, _o: i64, _max: i32) -> crate::error::Result<(Vec<u8>, i64)> {
                Ok((vec![], 0))
            }
            fn high_watermark(&self, _t: &str, _p: i32) -> crate::error::Result<i64> { Ok(0) }
            fn log_start_offset(&self, _t: &str, _p: i32) -> crate::error::Result<i64> { Ok(0) }
        }

        let config = test_config(true);
        let (tx, rx) = std_mpsc::sync_channel::<Vec<u8>>(16);
        let backend: Arc<dyn crate::storage::LogBackend> = Arc::new(QueueSinkBackend {
            caps: BackendCapabilities::minimal(),
            sink: tx,
        });
        let consumer_groups = test_consumer_groups(config.clone());
        let cluster_view = SingleNodeClusterView::arc_from_config(&config);
        let router = Router::new(backend, consumer_groups, cluster_view);

        let batch = encode_record_batch(&[kafka_protocol::records::Record {
            transactional: false, control: false, partition_leader_epoch: 0,
            producer_id: -1, producer_epoch: -1,
            timestamp_type: kafka_protocol::records::TimestampType::Creation,
            timestamp: 0, sequence: 0, offset: 0,
            key: Some("k".into()), value: Some("v".into()), headers: Default::default(),
        }]);
        let mut partition = PartitionProduceData::default();
        partition.index = 0;
        partition.records = Some(batch);
        let mut topic_data = TopicProduceData::default();
        topic_data.name = TopicName(StrBytes::from_string("queue-topic".to_string()));
        topic_data.partition_data = vec![partition];
        let mut produce = ProduceRequest::default();
        produce.acks = 1;
        produce.timeout_ms = 1000;
        produce.topic_data = vec![topic_data];
        let body = encode_body(&produce, 2);
        let req = build_request(0, 2, 99, &body);

        let resp = router.route(&req).unwrap();
        assert_eq!(response_correlation_id(resp), 99);

        // Verify the record batch arrived at the queue sink
        let received = rx.try_recv().expect("queue sink must have received a batch");
        assert!(!received.is_empty(), "received batch must be non-empty");
    }
}
