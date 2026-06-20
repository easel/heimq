//! heimq-wire: Kafka TCP framing, connection loop, and handler registry.
//!
//! Provides the shared wire infrastructure used by heimq and by consumer
//! projects (pqueue-kafka, fjord-broker) that embed a Kafka producer or
//! broker endpoint. Consumers implement [`FrameHandler`] to plug in their
//! own request routing; heimq-wire owns the TCP accept loop, frame parsing,
//! pipelining, and error-frame policy (WIRE-001).

#![forbid(unsafe_code)]

use async_trait::async_trait;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing::{debug, error, info, warn};

/// Maximum accepted frame size (100 MiB).
pub const MAX_FRAME_BYTES: usize = 100 * 1024 * 1024;

/// Channel depth for the reader→writer pipeline within a single connection.
pub const CHANNEL_DEPTH: usize = 64;

/// Consecutive routing errors before the connection is dropped.
pub const MAX_CONSECUTIVE_ERRORS: usize = 10;

/// Error type for wire-layer failures.
#[derive(Debug, thiserror::Error)]
pub enum WireError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("protocol: {0}")]
    Protocol(String),
    #[error("handler: {0}")]
    Handler(String),
}

/// Error returned from a [`FrameHandler`].
#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    #[error("protocol: {0}")]
    Protocol(String),
    #[error("storage: {0}")]
    Storage(String),
    #[error("handler: {0}")]
    Handler(String),
}

impl From<FrameError> for WireError {
    fn from(e: FrameError) -> Self {
        WireError::Handler(e.to_string())
    }
}

/// Application-level Kafka frame handler.
///
/// Implementations receive a raw Kafka request frame (without the 4-byte length
/// prefix) and return an encoded response frame (also without the prefix).
/// The wire server prepends the 4-byte length prefix before writing.
///
/// Persistence, routing, and all protocol semantics live in the implementation.
/// The handler is called from the writer task with frames in FIFO order.
#[async_trait]
pub trait FrameHandler: Send + Sync + 'static {
    async fn handle(&self, frame: Bytes) -> Result<Bytes, FrameError>;
}

/// Factory for stateful per-connection frame handlers.
///
/// Stateless embedders can use [`WireServer`]. Embedders that need per-client
/// authentication, tenant routing, or quota state can use
/// [`WireServerWithFactory`] so each accepted socket gets an isolated handler.
pub trait FrameHandlerFactory: Send + Sync + 'static {
    type Handler: FrameHandler;

    fn create(&self, peer: SocketAddr) -> Self::Handler;
}

/// TCP server that accepts connections and dispatches frames to a [`FrameHandler`].
///
/// Each accepted connection gets a reader task (parses frames from the TCP
/// stream) and a writer task (calls the handler and writes responses). The
/// reader→writer pipeline uses a bounded channel of depth [`CHANNEL_DEPTH`],
/// providing backpressure without head-of-line blocking across connections.
pub struct WireServer<H: FrameHandler> {
    handler: Arc<H>,
}

impl<H: FrameHandler> WireServer<H> {
    pub fn new(handler: H) -> Self {
        Self {
            handler: Arc::new(handler),
        }
    }

    pub fn with_arc(handler: Arc<H>) -> Self {
        Self { handler }
    }

    /// Bind to `addr` and serve connections until the process exits.
    pub async fn run(&self, addr: &str) -> Result<(), WireError> {
        let listener = TcpListener::bind(addr).await?;
        info!(addr = %listener.local_addr()?, "heimq-wire listening");
        self.run_with_listener(listener, None).await
    }

    /// Serve connections from an already-bound listener.
    ///
    /// If `max_connections` is `Some(n)`, the server returns after accepting
    /// exactly `n` connections (useful for tests).
    pub async fn run_with_listener(
        &self,
        listener: TcpListener,
        max_connections: Option<usize>,
    ) -> Result<(), WireError> {
        let mut served = 0usize;
        loop {
            match listener.accept().await {
                Ok((socket, peer)) => {
                    let _ = socket.set_nodelay(true);
                    let handler = self.handler.clone();
                    debug!(peer = %peer, "accepted connection");
                    tokio::spawn(async move {
                        if let Err(e) = serve_connection(socket, peer, handler).await {
                            debug!(peer = %peer, error = %e, "connection closed");
                        }
                    });
                    served += 1;
                    if max_connections.is_some_and(|limit| served >= limit) {
                        break;
                    }
                }
                Err(e) => {
                    error!(error = %e, "accept error");
                }
            }
        }
        Ok(())
    }
}

/// TCP server that creates one [`FrameHandler`] per accepted connection.
pub struct WireServerWithFactory<F: FrameHandlerFactory> {
    factory: Arc<F>,
}

