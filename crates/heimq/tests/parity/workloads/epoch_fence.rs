//! Parity workload: zombie-producer fencing (fault injection).
//!
//! Producer A opens a transaction and produces (without committing). Producer B,
//! sharing the same `transactional.id`, initialises transactions — which fences
//! producer A by bumping the producer epoch. Producer A's subsequent commit must
//! then fail on every broker. heimq enforces this via INVALID_PRODUCER_EPOCH (47)
//! in its EndTxn handler; the differential assertion here is the observable
//! outcome (the zombie's commit is rejected), normalized across brokers since the
//! client surfaces a translated transaction error rather than the raw code.

use crate::broker::BrokerTarget;
use crate::driver::{Observation, ObservationEvent, WorkloadDriver};
use anyhow::Result;
use async_trait::async_trait;
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::client::DefaultClientContext;
use rdkafka::producer::{BaseProducer, BaseRecord, Producer};
use std::time::Duration;

const TOPIC: &str = "parity-epoch-fence";
const TXN_ID: &str = "parity-epoch-fence-txn";

pub struct EpochFence;

#[async_trait]
impl WorkloadDriver for EpochFence {
    fn name(&self) -> &'static str {
        "epoch_fence"
    }

    async fn run(&self, target: &BrokerTarget) -> Result<Vec<Observation>> {
        let bootstrap = target.bootstrap_servers.clone();
        tokio::task::spawn_blocking(move || run_sync(&bootstrap)).await?
    }
}

fn txn_producer(bootstrap: &str) -> Result<BaseProducer> {
    Ok(rdkafka::ClientConfig::new()
        .set("bootstrap.servers", bootstrap)
        .set("socket.timeout.ms", "15000")
        .set("message.timeout.ms", "15000")
        .set("transactional.id", TXN_ID)
        .set("transaction.timeout.ms", "30000")
        .create()?)
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

    // Producer A: open a transaction and produce, but do not commit.
    let producer_a = txn_producer(bootstrap)?;
    producer_a.init_transactions(Duration::from_secs(10))?;
    producer_a.begin_transaction()?;
    producer_a
        .send(BaseRecord::to(TOPIC).key("zombie").payload("zombie"))
        .map_err(|(e, _)| anyhow::anyhow!("producer A send failed: {}", e))?;
    producer_a.flush(Duration::from_secs(10))?;

    // Producer B: same transactional.id — init_transactions fences producer A.
    let producer_b = txn_producer(bootstrap)?;
    producer_b.init_transactions(Duration::from_secs(10))?;

    // Producer A is now a zombie; its commit must be rejected on every broker.
    let fenced = producer_a.commit_transaction(Duration::from_secs(10)).is_err();

    Ok(vec![Observation {
        workload: "epoch_fence",
        step: 0,
        event: ObservationEvent::ErrorCode {
            api: "EndTxn",
            // 1 = fenced (commit rejected), 0 = not fenced. Every broker must fence.
            code: if fenced { 1 } else { 0 },
        },
    }])
}
