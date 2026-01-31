//! TCP server implementation

use crate::config::Config;
use crate::consumer_group::ConsumerGroupManager;
use crate::error::Result;
use crate::protocol::Router;
use crate::storage::Storage;
use bytes::{Buf, BufMut, BytesMut};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use std::net::SocketAddr;
use tracing::{debug, error, info, warn};

/// The heimq server
pub struct Server {
    config: Arc<Config>,
    storage: Arc<Storage>,
    consumer_groups: Arc<ConsumerGroupManager>,
}

impl Server {
    /// Create a new server instance
    pub fn new(config: Config) -> Result<Self> {
        let config = Arc::new(config);
        let storage = Arc::new(Storage::new(config.clone()));
        let consumer_groups = Arc::new(ConsumerGroupManager::new(config.clone()));

        Ok(Self {
            config,
            storage,
            consumer_groups,
        })
    }

    /// Run the server
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
                    if let Err(e) = handle_connection(socket, router).await {
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

/// Handle a single connection
async fn handle_connection(mut socket: TcpStream, router: Router) -> Result<()> {
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
        };

        let server = Server::new(config);
        assert!(server.is_ok());
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
            handle_connection(socket, router).await
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
            handle_connection(socket, router).await
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
            handle_connection(socket, router).await
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
}