impl<F: FrameHandlerFactory> WireServerWithFactory<F> {
    pub fn new(factory: F) -> Self {
        Self {
            factory: Arc::new(factory),
        }
    }

    pub fn with_arc(factory: Arc<F>) -> Self {
        Self { factory }
    }

    /// Bind to `addr` and serve connections until the process exits.
    pub async fn run(&self, addr: &str) -> Result<(), WireError> {
        let listener = TcpListener::bind(addr).await?;
        info!(addr = %listener.local_addr()?, "heimq-wire listening");
        self.run_with_listener(listener, None).await
    }

    /// Serve connections from an already-bound listener.
    ///
    /// If `max_connections` is `Some(n)`, the server returns after accepting
    /// exactly `n` connections (useful for tests).
    pub async fn run_with_listener(
        &self,
        listener: TcpListener,
        max_connections: Option<usize>,
    ) -> Result<(), WireError> {
        let mut served = 0usize;
        loop {
            match listener.accept().await {
                Ok((socket, peer)) => {
                    let _ = socket.set_nodelay(true);
                    let handler = Arc::new(self.factory.create(peer));
                    debug!(peer = %peer, "accepted connection");
                    tokio::spawn(async move {
                        if let Err(e) = serve_connection(socket, peer, handler).await {
                            debug!(peer = %peer, error = %e, "connection closed");
                        }
                    });
                    served += 1;
                    if max_connections.is_some_and(|limit| served >= limit) {
                        break;
                    }
                }
                Err(e) => {
                    error!(error = %e, "accept error");
                }
            }
        }
        Ok(())
    }
}

/// Serve one already-accepted stream with the shared Kafka frame pipeline.
///
/// This is useful for embedders that wrap TCP before Kafka framing, such as
/// TLS acceptors. The stream is split into a reader task and writer loop with
/// the same bounded reader-to-writer queue used by [`WireServer`].
pub async fn serve_connection<S, H>(
    stream: S,
    _peer: SocketAddr,
    handler: Arc<H>,
) -> Result<(), WireError>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    H: FrameHandler,
{
    let (read_half, write_half) = tokio::io::split(stream);
    let (tx, rx) = tokio::sync::mpsc::channel::<Bytes>(CHANNEL_DEPTH);
    let reader_handle = tokio::spawn(run_reader(read_half, tx));
    let writer_result = run_writer(write_half, rx, handler).await;
    reader_handle.abort();
    writer_result
}

async fn run_reader<R: AsyncRead + Unpin + Send + 'static>(
    mut stream: R,
    tx: tokio::sync::mpsc::Sender<Bytes>,
) -> Result<(), WireError> {
    let mut buf = BytesMut::with_capacity(64 * 1024);
    loop {
        let n = stream.read_buf(&mut buf).await?;
        if n == 0 {
            if buf.is_empty() {
                return Ok(());
            }
            return Err(WireError::Protocol(
                "connection closed with pending data".into(),
            ));
        }
        while buf.len() >= 4 {
            let msg_len = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
            if msg_len > MAX_FRAME_BYTES {
                return Err(WireError::Protocol(format!(
                    "frame size {msg_len} exceeds max {MAX_FRAME_BYTES}"
                )));
            }
            if buf.len() < 4 + msg_len {
                break;
            }
            buf.advance(4);
            let frame = buf.split_to(msg_len).freeze();
            if tx.send(frame).await.is_err() {
                return Ok(());
            }
        }
    }
}

async fn run_writer<W: AsyncWrite + Unpin, H: FrameHandler>(
    mut stream: W,
    mut rx: tokio::sync::mpsc::Receiver<Bytes>,
    handler: Arc<H>,
) -> Result<(), WireError> {
    let mut consecutive_errors = 0usize;
    while let Some(frame) = rx.recv().await {
        match handler.handle(frame.clone()).await {
            Ok(response) => {
                let framed = prepend_length(response);
                stream.write_all(&framed).await?;
                consecutive_errors = 0;
            }
            Err(e) => {
                warn!(error = %e, "frame handler error");
                let close_after_response = matches!(e, FrameError::Protocol(_));
                if let Some(corr_id) = peek_correlation_id(&frame) {
                    let error_frame = make_error_frame(corr_id, 10);
                    let _ = stream.write_all(&error_frame).await;
                    consecutive_errors += 1;
                    if close_after_response || consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                        return Err(WireError::from(e));
                    }
                } else {
                    return Err(WireError::from(e));
                }
            }
        }
    }
    Ok(())
}

/// Prepend a 4-byte big-endian length prefix to a response body.
pub fn prepend_length(body: Bytes) -> Bytes {
    let mut out = BytesMut::with_capacity(4 + body.len());
    out.put_i32(body.len() as i32);
    out.extend_from_slice(&body);
    out.freeze()
}

