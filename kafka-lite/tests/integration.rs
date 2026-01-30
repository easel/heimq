//! Integration tests for heimq using rdkafka client library
//!
//! These tests verify heimq compatibility with real Kafka clients.

use rdkafka::config::ClientConfig;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::message::Message;
use rdkafka::producer::{BaseProducer, BaseRecord, Producer};
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::Duration;
use tokio::time::sleep;

static PORT_COUNTER: AtomicU16 = AtomicU16::new(19092);

fn get_test_port() -> u16 {
    PORT_COUNTER.fetch_add(1, Ordering::SeqCst)
}

struct TestServer {
    child: Child,
    port: u16,
}

impl TestServer {
    fn start() -> Self {
        let port = get_test_port();

        // Start the server
        let child = Command::new("./target/release/heimq")
            .args(["--port", &port.to_string(), "--memory-only"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to start heimq");

        // Wait for server to be ready
        std::thread::sleep(Duration::from_millis(500));

        TestServer { child, port }
    }

    fn bootstrap_servers(&self) -> String {
        format!("localhost:{}", self.port)
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

// ============================================================================
// Basic Connection Tests
// ============================================================================

#[tokio::test]
async fn test_metadata_fetch() {
    let server = TestServer::start();
    let bootstrap = server.bootstrap_servers();

    let producer: BaseProducer = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap)
        .set("message.timeout.ms", "5000")
        .create()
        .expect("Failed to create producer");

    // Fetch metadata
    let metadata = producer
        .client()
        .fetch_metadata(None, Duration::from_secs(5))
        .expect("Failed to fetch metadata");

    // Should have at least one broker
    assert!(!metadata.brokers().is_empty(), "Should have brokers");

    // Broker should be on our port
    let broker = &metadata.brokers()[0];
    assert_eq!(broker.port(), server.port as i32);
}

#[tokio::test]
async fn test_topic_auto_creation() {
    let server = TestServer::start();
    let bootstrap = server.bootstrap_servers();

    let producer: BaseProducer = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap)
        .set("message.timeout.ms", "5000")
        .create()
        .expect("Failed to create producer");

    // Send a message to trigger topic auto-creation
    let topic = "auto-created-topic";
    producer
        .send(BaseRecord::<str, str>::to(topic).payload("test"))
        .expect("Failed to send");
    producer.flush(Duration::from_secs(5)).expect("Failed to flush");

    // Fetch metadata and verify topic exists
    let metadata = producer
        .client()
        .fetch_metadata(Some(topic), Duration::from_secs(5))
        .expect("Failed to fetch metadata");

    assert_eq!(metadata.topics().len(), 1);
    assert_eq!(metadata.topics()[0].name(), topic);
}

// ============================================================================
// Producer Tests
// ============================================================================

#[tokio::test]
async fn test_simple_produce() {
    let server = TestServer::start();
    let bootstrap = server.bootstrap_servers();

    let producer: BaseProducer = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap)
        .set("message.timeout.ms", "5000")
        .create()
        .expect("Failed to create producer");

    let topic = "simple-produce-test";

    // Send 10 messages
    for i in 0..10 {
        let payload = format!("message-{}", i);
        let key = format!("key-{}", i);
        producer
            .send(BaseRecord::<str, str>::to(topic).payload(&payload).key(&key))
            .expect("Failed to send");
    }

    producer.flush(Duration::from_secs(5)).expect("Failed to flush");
}

#[tokio::test]
async fn test_produce_with_key() {
    let server = TestServer::start();
    let bootstrap = server.bootstrap_servers();

    let producer: BaseProducer = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap)
        .create()
        .expect("Failed to create producer");

    let topic = "keyed-produce-test";

    producer
        .send(BaseRecord::<str, str>::to(topic).payload("value").key("my-key"))
        .expect("Failed to send");

    producer.flush(Duration::from_secs(5)).expect("Failed to flush");
}

#[tokio::test]
async fn test_produce_large_message() {
    let server = TestServer::start();
    let bootstrap = server.bootstrap_servers();

    let producer: BaseProducer = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap)
        .set("message.max.bytes", "10485760")
        .create()
        .expect("Failed to create producer");

    let topic = "large-message-test";

    // 1MB message
    let payload = "x".repeat(1024 * 1024);
    producer
        .send(BaseRecord::<(), str>::to(topic).payload(&payload))
        .expect("Failed to send large message");

    producer.flush(Duration::from_secs(10)).expect("Failed to flush");
}

