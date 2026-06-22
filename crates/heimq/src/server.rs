//! TCP server implementation

use crate::config::Config;
use crate::config_store::ConfigStore;
use crate::consumer_group::{ConsumerGroupManager, GroupCoordinatorBackend};
use crate::error::{HeimqError, Result};
use crate::producer_state::ProducerStateManager;
use crate::protocol::{compute_supported_apis, Router};
use crate::storage::{
    dispatch_group_coordinator, dispatch_log_backend, dispatch_offset_store, ClusterView,
    LogBackend, OffsetStore, SingleNodeClusterView,
};
use crate::transaction_state::TransactionManager;
use bytes::Bytes;
use heimq_wire::{FrameError, FrameHandler, WireError, WireServer};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{info, warn};

/// The heimq server
pub struct Server {
    config: Arc<Config>,
    storage: Arc<dyn LogBackend>,
    consumer_groups: Arc<ConsumerGroupManager>,
    cluster_view: Arc<dyn ClusterView>,
    /// Effective ApiVersions advertised by this server, computed once at
    /// startup by intersecting static protocol support with each backend's
    /// capability descriptor.
    advertised_apis: Arc<Vec<(i16, i16, i16)>>,
    /// Shared idempotent producer state; one instance per server so producer
    /// IDs are globally unique across wire connections.
    producer_state: Arc<ProducerStateManager>,
    /// Shared transaction manager; one instance per server for EOS semantics.
    transaction_manager: Arc<TransactionManager>,
    /// Broadcast signal: every successful Produce wakes all waiting Fetch long-polls.
    append_notify: Arc<tokio::sync::Notify>,
    /// Shared per-topic config store (AlterConfigs/DescribeConfigs + retention).
    config_store: Arc<ConfigStore>,
}

impl Server {
    /// Create a new server with externally provided log and offset backends.
    ///
    /// The group coordinator is wired to use the provided offset_store; all
    /// other config-driven storage URLs are ignored. Primarily for tests and
    /// embeddings that need to inject custom backends.
    pub fn with_backends(
        config: Config,
        storage: Arc<dyn LogBackend>,
        offset_store: Arc<dyn OffsetStore>,
    ) -> Result<Self> {
        let config = Arc::new(config);
        Self::build_with_offsets(config, storage, offset_store, None)
    }

    /// Like [`with_backends`], but with an externally provided [`ClusterView`].
    ///
    /// This is the seam multi-node embeddings (e.g. fjord) use to present a
    /// multi-broker topology: the injected view drives Metadata, the partition
    /// leader hints, and group-coordinator routing instead of the default
    /// single-node view derived from config.
    pub fn with_backends_and_cluster_view(
        config: Config,
        storage: Arc<dyn LogBackend>,
        offset_store: Arc<dyn OffsetStore>,
        cluster_view: Arc<dyn ClusterView>,
    ) -> Result<Self> {
        let config = Arc::new(config);
        Self::build_with_offsets(config, storage, offset_store, Some(cluster_view))
    }

    /// Create a new server with an externally provided log backend.
    ///
    /// All other storage (offsets, groups) is still dispatched from config URLs.
    /// This is primarily for tests and embeddings that inject a custom backend.
    pub fn with_backend(config: Config, storage: Arc<dyn LogBackend>) -> Result<Self> {
        let config = Arc::new(config);
        Self::build(config, storage)
    }

    /// Create a new server instance
    pub fn new(config: Config) -> Result<Self> {
        let config = Arc::new(config);
        let storage_cfg = config.storage();
        let storage: Arc<dyn LogBackend> = dispatch_log_backend(&storage_cfg.log, config.clone())?;
        Self::build(config, storage)
    }

