//! Parity workload: hand-crafted out-of-order idempotent producer sequence.

use crate::broker::BrokerTarget;
use crate::driver::{Observation, ObservationEvent, WorkloadDriver};
use crate::raw_protocol::RawKafkaClient;
use anyhow::Result;
use async_trait::async_trait;
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::client::DefaultClientContext;
use std::time::Duration;

const TOPIC: &str = "parity-out-of-order-sequence";

pub struct OutOfOrderSequence;

#[async_trait]
impl WorkloadDriver for OutOfOrderSequence {
    fn name(&self) -> &'static str {
        "out_of_order_sequence"
    }

    async fn run(&self, target: &BrokerTarget) -> Result<Vec<Observation>> {
        let bootstrap = target.bootstrap_servers.clone();
        tokio::task::spawn_blocking(move || run_sync(&bootstrap)).await?
    }
}

fn run_sync(bootstrap: &str) -> Result<Vec<Observation>> {
    create_topic(bootstrap, TOPIC)?;

    let mut client = RawKafkaClient::connect(bootstrap)?;
    client.api_versions()?;
    let (producer_id, producer_epoch) = client.init_producer_id()?;

    let first_code = client.produce_sequence(TOPIC, producer_id, producer_epoch, 0)?;
    if first_code != 0 {
        anyhow::bail!("initial Produce returned error {first_code}");
    }

    let out_of_order_code = client.produce_sequence(TOPIC, producer_id, producer_epoch, 5)?;
    Ok(vec![Observation {
        workload: "out_of_order_sequence",
        step: 0,
        event: ObservationEvent::ErrorCode {
            api: "Produce",
            code: out_of_order_code,
        },
    }])
}

fn create_topic(bootstrap: &str, topic: &str) -> Result<()> {
    let admin: AdminClient<DefaultClientContext> = rdkafka::ClientConfig::new()
        .set("bootstrap.servers", bootstrap)
        .set("socket.timeout.ms", "15000")
        .create()?;
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let results = rt.block_on(admin.create_topics(
        &[NewTopic::new(topic, 1, TopicReplication::Fixed(1))],
        &AdminOptions::new().request_timeout(Some(Duration::from_secs(15))),
    ))?;
    for result in results {
        result.map_err(|(name, error)| anyhow::anyhow!("create topic {name}: {error:?}"))?;
    }
    Ok(())
}