#[tokio::test]
async fn test_produce_many_messages() {
    let server = TestServer::start();
    let bootstrap = server.bootstrap_servers();

    let producer: BaseProducer = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap)
        .set("queue.buffering.max.messages", "100000")
        .create()
        .expect("Failed to create producer");

    let topic = "many-messages-test";

    // Send 1000 messages
    for i in 0..1000 {
        let payload = format!("message-{}", i);
        producer
            .send(BaseRecord::<(), str>::to(topic).payload(&payload))
            .expect("Failed to send");
    }

    producer.flush(Duration::from_secs(30)).expect("Failed to flush");
}

// ============================================================================
// Consumer Tests
// ============================================================================

#[tokio::test]
async fn test_simple_consume() {
    let server = TestServer::start();
    let bootstrap = server.bootstrap_servers();
    let topic = "simple-consume-test";

    // Produce some messages first
    let producer: BaseProducer = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap)
        .create()
        .expect("Failed to create producer");

    for i in 0..5 {
        let payload = format!("msg-{}", i);
        producer
            .send(BaseRecord::<(), str>::to(topic).payload(&payload))
            .expect("Failed to send");
    }
    producer.flush(Duration::from_secs(5)).expect("Failed to flush");

    // Now consume
    let consumer: BaseConsumer = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap)
        .set("group.id", "test-group")
        .set("auto.offset.reset", "earliest")
        .set("enable.auto.commit", "false")
        .create()
        .expect("Failed to create consumer");

    consumer.subscribe(&[topic]).expect("Failed to subscribe");

    // Consume messages
    let mut received = 0;
    let deadline = std::time::Instant::now() + Duration::from_secs(10);

    while received < 5 && std::time::Instant::now() < deadline {
        match consumer.poll(Duration::from_millis(100)) {
            Some(Ok(msg)) => {
                let payload = msg.payload_view::<str>().unwrap().unwrap();
                assert!(payload.starts_with("msg-"));
                received += 1;
            }
            Some(Err(e)) => panic!("Consumer error: {}", e),
            None => continue,
        }
    }

    assert_eq!(received, 5, "Should have received all 5 messages");
}

#[tokio::test]
async fn test_consume_from_beginning() {
    let server = TestServer::start();
    let bootstrap = server.bootstrap_servers();
    let topic = "consume-beginning-test";

    // Produce messages
    let producer: BaseProducer = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap)
        .create()
        .expect("Failed to create producer");

    for i in 0..3 {
        let payload = format!("early-{}", i);
        producer
            .send(BaseRecord::<(), str>::to(topic).payload(&payload))
            .expect("Failed to send");
    }
    producer.flush(Duration::from_secs(5)).expect("Failed to flush");

    // Wait a bit
    sleep(Duration::from_millis(500)).await;

    // Create consumer with auto.offset.reset=earliest
    let consumer: BaseConsumer = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap)
        .set("group.id", "new-consumer-group")
        .set("auto.offset.reset", "earliest")
        .create()
        .expect("Failed to create consumer");

    consumer.subscribe(&[topic]).expect("Failed to subscribe");

    let mut count = 0;
    for _ in 0..20 {
        if let Some(Ok(_)) = consumer.poll(Duration::from_millis(200)) {
            count += 1;
        }
        if count >= 3 {
            break;
        }
    }

    assert!(count >= 3, "Should consume messages from beginning");
}

// ============================================================================
// Consumer Group Tests
// ============================================================================

#[tokio::test]
async fn test_consumer_group_join() {
    let server = TestServer::start();
    let bootstrap = server.bootstrap_servers();
    let topic = "group-join-test";
    let group = "test-join-group";

    // Create topic first
    let producer: BaseProducer = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap)
        .create()
        .expect("Failed to create producer");

    producer
        .send(BaseRecord::<(), str>::to(topic).payload("init"))
        .expect("Failed to send");
    producer.flush(Duration::from_secs(5)).expect("Failed to flush");

    // Create consumer with group
    let consumer: BaseConsumer = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap)
        .set("group.id", group)
        .set("auto.offset.reset", "earliest")
        .set("session.timeout.ms", "10000")
        .create()
        .expect("Failed to create consumer");

    consumer.subscribe(&[topic]).expect("Failed to subscribe");

    // Poll to trigger group join
    let mut joined = false;
    for _ in 0..30 {
        if consumer.poll(Duration::from_millis(500)).is_some() {
            joined = true;
            break;
        }
    }

    assert!(joined, "Consumer should have joined group and received message");
}

