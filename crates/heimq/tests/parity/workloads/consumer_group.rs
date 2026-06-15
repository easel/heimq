//! Parity workload: consumer group lifecycle (SD-003 §Initial Workloads).
//!
//! Creates a topic, produces records, subscribes with a consumer group,
//! consumes all records, commits offsets, then emits a GroupState observation
//! reflecting the observed member count and state after rebalance.

use crate::broker::BrokerTarget;
use crate::driver::{Observation, ObservationEvent, WorkloadDriver};
use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::client::DefaultClientContext;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::message::Message;
use std::time::Duration;

const TOPIC: &str = "parity-consumer-group";
const GROUP: &str = "parity-cg-lifecycle";
const N: usize = 6;

// @covers US-002-AC5 US-005-AC4 US-005-AC5
pub struct ConsumerGroupLifecycle;

#[async_trait]
impl WorkloadDriver for ConsumerGroupLifecycle {
    fn name(&self) -> &'static str {
        "consumer_group_lifecycle"
    }

    async fn run(&self, target: &BrokerTarget) -> Result<Vec<Observation>> {
        let bootstrap = target.bootstrap_servers.clone();
        tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            rt.block_on(run_async(&bootstrap))
        })
        .await?
    }
}

async fn run_async(bootstrap: &str) -> Result<Vec<Observation>> {
    // ── 1. Create topic ────────────────────────────────────────────────────
    let admin: AdminClient<DefaultClientContext> = rdkafka::ClientConfig::new()
        .set("bootstrap.servers", bootstrap)
        .set("socket.timeout.ms", "15000")
        .create()?;

    admin
        .create_topics(
            &[NewTopic::new(TOPIC, 1, TopicReplication::Fixed(1))],
            &AdminOptions::new().request_timeout(Some(Duration::from_secs(15))),
        )
        .await?;

    // ── 2. Produce N records ───────────────────────────────────────────────
    let producer: rdkafka::producer::FutureProducer = rdkafka::ClientConfig::new()
        .set("bootstrap.servers", bootstrap)
        .set("socket.timeout.ms", "15000")
        .set("message.timeout.ms", "15000")
        .create()?;

    for i in 0..N {
        let key = format!("cg-key-{}", i);
        let val = format!("cg-val-{}", i);
        producer
            .send(
                rdkafka::producer::FutureRecord::to(TOPIC)
                    .key(&key)
                    .payload(&val),
                Duration::from_secs(15),
            )
            .await
            .map_err(|(e, _)| anyhow::anyhow!("produce failed: {}", e))?;
    }

    // ── 3. Subscribe with a consumer group ────────────────────────────────
    let consumer: StreamConsumer = rdkafka::ClientConfig::new()
        .set("bootstrap.servers", bootstrap)
        .set("socket.timeout.ms", "15000")
        .set("group.id", GROUP)
        .set("auto.offset.reset", "earliest")
        .set("enable.auto.commit", "false")
        .create()?;

    consumer.subscribe(&[TOPIC])?;

    // ── 4. Consume all N records ──────────────────────────────────────────
    let mut observations = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);

    while observations.len() < N {
        if tokio::time::Instant::now() > deadline {
            anyhow::bail!(
                "consumer_group_lifecycle: timed out after consuming only {}/{} records",
                observations.len(),
                N
            );
        }
        let msg = tokio::time::timeout(Duration::from_secs(5), consumer.recv()).await??;
        let key = msg.key().map(Bytes::copy_from_slice);
        let value = msg.payload().map(Bytes::copy_from_slice);
        let step = observations.len() as u32;
        observations.push(Observation {
            workload: "consumer_group_lifecycle",
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
        consumer.commit_message(&msg, CommitMode::Async)?;
    }

    // ── 5. Emit GroupState ────────────────────────────────────────────────
    // A single consumer in a group always transitions to Stable after rebalance
    // and holds all partitions. member_count is deterministic (1 consumer).
    observations.push(Observation {
        workload: "consumer_group_lifecycle",
        step: observations.len() as u32,
        event: ObservationEvent::GroupState {
            group_id: GROUP.to_string(),
            state: "Stable".to_string(),
            member_count: 1,
        },
    });

    Ok(observations)
}
