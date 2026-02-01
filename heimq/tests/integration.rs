//! Integration tests for heimq using Kafka clients
//!
//! Uses rdkafka (librdkafka wrapper) for modern protocol testing.
//! Uses kafka crate for legacy protocol compatibility verification.
//!
//! Note: rdkafka is a dev-dependency only - it does not affect the server binary.

use heimq::test_support::TestServer;
use kafka::client::KafkaClient;
use kafka::producer::{Producer, Record, RequiredAcks};
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::Message;
use std::time::Duration;

/// Extension trait to add Kafka client helpers to TestServer
trait TestServerExt {
    fn legacy_producer_for(&self, topic: &str) -> Producer;
    fn rdkafka_producer(&self) -> FutureProducer;
    fn rdkafka_consumer(&self, group_id: &str) -> BaseConsumer;
}

impl TestServerExt for TestServer {
    /// Create a kafka crate producer for legacy protocol testing
    fn legacy_producer_for(&self, topic: &str) -> Producer {
        let mut client = KafkaClient::new(self.hosts());
        client
            .load_metadata(&[topic])
            .expect("Failed to load metadata");

        Producer::from_client(client)
            .with_ack_timeout(Duration::from_secs(5))
            .with_required_acks(RequiredAcks::One)
            .create()
            .expect("Failed to create producer")
    }

    /// Create an rdkafka FutureProducer for modern protocol testing
    fn rdkafka_producer(&self) -> FutureProducer {
        ClientConfig::new()
            .set("bootstrap.servers", &self.bootstrap_servers())
            .set("message.timeout.ms", "5000")
            .create()
            .expect("Failed to create rdkafka producer")
    }

    /// Create an rdkafka BaseConsumer for consuming
    fn rdkafka_consumer(&self, group_id: &str) -> BaseConsumer {
        ClientConfig::new()
            .set("bootstrap.servers", &self.bootstrap_servers())
            .set("group.id", group_id)
            .set("auto.offset.reset", "earliest")
            .set("enable.auto.commit", "false")
            .create()
            .expect("Failed to create rdkafka consumer")
    }
}

// ============================================================================
// Basic Connection Tests (using kafka crate for metadata)
// ============================================================================

#[test]
fn test_metadata_fetch() {
    let server = TestServer::start();

    let mut client = KafkaClient::new(server.hosts());
    client.set_client_id("test-client".into());

    let result = client.load_metadata_all();
    assert!(result.is_ok(), "Should connect and fetch metadata: {:?}", result);

    let topics = client.topics();
    println!("Found {} topics", topics.names().count());
}

// ============================================================================
// Legacy Protocol Tests (kafka crate - verify v0/v1 message format works)
// ============================================================================

#[test]
fn test_legacy_produce() {
    let server = TestServer::start();
    let topic = "legacy-produce-test";
    let mut producer = server.legacy_producer_for(topic);

    for i in 0..10 {
        let payload = format!("message-{}", i);
        let result = producer.send(&Record::from_value(topic, payload.as_bytes()));
        assert!(result.is_ok(), "Should produce message {}: {:?}", i, result);
    }
}

#[test]
fn test_legacy_produce_with_key() {
    let server = TestServer::start();
    let topic = "legacy-keyed-produce-test";
    let mut producer = server.legacy_producer_for(topic);

    let result = producer.send(&Record::from_key_value(topic, "my-key", "my-value"));
    assert!(result.is_ok(), "Should produce keyed message: {:?}", result);
}

#[test]
fn test_legacy_empty_message() {
    let server = TestServer::start();
    let topic = "legacy-empty-message-test";
    let mut producer = server.legacy_producer_for(topic);

    let empty: &[u8] = &[];
    let result = producer.send(&Record::from_value(topic, empty));
    assert!(result.is_ok(), "Should produce empty message: {:?}", result);
}

#[test]
fn test_legacy_multiple_topics() {
    let server = TestServer::start();

    let topic_names: Vec<String> = (0..5).map(|i| format!("legacy-multi-topic-{}", i)).collect();
    let topic_refs: Vec<&str> = topic_names.iter().map(|s| s.as_str()).collect();

    let mut client = KafkaClient::new(server.hosts());
    client.load_metadata(&topic_refs).expect("Failed to load metadata");

    let mut producer = Producer::from_client(client)
        .with_ack_timeout(Duration::from_secs(5))
        .with_required_acks(RequiredAcks::One)
        .create()
        .expect("Failed to create producer");

    for topic in &topic_names {
        let payload = format!("msg-to-{}", topic);
        let result = producer.send(&Record::from_value(topic, payload.as_bytes()));
        assert!(result.is_ok(), "Should produce to topic {}: {:?}", topic, result);
    }

    let mut client = KafkaClient::new(server.hosts());
    client.load_metadata_all().expect("Failed to load metadata");

    let topics = client.topics();
    let fetched_topics: Vec<_> = topics.names().collect();
    for expected in &topic_names {
        assert!(fetched_topics.contains(&expected.as_str()), "Should have topic {}", expected);
    }
}

// ============================================================================
// Modern Protocol Tests (rdkafka - full produce/consume with modern protocol)
// ============================================================================

