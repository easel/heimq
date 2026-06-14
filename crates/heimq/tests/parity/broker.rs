use anyhow::Result;
use heimq::test_support::TestServer;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::ClientConfig;
use std::time::Duration;
use testcontainers::core::WaitFor;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, GenericImage, ImageExt};

const REDPANDA_IMAGE: &str = "docker.redpanda.com/redpandadata/redpanda";
const REDPANDA_TAG: &str = "v25.1.1";
const KAFKA_IMAGE: &str = "apache/kafka";
const KAFKA_TAG: &str = "3.9.0";
const KAFKA_PORT: u16 = 9092;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrokerKind {
    Heimq,
    Redpanda,
    Kafka,
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
    pub kafka: BrokerTarget,
}

pub async fn boot() -> Result<(
    ContainerAsync<GenericImage>,
    ContainerAsync<GenericImage>,
    TestServer,
    Targets,
)> {
    // Start Redpanda using a shell entrypoint that auto-detects the container's bridge IP
    // and passes it as --advertise-kafka-addr. This avoids the port-mapping issues that
    // arise in OrbStack where Docker's bridge IPs are routable from the Linux VM but
    // host port mappings are not.
    //
    // The WaitFor message confirms the Kafka listener is up, then we query
    // the container's bridge IP via Docker inspect to build the bootstrap address.
    let startup_script = "\
        IP=$(hostname -I | awk '{print $1}'); \
        exec /entrypoint.sh redpanda start \
            --smp 1 \
            --memory 512M \
            --overprovisioned \
            --kafka-addr 0.0.0.0:9092 \
            --advertise-kafka-addr ${IP}:9092";

    let redpanda_container = GenericImage::new(REDPANDA_IMAGE, REDPANDA_TAG)
        .with_wait_for(WaitFor::message_on_stderr("Successfully started Redpanda!"))
        .with_entrypoint("/bin/sh")
        .with_cmd(["-c", startup_script])
        .start()
        .await?;

    // Query the container's bridge IP — this is the address the container advertises
    // and the address our test binary uses to connect.
    let bridge_ip = redpanda_container.get_bridge_ip_address().await?;
    let redpanda_bootstrap = format!("{}:{}", bridge_ip, KAFKA_PORT);

    // Start Apache Kafka in single-node KRaft mode. The broker (PLAINTEXT) listener is
    // bound to the container's bridge IP — resolved at runtime via BusyBox `hostname -i`
    // — so the test binary can reach it under OrbStack, and advertised at that same IP.
    // It must not be 0.0.0.0: the storage-format step rejects a non-routable advertised
    // address. The CONTROLLER listener stays on loopback (127.0.0.1) to match
    // `controller.quorum.voters`; it is internal to this single combined node and never
    // reached externally. Remaining single-node KRaft settings are passed as KAFKA_* env
    // vars and consumed by the image's standard `/etc/kafka/docker/run` config step. We
    // pass the script as the *command* so the image's `/__cacert_entrypoint.sh` (which
    // ends in `exec "$@"`) still runs ahead of it.
    let kafka_script = "\
        IP=$(hostname -i | awk '{print $1}'); \
        export KAFKA_LISTENERS=PLAINTEXT://${IP}:9092,CONTROLLER://127.0.0.1:9093; \
        export KAFKA_ADVERTISED_LISTENERS=PLAINTEXT://${IP}:9092; \
        exec /etc/kafka/docker/run";

    let kafka_container = GenericImage::new(KAFKA_IMAGE, KAFKA_TAG)
        .with_wait_for(WaitFor::message_on_stdout("Startup complete."))
        .with_env_var("KAFKA_NODE_ID", "1")
        .with_env_var("KAFKA_PROCESS_ROLES", "broker,controller")
        .with_env_var("KAFKA_CONTROLLER_QUORUM_VOTERS", "1@127.0.0.1:9093")
        .with_env_var(
            "KAFKA_LISTENER_SECURITY_PROTOCOL_MAP",
            "CONTROLLER:PLAINTEXT,PLAINTEXT:PLAINTEXT",
        )
        .with_env_var("KAFKA_CONTROLLER_LISTENER_NAMES", "CONTROLLER")
        .with_env_var("KAFKA_INTER_BROKER_LISTENER_NAME", "PLAINTEXT")
        .with_env_var("KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR", "1")
        .with_env_var("KAFKA_TRANSACTION_STATE_LOG_REPLICATION_FACTOR", "1")
        .with_env_var("KAFKA_TRANSACTION_STATE_LOG_MIN_ISR", "1")
        .with_env_var("KAFKA_GROUP_INITIAL_REBALANCE_DELAY_MS", "0")
        .with_cmd(["/bin/sh", "-c", kafka_script])
        .start()
        .await?;

    let kafka_bridge_ip = kafka_container.get_bridge_ip_address().await?;
    let kafka_bootstrap = format!("{}:{}", kafka_bridge_ip, KAFKA_PORT);

    let heimq_server = TestServer::start();
    let heimq_bootstrap = heimq_server.bootstrap_servers();

    verify_connection(redpanda_bootstrap.clone(), "redpanda").await?;
    verify_connection(kafka_bootstrap.clone(), "kafka").await?;
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
        kafka: BrokerTarget {
            kind: BrokerKind::Kafka,
            bootstrap_servers: kafka_bootstrap,
        },
    };

    Ok((redpanda_container, kafka_container, heimq_server, targets))
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

    // Poll metadata until the broker answers. Kafka's KRaft startup can lag behind
    // the log line we wait on, so retry rather than fail on the first refused probe.
    let mut last_err = None;
    for _ in 0..30 {
        match consumer.fetch_metadata(None, Duration::from_secs(5)) {
            Ok(_) => return Ok(()),
            Err(e) => {
                last_err = Some(e);
                std::thread::sleep(Duration::from_secs(1));
            }
        }
    }

    Err(anyhow::anyhow!(
        "{} rdkafka connection probe failed: {}",
        name,
        last_err.expect("loop runs at least once")
    ))
}
