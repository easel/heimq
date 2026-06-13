//! Parity workload: idempotent produce round-trip (US-003-AC5).
//!
//! Produces N records with `enable.idempotence=true`, then consumes from
//! offset 0.  The broker must correctly handle InitProducerId and sequence
//! tracking.  Emits one `RecordConsumed` observation per record, identical
//! to the produce_fetch_roundtrip observations so the diff engine can compare.

use crate::broker::BrokerTarget;
use crate::driver::{Observation, ObservationEvent, WorkloadDriver};
use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::client::DefaultClientContext;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::message::Message;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::topic_partition_list::TopicPartitionList;
use rdkafka::Offset;
use std::time::Duration;

const TOPIC: &str = "parity-idempotent-produce";
const N: usize = 10;

// @covers US-003-AC5
pub struct IdempotentProduceRoundtrip;

#[async_trait]
impl WorkloadDriver for IdempotentProduceRoundtrip {
    fn name(&self) -> &'static str {
        "idempotent_produce_roundtrip"
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

    // Idempotent producer.
    let producer: FutureProducer = rdkafka::ClientConfig::new()
        .set("bootstrap.servers", bootstrap)
        .set("socket.timeout.ms", "15000")
        .set("message.timeout.ms", "15000")
        .set("enable.idempotence", "true")
        .create()?;

    for i in 0..N {
        let key = format!("key-{}", i);
        let val = format!("val-{}", i);
        rt.block_on(producer.send(
            FutureRecord::to(TOPIC).key(&key).payload(&val),
            Duration::from_secs(15),
        ))
        .map_err(|(e, _)| anyhow::anyhow!("idempotent produce failed: {}", e))?;
    }

    let consumer: BaseConsumer = rdkafka::ClientConfig::new()
        .set("bootstrap.servers", bootstrap)
        .set("socket.timeout.ms", "15000")
        .set("group.id", "parity-idempotent-probe")
        .set("enable.auto.commit", "false")
        .create()?;

    let mut tpl = TopicPartitionList::new();
    tpl.add_partition_offset(TOPIC, 0, Offset::Beginning)?;
    consumer.assign(&tpl)?;

    let mut observations = Vec::with_capacity(N);
    let deadline = std::time::Instant::now() + Duration::from_secs(30);

    while observations.len() < N {
        if std::time::Instant::now() > deadline {
            anyhow::bail!(
                "idempotent_produce_roundtrip: timed out after consuming only {}/{} records",
                observations.len(),
                N
            );
        }
        match consumer.poll(Duration::from_millis(500)) {
            Some(Ok(msg)) => {
                let key = msg.key().map(Bytes::copy_from_slice);
                let value = msg.payload().map(Bytes::copy_from_slice);
                let step = observations.len() as u32;
                observations.push(Observation {
                    workload: "idempotent_produce_roundtrip",
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
            }
            Some(Err(e)) => anyhow::bail!("consumer error: {}", e),
            None => {}
        }
    }

    Ok(observations)
}
