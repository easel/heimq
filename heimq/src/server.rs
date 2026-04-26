//! TCP server implementation

use crate::config::Config;
use crate::consumer_group::{ConsumerGroupManager, GroupCoordinatorBackend};
use crate::error::Result;
use crate::protocol::Router;
use crate::storage::{
    dispatch_group_coordinator, dispatch_log_backend, dispatch_offset_store, LogBackend,
};
use bytes::{Buf, BytesMut};
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use std::net::SocketAddr;
use tracing::{debug, error, info, warn};

/// The heimq server
pub struct Server {
    config: Arc<Config>,
    storage: Arc<dyn LogBackend>,
    consumer_groups: Arc<ConsumerGroupManager>,
}

impl Server {
    /// Create a new server instance
    pub fn new(config: Config) -> Result<Self> {
        let config = Arc::new(config);
        let storage_cfg = config.storage();

        let storage: Arc<dyn LogBackend> =
            dispatch_log_backend(&storage_cfg.log, config.clone())?;
        let offset_store = dispatch_offset_store(&storage_cfg.offsets)?;
        let consumer_groups = Arc::new(ConsumerGroupManager::with_offset_store(
            config.clone(),
            offset_store,
        ));
        // Validate the group-coordinator URL through the dispatcher; the
        // memory:// scheme returns the manager we just constructed, while any
        // unknown scheme fails fast at startup.
        let _coordinator: Arc<dyn GroupCoordinatorBackend> =
            dispatch_group_coordinator(&storage_cfg.groups, consumer_groups.clone())?;

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
        let mut served = 0usize;
        loop {
            let result = listener.accept().await;
            if !self.handle_accept_result(result, max_connections, &mut served) { break; }
        }
        Ok(())
    }

    fn handle_accept_result(
        &self,
        result: std::io::Result<(TcpStream, SocketAddr)>,
        max_connections: Option<usize>,
        served: &mut usize,
    ) -> bool {
        match result {
            Ok((socket, addr)) => {
                debug!(peer = %addr, "New connection");

                let router = Router::new(
                    self.storage.clone(),
                    self.consumer_groups.clone(),
                    self.config.clone(),
                );

                tokio::spawn(async move {
                    if let Err(e) = handle_connection(Box::new(socket), router).await {
                        debug!(error = %e, "Connection error");
                    }
                });
                *served += 1;
                if max_connections.is_some_and(|limit| *served >= limit) { return false; }
            }
            Err(e) => {
                error!(error = %e, "Accept error");
            }
        }
        true
    }
}

trait AsyncStream: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T> AsyncStream for T where T: AsyncRead + AsyncWrite + Unpin + Send {}