/// Peek the correlation ID from byte offset 4 of a raw request frame.
///
/// The Kafka wire protocol places the correlation ID at bytes 4–7 (i32 BE)
/// in both legacy and flexible request headers.
pub fn peek_correlation_id(frame: &[u8]) -> Option<i32> {
    if frame.len() >= 8 {
        Some(i32::from_be_bytes([frame[4], frame[5], frame[6], frame[7]]))
    } else {
        None
    }
}

/// Build a minimal error response frame (correlation_id + error_code).
///
/// Used to send typed error responses when request routing fails but the
/// correlation ID is recoverable.
pub fn make_error_frame(correlation_id: i32, error_code: i16) -> Bytes {
    let mut body = BytesMut::with_capacity(6);
    body.put_i32(correlation_id);
    body.put_i16(error_code);
    prepend_length(body.freeze())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peek_correlation_id_happy_path() {
        let mut frame = BytesMut::with_capacity(12);
        frame.put_i16(0); // api_key
        frame.put_i16(0); // api_version
        frame.put_i32(42); // correlation_id
        frame.put_i32(0); // client_id_length (-1 for null)
        assert_eq!(peek_correlation_id(&frame), Some(42));
    }

    #[test]
    fn test_peek_correlation_id_too_short() {
        assert_eq!(peek_correlation_id(&[0u8; 7]), None);
    }

    #[test]
    fn test_make_error_frame_structure() {
        let frame = make_error_frame(99, 10);
        assert_eq!(frame.len(), 10); // 4 (len) + 4 (corr_id) + 2 (error_code)
        let len = i32::from_be_bytes([frame[0], frame[1], frame[2], frame[3]]);
        assert_eq!(len, 6);
        let corr = i32::from_be_bytes([frame[4], frame[5], frame[6], frame[7]]);
        assert_eq!(corr, 99);
        let err = i16::from_be_bytes([frame[8], frame[9]]);
        assert_eq!(err, 10);
    }

    #[test]
    fn test_prepend_length() {
        let body = Bytes::from_static(b"hello");
        let framed = prepend_length(body);
        assert_eq!(framed.len(), 9);
        let len = i32::from_be_bytes([framed[0], framed[1], framed[2], framed[3]]);
        assert_eq!(len, 5);
        assert_eq!(&framed[4..], b"hello");
    }

    struct EchoHandler;

    #[async_trait]
    impl FrameHandler for EchoHandler {
        async fn handle(&self, frame: Bytes) -> Result<Bytes, FrameError> {
            Ok(frame)
        }
    }

    #[tokio::test]
    async fn test_wire_server_echo_round_trip() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = WireServer::new(EchoHandler);
        tokio::spawn(async move {
            server.run_with_listener(listener, Some(1)).await.unwrap();
        });

        let mut client = TcpStream::connect(addr).await.unwrap();
        let payload = b"\x00\x01\x00\x00\x00\x00\x00\x2a"; // 8 bytes
        let mut frame = BytesMut::with_capacity(12);
        frame.put_i32(payload.len() as i32);
        frame.extend_from_slice(payload);
        client.write_all(&frame).await.unwrap();

        let mut response = vec![0u8; 12];
        client.read_exact(&mut response).await.unwrap();
        let resp_len = i32::from_be_bytes([response[0], response[1], response[2], response[3]]);
        assert_eq!(resp_len, payload.len() as i32);
        assert_eq!(&response[4..], payload);
    }

    struct CountingFactory {
        next: std::sync::atomic::AtomicU8,
    }

    struct CountingHandler {
        id: u8,
    }

    impl FrameHandlerFactory for CountingFactory {
        type Handler = CountingHandler;

        fn create(&self, _peer: SocketAddr) -> Self::Handler {
            let id = self.next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            CountingHandler { id }
        }
    }

    #[async_trait]
    impl FrameHandler for CountingHandler {
        async fn handle(&self, _frame: Bytes) -> Result<Bytes, FrameError> {
            Ok(Bytes::copy_from_slice(&[self.id]))
        }
    }

    #[tokio::test]
    async fn test_wire_server_factory_creates_handler_per_connection() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = WireServerWithFactory::new(CountingFactory {
            next: std::sync::atomic::AtomicU8::new(1),
        });
        tokio::spawn(async move {
            server.run_with_listener(listener, Some(2)).await.unwrap();
        });

        async fn one_request(addr: SocketAddr) -> u8 {
            let mut client = TcpStream::connect(addr).await.unwrap();
            client.write_all(&0i32.to_be_bytes()).await.unwrap();

            let mut response = [0u8; 5];
            client.read_exact(&mut response).await.unwrap();
            let resp_len = i32::from_be_bytes([response[0], response[1], response[2], response[3]]);
            assert_eq!(resp_len, 1);
            response[4]
        }

        assert_eq!(one_request(addr).await, 1);
        assert_eq!(one_request(addr).await, 2);
    }
}