#[tokio::test]
async fn test_offset_commit() {
    let server = TestServer::start();
    let bootstrap = server.bootstrap_servers();
    let topic = "offset-commit-test";
    let group = "offset-commit-group";

    // Produce messages
    let producer: BaseProducer = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap)
        .create()
        .expect("Failed to create producer");

    for i in 0..10 {
        let payload = format!("msg-{}", i);
        producer
            .send(BaseRecord::<(), str>::to(topic).payload(&payload))
            .expect("Failed to send");
    }
    producer.flush(Duration::from_secs(5)).expect("Failed to flush");

    // Create consumer
    let consumer: BaseConsumer = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap)
        .set("group.id", group)
        .set("auto.offset.reset", "earliest")
        .set("enable.auto.commit", "false")
        .create()
        .expect("Failed to create consumer");

    consumer.subscribe(&[topic]).expect("Failed to subscribe");

    // Consume some messages
    let mut count = 0;
    for _ in 0..20 {
        if let Some(Ok(msg)) = consumer.poll(Duration::from_millis(200)) {
            count += 1;
            if count >= 5 {
                // Commit offset manually
                consumer.commit_message(&msg, rdkafka::consumer::CommitMode::Sync)
                    .expect("Failed to commit");
                break;
            }
        }
    }

    assert!(count >= 5, "Should have consumed at least 5 messages");
}

// ============================================================================
// Admin API Tests
// ============================================================================

#[tokio::test]
async fn test_create_topic_admin() {
    let server = TestServer::start();
    let bootstrap = server.bootstrap_servers();

    let admin: AdminClient<_> = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap)
        .create()
        .expect("Failed to create admin client");

    let topic = NewTopic::new("admin-created-topic", 3, TopicReplication::Fixed(1));
    let opts = AdminOptions::new().operation_timeout(Some(Duration::from_secs(5)));

    let result = admin
        .create_topics(&[topic], &opts)
        .await;

    // Should succeed (or topic already exists)
    assert!(result.is_ok(), "Create topic should not error: {:?}", result);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[tokio::test]
async fn test_empty_message() {
    let server = TestServer::start();
    let bootstrap = server.bootstrap_servers();

    let producer: BaseProducer = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap)
        .create()
        .expect("Failed to create producer");

    let topic = "empty-message-test";

    // Send empty payload
    producer
        .send(BaseRecord::<(), ()>::to(topic))
        .expect("Failed to send empty message");

    producer.flush(Duration::from_secs(5)).expect("Failed to flush");
}

#[tokio::test]
async fn test_null_key() {
    let server = TestServer::start();
    let bootstrap = server.bootstrap_servers();

    let producer: BaseProducer = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap)
        .create()
        .expect("Failed to create producer");

    let topic = "null-key-test";

    producer
        .send(BaseRecord::<(), str>::to(topic).payload("value-with-null-key"))
        .expect("Failed to send");

    producer.flush(Duration::from_secs(5)).expect("Failed to flush");
}

#[tokio::test]
async fn test_multiple_topics() {
    let server = TestServer::start();
    let bootstrap = server.bootstrap_servers();

    let producer: BaseProducer = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap)
        .create()
        .expect("Failed to create producer");

    // Send to multiple topics
    for i in 0..5 {
        let topic = format!("multi-topic-{}", i);
        let payload = format!("msg-to-{}", topic);
        producer
            .send(BaseRecord::<(), str>::to(&topic).payload(&payload))
            .expect("Failed to send");
    }

    producer.flush(Duration::from_secs(5)).expect("Failed to flush");

    // Verify all topics exist
    let metadata = producer
        .client()
        .fetch_metadata(None, Duration::from_secs(5))
        .expect("Failed to fetch metadata");

    let topic_names: Vec<_> = metadata.topics().iter().map(|t| t.name()).collect();
    for i in 0..5 {
        let expected = format!("multi-topic-{}", i);
        assert!(topic_names.contains(&expected.as_str()), "Should have topic {}", expected);
    }
}

// ============================================================================
// Stress Tests
// ============================================================================

#[tokio::test]
async fn test_rapid_produce_consume() {
    let server = TestServer::start();
    let bootstrap = server.bootstrap_servers();
    let topic = "rapid-test";
    let message_count = 100;

    // Produce rapidly
    let producer: BaseProducer = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap)
        .set("batch.size", "65536")
        .set("linger.ms", "5")
        .create()
        .expect("Failed to create producer");

    for i in 0..message_count {
        let payload = format!("rapid-{}", i);
        producer
            .send(BaseRecord::<(), str>::to(topic).payload(&payload))
            .expect("Failed to send");
    }
    producer.flush(Duration::from_secs(10)).expect("Failed to flush");

    // Consume all
    let consumer: BaseConsumer = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap)
        .set("group.id", "rapid-consumer")
        .set("auto.offset.reset", "earliest")
        .create()
        .expect("Failed to create consumer");

    consumer.subscribe(&[topic]).expect("Failed to subscribe");

    let mut received = 0;
    let deadline = std::time::Instant::now() + Duration::from_secs(30);

    while received < message_count && std::time::Instant::now() < deadline {
        if consumer.poll(Duration::from_millis(100)).is_some() {
            received += 1;
        }
    }

    assert_eq!(received, message_count, "Should receive all {} messages", message_count);
}
