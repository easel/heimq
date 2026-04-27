use anyhow::Result;
use heimq::test_support::TestServer;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::ClientConfig;
use std::time::Duration;
use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, GenericImage, ImageExt};

const REDPANDA_IMAGE: &str = "docker.redpanda.com/redpandadata/redpanda";
const REDPANDA_TAG: &str = "v25.1.1";
const KAFKA_PORT: u16 = 9092;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrokerKind {
    Heimq,
    Redpanda,
}

pub struct BrokerTarget {
    pub kind: BrokerKind,
    pub bootstrap_servers: String,
}

impl BrokerTarget {
    pub fn client_config(&self) -> ClientConfig {
        let mut cfg = ClientConfig::new();
        cfg.set("bootstrap.servers", &self.bootstrap_servers);
        cfg.set("socket.timeout.ms", "10000");
        cfg
    }
}

pub struct Targets {
    pub heimq: BrokerTarget,
    pub redpanda: BrokerTarget,
}

pub async fn boot() -> Result<(ContainerAsync<GenericImage>, TestServer, Targets)> {
    // Use fixed host port = container port so Redpanda's advertised address is correct.
    // HARNESS-001: --advertise-kafka-addr localhost:<port>
    // Configure image first, then apply mapped port (which returns ContainerRequest).
    // Fixed port binding: host 9092 → container 9092, so advertised localhost:9092 resolves.
    let redpanda_container = GenericImage::new(REDPANDA_IMAGE, REDPANDA_TAG)
        .with_exposed_port(KAFKA_PORT.tcp())
        .with_wait_for(WaitFor::message_on_stderr("Successfully started Redpanda!"))
        .with_cmd([
            "redpanda",
            "start",
            "--smp",
            "1",
            "--memory",
            "512M",
            "--overprovisioned",
            "--kafka-addr",
            "0.0.0.0:9092",
            "--advertise-kafka-addr",
            "localhost:9092",
        ])
        .with_mapped_port(KAFKA_PORT, KAFKA_PORT.tcp())
        .start()
        .await?;

    let redpanda_bootstrap = format!("127.0.0.1:{}", KAFKA_PORT);

    let heimq_server = TestServer::start();
    let heimq_bootstrap = heimq_server.bootstrap_servers();

    verify_connection(redpanda_bootstrap.clone(), "redpanda").await?;
    verify_connection(heimq_bootstrap.clone(), "heimq").await?;

    let targets = Targets {
        heimq: BrokerTarget {
            kind: BrokerKind::Heimq,
            bootstrap_servers: heimq_bootstrap,
        },
        redpanda: BrokerTarget {
            kind: BrokerKind::Redpanda,
            bootstrap_servers: redpanda_bootstrap,
        },
    };

    Ok((redpanda_container, heimq_server, targets))
}

async fn verify_connection(bootstrap: String, name: &'static str) -> Result<()> {
    tokio::task::spawn_blocking(move || verify_connection_sync(&bootstrap, name)).await?
}

fn verify_connection_sync(bootstrap: &str, name: &str) -> Result<()> {
    let consumer: BaseConsumer = ClientConfig::new()
        .set("bootstrap.servers", bootstrap)
        .set("group.id", "parity-probe")
        .set("socket.timeout.ms", "10000")
        .create()?;

    consumer
        .fetch_metadata(None, Duration::from_secs(15))
        .map_err(|e| anyhow::anyhow!("{} rdkafka connection probe failed: {}", name, e))?;

    Ok(())
}