/// Handle a single connection
async fn handle_connection(mut socket: Box<dyn AsyncStream>, router: Router) -> Result<()> {
    let mut buffer = BytesMut::with_capacity(64 * 1024);

    loop {
        // Read more data
        let n = socket.read_buf(&mut buffer).await?;
        if n == 0 {
            if buffer.is_empty() {
                return Ok(()); // Clean disconnect
            } else {
                return Err(crate::error::HeimqError::Protocol(
                    "Connection closed with pending data".to_string(),
                ));
            }
        }

        // Process complete messages
        while buffer.len() >= 4 {
            // Read message length
            let msg_len = u32::from_be_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]) as usize;

            if buffer.len() < 4 + msg_len {
                break; // Need more data
            }

            // Extract the message
            buffer.advance(4); // Skip length
            let msg_data = buffer.split_to(msg_len);

            // Route and get response
            match router.route(&msg_data) {
                Ok(response) => {
                    socket.write_all(&response).await?;
                }
                Err(e) => {
                    warn!(error = %e, "Request handling error");
                    // Try to send an error response
                    // For now, just log and continue
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{init_tracing, test_config, test_consumer_groups, test_storage};
    use bytes::BufMut;
    use kafka_protocol::messages::api_versions_request::ApiVersionsRequest;
    use kafka_protocol::protocol::Encodable;
    use clap::Parser;
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use tokio::io::AsyncWriteExt;
    use tokio::io::ReadBuf;

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
                return Poll::Ready(Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "read failed",
                )));
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
                return Poll::Ready(Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "write failed",
                )));
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

    #[tokio::test]
    async fn test_server_creation() {
        let config = Config {
            host: "127.0.0.1".to_string(),
            port: 0, // Random port
            data_dir: std::path::PathBuf::from("/tmp/test"),
            memory_only: true,
            segment_size: 1024 * 1024,
            retention_ms: 60000,
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

        let server_task = tokio::spawn(async move { server.run_with_listener(listener, Some(1)).await });

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

        let server_task = tokio::spawn(async move { server.run_with_listener(listener, Some(1)).await });
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

        let server_task = tokio::spawn(async move { server.run_with_listener(listener, Some(2)).await });

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

        let server_task = tokio::spawn(async move { server.run_with_max_connections(Some(1)).await });
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

    #[test]
    fn test_handle_accept_error() {
        init_tracing();
        let config = Config::parse_from(["heimq"]);
        let server = Server::new(config).unwrap();
        let mut served = 0usize;
        let error = std::io::Error::new(std::io::ErrorKind::Other, "accept failed");
        let should_continue = server.handle_accept_result(Err(error), Some(1), &mut served);
        assert!(should_continue);
        assert_eq!(served, 0);
    }

    #[tokio::test]
    async fn test_handle_accept_result_no_limit() {
        init_tracing();
        let config = Config::parse_from(["heimq"]);
        let server = Server::new(config).unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let client_task = tokio::spawn(async move { TcpStream::connect(addr).await.unwrap() });

        let (socket, peer) = listener.accept().await.unwrap();
        client_task.await.unwrap();

        let mut served = 0usize;
        let should_continue = server.handle_accept_result(Ok((socket, peer)), None, &mut served);
        assert!(should_continue);
        assert_eq!(served, 1);
    }

    #[tokio::test]
    async fn test_handle_accept_result_limit_and_spawn_error() {
        init_tracing();
        let config = Config::parse_from(["heimq"]);
        let server = Server::new(config).unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let client_task = tokio::spawn(async move {
            let mut client = TcpStream::connect(addr).await.unwrap();
            client.write_all(&[1, 2, 3]).await.unwrap();
        });

        let (socket, peer) = listener.accept().await.unwrap();
        client_task.await.unwrap();

        let mut served = 0usize;
        let should_continue = server.handle_accept_result(Ok((socket, peer)), Some(1), &mut served);
        assert!(!should_continue);
        assert_eq!(served, 1);

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    #[tokio::test]
    async fn test_handle_connection_clean_disconnect() {
        init_tracing();
        let config = test_config(true);
        let storage = test_storage(true);
        let consumer_groups = test_consumer_groups(config.clone());
        let router = Router::new(storage, consumer_groups, config);

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server_task = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            handle_connection(Box::new(socket), router).await
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
        let router = Router::new(storage, consumer_groups, config);

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server_task = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            handle_connection(Box::new(socket), router).await
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
        let router = Router::new(storage, consumer_groups, config);

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server_task = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            handle_connection(Box::new(socket), router).await
        });

        let mut stream = TcpStream::connect(addr).await.unwrap();
        let mut buf = BytesMut::new();
        buf.put_i32(2);
        buf.extend_from_slice(&[0x00, 0x01]);
        stream.write_all(&buf).await.unwrap();
        drop(stream);

        let result = server_task.await.unwrap();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_connection_scripted_ok() {
        init_tracing();
        let config = test_config(true);
        let storage = test_storage(true);
        let consumer_groups = test_consumer_groups(config.clone());
        let router = Router::new(storage, consumer_groups, config);

        let request = framed_request(18, 0, 1, &[]);
        let stream = ScriptedStream::new(request);
        let result = handle_connection(Box::new(stream), router).await;
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
        let router = Router::new(storage, consumer_groups, config);

        let stream = ScriptedStream::with_read_error();
        let result = handle_connection(Box::new(stream), router).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_connection_write_error() {
        init_tracing();
        let config = test_config(true);
        let storage = test_storage(true);
        let consumer_groups = test_consumer_groups(config.clone());
        let router = Router::new(storage, consumer_groups, config);

        let request = framed_request(18, 0, 1, &[]);
        let stream = ScriptedStream::with_write_error(request);
        let result = handle_connection(Box::new(stream), router).await;
        assert!(result.is_err());
    }
}