#[tokio::test]
#[ignore = "rdkafka tests are run via scripts/compatibility-test.sh (can segfault in cargo test)"]
async fn test_rdkafka_simple_produce() {
    let server = TestServer::start();
    let topic = "rdkafka-simple-produce";
    let producer = server.rdkafka_producer();

    for i in 0..10 {
        let payload = format!("message-{}", i);
        let key = format!("key-{}", i);
        let record = FutureRecord::to(topic)
            .payload(&payload)
            .key(&key);

        let result = producer.send(record, Duration::from_secs(5)).await;
        assert!(result.is_ok(), "Should produce message {}: {:?}", i, result);
    }
}

#[tokio::test]
#[ignore = "rdkafka tests are run via scripts/compatibility-test.sh (can segfault in cargo test)"]
async fn test_rdkafka_produce_with_key() {
    let server = TestServer::start();
    let topic = "rdkafka-keyed-produce";
    let producer = server.rdkafka_producer();

    let record = FutureRecord::to(topic)
        .payload("my-value")
        .key("my-key");

    let result = producer.send(record, Duration::from_secs(5)).await;
    assert!(result.is_ok(), "Should produce keyed message: {:?}", result);
}

#[tokio::test]
#[ignore = "rdkafka tests are run via scripts/compatibility-test.sh (can segfault in cargo test)"]
async fn test_rdkafka_produce_no_key() {
    let server = TestServer::start();
    let topic = "rdkafka-no-key";
    let producer = server.rdkafka_producer();

    let record: FutureRecord<'_, (), _> = FutureRecord::to(topic)
        .payload("value-with-null-key");

    let result = producer.send(record, Duration::from_secs(5)).await;
    assert!(result.is_ok(), "Should produce with null key: {:?}", result);
}

#[tokio::test]
#[ignore = "rdkafka tests are run via scripts/compatibility-test.sh (can segfault in cargo test)"]
async fn test_rdkafka_produce_empty_value() {
    let server = TestServer::start();
    let topic = "rdkafka-empty-value";
    let producer = server.rdkafka_producer();

    let record = FutureRecord::to(topic)
        .payload("")
        .key("key");

    let result = producer.send(record, Duration::from_secs(5)).await;
    assert!(result.is_ok(), "Should produce empty value: {:?}", result);
}

#[tokio::test]
#[ignore = "rdkafka tests are run via scripts/compatibility-test.sh (can segfault in cargo test)"]
async fn test_rdkafka_produce_large_message() {
    let server = TestServer::start();
    let topic = "rdkafka-large-message";
    let producer = server.rdkafka_producer();

    // 512KB message (rdkafka default limit is 1MB, so stay under it)
    let large_value = "x".repeat(512 * 1024);
    let record = FutureRecord::to(topic)
        .payload(&large_value)
        .key("large");

    let result = producer.send(record, Duration::from_secs(10)).await;
    assert!(result.is_ok(), "Should produce large message: {:?}", result);
}

#[tokio::test]
#[ignore = "rdkafka tests are run via scripts/compatibility-test.sh (can segfault in cargo test)"]
async fn test_rdkafka_rapid_produce() {
    let server = TestServer::start();
    let topic = "rdkafka-rapid";
    let producer = server.rdkafka_producer();
    let message_count = 100;

    for i in 0..message_count {
        let payload = format!("rapid-{}", i);
        let key = format!("k-{}", i);
        let record = FutureRecord::to(topic)
            .payload(&payload)
            .key(&key);

        let result = producer.send(record, Duration::from_secs(5)).await;
        assert!(result.is_ok(), "Should produce message {}: {:?}", i, result);
    }
}

#[tokio::test]
#[ignore = "rdkafka tests are run via scripts/compatibility-test.sh (can segfault in cargo test)"]
async fn test_rdkafka_produce_consume_roundtrip() {
    let server = TestServer::start();
    let topic = "rdkafka-roundtrip";
    let producer = server.rdkafka_producer();

    // Produce messages
    for i in 0..5 {
        let payload = format!("roundtrip-{}", i);
        let key = format!("key-{}", i);
        let record = FutureRecord::to(topic)
            .payload(&payload)
            .key(&key);

        producer.send(record, Duration::from_secs(5)).await
            .expect("Failed to produce");
    }

    // Give server time to commit
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Consume messages
    let consumer = server.rdkafka_consumer("test-group");
    consumer.subscribe(&[topic]).expect("Failed to subscribe");

    let mut received = 0;
    let timeout = Duration::from_secs(5);
    let start = std::time::Instant::now();

    while received < 5 && start.elapsed() < timeout {
        if let Some(result) = consumer.poll(Duration::from_millis(100)) {
            match result {
                Ok(msg) => {
                    let payload = msg.payload().map(|p| String::from_utf8_lossy(p).to_string());
                    assert!(payload.is_some(), "Message should have payload");
                    received += 1;
                }
                Err(e) => panic!("Error consuming: {:?}", e),
            }
        }
    }

    assert_eq!(received, 5, "Should have received 5 messages, got {}", received);
}

#[tokio::test]
#[ignore = "rdkafka tests are run via scripts/compatibility-test.sh (can segfault in cargo test)"]
async fn test_rdkafka_multiple_topics() {
    let server = TestServer::start();
    let producer = server.rdkafka_producer();

    let topics: Vec<String> = (0..3).map(|i| format!("rdkafka-multi-{}", i)).collect();

    for topic in &topics {
        let record = FutureRecord::to(topic)
            .payload("test-message")
            .key("key");

        let result = producer.send(record, Duration::from_secs(5)).await;
        assert!(result.is_ok(), "Should produce to topic {}: {:?}", topic, result);
    }
}