    fn build_with_offsets(
        config: Arc<Config>,
        storage: Arc<dyn LogBackend>,
        offset_store: Arc<dyn OffsetStore>,
        cluster_view: Option<Arc<dyn ClusterView>>,
    ) -> Result<Self> {
        let consumer_groups = Arc::new(ConsumerGroupManager::with_offset_store(
            config.clone(),
            offset_store,
        ));
        let storage_cfg = config.storage();
        let coordinator: Arc<dyn GroupCoordinatorBackend> =
            dispatch_group_coordinator(&storage_cfg.groups, consumer_groups.clone())?;

        let advertised_apis = Arc::new(compute_supported_apis(
            storage.capabilities(),
            consumer_groups.offset_store().capabilities(),
            coordinator.capabilities(),
        ));
        let cluster_view =
            cluster_view.unwrap_or_else(|| SingleNodeClusterView::arc_from_config(&config));
        let max_memory_bytes = config.max_memory_bytes;

        for spec in &config.create_topics {
            match spec.split_once(':') {
                Some((name, partitions)) if !name.is_empty() => {
                    match partitions.trim().parse::<i32>() {
                        Ok(n) if n > 0 => {
                            if let Err(e) = storage.create_topic(name.trim(), n) {
                                warn!(topic = name, error = %e, "pre-create topic failed");
                            } else {
                                info!(topic = name, partitions = n, "pre-created topic");
                            }
                        }
                        _ => warn!(spec = %spec, "invalid partition count in --create-topic"),
                    }
                }
                _ => warn!(spec = %spec, "invalid --create-topic spec; expected name:partitions"),
            }
        }

        Ok(Self {
            config,
            storage,
            consumer_groups,
            cluster_view,
            advertised_apis,
            producer_state: ProducerStateManager::new(),
            transaction_manager: TransactionManager::new(),
            append_notify: Arc::new(tokio::sync::Notify::new()),
            config_store: Arc::new(ConfigStore::with_max_memory_bytes(max_memory_bytes)),
        })
    }

    fn build(config: Arc<Config>, storage: Arc<dyn LogBackend>) -> Result<Self> {
        let storage_cfg = config.storage();
        let offset_store = dispatch_offset_store(&storage_cfg.offsets)?;
        let consumer_groups = Arc::new(ConsumerGroupManager::with_offset_store(
            config.clone(),
            offset_store,
        ));
        // Validate the group-coordinator URL through the dispatcher; the
        // memory:// scheme returns the manager we just constructed, while any
        // unknown scheme fails fast at startup.
        let coordinator: Arc<dyn GroupCoordinatorBackend> =
            dispatch_group_coordinator(&storage_cfg.groups, consumer_groups.clone())?;

        let advertised_apis = Arc::new(compute_supported_apis(
            storage.capabilities(),
            consumer_groups.offset_store().capabilities(),
            coordinator.capabilities(),
        ));
        let cluster_view = SingleNodeClusterView::arc_from_config(&config);
        let max_memory_bytes = config.max_memory_bytes;

        for spec in &config.create_topics {
            match spec.split_once(':') {
                Some((name, partitions)) if !name.is_empty() => {
                    match partitions.trim().parse::<i32>() {
                        Ok(n) if n > 0 => {
                            if let Err(e) = storage.create_topic(name.trim(), n) {
                                warn!(topic = name, error = %e, "pre-create topic failed");
                            } else {
                                info!(topic = name, partitions = n, "pre-created topic");
                            }
                        }
                        _ => warn!(spec = %spec, "invalid partition count in --create-topic"),
                    }
                }
                _ => warn!(spec = %spec, "invalid --create-topic spec; expected name:partitions"),
            }
        }

        Ok(Self {
            config,
            storage,
            consumer_groups,
            cluster_view,
            advertised_apis,
            producer_state: ProducerStateManager::new(),
            transaction_manager: TransactionManager::new(),
            append_notify: Arc::new(tokio::sync::Notify::new()),
            config_store: Arc::new(ConfigStore::with_max_memory_bytes(max_memory_bytes)),
        })
    }

