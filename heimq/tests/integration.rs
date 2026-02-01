//! Integration tests for heimq using Kafka clients
//!
//! Uses rdkafka (librdkafka wrapper) for modern protocol testing.
//! Uses kafka crate for legacy protocol compatibility verification.
//!
//! Note: rdkafka is a dev-dependency only - it does not affect the server binary.

use heimq::test_support::TestServer;
use kafka::client::KafkaClient;
// kafka::consumer::FetchOffset removed - fetch API requires v0/v1 protocol support
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

// ============================================================================
// Consumer Group Tests (rdkafka - test consumer group protocol)
// ============================================================================

use heimq::test_support::{unique_group, unique_topic};
use rdkafka::consumer::CommitMode;
use rdkafka::TopicPartitionList;

/// Helper to create a consumer with specific settings for group tests
fn create_group_consumer(server: &TestServer, group_id: &str, session_timeout_ms: &str) -> BaseConsumer {
    ClientConfig::new()
        .set("bootstrap.servers", &server.bootstrap_servers())
        .set("group.id", group_id)
        .set("auto.offset.reset", "earliest")
        .set("enable.auto.commit", "false")
        .set("session.timeout.ms", session_timeout_ms)
        .set("heartbeat.interval.ms", "1000")
        .create()
        .expect("Failed to create consumer")
}

