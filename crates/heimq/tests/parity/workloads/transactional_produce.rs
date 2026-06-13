//! Parity workload: transactional produce + read_committed consume (US-004-AC6).
//!
//! Commits one batch of 3 records, aborts another.
//! A read_committed consumer should see only the 3 committed records.

use crate::broker::BrokerTarget;
use crate::driver::{Observation, ObservationEvent, WorkloadDriver};
use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::client::DefaultClientContext;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::message::Message;
use rdkafka::producer::{BaseProducer, BaseRecord, Producer};
use rdkafka::topic_partition_list::TopicPartitionList;
use rdkafka::Offset;
use std::time::Duration;

const TOPIC: &str = "parity-txn-produce";
const TXN_ID: &str = "parity-txn-test";

// @covers US-004-AC6
pub struct TransactionalProduceRoundtrip;

#[async_trait]
impl WorkloadDriver for TransactionalProduceRoundtrip {
    fn name(&self) -> &'static str {
        "transactional_produce_roundtrip"
    }

    async fn run(&self, target: &BrokerTarget) -> Result<Vec<Observation>> {
        let bootstrap = target.bootstrap_servers.clone();
        tokio::task::spawn_blocking(move || run_sync(&bootstrap)).await?
    }
}

fn run_sync(bootstrap: &str) -> Result<Vec<Observation>> {
    let admin: AdminClient<DefaultClientContext> = rdkafka::ClientConfig::new()
        .set("bootstrap.servers", bootstrap)
        .set("socket.timeout.ms", "15000")
        .create()?;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(admin.create_topics(
        &[NewTopic::new(TOPIC, 1, TopicReplication::Fixed(1))],
        &AdminOptions::new().request_timeout(Some(Duration::from_secs(15))),
    ))?;

    // Use BaseProducer for transaction control.
    let producer: BaseProducer = rdkafka::ClientConfig::new()
        .set("bootstrap.servers", bootstrap)
        .set("socket.timeout.ms", "15000")
        .set("message.timeout.ms", "15000")
        .set("transactional.id", TXN_ID)
        .set("transaction.timeout.ms", "30000")
        .create()?;

    producer.init_transactions(Duration::from_secs(10))?;

    // Transaction 1: commit 3 records.
    producer.begin_transaction()?;
    for i in 0..3i32 {
        let key = format!("commit-key-{}", i);
        let val = format!("commit-val-{}", i);
        producer.send(
            BaseRecord::to(TOPIC).key(key.as_str()).payload(val.as_str()),
        ).map_err(|(e, _)| anyhow::anyhow!("produce failed: {}", e))?;
    }
    producer.flush(Duration::from_secs(10))?;
    producer.commit_transaction(Duration::from_secs(10))?;

    // Transaction 2: abort 3 records.
    producer.begin_transaction()?;
    for i in 0..3i32 {
        let key = format!("abort-key-{}", i);
        let val = format!("abort-val-{}", i);
        producer.send(
            BaseRecord::to(TOPIC).key(key.as_str()).payload(val.as_str()),
        ).map_err(|(e, _)| anyhow::anyhow!("produce failed: {}", e))?;
    }
    producer.flush(Duration::from_secs(10))?;
    producer.abort_transaction(Duration::from_secs(10))?;

    // read_committed consumer — expects only the 3 committed records.
    let consumer: BaseConsumer = rdkafka::ClientConfig::new()
        .set("bootstrap.servers", bootstrap)
        .set("socket.timeout.ms", "15000")
        .set("group.id", "parity-txn-rc-probe")
        .set("enable.auto.commit", "false")
        .set("isolation.level", "read_committed")
        .create()?;

    let mut tpl = TopicPartitionList::new();
    tpl.add_partition_offset(TOPIC, 0, Offset::Beginning)?;
    consumer.assign(&tpl)?;

    let mut observations = Vec::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(30);

    loop {
        if std::time::Instant::now() > deadline {
            anyhow::bail!(
                "transactional_produce_roundtrip: timed out, got {} read_committed records",
                observations.len()
            );
        }
        match consumer.poll(Duration::from_millis(500)) {
            Some(Ok(msg)) => {
                let key = msg.key().map(Bytes::copy_from_slice);
                let value = msg.payload().map(Bytes::copy_from_slice);
                let step = observations.len() as u32;
                observations.push(Observation {
                    workload: "transactional_produce_roundtrip",
                    step,
                    event: ObservationEvent::RecordConsumed {
                        key,
                        value,
                        headers: vec![],
                        partition: msg.partition(),
                        offset: msg.offset(),
                        timestamp: 0,
                    },
                });
                if observations.len() == 3 {
                    break;
                }
            }
            Some(Err(e)) => anyhow::bail!("consumer error: {}", e),
            None => {
                if observations.len() == 3 {
                    break;
                }
            }
        }
    }

    Ok(observations)
}
