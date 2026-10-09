use super::*;
use crate::storage::SingleNodeClusterView;
use crate::storage::{BackendCapabilities, TopicLog};
use crate::test_support::{encode_body, test_config, test_consumer_groups, test_storage};
use heimq_broker::storage::CatalogFuture;
use heimq_protocol::messages::metadata_request::{MetadataRequest, MetadataRequestTopic};
use heimq_protocol::messages::{MetadataResponse, TopicName};
use heimq_protocol::protocol::StrBytes;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

struct AsyncCatalog {
    inner: Arc<dyn LogBackend>,
    lookups: AtomicUsize,
    lists: AtomicUsize,
    creates: AtomicUsize,
    fail: AtomicBool,
    fail_create: AtomicBool,
}

impl AsyncCatalog {
    fn new() -> Self {
        Self {
            inner: test_storage(true),
            lookups: AtomicUsize::new(0),
            lists: AtomicUsize::new(0),
            creates: AtomicUsize::new(0),
            fail: AtomicBool::new(false),
            fail_create: AtomicBool::new(false),
        }
    }
    async fn defer(&self) -> Result<()> {
        tokio::task::yield_now().await;
        if self.fail.load(Ordering::SeqCst) {
            Err(crate::error::HeimqError::Storage(
                "catalog unavailable".into(),
            ))
        } else {
            Ok(())
        }
    }
}

impl LogBackend for AsyncCatalog {
    fn create_topic(&self, _: &str, _: i32) -> Result<Arc<dyn TopicLog>> {
        panic!("synchronous catalog creation")
    }
    fn delete_topic(&self, _: &str) -> Result<()> {
        panic!("unexpected deletion")
    }
    fn list_topics(&self) -> Vec<String> {
        panic!("synchronous catalog listing")
    }
    fn topic(&self, _: &str) -> Option<Arc<dyn TopicLog>> {
        panic!("synchronous catalog lookup")
    }
    fn get_or_create_topic(&self, _: &str, _: i32) -> Arc<dyn TopicLog> {
        panic!("synchronous catalog creation")
    }
    fn get_all_topic_metadata(&self) -> Vec<(String, i32)> {
        panic!("synchronous metadata discovery")
    }
    fn capabilities(&self) -> &BackendCapabilities {
        self.inner.capabilities()
    }
    fn default_num_partitions(&self) -> i32 {
        1
    }
    fn auto_create_topics(&self) -> bool {
        true
    }
    fn append(&self, _: &str, _: i32, _: &[u8]) -> Result<(i64, i64)> {
        panic!("unexpected append")
    }
    fn fetch(&self, _: &str, _: i32, _: i64, _: i32) -> Result<(Vec<u8>, i64)> {
        panic!("unexpected fetch")
    }
    fn high_watermark(&self, _: &str, _: i32) -> Result<i64> {
        panic!("unexpected high watermark")
    }
    fn log_start_offset(&self, _: &str, _: i32) -> Result<i64> {
        panic!("unexpected log start")
    }
    fn topic_async<'a>(&'a self, name: &'a str) -> CatalogFuture<'a, Option<Arc<dyn TopicLog>>> {
        Box::pin(async move {
            self.lookups.fetch_add(1, Ordering::SeqCst);
            self.defer().await?;
            Ok(self.inner.topic(name))
        })
    }
    fn get_all_topic_metadata_async(&self) -> CatalogFuture<'_, Vec<(String, i32)>> {
        Box::pin(async move {
            self.lists.fetch_add(1, Ordering::SeqCst);
            self.defer().await?;
            Ok(self.inner.get_all_topic_metadata())
        })
    }
    fn create_topic_async<'a>(
        &'a self,
        name: &'a str,
        partitions: i32,
    ) -> CatalogFuture<'a, Arc<dyn TopicLog>> {
        Box::pin(async move {
            self.creates.fetch_add(1, Ordering::SeqCst);
            self.defer().await?;
            if self.fail_create.load(Ordering::SeqCst) {
                return Err(crate::error::HeimqError::Storage(
                    "catalog creation unavailable".into(),
                ));
            }
            self.inner.create_topic(name, partitions)
        })
    }
    fn get_or_create_topic_async<'a>(
        &'a self,
        name: &'a str,
        partitions: i32,
    ) -> CatalogFuture<'a, Arc<dyn TopicLog>> {
        self.create_topic_async(name, partitions)
    }
}

fn request(topics: Option<Vec<MetadataRequestTopic>>) -> Vec<u8> {
    let body = encode_body(&MetadataRequest::default().with_topics(topics), 1);
    let mut bytes = BytesMut::new();
    bytes.put_i16(3);
    bytes.put_i16(1);
    bytes.put_i32(17);
    bytes.put_i16(-1);
    bytes.extend_from_slice(&body);
    bytes.to_vec()
}

