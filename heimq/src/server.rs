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
        let addr = self.config.bind_addr();
        let listener = TcpListener::bind(&addr).await?;

        info!("Listening on {}", addr);

        loop {
            match listener.accept().await {
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
                }
                Err(e) => {
                    error!(error = %e, "Accept error");
                }
            }
        }
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
}