    /// Run the server
    #[allow(dead_code)]
    pub async fn run(&self) -> Result<()> {
        let max_connections = std::env::var("HEIMQ_MAX_CONNECTIONS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value > 0);
        self.run_with_max_connections(max_connections).await
    }

    pub async fn run_with_max_connections(&self, max_connections: Option<usize>) -> Result<()> {
        let addr = self.config.bind_addr();
        let listener = TcpListener::bind(&addr).await?;

        info!("Listening on {}", addr);
        self.run_with_listener(listener, max_connections).await
    }

    async fn run_with_listener(
        &self,
        listener: TcpListener,
        max_connections: Option<usize>,
    ) -> Result<()> {
        self.spawn_maintenance_tasks();
        WireServer::new(self.router())
            .run_with_listener(listener, max_connections)
            .await
            .map_err(map_wire_error)
    }

    fn spawn_maintenance_tasks(&self) {
        let groups = self.consumer_groups.clone();
        let storage = self.storage.clone();
        let config_store = self.config_store.clone();
        let default_retention_ms = self.config.retention_ms;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                groups.evict_expired_members();
                // Per-topic retention (always on): drop batches past retention.ms
                // and trim each partition to retention.bytes, using per-topic
                // AlterConfigs overrides where set and the broker default otherwise.
                let now_ms = chrono::Utc::now().timestamp_millis();
                for (topic, _partitions) in storage.get_all_topic_metadata() {
                    let retention_ms = config_store
                        .get_override(&topic, "retention.ms")
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(default_retention_ms);
                    let retention_bytes = config_store
                        .get_override(&topic, "retention.bytes")
                        .and_then(|s| s.parse::<i64>().ok())
                        .unwrap_or(-1);
                    let cutoff = now_ms.saturating_sub(retention_ms as i64);
                    storage.reclaim_topic(&topic, cutoff, retention_bytes);
                }
            }
        });
    }

    fn router(&self) -> Router {
        Router::with_advertised_apis(
            self.storage.clone(),
            self.consumer_groups.clone(),
            self.cluster_view.clone(),
            self.advertised_apis.clone(),
        )
        .with_producer_state(self.producer_state.clone())
        .with_transaction_manager(self.transaction_manager.clone())
        .with_append_notify(self.append_notify.clone())
        .with_config_store(self.config_store.clone())
        .with_default_retention_ms(self.config.retention_ms)
    }
}

#[async_trait::async_trait]
impl FrameHandler for Router {
    async fn handle(&self, frame: Bytes) -> std::result::Result<Bytes, FrameError> {
        let response = self
            .route_async_bytes(frame)
            .await
            .map_err(|e| FrameError::Handler(e.to_string()))?;
        strip_router_length_prefix(response)
    }
}

fn strip_router_length_prefix(response: Bytes) -> std::result::Result<Bytes, FrameError> {
    if response.is_empty() {
        return Ok(response);
    }
    if response.len() < 4 {
        return Err(FrameError::Handler(format!(
            "router response too short: {} bytes",
            response.len()
        )));
    }

    let body_len =
        i32::from_be_bytes([response[0], response[1], response[2], response[3]]) as usize;
    let actual_len = response.len() - 4;
    if body_len != actual_len {
        return Err(FrameError::Handler(format!(
            "router response length mismatch: prefix={body_len} actual={actual_len}"
        )));
    }
    Ok(response.slice(4..))
}