fn router(catalog: Arc<AsyncCatalog>) -> Router {
    let config = test_config(true);
    Router::new(
        catalog,
        test_consumer_groups(config.clone()),
        SingleNodeClusterView::arc_from_config(&config),
    )
}

#[tokio::test(flavor = "current_thread")]
async fn metadata_router_awaits_catalog_lookup_creation_and_listing() {
    let catalog = Arc::new(AsyncCatalog::new());
    let router = router(catalog.clone());
    let topic = MetadataRequestTopic::default()
        .with_name(Some(TopicName(StrBytes::from_static_str("events"))));
    let response = router
        .route_async(&request(Some(vec![topic.clone()])))
        .await
        .expect("async create");
    let mut body = response.slice(8..);
    let decoded = MetadataResponse::decode(&mut body, 1).expect("wire response");
    assert_eq!(decoded.topics.len(), 1);
    assert_eq!(decoded.topics[0].error_code, 0);
    assert_eq!(decoded.topics[0].partitions.len(), 1);
    assert_eq!(catalog.lookups.load(Ordering::SeqCst), 1);
    assert_eq!(catalog.creates.load(Ordering::SeqCst), 1);
    router
        .route_async(&request(Some(vec![topic])))
        .await
        .expect("existing topic");
    assert_eq!(
        catalog.creates.load(Ordering::SeqCst),
        1,
        "existing metadata must not create again"
    );
    let response = router
        .route_async(&request(None))
        .await
        .expect("async discovery");
    let mut body = response.slice(8..);
    let decoded = MetadataResponse::decode(&mut body, 1).expect("discovered wire response");
    assert_eq!(decoded.topics.len(), 1);
    assert_eq!(catalog.lists.load(Ordering::SeqCst), 1);
}

#[tokio::test(flavor = "current_thread")]
async fn metadata_router_propagates_catalog_failure_without_sync_fallback() {
    let catalog = Arc::new(AsyncCatalog::new());
    catalog.fail.store(true, Ordering::SeqCst);
    let router = router(catalog.clone());
    assert!(router.route_async(&request(None)).await.is_err());
    let topic = MetadataRequestTopic::default()
        .with_name(Some(TopicName(StrBytes::from_static_str("missing"))));
    assert!(router
        .route_async(&request(Some(vec![topic])))
        .await
        .is_err());
    assert_eq!(catalog.creates.load(Ordering::SeqCst), 0);
}

#[tokio::test(flavor = "current_thread")]
async fn metadata_creation_failure_does_not_publish_a_topic_or_use_sync_fallback() {
    let catalog = Arc::new(AsyncCatalog::new());
    catalog.fail_create.store(true, Ordering::SeqCst);
    let router = router(catalog.clone());
    let topic = MetadataRequestTopic::default()
        .with_name(Some(TopicName(StrBytes::from_static_str("missing"))));
    assert!(router
        .route_async(&request(Some(vec![topic])))
        .await
        .is_err());
    assert_eq!(catalog.creates.load(Ordering::SeqCst), 1);
    assert!(catalog.inner.topic("missing").is_none());
}

#[tokio::test(flavor = "current_thread")]
async fn async_metadata_preserves_memory_wire_results_and_existing_then_created_order() {
    let config = test_config(true);
    for version in [1, 4] {
        for auto_create in [true, false] {
            let sync_storage = test_storage(auto_create);
            let async_storage = test_storage(auto_create);
            sync_storage
                .create_topic("known", 1)
                .expect("existing sync topic");
            async_storage
                .create_topic("known", 1)
                .expect("existing async topic");
            let topics = ["missing", "known"]
                .into_iter()
                .map(|name| {
                    MetadataRequestTopic::default()
                        .with_name(Some(TopicName(StrBytes::from_string(name.to_owned()))))
                })
                .collect();
            let request = MetadataRequest::default().with_topics(Some(topics));
            let body = encode_body(&request, version);
            let view = SingleNodeClusterView::new(&config);
            let expected = metadata::handle(version, &body, &sync_storage, &view)
                .expect("sync memory response");
            let actual = metadata::handle_async(version, &body, &async_storage, &view)
                .await
                .expect("async memory response");
            assert_eq!(
                encode_body(&actual, version),
                encode_body(&expected, version)
            );
            assert_eq!(
                actual.topics[0]
                    .name
                    .as_ref()
                    .expect("existing name")
                    .0
                    .as_str(),
                "known"
            );
            assert_eq!(actual.topics[1].error_code, if auto_create { 0 } else { 3 });
        }
    }
}
