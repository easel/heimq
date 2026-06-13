//! External compatibility oracle tests.
//!
//! These tests drive heimq through client libraries that are completely
//! independent of rdkafka/librdkafka, providing a "who watches the watcher"
//! check that our hand-rolled contract tests cannot give.
//!
//! Current oracles:
//!   - franz-go (pure-Go Kafka client, no librdkafka dependency)
//!
//! Tests are skipped when the required runtime (go, java, …) is absent from
//! PATH, so they never break a developer's environment. In CI the runtimes are
//! assumed present.

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