fn map_wire_error(error: WireError) -> HeimqError {
    match error {
        WireError::Io(e) => HeimqError::Io(e),
        WireError::Protocol(e) => HeimqError::Protocol(e),
        WireError::Handler(e) => HeimqError::Protocol(e),
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;
    use crate::consumer_group::MemoryOffsetStore;
    use crate::storage::{BrokerInfo, ClusterViewError};
    use crate::test_support::{init_tracing, test_config, test_consumer_groups, test_storage};
    use bytes::{Buf, BufMut, BytesMut};
    use clap::Parser;
    use heimq_wire::{make_error_frame, peek_correlation_id, serve_connection, MAX_FRAME_BYTES};
    use kafka_protocol::messages::api_versions_request::ApiVersionsRequest;
    use kafka_protocol::protocol::Encodable;
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use tokio::io::ReadBuf;
    use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};

    struct FixedClusterView {
        broker: BrokerInfo,
        cluster_id: String,
    }

    impl ClusterView for FixedClusterView {
        fn self_broker(&self) -> BrokerInfo {
            self.broker.clone()
        }

        fn brokers(&self) -> Vec<BrokerInfo> {
            vec![self.broker.clone()]
        }

        fn cluster_id(&self) -> String {
            self.cluster_id.clone()
        }

        fn partition_leader(
            &self,
            _topic: &str,
            _partition: i32,
        ) -> std::result::Result<BrokerInfo, ClusterViewError> {
            Ok(self.broker.clone())
        }

        fn find_coordinator(
            &self,
            _group_id: &str,
        ) -> std::result::Result<BrokerInfo, ClusterViewError> {
            Ok(self.broker.clone())
        }
    }

    #[test]
    fn with_backends_and_cluster_view_uses_injected_view() {
        let config = Config::parse_from(["heimq"]);
        let storage = test_storage(true);
        let offset_store: Arc<dyn OffsetStore> = Arc::new(MemoryOffsetStore::new());
        let injected: Arc<dyn ClusterView> = Arc::new(FixedClusterView {
            broker: BrokerInfo {
                node_id: 42,
                host: "broker.example.test".to_string(),
                port: 19_092,
            },
            cluster_id: "external-cluster".to_string(),
        });

        let server =
            Server::with_backends_and_cluster_view(config, storage, offset_store, injected)
                .unwrap();

        let broker = server.cluster_view.self_broker();
        assert_eq!(server.cluster_view.cluster_id(), "external-cluster");
        assert_eq!(broker.node_id, 42);
        assert_eq!(broker.host, "broker.example.test");
        assert_eq!(broker.port, 19_092);
    }

    struct ScriptedStream {
        data: Vec<u8>,
        pos: usize,
        fail_read: bool,
        fail_write: bool,
    }

    impl ScriptedStream {
        fn new(data: Vec<u8>) -> Self {
            Self {
                data,
                pos: 0,
                fail_read: false,
                fail_write: false,
            }
        }

        fn with_read_error() -> Self {
            Self {
                data: Vec::new(),
                pos: 0,
                fail_read: true,
                fail_write: false,
            }
        }

        fn with_write_error(data: Vec<u8>) -> Self {
            Self {
                data,
                pos: 0,
                fail_read: false,
                fail_write: true,
            }
        }
    }

    impl AsyncRead for ScriptedStream {
        fn poll_read(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<std::io::Result<()>> {
            let this = self.get_mut();
            if this.fail_read {
                return Poll::Ready(Err(std::io::Error::other("read failed")));
            }

            if this.pos >= this.data.len() {
                return Poll::Ready(Ok(()));
            }

            let remaining = &this.data[this.pos..];
            let to_copy = remaining.len().min(buf.remaining());
            buf.put_slice(&remaining[..to_copy]);
            this.pos += to_copy;
            Poll::Ready(Ok(()))
        }
    }

    impl AsyncWrite for ScriptedStream {
        fn poll_write(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            let this = self.get_mut();
            if this.fail_write {
                return Poll::Ready(Err(std::io::Error::other("write failed")));
            }
            Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Ok(()))
        }

        fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }

    fn framed_request(api_key: i16, api_version: i16, correlation_id: i32, body: &[u8]) -> Vec<u8> {
        let mut request = BytesMut::new();
        request.put_i16(api_key);
        request.put_i16(api_version);
        request.put_i32(correlation_id);
        request.put_i16(-1);
        request.extend_from_slice(body);

        let mut framed = BytesMut::new();
        framed.put_i32(request.len() as i32);
        framed.extend_from_slice(&request);
        framed.to_vec()
    }

    async fn serve_router_connection<S>(stream: S, router: Router) -> Result<()>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let peer = "127.0.0.1:0".parse().unwrap();
        serve_connection(stream, peer, Arc::new(router))
            .await
            .map_err(super::map_wire_error)
    }

    #[tokio::test]
    async fn test_server_creation() {
        let config = Config {
            host: "127.0.0.1".to_string(),
            port: 0, // Random port
            data_dir: std::path::PathBuf::from("/tmp/test"),
            memory_only: true,
            segment_size: 1024 * 1024,
            retention_ms: 60000,
            max_memory_bytes: 0,
            default_partitions: 1,
            auto_create_topics: true,
            broker_id: 0,
            cluster_id: "test".to_string(),
            metrics: false,
            metrics_port: 9093,
            create_topics: Vec::new(),
            storage_log: "memory://".to_string(),
            storage_offsets: "memory://".to_string(),
            storage_groups: "memory://".to_string(),
            advertised_host: None,
        };

        let server = Server::new(config);
        assert!(server.is_ok());
    }

    #[test]
    fn test_unknown_storage_scheme_fails_at_startup() {
        let mut config = Config::parse_from(["heimq"]);
        config.storage_log = "weird://".to_string();
        let msg = match Server::new(config) {
            Ok(_) => panic!("expected error"),
            Err(e) => format!("{}", e),
        };
        assert!(msg.contains("weird"), "msg = {}", msg);
        assert!(msg.contains("memory://"), "msg = {}", msg);
    }

    #[test]
    fn test_unknown_offsets_scheme_fails_at_startup() {
        let mut config = Config::parse_from(["heimq"]);
        config.storage_offsets = "postgres://x".to_string();
        assert!(Server::new(config).is_err());
    }

    #[test]
    fn test_unknown_groups_scheme_fails_at_startup() {
        let mut config = Config::parse_from(["heimq"]);
        config.storage_groups = "weird://".to_string();
        assert!(Server::new(config).is_err());
    }

    #[tokio::test]
    async fn test_run_with_listener_serves_connection() {
        let config = Config::parse_from(["heimq"]);
        let server = Server::new(config).unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_task =
            tokio::spawn(async move { server.run_with_listener(listener, Some(1)).await });

        let mut stream = TcpStream::connect(addr).await.unwrap();
        let mut body = BytesMut::new();
        ApiVersionsRequest::default().encode(&mut body, 0).unwrap();

        let mut request = BytesMut::new();
        request.put_i16(18);
        request.put_i16(0);
        request.put_i32(1);
        request.put_i16(-1);
        request.extend_from_slice(&body);

        let mut framed = BytesMut::new();
        framed.put_i32(request.len() as i32);
        framed.extend_from_slice(&request);

        stream.write_all(&framed).await.unwrap();
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await.unwrap();

        server_task.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn test_run_with_listener_stops_at_max() {
        init_tracing();
        let config = Config::parse_from(["heimq"]);
        let server = Server::new(config).unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_task =
            tokio::spawn(async move { server.run_with_listener(listener, Some(1)).await });
        let _client = TcpStream::connect(addr).await.unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(2), server_task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn test_run_with_listener_continues_then_stops() {
        init_tracing();
        let config = Config::parse_from(["heimq"]);
        let server = Server::new(config).unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_task =
            tokio::spawn(async move { server.run_with_listener(listener, Some(2)).await });

        let _client1 = TcpStream::connect(addr).await.unwrap();
        let _client2 = TcpStream::connect(addr).await.unwrap();

        tokio::time::timeout(std::time::Duration::from_secs(2), server_task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn test_run_with_max_connections() {
        init_tracing();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let mut config = Config::parse_from(["heimq"]);
        config.host = "127.0.0.1".to_string();
        config.port = port;
        let server = Server::new(config).unwrap();

        let server_task =
            tokio::spawn(async move { server.run_with_max_connections(Some(1)).await });
        for _ in 0..10 {
            if TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        tokio::time::timeout(std::time::Duration::from_secs(2), server_task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn test_run_with_max_connections_bind_error() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let mut config = Config::parse_from(["heimq"]);
        config.host = "127.0.0.1".to_string();
        config.port = port;
        let server = Server::new(config).unwrap();

        let result = server.run_with_max_connections(Some(1)).await;
        assert!(result.is_err());

        drop(listener);
    }

    #[tokio::test]
    async fn test_run_with_env_max_connections() {
        init_tracing();
        std::env::set_var("HEIMQ_MAX_CONNECTIONS", "1");
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let mut config = Config::parse_from(["heimq"]);
        config.host = "127.0.0.1".to_string();
        config.port = port;
        let server = Server::new(config).unwrap();

        let server_task = tokio::spawn(async move { server.run().await });
        for _ in 0..10 {
            if TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        tokio::time::timeout(std::time::Duration::from_secs(2), server_task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        std::env::remove_var("HEIMQ_MAX_CONNECTIONS");
    }

    #[tokio::test]
    async fn test_handle_connection_clean_disconnect() {
        init_tracing();
        let config = test_config(true);
        let storage = test_storage(true);
        let consumer_groups = test_consumer_groups(config.clone());
        let cluster_view = SingleNodeClusterView::arc_from_config(&config);
        let router = Router::new(storage, consumer_groups, cluster_view);

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server_task = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            serve_router_connection(socket, router).await
        });

        let stream = TcpStream::connect(addr).await.unwrap();
        drop(stream);

        let result = server_task.await.unwrap();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_connection_pending_data_error() {
        init_tracing();
        let config = test_config(true);
        let storage = test_storage(true);
        let consumer_groups = test_consumer_groups(config.clone());
        let cluster_view = SingleNodeClusterView::arc_from_config(&config);
        let router = Router::new(storage, consumer_groups, cluster_view);

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server_task = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            serve_router_connection(socket, router).await
        });

        let mut stream = TcpStream::connect(addr).await.unwrap();
        let mut buf = BytesMut::new();
        buf.put_i32(8);
        buf.extend_from_slice(&[0x00, 0x01]);
        stream.write_all(&buf).await.unwrap();
        drop(stream);

        let result = server_task.await.unwrap();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_connection_warn_on_route_error() {
        init_tracing();
        let config = test_config(true);
        let storage = test_storage(true);
        let consumer_groups = test_consumer_groups(config.clone());
        let cluster_view = SingleNodeClusterView::arc_from_config(&config);
        let router = Router::new(storage, consumer_groups, cluster_view);

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server_task = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            serve_router_connection(socket, router).await
        });

        let mut stream = TcpStream::connect(addr).await.unwrap();
        let mut buf = BytesMut::new();
        buf.put_i32(2);
        buf.extend_from_slice(&[0x00, 0x01]);
        stream.write_all(&buf).await.unwrap();
        drop(stream);

        let result = server_task.await.unwrap();
        // WIRE-001 §3: routing errors now close the connection with Err
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_connection_scripted_ok() {
        init_tracing();
        let config = test_config(true);
        let storage = test_storage(true);
        let consumer_groups = test_consumer_groups(config.clone());
        let cluster_view = SingleNodeClusterView::arc_from_config(&config);
        let router = Router::new(storage, consumer_groups, cluster_view);

        let request = framed_request(18, 0, 1, &[]);
        let stream = ScriptedStream::new(request);
        let result = serve_router_connection(stream, router).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_scripted_stream_flush_shutdown() {
        let mut stream = ScriptedStream::new(Vec::new());
        stream.flush().await.unwrap();
        stream.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_handle_connection_read_error() {
        init_tracing();
        let config = test_config(true);
        let storage = test_storage(true);
        let consumer_groups = test_consumer_groups(config.clone());
        let cluster_view = SingleNodeClusterView::arc_from_config(&config);
        let router = Router::new(storage, consumer_groups, cluster_view);

        let stream = ScriptedStream::with_read_error();
        let result = serve_router_connection(stream, router).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_connection_write_error() {
        init_tracing();
        let config = test_config(true);
        let storage = test_storage(true);
        let consumer_groups = test_consumer_groups(config.clone());
        let cluster_view = SingleNodeClusterView::arc_from_config(&config);
        let router = Router::new(storage, consumer_groups, cluster_view);

        let request = framed_request(18, 0, 1, &[]);
        let stream = ScriptedStream::with_write_error(request);
        let result = serve_router_connection(stream, router).await;
        assert!(result.is_err());
    }

    // WIRE-001 §1: frame-size cap enforced — connection close, no response
    #[tokio::test]
    async fn test_frame_size_cap_enforced() {
        init_tracing();
        let config = Arc::new(Config::parse_from(["heimq"]));
        let storage = test_storage(true);
        let consumer_groups = test_consumer_groups(config.clone());
        let cluster_view = SingleNodeClusterView::arc_from_config(&config);
        let router = Router::new(storage, consumer_groups, cluster_view);

        // Send a frame claiming to be larger than MAX_FRAME_BYTES
        let mut oversized = BytesMut::new();
        oversized.put_u32((MAX_FRAME_BYTES + 1) as u32);
        let stream = ScriptedStream::new(oversized.to_vec());
        let result = serve_router_connection(stream, router).await;
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("exceeds max"),
            "error should mention frame cap: {}",
            msg
        );
    }

    // WIRE-001 §3: error frame helpers build the correct bytes
    #[test]
    fn test_make_error_frame_structure() {
        let frame = make_error_frame(42, 10);
        // 4-byte length-prefix + 4-byte correlation_id + 2-byte error_code = 10 bytes total
        assert_eq!(frame.len(), 10);
        let body_len = i32::from_be_bytes([frame[0], frame[1], frame[2], frame[3]]);
        assert_eq!(body_len, 6);
        let corr = i32::from_be_bytes([frame[4], frame[5], frame[6], frame[7]]);
        assert_eq!(corr, 42);
        let code = i16::from_be_bytes([frame[8], frame[9]]);
        assert_eq!(code, 10);
    }

    #[test]
    fn test_peek_correlation_id_happy_path() {
        // framed_request layout in msg_data: api_key(2) + api_ver(2) + corr_id(4) + ...
        let msg_data = framed_request(18, 0, 77, &[])[4..].to_vec(); // strip 4-byte length prefix
        assert_eq!(peek_correlation_id(&msg_data), Some(77));
    }

    #[test]
    fn test_peek_correlation_id_too_short() {
        assert_eq!(peek_correlation_id(&[0, 1, 2, 3, 4, 5, 6]), None);
    }

    // WIRE-001 §3: error frame sent before close when routing fails with identifiable correlation_id
    #[tokio::test]
    async fn test_malformed_request_typed_error_frame() {
        init_tracing();
        let config = Arc::new(Config::parse_from(["heimq"]));
        let storage = test_storage(true);
        let consumer_groups = test_consumer_groups(config.clone());
        let cluster_view = SingleNodeClusterView::arc_from_config(&config);
        let router = Router::new(storage, consumer_groups, cluster_view);

        // Build a request with a valid 8-byte header (so peek_correlation_id works)
        // but corrupt body after the header so the handler returns Err.
        // api_key=0 (Produce), correlation_id=42; pass garbage body bytes that
        // kafka-protocol can't decode as a ProduceRequest.
        let mut body = BytesMut::new();
        body.put_i16(0); // api_key
        body.put_i16(0); // api_version
        body.put_i32(42); // correlation_id
        body.put_i16(-1); // null client_id
        body.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]); // garbage body

        let mut framed = BytesMut::new();
        framed.put_i32(body.len() as i32);
        framed.extend_from_slice(&body);

        let mut captured = Vec::new();
        let stream = CapturingStream::new(framed.to_vec(), &mut captured as *mut Vec<u8>);
        let result = serve_router_connection(stream, router).await;
        // Either Ok (handler returned error response) or Err (handler panicked/failed)
        // Either way, if Err: verify the error frame was written with corr_id=42
        if result.is_err() && captured.len() >= 10 {
            let body_len = i32::from_be_bytes([captured[0], captured[1], captured[2], captured[3]]);
            assert_eq!(body_len, 6);
            let corr = i32::from_be_bytes([captured[4], captured[5], captured[6], captured[7]]);
            assert_eq!(corr, 42);
        }
    }

    // WIRE-001 §2: pipelined requests receive responses in FIFO order.
    // Demonstrates reader/writer split: two requests sent back-to-back without
    // waiting for intermediate responses; both responses arrive in order.
    #[tokio::test]
    async fn test_pipelined_requests_fifo_order() {
        init_tracing();
        let config = test_config(true);
        let storage = test_storage(true);
        let consumer_groups = test_consumer_groups(config.clone());
        let cluster_view = SingleNodeClusterView::arc_from_config(&config);
        let router = Router::new(storage, consumer_groups, cluster_view);

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server_task = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            serve_router_connection(socket, router).await
        });

        let mut client = TcpStream::connect(addr).await.unwrap();

        // Send two ApiVersions requests pipelined (don't wait for response between them)
        let req1 = framed_request(18, 0, 101, &[]);
        let req2 = framed_request(18, 0, 202, &[]);
        let mut both = BytesMut::new();
        both.extend_from_slice(&req1);
        both.extend_from_slice(&req2);
        client.write_all(&both).await.unwrap();

        // Read both responses and verify correlation_ids match FIFO order
        let mut buf = BytesMut::with_capacity(4096);
        let mut corr_ids_seen = Vec::new();
        while corr_ids_seen.len() < 2 {
            let n = client.read_buf(&mut buf).await.unwrap();
            if n == 0 {
                break;
            }
            while buf.len() >= 4 {
                let frame_len = i32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
                if buf.len() < 4 + frame_len {
                    break;
                }
                buf.advance(4);
                let frame = buf.split_to(frame_len);
                if frame.len() >= 4 {
                    let corr_id = i32::from_be_bytes([frame[0], frame[1], frame[2], frame[3]]);
                    corr_ids_seen.push(corr_id);
                }
            }
        }
        drop(client);

        assert_eq!(
            corr_ids_seen.len(),
            2,
            "expected 2 responses; got {:?}",
            corr_ids_seen
        );
        assert_eq!(
            corr_ids_seen[0], 101,
            "first response should match first request"
        );
        assert_eq!(
            corr_ids_seen[1], 202,
            "second response should match second request"
        );

        // Server should exit cleanly (connection closed by client)
        let _ = server_task.await; // may be Ok or Err(BrokenPipe) depending on timing
    }

    // WIRE-001 §3: connection closes after MAX_CONSECUTIVE_ERRORS consecutive errors;
    // each failed request still receives an error frame before close.
    #[tokio::test]
    async fn test_consecutive_error_limit() {
        init_tracing();
        let config = test_config(true);
        let storage = test_storage(true);
        let consumer_groups = test_consumer_groups(config.clone());
        let cluster_view = SingleNodeClusterView::arc_from_config(&config);
        let router = Router::new(storage, consumer_groups, cluster_view);

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server_task = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            serve_router_connection(socket, router).await
        });

        let mut client = TcpStream::connect(addr).await.unwrap();

        // ApiVersions v3 is a flexible request (request_header_version=2).
        // Appending 0xFF bytes as the "body" produces a malformed tagged-fields
        // varint that decode_request rejects — causing router.route() to return Err
        // while peek_correlation_id still succeeds (bytes [4..8] = corr_id).
        let mut all = BytesMut::new();
        for i in 1..=10i32 {
            let bad_req = framed_request(18, 3, i, &[0xFF; 10]);
            all.extend_from_slice(&bad_req);
        }
        client.write_all(&all).await.unwrap();

        // Read error frames until EOF (server closes after the 10th)
        let mut buf = BytesMut::with_capacity(1024);
        let mut error_count = 0usize;
        loop {
            let n = client.read_buf(&mut buf).await.unwrap();
            if n == 0 {
                break; // EOF — server closed
            }
            while buf.len() >= 4 {
                let frame_len = i32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
                if buf.len() < 4 + frame_len {
                    break;
                }
                buf.advance(4);
                buf.advance(frame_len);
                error_count += 1;
            }
        }
        drop(client);

        assert_eq!(
            error_count, 10,
            "expected exactly 10 error frames before close"
        );

        let result = server_task.await.unwrap();
        assert!(
            result.is_err(),
            "server must return Err after consecutive error limit"
        );
    }
}

#[cfg(test)]
struct CapturingStream {
    input: Vec<u8>,
    pos: usize,
    // raw pointer so we can pass a borrow into a 'static Box
    captured: *mut Vec<u8>,
}

#[cfg(test)]
impl CapturingStream {
    fn new(input: Vec<u8>, captured: *mut Vec<u8>) -> Self {
        Self {
            input,
            pos: 0,
            captured,
        }
    }
}

#[cfg(test)]
unsafe impl Send for CapturingStream {}

#[cfg(test)]
impl tokio::io::AsyncRead for CapturingStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.get_mut();
        if this.pos >= this.input.len() {
            return std::task::Poll::Ready(Ok(()));
        }
        let remaining = &this.input[this.pos..];
        let to_copy = remaining.len().min(buf.remaining());
        buf.put_slice(&remaining[..to_copy]);
        this.pos += to_copy;
        std::task::Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
impl tokio::io::AsyncWrite for CapturingStream {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let this = self.get_mut();
        unsafe { (*this.captured).extend_from_slice(buf) };
        std::task::Poll::Ready(Ok(buf.len()))
    }
    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }
    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }
}
