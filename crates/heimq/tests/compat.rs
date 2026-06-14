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
//!   - kcat (CLI tool; tests offset-based Fetch and consumer group via librdkafka)
//!   - KafkaJS (pure-JavaScript Kafka client, implements protocol from scratch)
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

fn kcat_available() -> bool {
    Command::new("kcat")
        .arg("-V")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Run kcat (CLI Kafka tool) against a live heimq instance.
///
/// kcat uses librdkafka internally. Tests:
///   1. Produce via `kcat -P` (line-delimited stdin)
///   2. Consume via raw offset (`-o beginning -c N -e`) — no group protocol
///   3. Key-value round-trip (`-P -K:` / `-C -K:`)
///   4. Metadata listing (`kcat -L`) — verifies topic appears in metadata
///   5. Consumer group consume (`kcat -G`) — drives JoinGroup/SyncGroup/Heartbeat
///   6. Consumer group offset resume — OffsetCommit + OffsetFetch round-trip
#[test]
fn test_kcat_produce_consume_roundtrip() {
    if !kcat_available() {
        eprintln!("SKIP: kcat not in PATH");
        return;
    }

    let server = TestServer::start();
    let bootstrap = server.bootstrap_servers();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let topic = format!("kcat-compat-{ts}");
    let kv_topic = format!("kcat-kv-{ts}");

    // --- 1. Plain produce (one message per line) ---
    let messages = ["alpha", "beta", "gamma", "delta", "epsilon"];
    let input = messages.join("\n");

    let produce_out = Command::new("kcat")
        .args(["-b", &bootstrap, "-t", &topic, "-P"])
        .env("KCAT_SKIP_BOOTSTRAP_LOG", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write as _;
            child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
            child.wait_with_output()
        })
        .expect("failed to spawn kcat -P");

    assert!(
        produce_out.status.success(),
        "kcat -P failed: {}",
        String::from_utf8_lossy(&produce_out.stderr)
    );

    // --- 2. Offset-based consume (no consumer group) ---
    let consume_out = Command::new("kcat")
        .args([
            "-b", &bootstrap,
            "-t", &topic,
            "-C",
            "-o", "beginning",
            "-c", &messages.len().to_string(),
            "-e",
            "-q",
        ])
        .output()
        .expect("failed to spawn kcat -C");

    assert!(
        consume_out.status.success(),
        "kcat -C failed: {}",
        String::from_utf8_lossy(&consume_out.stderr)
    );

    let consumed = String::from_utf8_lossy(&consume_out.stdout);
    let consumed_lines: Vec<&str> = consumed.trim_end_matches('\n').split('\n').collect();
    assert_eq!(
        consumed_lines, messages,
        "kcat offset-consume: messages don't match produced\nconsumed: {consumed:?}"
    );

    // --- 3. Key-value round-trip ---
    let kv_messages = ["k1:v1", "k2:v2", "k3:v3"];
    let kv_input = kv_messages.join("\n");

    Command::new("kcat")
        .args(["-b", &bootstrap, "-t", &kv_topic, "-P", "-K", ":"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write as _;
            child.stdin.take().unwrap().write_all(kv_input.as_bytes()).unwrap();
            child.wait_with_output()
        })
        .expect("failed to spawn kcat -P -K:");

    let kv_consume = Command::new("kcat")
        .args([
            "-b", &bootstrap,
            "-t", &kv_topic,
            "-C",
            "-o", "beginning",
            "-c", &kv_messages.len().to_string(),
            "-e",
            "-q",
            "-f", "%k:%s\n",
        ])
        .output()
        .expect("failed to spawn kcat -C -f %k:%s");

    assert!(
        kv_consume.status.success(),
        "kcat kv consume failed: {}",
        String::from_utf8_lossy(&kv_consume.stderr)
    );

    let kv_consumed = String::from_utf8_lossy(&kv_consume.stdout);
    let kv_lines: Vec<&str> = kv_consumed.trim_end_matches('\n').split('\n').collect();
    assert_eq!(
        kv_lines, kv_messages,
        "kcat kv round-trip: messages don't match\nconsumed: {kv_consumed:?}"
    );

    // --- 4. Metadata listing: topic appears in kcat -L output ---
    let meta_out = Command::new("kcat")
        .args(["-b", &bootstrap, "-L"])
        .output()
        .expect("failed to spawn kcat -L");

    assert!(
        meta_out.status.success(),
        "kcat -L failed: {}",
        String::from_utf8_lossy(&meta_out.stderr)
    );

    let meta_str = String::from_utf8_lossy(&meta_out.stdout);
    assert!(
        meta_str.contains(&topic),
        "kcat -L: produced topic {topic:?} not found in metadata output:\n{meta_str}"
    );

    // --- 5. Consumer group consume: kcat -G drives JoinGroup/SyncGroup/Heartbeat ---
    let cg_topic = format!("kcat-cg-{ts}");
    let cg_group = format!("kcat-cg-grp-{ts}");
    let cg_messages = ["msg-0", "msg-1", "msg-2", "msg-3", "msg-4"];
    let cg_input = cg_messages.join("\n");

    Command::new("kcat")
        .args(["-b", &bootstrap, "-t", &cg_topic, "-P"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write as _;
            child.stdin.take().unwrap().write_all(cg_input.as_bytes()).unwrap();
            child.wait_with_output()
        })
        .expect("failed to spawn kcat -P for cg topic");

    let cg_out = Command::new("kcat")
        .args([
            "-b", &bootstrap,
            "-G", &cg_group,
            &cg_topic,
            "-c", &cg_messages.len().to_string(),
            "-e",
            "-q",
            "-X", "auto.offset.reset=earliest",
        ])
        .output()
        .expect("failed to spawn kcat -G");

    assert!(
        cg_out.status.success(),
        "kcat -G failed: {}",
        String::from_utf8_lossy(&cg_out.stderr)
    );

    let cg_consumed = String::from_utf8_lossy(&cg_out.stdout);
    let mut cg_lines: Vec<&str> = cg_consumed
        .trim_end_matches('\n')
        .split('\n')
        .filter(|l| !l.is_empty())
        .collect();
    cg_lines.sort_unstable();
    let mut expected_sorted = cg_messages.to_vec();
    expected_sorted.sort_unstable();
    assert_eq!(
        cg_lines, expected_sorted,
        "kcat -G: consumed messages don't match produced\nconsumed: {cg_consumed:?}"
    );

    // --- 6. Consumer group offset resume via kcat -G ---
    // Produce 6 messages, consume first 3 with group A (auto-commit), then
    // start a fresh kcat -G with the same group and verify it resumes from
    // offset 3 (gets the remaining 3 messages, not all 6).
    let resume_topic = format!("kcat-resume-{ts}");
    let resume_group = format!("kcat-resume-grp-{ts}");
    let resume_messages = ["r0", "r1", "r2", "r3", "r4", "r5"];
    let resume_input = resume_messages.join("\n");

    Command::new("kcat")
        .args(["-b", &bootstrap, "-t", &resume_topic, "-P"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write as _;
            child.stdin.take().unwrap().write_all(resume_input.as_bytes()).unwrap();
            child.wait_with_output()
        })
        .expect("failed to spawn kcat -P for resume topic");

    // Session A: consume exactly 3, then exit (kcat auto-commits on clean exit).
    let session_a = Command::new("kcat")
        .args([
            "-b", &bootstrap,
            "-G", &resume_group,
            &resume_topic,
            "-c", "3",
            "-e",
            "-q",
            "-X", "auto.offset.reset=earliest",
        ])
        .output()
        .expect("failed to spawn kcat -G session A");

    assert!(
        session_a.status.success(),
        "kcat -G session A failed: {}",
        String::from_utf8_lossy(&session_a.stderr)
    );
    let session_a_out = String::from_utf8_lossy(&session_a.stdout);
    let session_a_lines: Vec<&str> = session_a_out
        .trim_end_matches('\n')
        .split('\n')
        .filter(|l| !l.is_empty())
        .collect();
    assert_eq!(session_a_lines.len(), 3, "kcat session A: expected 3 messages");

    // Session B: same group — should resume from committed offset and get only the remaining 3.
    let session_b = Command::new("kcat")
        .args([
            "-b", &bootstrap,
            "-G", &resume_group,
            &resume_topic,
            "-c", "3",
            "-e",
            "-q",
            "-X", "auto.offset.reset=earliest",
        ])
        .output()
        .expect("failed to spawn kcat -G session B");

    assert!(
        session_b.status.success(),
        "kcat -G session B failed: {}",
        String::from_utf8_lossy(&session_b.stderr)
    );
    let session_b_out = String::from_utf8_lossy(&session_b.stdout);
    let session_b_lines: Vec<&str> = session_b_out
        .trim_end_matches('\n')
        .split('\n')
        .filter(|l| !l.is_empty())
        .collect();
    assert_eq!(
        session_b_lines.len(), 3,
        "kcat session B: expected 3 messages after offset resume, got: {session_b_out:?}"
    );
    // The resumed messages must be the SECOND half, not the first.
    let session_b_set: std::collections::HashSet<&str> = session_b_lines.iter().copied().collect();
    for first_half in ["r0", "r1", "r2"] {
        assert!(
            !session_b_set.contains(first_half),
            "kcat session B: got already-consumed message {first_half:?}; offset resume not working"
        );
    }
}

fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn kafkajs_oracle_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("compat")
        .join("kafkajs_oracle")
}

fn kafkajs_installed() -> bool {
    kafkajs_oracle_dir()
        .join("node_modules")
        .join("kafkajs")
        .exists()
}

/// Run the KafkaJS oracle against a live heimq instance.
///
/// KafkaJS is a pure-JavaScript Kafka client that implements the protocol
/// from scratch — no librdkafka, no Go runtime, no JVM. It provides a 6th
/// independent client implementation alongside rdkafka (C), franz-go (Go),
/// sarama (Go), the Java reference client, and kcat. Tests: create-topic,
/// produce, consume-via-group, produce-with-headers, consume-headers-roundtrip,
/// fetch-topic-offsets, describe-groups, delete-groups, delete-topic.
#[test]
fn test_kafkajs_produce_consume_consumer_group() {
    if !node_available() {
        eprintln!("SKIP: node not in PATH");
        return;
    }
    if !kafkajs_installed() {
        eprintln!("SKIP: kafkajs not installed (run `npm install` in tests/compat/kafkajs_oracle/)");
        return;
    }

    let server = TestServer::start();
    let dir = kafkajs_oracle_dir();

    let out = Command::new("node")
        .arg("main.js")
        .arg(server.bootstrap_servers())
        .current_dir(&dir)
        .output()
        .expect("failed to spawn node kafkajs oracle");

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    print!("{stdout}");
    let non_log_stderr: Vec<&str> = stderr
        .lines()
        .filter(|l| !l.trim_start().starts_with('{'))
        .collect();
    if !non_log_stderr.is_empty() {
        eprint!("{}", non_log_stderr.join("\n"));
    }

    assert!(
        out.status.success(),
        "kafkajs oracle failed (exit {:?})\nstdout: {stdout}\nstderr: {stderr}",
        out.status.code()
    );
}
