//! External compatibility oracle tests.
//!
//! These tests drive heimq through client libraries that are completely
//! independent of rdkafka/librdkafka, providing a "who watches the watcher"
//! check that our hand-rolled contract tests cannot give.
//!
//! Current oracles:
//!   - franz-go (pure-Go Kafka client, no librdkafka dependency)
//!   - sarama (IBM/sarama pure-Go Kafka client, independent of franz-go)
//!   - java kafka-clients (Apache reference implementation)
//!
//! Tests are skipped when the required runtime (go, java, mvn, …) is absent
//! from PATH, so they never break a developer's environment. In CI the
//! runtimes are assumed present.

use heimq::test_support::TestServer;
use std::path::PathBuf;
use std::process::Command;

fn go_available() -> bool {
    Command::new("go")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn franz_go_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("compat")
        .join("franz_go")
}

fn sarama_oracle_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("compat")
        .join("sarama_oracle")
}

fn java_oracle_jar() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("compat")
        .join("java_oracle")
        .join("target")
        .join("kafka-oracle-1.0-SNAPSHOT.jar")
}

fn java_available() -> bool {
    Command::new("java")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn mvn_available() -> bool {
    Command::new("mvn")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Run the franz-go compat binary against a live heimq instance.
///
/// Exercises: CreateTopics (admin), Produce (sync), JoinGroup, SyncGroup,
/// Heartbeat, Fetch, OffsetCommit — through franz-go's independent wire
/// implementation. A pass here means heimq speaks correct Kafka to at least
/// two unrelated client stacks.
#[test]
fn test_franz_go_produce_consume_consumer_group() {
    if !go_available() {
        eprintln!("SKIP: go not in PATH");
        return;
    }

    let server = TestServer::start();
    let dir = franz_go_dir();

    let out = Command::new("go")
        .args(["run", "."])
        .arg(server.bootstrap_servers())
        .current_dir(&dir)
        .output()
        .expect("failed to spawn go run");

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    print!("{stdout}");
    if !stderr.is_empty() {
        eprint!("{stderr}");
    }

    assert!(
        out.status.success(),
        "franz-go compat oracle failed (exit {:?})\nstdout: {stdout}\nstderr: {stderr}",
        out.status.code()
    );
}

/// Run the sarama oracle against a live heimq instance.
///
/// Exercises: Produce (SyncProducer), JoinGroup, SyncGroup, Heartbeat,
/// Fetch, OffsetCommit — through IBM/sarama's independent Go wire
/// implementation. Together with rdkafka and franz-go, this gives three
/// independent client stacks all verifying the same heimq wire behaviour.
#[test]
fn test_sarama_produce_consume_consumer_group() {
    if !go_available() {
        eprintln!("SKIP: go not in PATH");
        return;
    }

    let topic = "sarama-compat-topic";
    let server = TestServer::start();
    let dir = sarama_oracle_dir();

    let out = Command::new("go")
        .args(["run", "."])
        .arg(server.bootstrap_servers())
        .arg(topic)
        .current_dir(&dir)
        .output()
        .expect("failed to spawn go run");

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    print!("{stdout}");
    if !stderr.is_empty() {
        eprint!("{stderr}");
    }

    assert!(
        out.status.success(),
        "sarama oracle failed (exit {:?})\nstdout: {stdout}\nstderr: {stderr}",
        out.status.code()
    );
}

/// Run the official Apache Kafka Java client (kafka-clients) oracle against heimq.
///
/// Exercises: Produce (synchronous), JoinGroup/SyncGroup/Heartbeat (consumer
/// group), Fetch, OffsetCommit, and record headers — through the reference
/// Java implementation. This is the fourth independent client stack alongside
/// rdkafka (C), franz-go (Go), and sarama (Go).
///
/// The JAR is built at test time via `mvn package` if not already present.
#[test]
fn test_java_kafka_clients_produce_consume() {
    if !java_available() {
        eprintln!("SKIP: java not in PATH");
        return;
    }

    let jar = java_oracle_jar();

    // Build (or rebuild) the JAR. We always invoke mvn so that source changes
    // are picked up without manual intervention; mvn's incremental compilation
    // makes this cheap when nothing changed.
    if !mvn_available() {
        if !jar.exists() {
            eprintln!("SKIP: mvn not in PATH and kafka-oracle jar not pre-built");
            return;
        }
        // JAR exists but mvn is absent — run with the stale JAR and hope it's close enough.
    } else {
        let jar_dir = java_oracle_jar().parent().unwrap().parent().unwrap().to_path_buf();
        let build_out = Command::new("mvn")
            .args(["-q", "package", "-DskipTests"])
            .current_dir(&jar_dir)
            .output()
            .expect("mvn package failed to spawn");
        assert!(
            build_out.status.success(),
            "mvn package failed:\n{}",
            String::from_utf8_lossy(&build_out.stderr)
        );
    }

    let topic = "java-compat-topic";
    let server = TestServer::start();

    let out = Command::new("java")
        .args(["-jar"])
        .arg(&jar)
        .arg(server.bootstrap_servers())
        .arg(topic)
        .output()
        .expect("failed to spawn java oracle");

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    print!("{stdout}");
    if !stderr.is_empty() {
        eprint!("{stderr}");
    }

    assert!(
        out.status.success(),
        "java kafka-clients oracle failed (exit {:?})\nstdout: {stdout}\nstderr: {stderr}",
        out.status.code()
    );
}