#[tokio::test]
#[ignore = "rdkafka tests are run via scripts/compatibility-test.sh"]
async fn test_rdkafka_consumer_group_join() {
    let server = TestServer::start();
    let topic = unique_topic("cg-join");
    let group = unique_group("test-group");
    let producer = server.rdkafka_producer();

    // Produce some messages first
    for i in 0..5 {
        let payload = format!("join-test-{}", i);
        let record = FutureRecord::to(&topic)
            .payload(&payload)
            .key("key");
        producer.send(record, Duration::from_secs(5)).await
            .expect("Failed to produce");
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Create consumer and join group
    let consumer = server.rdkafka_consumer(&group);
    consumer.subscribe(&[&topic]).expect("Failed to subscribe");

    // Poll to trigger group join - consumer should receive messages
    let mut received = 0;
    let timeout = Duration::from_secs(10);
    let start = std::time::Instant::now();

    while received < 5 && start.elapsed() < timeout {
        if let Some(result) = consumer.poll(Duration::from_millis(100)) {
            match result {
                Ok(_msg) => received += 1,
                Err(e) => panic!("Error consuming: {:?}", e),
            }
        }
    }

    assert_eq!(received, 5, "Consumer should have joined group and received all messages");
}

#[tokio::test]
#[ignore = "rdkafka tests are run via scripts/compatibility-test.sh"]
async fn test_rdkafka_consumer_group_offset_commit() {
    let server = TestServer::start();
    let topic = unique_topic("cg-offset");
    let group = unique_group("offset-group");
    let producer = server.rdkafka_producer();

    // Produce messages
    for i in 0..10 {
        let payload = format!("offset-test-{}", i);
        let record = FutureRecord::to(&topic)
            .payload(&payload)
            .key("key");
        producer.send(record, Duration::from_secs(5)).await
            .expect("Failed to produce");
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    // First consumer: consume 5 messages and commit
    {
        let consumer = server.rdkafka_consumer(&group);
        consumer.subscribe(&[&topic]).expect("Failed to subscribe");

        let mut received = 0;
        let timeout = Duration::from_secs(10);
        let start = std::time::Instant::now();

        while received < 5 && start.elapsed() < timeout {
            if let Some(result) = consumer.poll(Duration::from_millis(100)) {
                match result {
                    Ok(msg) => {
                        received += 1;
                        if received == 5 {
                            // Commit sync after 5 messages
                            consumer.commit_message(&msg, CommitMode::Sync)
                                .expect("Failed to commit offset");
                        }
                    }
                    Err(e) => panic!("Error consuming: {:?}", e),
                }
            }
        }

        assert_eq!(received, 5, "First consumer should receive 5 messages");
    }

    // Second consumer: should start from offset 5
    tokio::time::sleep(Duration::from_millis(500)).await;

    {
        let consumer = server.rdkafka_consumer(&group);
        consumer.subscribe(&[&topic]).expect("Failed to subscribe");

        let mut received = 0;
        let mut first_offset: Option<i64> = None;
        let timeout = Duration::from_secs(10);
        let start = std::time::Instant::now();

        while received < 5 && start.elapsed() < timeout {
            if let Some(result) = consumer.poll(Duration::from_millis(100)) {
                match result {
                    Ok(msg) => {
                        if first_offset.is_none() {
                            first_offset = Some(msg.offset());
                        }
                        received += 1;
                    }
                    Err(e) => panic!("Error consuming: {:?}", e),
                }
            }
        }

        assert_eq!(received, 5, "Second consumer should receive remaining 5 messages");
        assert!(first_offset.unwrap_or(0) >= 5,
            "Second consumer should start at offset 5 or later, got {:?}", first_offset);
    }
}

#[tokio::test]
#[ignore = "rdkafka tests are run via scripts/compatibility-test.sh"]
async fn test_rdkafka_consumer_group_manual_offset_fetch() {
    let server = TestServer::start();
    let topic = unique_topic("cg-fetch");
    let group = unique_group("fetch-group");
    let producer = server.rdkafka_producer();

    // Produce messages
    for i in 0..5 {
        let payload = format!("fetch-test-{}", i);
        let record = FutureRecord::to(&topic)
            .payload(&payload)
            .key("key");
        producer.send(record, Duration::from_secs(5)).await
            .expect("Failed to produce");
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    let consumer = server.rdkafka_consumer(&group);
    consumer.subscribe(&[&topic]).expect("Failed to subscribe");

    // Consume all and commit
    let mut received = 0;
    let timeout = Duration::from_secs(10);
    let start = std::time::Instant::now();
    let mut last_msg = None;

    while received < 5 && start.elapsed() < timeout {
        if let Some(result) = consumer.poll(Duration::from_millis(100)) {
            match result {
                Ok(msg) => {
                    received += 1;
                    last_msg = Some((msg.topic().to_string(), msg.partition(), msg.offset()));
                }
                Err(e) => panic!("Error consuming: {:?}", e),
            }
        }
    }

    assert_eq!(received, 5, "Should receive all messages");

    // Commit the last position
    if let Some((topic_name, partition, offset)) = last_msg {
        let mut tpl = TopicPartitionList::new();
        tpl.add_partition_offset(&topic_name, partition, rdkafka::Offset::Offset(offset + 1))
            .expect("Failed to add partition offset");
        consumer.commit(&tpl, CommitMode::Sync).expect("Failed to commit");

        // Fetch committed offsets
        let committed = consumer.committed(Duration::from_secs(5))
            .expect("Failed to fetch committed offsets");

        let committed_offset = committed.find_partition(&topic_name, partition)
            .map(|e| e.offset());

        assert!(committed_offset.is_some(), "Should have committed offset for partition");
    }
}

#[tokio::test]
#[ignore = "rdkafka tests are run via scripts/compatibility-test.sh"]
async fn test_rdkafka_multiple_consumers_same_group() {
    let server = TestServer::start();
    let topic = unique_topic("cg-multi");
    let group = unique_group("multi-group");
    let producer = server.rdkafka_producer();
    let message_count = 20;

    // Produce messages
    for i in 0..message_count {
        let payload = format!("multi-consumer-{}", i);
        let key = format!("key-{}", i);
        let record = FutureRecord::to(&topic)
            .payload(&payload)
            .key(&key);
        producer.send(record, Duration::from_secs(5)).await
            .expect("Failed to produce");
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Create two consumers in same group
    let consumer1 = create_group_consumer(&server, &group, "30000");
    let consumer2 = create_group_consumer(&server, &group, "30000");

    consumer1.subscribe(&[&topic]).expect("Consumer 1 failed to subscribe");
    consumer2.subscribe(&[&topic]).expect("Consumer 2 failed to subscribe");

    // Both consumers poll - messages should be distributed
    let mut total_received = 0;
    let mut c1_received = 0;
    let mut c2_received = 0;
    let timeout = Duration::from_secs(15);
    let start = std::time::Instant::now();

    while total_received < message_count && start.elapsed() < timeout {
        // Poll consumer 1
        if let Some(result) = consumer1.poll(Duration::from_millis(50)) {
            if result.is_ok() {
                c1_received += 1;
                total_received += 1;
            }
        }

        // Poll consumer 2
        if let Some(result) = consumer2.poll(Duration::from_millis(50)) {
            if result.is_ok() {
                c2_received += 1;
                total_received += 1;
            }
        }
    }

    assert_eq!(total_received, message_count,
        "Should receive all {} messages between both consumers, got {}", message_count, total_received);

    // With a single partition topic, one consumer should get all messages
    // But we verify total delivery is correct
    println!("Consumer 1 received: {}, Consumer 2 received: {}", c1_received, c2_received);
}

#[tokio::test]
#[ignore = "rdkafka tests are run via scripts/compatibility-test.sh"]
async fn test_rdkafka_consumer_group_lifecycle() {
    let server = TestServer::start();
    let topic = unique_topic("cg-lifecycle");
    let group = unique_group("lifecycle-group");
    let producer = server.rdkafka_producer();

    // Step 1: Produce initial messages
    for i in 0..5 {
        let payload = format!("lifecycle-batch1-{}", i);
        let record = FutureRecord::to(&topic)
            .payload(&payload)
            .key("key");
        producer.send(record, Duration::from_secs(5)).await
            .expect("Failed to produce");
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Step 2: Consumer joins, consumes, commits
    {
        let consumer = server.rdkafka_consumer(&group);
        consumer.subscribe(&[&topic]).expect("Failed to subscribe");

        let mut received = 0;
        let timeout = Duration::from_secs(10);
        let start = std::time::Instant::now();

        while received < 5 && start.elapsed() < timeout {
            if let Some(result) = consumer.poll(Duration::from_millis(100)) {
                match result {
                    Ok(msg) => {
                        received += 1;
                        // Commit each message
                        consumer.commit_message(&msg, CommitMode::Sync)
                            .expect("Failed to commit");
                    }
                    Err(e) => panic!("Error consuming: {:?}", e),
                }
            }
        }

        assert_eq!(received, 5, "Lifecycle step 2: should consume 5 messages");

        // Consumer drops here (leaves group implicitly)
    }

    // Step 3: Produce more messages
    for i in 0..5 {
        let payload = format!("lifecycle-batch2-{}", i);
        let record = FutureRecord::to(&topic)
            .payload(&payload)
            .key("key");
        producer.send(record, Duration::from_secs(5)).await
            .expect("Failed to produce");
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Step 4: New consumer joins same group, should resume from committed offset
    {
        let consumer = server.rdkafka_consumer(&group);
        consumer.subscribe(&[&topic]).expect("Failed to subscribe");

        let mut received = 0;
        let mut payloads = Vec::new();
        let timeout = Duration::from_secs(10);
        let start = std::time::Instant::now();

        while received < 5 && start.elapsed() < timeout {
            if let Some(result) = consumer.poll(Duration::from_millis(100)) {
                match result {
                    Ok(msg) => {
                        if let Some(payload) = msg.payload() {
                            payloads.push(String::from_utf8_lossy(payload).to_string());
                        }
                        received += 1;
                    }
                    Err(e) => panic!("Error consuming: {:?}", e),
                }
            }
        }

        assert_eq!(received, 5, "Lifecycle step 4: should consume 5 new messages");

        // Verify we got batch2 messages (not batch1 again)
        for payload in &payloads {
            assert!(payload.contains("batch2"),
                "Should receive batch2 messages, got: {}", payload);
        }
    }
}

#[tokio::test]
#[ignore = "rdkafka tests are run via scripts/compatibility-test.sh"]
async fn test_rdkafka_consumer_rebalance_on_new_member() {
    let server = TestServer::start();
    let topic = unique_topic("cg-rebalance");
    let group = unique_group("rebalance-group");
    let producer = server.rdkafka_producer();

    // Produce messages
    for i in 0..10 {
        let payload = format!("rebalance-{}", i);
        let key = format!("key-{}", i);
        let record = FutureRecord::to(&topic)
            .payload(&payload)
            .key(&key);
        producer.send(record, Duration::from_secs(5)).await
            .expect("Failed to produce");
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    // First consumer joins and starts consuming
    let consumer1 = create_group_consumer(&server, &group, "30000");
    consumer1.subscribe(&[&topic]).expect("Consumer 1 failed to subscribe");

    // Consume a few messages with consumer1
    let mut c1_count = 0;
    for _ in 0..5 {
        if let Some(Ok(_)) = consumer1.poll(Duration::from_millis(500)) {
            c1_count += 1;
        }
    }

    // Second consumer joins - triggers rebalance
    let consumer2 = create_group_consumer(&server, &group, "30000");
    consumer2.subscribe(&[&topic]).expect("Consumer 2 failed to subscribe");

    // Both consumers continue polling
    let mut total = c1_count;
    let timeout = Duration::from_secs(15);
    let start = std::time::Instant::now();

    while total < 10 && start.elapsed() < timeout {
        if let Some(Ok(_)) = consumer1.poll(Duration::from_millis(100)) {
            total += 1;
        }
        if let Some(Ok(_)) = consumer2.poll(Duration::from_millis(100)) {
            total += 1;
        }
    }

    // Both consumers should be able to receive messages after rebalance
    assert!(total >= 10, "Should receive all messages even after rebalance, got {}", total);
}

#[tokio::test]
#[ignore = "rdkafka tests are run via scripts/compatibility-test.sh"]
async fn test_rdkafka_consumer_group_empty_topic() {
    let server = TestServer::start();
    let topic = unique_topic("cg-empty");
    let group = unique_group("empty-group");

    // Create consumer for empty topic
    let consumer = server.rdkafka_consumer(&group);
    consumer.subscribe(&[&topic]).expect("Failed to subscribe");

    // Poll should return None for empty topic (not error)
    let result = consumer.poll(Duration::from_millis(500));

    // Empty topic should just timeout, not error
    match result {
        None => {} // Expected - no messages
        Some(Ok(_)) => panic!("Should not receive message from empty topic"),
        Some(Err(e)) => {
            // Some errors are acceptable for empty/new topic
            let err_str = format!("{:?}", e);
            assert!(
                err_str.contains("UnknownTopicOrPartition") ||
                err_str.contains("NoPartitionsAssigned") ||
                err_str.contains("BrokerTransportFailure"),
                "Unexpected error for empty topic: {:?}", e
            );
        }
    }
}

#[tokio::test]
#[ignore = "rdkafka tests are run via scripts/compatibility-test.sh"]
async fn test_rdkafka_consumer_seek_to_beginning() {
    let server = TestServer::start();
    let topic = unique_topic("cg-seek");
    let group = unique_group("seek-group");
    let producer = server.rdkafka_producer();

    // Produce messages
    for i in 0..10 {
        let payload = format!("seek-{}", i);
        let record = FutureRecord::to(&topic)
            .payload(&payload)
            .key("key");
        producer.send(record, Duration::from_secs(5)).await
            .expect("Failed to produce");
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    let consumer = server.rdkafka_consumer(&group);
    consumer.subscribe(&[&topic]).expect("Failed to subscribe");

    // Consume all messages first
    let mut received = 0;
    let timeout = Duration::from_secs(10);
    let start = std::time::Instant::now();

    while received < 10 && start.elapsed() < timeout {
        if let Some(Ok(msg)) = consumer.poll(Duration::from_millis(100)) {
            received += 1;
            // Commit as we go
            consumer.commit_message(&msg, CommitMode::Sync).ok();
        }
    }

    assert_eq!(received, 10, "Should receive all 10 messages initially");

    // Get assignment and seek to beginning
    let assignment = consumer.assignment().expect("Failed to get assignment");
    if !assignment.elements().is_empty() {
        for elem in assignment.elements() {
            consumer.seek(elem.topic(), elem.partition(), rdkafka::Offset::Beginning, Duration::from_secs(5))
                .expect("Failed to seek to beginning");
        }

        // Should be able to consume from beginning again
        let mut re_received = 0;
        let start = std::time::Instant::now();

        while re_received < 10 && start.elapsed() < timeout {
            if let Some(Ok(_)) = consumer.poll(Duration::from_millis(100)) {
                re_received += 1;
            }
        }

        assert_eq!(re_received, 10, "Should re-receive all 10 messages after seek to beginning");
    }
}

// ============================================================================
// Legacy Protocol Edge Case Tests (kafka crate - Phase 3)
// These tests verify edge cases using the pure Rust kafka crate (v0-v2 protocol)
// Note: Tests that consume require v0/v1 Fetch protocol support
// ============================================================================

#[test]
fn test_legacy_large_batch_produce() {
    let server = TestServer::start();
    let topic = "legacy-large-batch-test";
    let mut producer = server.legacy_producer_for(topic);

    // Produce 150 messages in rapid succession - tests large batch handling
    let message_count = 150;
    for i in 0..message_count {
        let payload = format!("batch-message-{:05}", i);
        let result = producer.send(&Record::from_value(topic, payload.as_bytes()));
        assert!(result.is_ok(), "Should produce message {}: {:?}", i, result);
    }

    // Verify via metadata that topic exists and has messages
    let mut client = KafkaClient::new(server.hosts());
    client.load_metadata(&[topic]).expect("Failed to load metadata");
    let topics = client.topics();
    assert!(
        topics.names().any(|n| n == topic),
        "Topic should exist after producing"
    );
}

#[test]
fn test_legacy_binary_payload_produce() {
    let server = TestServer::start();
    let topic = "legacy-binary-payload-test";
    let mut producer = server.legacy_producer_for(topic);

    // Binary data with non-UTF8 bytes, null bytes, and control characters
    let binary_payloads: Vec<Vec<u8>> = vec![
        vec![0x00, 0x01, 0x02, 0xFF, 0xFE, 0xFD],                    // Raw bytes
        vec![0x00, 0x00, 0x00, 0x00],                                 // All nulls
        (0u8..=255u8).collect(),                                      // All byte values
        vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],        // PNG header
        vec![0x1F, 0x8B, 0x08, 0x00],                                 // gzip header
        vec![0xEF, 0xBB, 0xBF, 0x68, 0x65, 0x6C, 0x6C, 0x6F],        // UTF-8 BOM + hello
        vec![0x80, 0x81, 0x82, 0x83],                                 // Invalid UTF-8
    ];

    for (i, payload) in binary_payloads.iter().enumerate() {
        let result = producer.send(&Record::from_value(topic, payload.as_slice()));
        assert!(
            result.is_ok(),
            "Should produce binary message {}: {:?}",
            i,
            result
        );
    }

    // Verify topic was created
    let mut client = KafkaClient::new(server.hosts());
    client.load_metadata(&[topic]).expect("Failed to load metadata");
    let topics = client.topics();
    assert!(topics.names().any(|n| n == topic), "Topic should exist");
}

#[test]
fn test_legacy_empty_topic_name_handling() {
    let server = TestServer::start();

    // Test that empty topic name is handled gracefully
    let mut client = KafkaClient::new(server.hosts());

    // Loading metadata for empty topic should either fail gracefully or succeed
    let result = client.load_metadata(&[""]);
    // Server should either reject empty topic or create it - both are valid behaviors
    // The key is that it should not crash or hang
    println!("Empty topic metadata result: {:?}", result);
}

#[test]
fn test_legacy_metadata_refresh_after_topic_creation() {
    let server = TestServer::start();

    let mut client = KafkaClient::new(server.hosts());

    // Initial metadata fetch - no topics yet
    client.load_metadata_all().expect("Failed to load initial metadata");
    {
        let topics = client.topics();
        let initial_topics: Vec<_> = topics.names().collect();
        println!("Initial topics: {:?}", initial_topics);
    }

    // Create new topics by producing to them
    let new_topics: Vec<String> = (0..3)
        .map(|i| format!("legacy-metadata-refresh-topic-{}", i))
        .collect();

    for topic in &new_topics {
        let topic_refs = [topic.as_str()];
        let mut temp_client = KafkaClient::new(server.hosts());
        temp_client.load_metadata(&topic_refs).expect("Failed to load metadata");

        let mut producer = Producer::from_client(temp_client)
            .with_ack_timeout(Duration::from_secs(5))
            .with_required_acks(RequiredAcks::One)
            .create()
            .expect("Failed to create producer");

        producer
            .send(&Record::from_value(topic, &b"init"[..]))
            .expect("Failed to produce");
    }

    // Refresh metadata on original client
    client.load_metadata_all().expect("Failed to refresh metadata");
    let topics = client.topics();
    let refreshed_topics: Vec<_> = topics.names().collect();
    println!("Refreshed topics: {:?}", refreshed_topics);

    // Verify new topics are now visible
    for expected in &new_topics {
        assert!(
            refreshed_topics.contains(&expected.as_str()),
            "Should see topic {} after refresh, got {:?}",
            expected,
            refreshed_topics
        );
    }
}

#[test]
fn test_legacy_various_message_formats() {
    let server = TestServer::start();
    let topic = "legacy-formats-test";
    let mut producer = server.legacy_producer_for(topic);

    // Test various message formats
    // Keyed messages
    let keyed_messages: Vec<(&str, &[u8])> = vec![
        ("key1", b"value1"),
        ("key2", b""),                           // Empty value
        ("key3", b"value-for-key3"),             // Normal
        ("key-with-unicode", b"val"),            // Normal key
        ("k", b"large-value-placeholder"),       // Keyed message
    ];

    for (key, value) in &keyed_messages {
        let result = producer.send(&Record::from_key_value(topic, *key, *value));
        assert!(result.is_ok(), "Should produce keyed message: {:?}", result);
    }

    // Also produce a null-keyed message separately
    let null_key_result = producer.send(&Record::from_value(topic, &b"value-with-null-key"[..]));
    assert!(null_key_result.is_ok(), "Should produce null-keyed message");

    // Verify topic was created
    let mut client = KafkaClient::new(server.hosts());
    client.load_metadata(&[topic]).expect("Failed to load metadata");
    let topics = client.topics();
    assert!(topics.names().any(|n| n == topic), "Topic should exist");
}

#[test]
fn test_legacy_produce_many_small_messages() {
    let server = TestServer::start();
    let topic = "legacy-many-small-test";
    let mut producer = server.legacy_producer_for(topic);

    // Produce many small messages
    for i in 0..100 {
        let payload = format!("small-{}", i);
        let result = producer.send(&Record::from_value(topic, payload.as_bytes()));
        assert!(result.is_ok(), "Should produce small message {}: {:?}", i, result);
    }

    // Verify topic exists
    let mut client = KafkaClient::new(server.hosts());
    client.load_metadata(&[topic]).expect("Failed to load metadata");
    let topics = client.topics();
    assert!(topics.names().any(|n| n == topic), "Topic should exist");
}

#[test]
fn test_legacy_produce_varying_sizes() {
    let server = TestServer::start();
    let topic = "legacy-varying-sizes-test";
    let mut producer = server.legacy_producer_for(topic);

    // Produce messages of varying sizes
    let sizes = vec![1, 10, 100, 1000, 10000, 50000];

    for size in sizes {
        let payload = vec![b'x'; size];
        let result = producer.send(&Record::from_value(topic, payload.as_slice()));
        assert!(
            result.is_ok(),
            "Should produce message of size {}: {:?}",
            size,
            result
        );
    }

    // Verify topic exists
    let mut client = KafkaClient::new(server.hosts());
    client.load_metadata(&[topic]).expect("Failed to load metadata");
    let topics = client.topics();
    assert!(topics.names().any(|n| n == topic), "Topic should exist");
}

#[test]
fn test_legacy_sequential_produce() {
    let server = TestServer::start();
    let topic = "legacy-sequential-produce-test";
    let mut producer = server.legacy_producer_for(topic);

    // Produce messages with sequence numbers
    let count = 50;
    for i in 0..count {
        let payload = format!("{:08}", i); // Zero-padded for easy parsing
        let result = producer.send(&Record::from_value(topic, payload.as_bytes()));
        assert!(
            result.is_ok(),
            "Should produce sequential message {}: {:?}",
            i,
            result
        );
    }

    // Verify topic exists
    let mut client = KafkaClient::new(server.hosts());
    client.load_metadata(&[topic]).expect("Failed to load metadata");
    let topics = client.topics();
    assert!(topics.names().any(|n| n == topic), "Topic should exist");
}

#[test]
fn test_legacy_concurrent_producers() {
    let server = TestServer::start();
    let topic = "legacy-concurrent-producers-test";

    // Create multiple producers in threads
    let handles: Vec<_> = (0..3)
        .map(|producer_id| {
            let hosts = server.hosts();
            std::thread::spawn(move || {
                let mut client = KafkaClient::new(hosts);
                client.load_metadata(&[topic]).expect("Failed to load metadata");

                let mut producer = Producer::from_client(client)
                    .with_ack_timeout(Duration::from_secs(5))
                    .with_required_acks(RequiredAcks::One)
                    .create()
                    .expect("Failed to create producer");

                for i in 0..20 {
                    let payload = format!("producer-{}-msg-{}", producer_id, i);
                    producer
                        .send(&Record::from_value(topic, payload.as_bytes()))
                        .expect("Failed to produce");
                }
            })
        })
        .collect();

    // Wait for all producers to finish
    for handle in handles {
        handle.join().expect("Producer thread panicked");
    }

    // Verify topic exists after concurrent production
    let mut client = KafkaClient::new(server.hosts());
    client.load_metadata(&[topic]).expect("Failed to load metadata");
    let topics = client.topics();
    assert!(
        topics.names().any(|n| n == topic),
        "Topic should exist after concurrent production"
    );
}

#[test]
fn test_legacy_special_characters_in_topic_name() {
    let server = TestServer::start();

    // Topics with various allowed special characters
    let special_topics = vec![
        "topic-with-dashes",
        "topic_with_underscores",
        "topic.with.dots",
        "TopicWithCamelCase",
        "topic123numbers",
    ];

    for topic in &special_topics {
        let mut client = KafkaClient::new(server.hosts());
        let result = client.load_metadata(&[topic]);

        assert!(
            result.is_ok(),
            "Should handle topic name '{}': {:?}",
            topic,
            result
        );

        let mut producer = Producer::from_client(client)
            .with_ack_timeout(Duration::from_secs(5))
            .with_required_acks(RequiredAcks::One)
            .create()
            .expect("Failed to create producer");

        let result = producer.send(&Record::from_value(topic, &b"test"[..]));
        assert!(
            result.is_ok(),
            "Should produce to topic '{}': {:?}",
            topic,
            result
        );
    }
}

#[test]
fn test_legacy_rapid_metadata_refresh() {
    let server = TestServer::start();

    // Rapidly refresh metadata multiple times
    let mut client = KafkaClient::new(server.hosts());

    for i in 0..10 {
        let result = client.load_metadata_all();
        assert!(
            result.is_ok(),
            "Metadata refresh {} should succeed: {:?}",
            i,
            result
        );
    }

    // Create topics and rapidly refresh
    let topics: Vec<String> = (0..5)
        .map(|i| format!("rapid-refresh-topic-{}", i))
        .collect();
    let topic_refs: Vec<&str> = topics.iter().map(|s| s.as_str()).collect();

    for _ in 0..5 {
        let result = client.load_metadata(&topic_refs);
        assert!(result.is_ok(), "Targeted metadata refresh should succeed");
    }
}

#[test]
fn test_legacy_keyed_messages_produce() {
    let server = TestServer::start();
    let topic = "legacy-keyed-messages-test";
    let mut producer = server.legacy_producer_for(topic);

    // Produce messages with specific key-value pairs
    let pairs: Vec<(&str, &str)> = vec![
        ("user:1", "alice"),
        ("user:2", "bob"),
        ("user:3", "charlie"),
        ("order:100", "pending"),
        ("order:101", "completed"),
    ];

    for (key, value) in &pairs {
        let result = producer.send(&Record::from_key_value(topic, *key, *value));
        assert!(
            result.is_ok(),
            "Should produce keyed message ({}, {}): {:?}",
            key,
            value,
            result
        );
    }

    // Verify topic exists
    let mut client = KafkaClient::new(server.hosts());
    client.load_metadata(&[topic]).expect("Failed to load metadata");
    let topics = client.topics();
    assert!(topics.names().any(|n| n == topic), "Topic should exist");
}
