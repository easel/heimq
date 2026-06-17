//! Helm fixed-memory end-to-end evidence harness.
//!
//! This test is intentionally ignored by default because it targets a broker
//! already deployed by `scripts/helm-memory-e2e.sh`.

use anyhow::{anyhow, bail, Context, Result};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use heimq::protocol::is_flexible;
use kafka_protocol::messages::incremental_alter_configs_request::{
    AlterConfigsResource, AlterableConfig, IncrementalAlterConfigsRequest,
};
use kafka_protocol::messages::incremental_alter_configs_response::IncrementalAlterConfigsResponse;
use kafka_protocol::protocol::{Decodable, Encodable, StrBytes};
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::client::DefaultClientContext;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::error::{KafkaError, RDKafkaErrorCode};
use rdkafka::message::Message;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::topic_partition_list::TopicPartitionList;
use rdkafka::util::Timeout;
use rdkafka::{ClientConfig, Offset};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const DEFAULT_BOOTSTRAP: &str = "127.0.0.1:9092";
const DEFAULT_METRICS: &str = "127.0.0.1:9093";
const MAX_MEMORY_BYTES: u64 = 8 * 1024 * 1024;
const STORAGE_FULL_CODE: i32 = 56;

#[test]
#[ignore]
fn helm_memory_e2e_requires_bootstrap_or_runs_when_bootstrap_set() {
    if std::env::var("HEIMQ_E2E_BOOTSTRAP").is_err() {
        eprintln!(
            "HEIMQ_E2E_BOOTSTRAP is not set; set it to 127.0.0.1:9092 after Helm port-forwarding to run the full suite"
        );
        return;
    }

    if let Err(err) = run() {
        panic!("helm fixed-memory e2e failed: {err:#}");
    }
}

fn run() -> Result<()> {
    let ctx = TestContext::from_env()?;
    match ctx.scenario.as_deref() {
        Some("A") => run_scenario_a(&ctx)?,
        Some("A2") => run_scenario_a2(&ctx)?,
        Some("B") => run_scenario_b(&ctx)?,
        Some("C") => run_scenario_c(&ctx)?,
        Some("D") => run_scenario_d(&ctx)?,
        Some("E") => run_scenario_e(&ctx)?,
        Some(other) => bail!("unknown HEIMQ_E2E_SCENARIO={other}"),
        None => {
            run_scenario_a(&ctx)?;
            run_scenario_a2(&ctx)?;
            run_scenario_b(&ctx)?;
            run_scenario_c(&ctx)?;
            run_scenario_d(&ctx)?;
            run_scenario_e(&ctx)?;
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct TestContext {
    bootstrap: String,
    metrics_addr: String,
    artifact_dir: PathBuf,
    scenario: Option<String>,
    run_id: String,
}

impl TestContext {
    fn from_env() -> Result<Self> {
        let bootstrap =
            std::env::var("HEIMQ_E2E_BOOTSTRAP").unwrap_or_else(|_| DEFAULT_BOOTSTRAP.into());
        let metrics_addr =
            std::env::var("HEIMQ_E2E_METRICS").unwrap_or_else(|_| DEFAULT_METRICS.into());
        let artifact_dir = std::env::var("HEIMQ_E2E_ARTIFACT_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("target/helm-memory-e2e/manual"));
        fs::create_dir_all(&artifact_dir)
            .with_context(|| format!("create artifact dir {}", artifact_dir.display()))?;
        let run_id = now_millis().to_string();
        Ok(Self {
            bootstrap,
            metrics_addr,
            artifact_dir,
            scenario: std::env::var("HEIMQ_E2E_SCENARIO").ok(),
            run_id,
        })
    }

    fn scenario_path(&self, scenario: &str) -> PathBuf {
        self.artifact_dir.join(format!("{scenario}.json"))
    }
}

#[derive(Clone, Debug)]
struct TopicSpec {
    name: String,
    partitions: i32,
    retention_ms: Option<u64>,
    retention_bytes: Option<u64>,
}

#[derive(Clone, Debug)]
struct RecordId {
    scenario: String,
    topic: String,
    partition: i32,
    sequence: u64,
}

impl RecordId {
    fn encode(&self) -> String {
        format!(
            "{}|{}|{}|{}",
            self.scenario, self.topic, self.partition, self.sequence
        )
    }

    fn decode(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('|').collect();
        if parts.len() != 4 {
            return None;
        }
        Some(Self {
            scenario: parts[0].to_string(),
            topic: parts[1].to_string(),
            partition: parts[2].parse().ok()?,
            sequence: parts[3].parse().ok()?,
        })
    }
}

#[derive(Clone, Debug)]
struct ProducedRecord {
    id: String,
    topic: String,
    partition: i32,
    sequence: u64,
    bytes: usize,
}

#[derive(Clone, Debug)]
struct ProduceStats {
    attempted: usize,
    acked: Vec<ProducedRecord>,
    errors: BTreeMap<i32, usize>,
    first_storage_full_attempt: Option<usize>,
    post_storage_full_errors: usize,
    elapsed: Duration,
}

fn run_scenario_a(ctx: &TestContext) -> Result<()> {
    let cases = [
        ("A-1-topic", 1usize, 4i32, 800usize, 512usize),
        ("A-10-topics", 10, 4, 160, 512),
        ("A-100-topics", 100, 2, 20, 512),
    ];
    let selected_case = std::env::var("HEIMQ_E2E_A_CASE").ok();
    let mut summaries = Vec::new();
    for (case, topic_count, partitions, records_per_topic, payload_bytes) in cases {
        if selected_case
            .as_deref()
            .is_some_and(|selected| selected != case)
        {
            continue;
        }
        let topics = numbered_topics(case, topic_count, partitions, None, None);
        create_topics(ctx, &topics)?;
        let before = scrape_metrics(ctx).unwrap_or_default();
        let stats = produce_matrix(
            ctx,
            case,
            &topics,
            records_per_topic,
            payload_bytes,
            ProducerProfile::Correctness,
        )?;
        let consumed = consume_records(
            ctx,
            &format!("group-{case}-{}", ctx.run_id),
            &topics,
            OffsetMode::Beginning,
            stats.acked.len(),
            Duration::from_secs(30),
        )?;
        let after = scrape_metrics(ctx).unwrap_or_default();
        let validation = validate_exact(&stats.acked, &consumed);
        let partition_balance = validate_partition_balance(&stats.acked, &topics);
        ensure!(
            validation.missing == 0 && validation.duplicates == 0,
            "{case}: consumed set mismatch: {:?}",
            validation
        );
        ensure!(
            partition_balance.ok,
            "{case}: partition balance failed: {:?}",
            partition_balance
        );
        summaries.push(format!(
            "{{\"case\":\"{}\",\"topics\":{},\"partitions_per_topic\":{},\"attempted\":{},\"acked\":{},\"errors\":{},\"first_storage_full_attempt\":{},\"post_storage_full_errors\":{},\"consumed\":{},\"missing\":{},\"duplicates\":{},\"non_empty_partition_ratio\":{:.4},\"min_partition_records\":{},\"max_partition_records\":{},\"metrics_before\":{},\"metrics_after\":{}}}",
            json_escape(case),
            topic_count,
            partitions,
            stats.attempted,
            stats.acked.len(),
            map_usize_json(&stats.errors),
            option_usize_json(stats.first_storage_full_attempt),
            stats.post_storage_full_errors,
            consumed.len(),
            validation.missing,
            validation.duplicates,
            partition_balance.non_empty_ratio,
            partition_balance.min_count,
            partition_balance.max_count,
            metrics_json(&before),
            metrics_json(&after)
        ));
    }
    ensure!(!summaries.is_empty(), "A: no case matched HEIMQ_E2E_A_CASE");
    let artifact_name = selected_case
        .as_deref()
        .map(|case| format!("scenario-a-{}", case.to_ascii_lowercase()))
        .unwrap_or_else(|| "scenario-a".to_string());
    write_artifact(
        &ctx.scenario_path(&artifact_name),
        &format!(
            "{{\"scenario\":\"A\",\"description\":\"load balanced across 1,10,100 topics\",\"accepted\":true,\"cases\":[{}]}}\n",
            summaries.join(",")
        ),
    )
}

fn run_scenario_a2(ctx: &TestContext) -> Result<()> {
    let topics = numbered_topics("A2-throughput", 10, 4, None, None);
    create_topics(ctx, &topics)?;
    let before = scrape_metrics(ctx).unwrap_or_default();
    let stats = produce_matrix(ctx, "A2", &topics, 400, 512, ProducerProfile::Throughput)?;
    let consumed = consume_records(
        ctx,
        &format!("group-A2-{}", ctx.run_id),
        &topics,
        OffsetMode::Beginning,
        stats.acked.len(),
        Duration::from_secs(45),
    )?;
    let after = scrape_metrics(ctx).unwrap_or_default();
    let validation = validate_exact(&stats.acked, &consumed);
    ensure!(
        validation.missing == 0 && validation.duplicates == 0,
        "A2: consumed set mismatch: {:?}",
        validation
    );
    let records_per_sec = stats.acked.len() as f64 / stats.elapsed.as_secs_f64();
    let mib_per_sec = (stats.acked.iter().map(|r| r.bytes).sum::<usize>() as f64 / 1_048_576.0)
        / stats.elapsed.as_secs_f64();
    ensure!(
        records_per_sec >= 1000.0 && mib_per_sec >= 0.49,
        "A2: throughput below threshold: {records_per_sec:.2} records/s, {mib_per_sec:.3} MiB/s"
    );
    write_artifact(
        &ctx.scenario_path("scenario-a2"),
        &format!(
            "{{\"scenario\":\"A2\",\"description\":\"port-forward throughput sample\",\"accepted\":true,\"attempted\":{},\"acked\":{},\"errors\":{},\"first_storage_full_attempt\":{},\"post_storage_full_errors\":{},\"consumed\":{},\"missing\":{},\"duplicates\":{},\"elapsed_ms\":{},\"records_per_sec\":{:.4},\"mib_per_sec\":{:.6},\"metrics_before\":{},\"metrics_after\":{}}}\n",
            stats.attempted,
            stats.acked.len(),
            map_usize_json(&stats.errors),
            option_usize_json(stats.first_storage_full_attempt),
            stats.post_storage_full_errors,
            consumed.len(),
            validation.missing,
            validation.duplicates,
            stats.elapsed.as_millis(),
            records_per_sec,
            mib_per_sec,
            metrics_json(&before),
            metrics_json(&after)
        ),
    )
}

fn run_scenario_b(ctx: &TestContext) -> Result<()> {
    let topics = numbered_topics("B-plateau", 10, 2, Some(3000), None);
    create_topics(ctx, &topics)?;
    let started = Instant::now();
    let before = scrape_metrics(ctx).unwrap_or_default();
    let producer = build_producer(ctx, ProducerProfile::Correctness)?;
    let mut produced = 0usize;
    let mut errors = BTreeMap::new();
    let mut samples = Vec::new();
    let mut next_sample_at = started;
    while started.elapsed() < Duration::from_secs(20) {
        let tick = Instant::now();
        for n in 0..50usize {
            let topic_index = (produced + n) % topics.len();
            let topic = &topics[topic_index];
            let partition = ((produced + n) as i32) % topic.partitions;
            let id = RecordId {
                scenario: "B".into(),
                topic: topic.name.clone(),
                partition,
                sequence: produced as u64,
            };
            match send_record(&producer, &topic.name, partition, &id.encode(), 512) {
                Ok(_) => produced += 1,
                Err(code) => *errors.entry(code).or_insert(0) += 1,
            }
        }
        if Instant::now() >= next_sample_at {
            samples.push(scrape_metrics(ctx).unwrap_or_default());
            next_sample_at += Duration::from_secs(1);
        }
        let elapsed = tick.elapsed();
        if elapsed < Duration::from_millis(100) {
            std::thread::sleep(Duration::from_millis(100) - elapsed);
        }
    }
    let after = scrape_metrics(ctx).unwrap_or_default();
    let retained: Vec<f64> = samples
        .iter()
        .map(|m| *m.get("heimq_memory_log_bytes").unwrap_or(&0.0))
        .collect();
    ensure!(!retained.is_empty(), "B: no metrics samples were captured");
    let first = mean_window(&retained, 5, 5);
    let last = mean_tail(&retained, 5);
    ensure!(
        last <= first * 1.15 + 1.0,
        "B: retained bytes did not plateau: first={first} last={last}"
    );
    ensure!(
        after
            .get("heimq_memory_log_bytes")
            .copied()
            .unwrap_or_default()
            <= MAX_MEMORY_BYTES as f64,
        "B: retained bytes exceeded fixed memory budget"
    );
    ensure!(
        metric_delta(&before, &after, "heimq_storage_full_errors_total") == 0.0,
        "B: unexpected storage-full errors"
    );
    write_artifact(
        &ctx.scenario_path("scenario-b"),
        &format!(
            "{{\"scenario\":\"B\",\"description\":\"memory plateau under retention.ms\",\"accepted\":true,\"produced\":{},\"errors\":{},\"samples\":{},\"first_post_warmup_mean_bytes\":{:.4},\"last_mean_bytes\":{:.4},\"retained_bytes_final\":{},\"retention_reclaimed_delta\":{},\"storage_full_delta\":{},\"metrics_before\":{},\"metrics_after\":{}}}\n",
            produced,
            map_usize_json(&errors),
            retained.len(),
            first,
            last,
            after.get("heimq_memory_log_bytes").copied().unwrap_or_default(),
            metric_delta_sum(&before, &after, "heimq_retention_reclaimed_bytes_total"),
            metric_delta(&before, &after, "heimq_storage_full_errors_total"),
            metrics_json(&before),
            metrics_json(&after)
        ),
    )
}

fn run_scenario_c(ctx: &TestContext) -> Result<()> {
    let topics = numbered_topics("C-boundary", 3, 2, Some(5000), None);
    create_topics(ctx, &topics)?;
    let before = scrape_metrics(ctx).unwrap_or_default();
    let stats = produce_matrix(ctx, "C", &topics, 100, 512, ProducerProfile::Correctness)?;
    let immediate = consume_records(
        ctx,
        &format!("group-C-immediate-{}", ctx.run_id),
        &topics,
        OffsetMode::Beginning,
        stats.acked.len(),
        Duration::from_secs(2),
    )?;
    let immediate_validation = validate_exact(&stats.acked, &immediate);
    ensure!(
        immediate_validation.missing == 0 && immediate_validation.duplicates == 0,
        "C: immediate consume failed: {:?}",
        immediate_validation
    );
    let latest_before = watermarks(ctx, &topics)?;
    std::thread::sleep(Duration::from_millis(6500));
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut advanced = false;
    let mut final_watermarks = latest_before.clone();
    while Instant::now() < deadline {
        final_watermarks = watermarks(ctx, &topics)?;
        advanced = final_watermarks
            .iter()
            .any(|((topic, partition), (low, high))| {
                let before = latest_before
                    .get(&(topic.clone(), *partition))
                    .copied()
                    .unwrap_or((0, 0));
                *low > before.0 && *high == before.1
            });
        if advanced {
            break;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    ensure!(
        advanced,
        "C: earliest offsets did not advance after retention.ms boundary"
    );
    let after = scrape_metrics(ctx).unwrap_or_default();
    write_artifact(
        &ctx.scenario_path("scenario-c"),
        &format!(
            "{{\"scenario\":\"C\",\"description\":\"retention.ms boundary preserves immediate reads then advances earliest offsets without appends\",\"accepted\":true,\"acked\":{},\"immediate_consumed\":{},\"missing\":{},\"duplicates\":{},\"latest_offsets_unchanged\":true,\"watermarks_before\":{},\"watermarks_after\":{},\"retention_reclaimed_delta\":{},\"metrics_before\":{},\"metrics_after\":{}}}\n",
            stats.acked.len(),
            immediate.len(),
            immediate_validation.missing,
            immediate_validation.duplicates,
            watermarks_json(&latest_before),
            watermarks_json(&final_watermarks),
            metric_delta_sum(&before, &after, "heimq_retention_reclaimed_bytes_total"),
            metrics_json(&before),
            metrics_json(&after)
        ),
    )
}

fn run_scenario_d(ctx: &TestContext) -> Result<()> {
    let topics = numbered_topics("D-backpressure", 1, 1, Some(60000), None);
    create_topics(ctx, &topics)?;
    let before = scrape_metrics(ctx).unwrap_or_default();
    let producer = build_producer(ctx, ProducerProfile::Correctness)?;
    let topic = &topics[0];
    let mut acked = Vec::new();
    let mut errors = BTreeMap::new();
    let mut first_storage_full = None;
    let mut attempts_after_first_storage_full = 0usize;
    for attempt in 0..4096usize {
        let id = RecordId {
            scenario: "D".into(),
            topic: topic.name.clone(),
            partition: 0,
            sequence: attempt as u64,
        };
        match send_record(&producer, &topic.name, 0, &id.encode(), 4096) {
            Ok(bytes) => {
                if first_storage_full.is_none() {
                    acked.push(ProducedRecord {
                        id: id.encode(),
                        topic: topic.name.clone(),
                        partition: 0,
                        sequence: attempt as u64,
                        bytes,
                    });
                }
            }
            Err(code) => {
                *errors.entry(code).or_insert(0) += 1;
                if code == STORAGE_FULL_CODE {
                    if first_storage_full.is_none() {
                        first_storage_full = Some(attempt);
                    } else {
                        attempts_after_first_storage_full += 1;
                        if attempts_after_first_storage_full >= 16 {
                            break;
                        }
                    }
                }
            }
        }
        if attempt == 1536 {
            std::thread::sleep(Duration::from_millis(1500));
        }
    }
    let after = scrape_metrics(ctx).unwrap_or_default();
    let consumed = consume_records(
        ctx,
        &format!("group-D-{}", ctx.run_id),
        &topics,
        OffsetMode::Beginning,
        acked.len(),
        Duration::from_secs(30),
    )?;
    let validation = validate_exact(&acked, &consumed);
    let accepted_bytes: usize = acked.iter().map(|r| r.bytes).sum();
    let max_batch = acked.iter().map(|r| r.bytes).max().unwrap_or(0);
    ensure!(
        first_storage_full.is_some(),
        "D: expected storage-full backpressure"
    );
    ensure!(
        acked.len() >= 256,
        "D: too few accepted records before backpressure"
    );
    ensure!(
        attempts_after_first_storage_full >= 16,
        "D: did not verify 16 post-storage-full attempts"
    );
    ensure!(
        validation.missing == 0 && validation.duplicates == 0,
        "D: accepted records not consumable: {:?}",
        validation
    );
    ensure!(
        metric_delta_sum(&before, &after, "heimq_retention_reclaimed_bytes_total") == 0.0,
        "D: retention reclaimed bytes before protected data filled memory"
    );
    write_artifact(
        &ctx.scenario_path("scenario-d"),
        &format!(
            "{{\"scenario\":\"D\",\"description\":\"retention.ms protected records cause producer backpressure at memory cap\",\"accepted\":true,\"attempted\":{},\"acked_before_storage_full\":{},\"accepted_batch_bytes_total\":{},\"max_accepted_batch_bytes\":{},\"first_storage_full_attempt\":{},\"post_storage_full_errors\":{},\"errors\":{},\"consumed\":{},\"missing\":{},\"duplicates\":{},\"retained_bytes_final\":{},\"retention_reclaimed_delta\":{},\"metrics_before\":{},\"metrics_after\":{}}}\n",
            first_storage_full.unwrap_or(0) + attempts_after_first_storage_full + 1,
            acked.len(),
            accepted_bytes,
            max_batch,
            first_storage_full.unwrap_or(0),
            attempts_after_first_storage_full,
            map_usize_json(&errors),
            consumed.len(),
            validation.missing,
            validation.duplicates,
            after.get("heimq_memory_log_bytes").copied().unwrap_or_default(),
            metric_delta_sum(&before, &after, "heimq_retention_reclaimed_bytes_total"),
            metrics_json(&before),
            metrics_json(&after)
        ),
    )
}

fn run_scenario_e(ctx: &TestContext) -> Result<()> {
    let retention_bytes = 524_288u64;
    let topics = numbered_topics("E-retention-bytes", 10, 1, None, Some(retention_bytes));
    create_topics(ctx, &topics)?;
    let before = scrape_metrics(ctx).unwrap_or_default();
    let stats = produce_matrix(ctx, "E", &topics, 400, 2048, ProducerProfile::Correctness)?;
    ensure!(
        metric_delta(
            &before,
            &scrape_metrics(ctx).unwrap_or_default(),
            "heimq_storage_full_errors_total"
        ) == 0.0,
        "E: unexpected storage-full while retention.bytes should drop old records"
    );
    let marks = wait_for_retention_bytes_watermarks(ctx, &topics, 400, Duration::from_secs(5))?;
    for topic in &topics {
        let (low, high) = marks
            .get(&(topic.name.clone(), 0))
            .copied()
            .ok_or_else(|| anyhow!("missing watermark for {}", topic.name))?;
        ensure!(
            low > 0 && low < 400,
            "E: {} low watermark {low} outside expected range",
            topic.name
        );
        ensure!(
            high == 400,
            "E: {} high watermark {high} != 400",
            topic.name
        );
    }
    let retained_expected: HashSet<String> = stats
        .acked
        .iter()
        .filter(|record| {
            let (low, high) = marks
                .get(&(record.topic.clone(), record.partition))
                .copied()
                .unwrap_or((0, 0));
            (record.sequence as i64) >= low && (record.sequence as i64) < high
        })
        .map(|record| record.id.clone())
        .collect();
    let consumed = consume_records(
        ctx,
        &format!("group-E-{}", ctx.run_id),
        &topics,
        OffsetMode::Watermarks(marks.clone()),
        retained_expected.len(),
        Duration::from_secs(30),
    )?;
    let consumed_ids: HashSet<String> = consumed.iter().cloned().collect();
    let missing = retained_expected.difference(&consumed_ids).count();
    let duplicates = consumed.len().saturating_sub(consumed_ids.len());
    ensure!(
        missing == 0 && duplicates == 0,
        "E: retained suffix mismatch: missing={missing} duplicates={duplicates}"
    );
    let after = scrape_metrics(ctx).unwrap_or_default();
    ensure!(
        after
            .get("heimq_memory_log_bytes")
            .copied()
            .unwrap_or_default()
            <= (retention_bytes * topics.len() as u64) as f64,
        "E: retained bytes exceed per-topic retention.bytes aggregate cap"
    );
    write_artifact(
        &ctx.scenario_path("scenario-e"),
        &format!(
            "{{\"scenario\":\"E\",\"description\":\"retention.bytes drops excess records without producer backpressure\",\"accepted\":true,\"attempted\":{},\"acked\":{},\"errors\":{},\"first_storage_full_attempt\":{},\"post_storage_full_errors\":{},\"retained_expected\":{},\"consumed\":{},\"missing\":{},\"duplicates\":{},\"watermarks\":{},\"storage_full_delta\":{},\"retention_reclaimed_delta\":{},\"retained_bytes_final\":{},\"metrics_before\":{},\"metrics_after\":{}}}\n",
            stats.attempted,
            stats.acked.len(),
            map_usize_json(&stats.errors),
            option_usize_json(stats.first_storage_full_attempt),
            stats.post_storage_full_errors,
            retained_expected.len(),
            consumed.len(),
            missing,
            duplicates,
            watermarks_json(&marks),
            metric_delta(&before, &after, "heimq_storage_full_errors_total"),
            metric_delta_sum(&before, &after, "heimq_retention_reclaimed_bytes_total"),
            after.get("heimq_memory_log_bytes").copied().unwrap_or_default(),
            metrics_json(&before),
            metrics_json(&after)
        ),
    )
}

fn wait_for_retention_bytes_watermarks(
    ctx: &TestContext,
    topics: &[TopicSpec],
    expected_latest: i64,
    timeout: Duration,
) -> Result<BTreeMap<(String, i32), (i64, i64)>> {
    let deadline = Instant::now() + timeout;
    let mut latest_marks = watermarks(ctx, topics)?;
    loop {
        let settled = topics.iter().all(|topic| {
            latest_marks
                .get(&(topic.name.clone(), 0))
                .is_some_and(|(low, high)| {
                    *low > 0 && *low < expected_latest && *high == expected_latest
                })
        });
        if settled {
            return Ok(latest_marks);
        }
        if Instant::now() >= deadline {
            return Ok(latest_marks);
        }
        std::thread::sleep(Duration::from_millis(250));
        latest_marks = watermarks(ctx, topics)?;
    }
}

fn numbered_topics(
    prefix: &str,
    count: usize,
    partitions: i32,
    retention_ms: Option<u64>,
    retention_bytes: Option<u64>,
) -> Vec<TopicSpec> {
    (0..count)
        .map(|i| TopicSpec {
            name: format!("{}-{}-{}", prefix.to_ascii_lowercase(), now_millis(), i),
            partitions,
            retention_ms,
            retention_bytes,
        })
        .collect()
}

fn create_topics(ctx: &TestContext, topics: &[TopicSpec]) -> Result<()> {
    let admin: AdminClient<DefaultClientContext> = ClientConfig::new()
        .set("bootstrap.servers", &ctx.bootstrap)
        .set("socket.timeout.ms", "15000")
        .create()
        .context("create admin client")?;

    let mut new_topics = Vec::with_capacity(topics.len());
    for spec in topics {
        new_topics.push(NewTopic::new(
            &spec.name,
            spec.partitions,
            TopicReplication::Fixed(1),
        ));
    }
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let results = rt.block_on(admin.create_topics(
        &new_topics,
        &AdminOptions::new().request_timeout(Some(Duration::from_secs(15))),
    ))?;
    for result in results {
        match result {
            Ok(_) => {}
            Err((topic, code)) if code == RDKafkaErrorCode::TopicAlreadyExists => {
                eprintln!("topic already exists during e2e setup: {topic}");
            }
            Err((topic, code)) => bail!("create topic {topic} failed: {code:?}"),
        }
    }
    incremental_alter_topic_configs(ctx, topics)?;
    Ok(())
}

fn incremental_alter_topic_configs(ctx: &TestContext, topics: &[TopicSpec]) -> Result<()> {
    let mut resources = Vec::new();
    for spec in topics {
        let mut configs = Vec::new();
        if let Some(retention_ms) = spec.retention_ms {
            configs.push(("retention.ms", retention_ms.to_string()));
        }
        if let Some(retention_bytes) = spec.retention_bytes {
            configs.push(("retention.bytes", retention_bytes.to_string()));
        }
        if configs.is_empty() {
            continue;
        }

        let mut resource = AlterConfigsResource::default();
        resource.resource_type = 2; // TOPIC
        resource.resource_name = StrBytes::from_string(spec.name.clone());
        resource.configs = configs
            .into_iter()
            .map(|(key, value)| {
                let mut entry = AlterableConfig::default();
                entry.name = StrBytes::from_static_str(key);
                entry.value = Some(StrBytes::from_string(value));
                entry.config_operation = 0; // SET
                entry
            })
            .collect();
        resources.push(resource);
    }
    if resources.is_empty() {
        return Ok(());
    }

    let mut request = IncrementalAlterConfigsRequest::default();
    request.resources = resources;
    let response: IncrementalAlterConfigsResponse =
        send_kafka_request(&ctx.bootstrap, 44, 0, &request)?;
    for resource in response.responses {
        ensure!(
            resource.error_code == 0,
            "IncrementalAlterConfigs failed for {} with code {}",
            resource.resource_name,
            resource.error_code
        );
    }
    Ok(())
}

fn encode_kafka_request<R: Encodable>(
    api_key: i16,
    api_version: i16,
    correlation_id: i32,
    client_id: Option<&str>,
    request: &R,
) -> Result<Vec<u8>> {
    let mut body = BytesMut::new();
    body.put_i16(api_key);
    body.put_i16(api_version);
    body.put_i32(correlation_id);
    match client_id {
        Some(id) => {
            body.put_i16(id.len() as i16);
            body.put_slice(id.as_bytes());
        }
        None => body.put_i16(-1),
    }
    if is_flexible(api_key, api_version) {
        body.put_u8(0x00);
    }
    request.encode(&mut body, api_version)?;

    let mut framed = BytesMut::new();
    framed.put_i32(body.len() as i32);
    framed.extend_from_slice(&body);
    Ok(framed.to_vec())
}

fn send_kafka_request<R, T>(
    bootstrap: &str,
    api_key: i16,
    api_version: i16,
    request: &R,
) -> Result<T>
where
    R: Encodable,
    T: Decodable,
{
    let correlation_id = (now_millis() % i32::MAX as u128) as i32;
    let request = encode_kafka_request(
        api_key,
        api_version,
        correlation_id,
        Some("heimq-helm-memory-e2e"),
        request,
    )?;
    let mut stream = TcpStream::connect(bootstrap)
        .with_context(|| format!("connect Kafka bootstrap {bootstrap}"))?;
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;
    stream.write_all(&request)?;

    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf)?;
    let len = i32::from_be_bytes(len_buf) as usize;
    let mut response = vec![0u8; len];
    stream.read_exact(&mut response)?;
    let mut bytes = Bytes::from(response);
    let actual_correlation = bytes.get_i32();
    ensure!(
        actual_correlation == correlation_id,
        "correlation mismatch for API {api_key}: got {actual_correlation}, expected {correlation_id}"
    );
    if is_flexible(api_key, api_version) {
        let _tagged_fields = bytes.get_u8();
    }
    T::decode(&mut bytes, api_version).context("decode Kafka response")
}

#[derive(Copy, Clone)]
enum ProducerProfile {
    Correctness,
    Throughput,
}

fn build_producer(ctx: &TestContext, profile: ProducerProfile) -> Result<FutureProducer> {
    let mut config = ClientConfig::new();
    config
        .set("bootstrap.servers", &ctx.bootstrap)
        .set("acks", "all")
        .set("enable.idempotence", "false")
        .set("retries", "0")
        .set("socket.timeout.ms", "3000")
        .set("request.timeout.ms", "3000");
    match profile {
        ProducerProfile::Correctness => {
            config
                .set("message.timeout.ms", "3000")
                .set("linger.ms", "0")
                .set("batch.num.messages", "1")
                .set("queue.buffering.max.messages", "1000");
        }
        ProducerProfile::Throughput => {
            config
                .set("message.timeout.ms", "5000")
                .set("linger.ms", "5")
                .set("batch.num.messages", "100")
                .set("queue.buffering.max.messages", "10000");
        }
    }
    config.create().context("create producer")
}

fn produce_matrix(
    ctx: &TestContext,
    scenario: &str,
    topics: &[TopicSpec],
    records_per_topic: usize,
    payload_bytes: usize,
    profile: ProducerProfile,
) -> Result<ProduceStats> {
    if matches!(profile, ProducerProfile::Throughput) {
        return produce_matrix_batched(
            ctx,
            scenario,
            topics,
            records_per_topic,
            payload_bytes,
            profile,
        );
    }
    let producer = build_producer(ctx, profile)?;
    let started = Instant::now();
    let mut acked = Vec::new();
    let mut errors = BTreeMap::new();
    let mut first_storage_full_attempt = None;
    let mut post_storage_full_errors = 0usize;
    let mut attempted = 0usize;
    for spec in topics {
        for sequence in 0..records_per_topic {
            let partition = (sequence as i32) % spec.partitions;
            let id = RecordId {
                scenario: scenario.to_string(),
                topic: spec.name.clone(),
                partition,
                sequence: sequence as u64,
            };
            attempted += 1;
            match send_record(
                &producer,
                &spec.name,
                partition,
                &id.encode(),
                payload_bytes,
            ) {
                Ok(bytes) => acked.push(ProducedRecord {
                    id: id.encode(),
                    topic: spec.name.clone(),
                    partition,
                    sequence: sequence as u64,
                    bytes,
                }),
                Err(code) => {
                    *errors.entry(code).or_insert(0) += 1;
                    if code == STORAGE_FULL_CODE {
                        if first_storage_full_attempt.is_none() {
                            first_storage_full_attempt = Some(attempted);
                        } else {
                            post_storage_full_errors += 1;
                        }
                    }
                }
            }
        }
    }
    Ok(ProduceStats {
        attempted,
        acked,
        errors,
        first_storage_full_attempt,
        post_storage_full_errors,
        elapsed: started.elapsed(),
    })
}

#[derive(Debug)]
struct PreparedRecord {
    id: String,
    topic: String,
    partition: i32,
    sequence: u64,
    payload: String,
    bytes: usize,
}

fn produce_matrix_batched(
    ctx: &TestContext,
    scenario: &str,
    topics: &[TopicSpec],
    records_per_topic: usize,
    payload_bytes: usize,
    profile: ProducerProfile,
) -> Result<ProduceStats> {
    let producer = build_producer(ctx, profile)?;
    let mut records = Vec::with_capacity(topics.len() * records_per_topic);
    for spec in topics {
        for sequence in 0..records_per_topic {
            let partition = (sequence as i32) % spec.partitions;
            let id = RecordId {
                scenario: scenario.to_string(),
                topic: spec.name.clone(),
                partition,
                sequence: sequence as u64,
            }
            .encode();
            let payload = payload_for(&id, payload_bytes);
            records.push(PreparedRecord {
                bytes: id.len() + payload.len(),
                id,
                topic: spec.name.clone(),
                partition,
                sequence: sequence as u64,
                payload,
            });
        }
    }

    let started = Instant::now();
    let mut deliveries = Vec::with_capacity(records.len());
    let mut errors = BTreeMap::new();
    let mut first_storage_full_attempt = None;
    let mut post_storage_full_errors = 0usize;
    for (index, record) in records.iter().enumerate() {
        let future_record = FutureRecord::to(&record.topic)
            .partition(record.partition)
            .key(&record.id)
            .payload(&record.payload);
        match producer.send_result(future_record) {
            Ok(delivery) => deliveries.push((index, delivery)),
            Err((err, _)) => {
                let code = kafka_error_code(&err);
                *errors.entry(code).or_insert(0) += 1;
                if code == STORAGE_FULL_CODE {
                    if first_storage_full_attempt.is_none() {
                        first_storage_full_attempt = Some(index + 1);
                    } else {
                        post_storage_full_errors += 1;
                    }
                }
            }
        }
    }

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let mut acked = Vec::with_capacity(deliveries.len());
    for (index, delivery) in deliveries {
        match rt.block_on(delivery) {
            Ok(Ok(_)) => {
                let record = &records[index];
                acked.push(ProducedRecord {
                    id: record.id.clone(),
                    topic: record.topic.clone(),
                    partition: record.partition,
                    sequence: record.sequence,
                    bytes: record.bytes,
                });
            }
            Ok(Err((err, _))) => {
                let code = kafka_error_code(&err);
                *errors.entry(code).or_insert(0) += 1;
                if code == STORAGE_FULL_CODE {
                    if first_storage_full_attempt.is_none() {
                        first_storage_full_attempt = Some(index + 1);
                    } else {
                        post_storage_full_errors += 1;
                    }
                }
            }
            Err(_) => {
                *errors.entry(-2).or_insert(0) += 1;
            }
        }
    }
    Ok(ProduceStats {
        attempted: records.len(),
        acked,
        errors,
        first_storage_full_attempt,
        post_storage_full_errors,
        elapsed: started.elapsed(),
    })
}

fn send_record(
    producer: &FutureProducer,
    topic: &str,
    partition: i32,
    key: &str,
    payload_bytes: usize,
) -> std::result::Result<usize, i32> {
    let payload = payload_for(key, payload_bytes);
    let record = FutureRecord::to(topic)
        .partition(partition)
        .key(key)
        .payload(&payload);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| -1)?;
    rt.block_on(producer.send(record, Duration::from_secs(5)))
        .map(|_| key.len() + payload.len())
        .map_err(|(err, _)| kafka_error_code(&err))
}

fn payload_for(key: &str, payload_bytes: usize) -> String {
    if payload_bytes <= key.len() + 1 {
        return key[..payload_bytes.min(key.len())].to_string();
    }
    let filler = payload_bytes - key.len() - 1;
    format!("{key}:{}", "x".repeat(filler))
}

fn kafka_error_code(err: &KafkaError) -> i32 {
    match err {
        KafkaError::MessageProduction(code) => *code as i32,
        KafkaError::MessageConsumption(code) => *code as i32,
        KafkaError::MetadataFetch(code) => *code as i32,
        KafkaError::Global(code) => *code as i32,
        _ => -1,
    }
}

enum OffsetMode {
    Beginning,
    Watermarks(BTreeMap<(String, i32), (i64, i64)>),
}

fn consume_records(
    ctx: &TestContext,
    group_id: &str,
    topics: &[TopicSpec],
    mode: OffsetMode,
    expected: usize,
    timeout: Duration,
) -> Result<Vec<String>> {
    let consumer: BaseConsumer = ClientConfig::new()
        .set("bootstrap.servers", &ctx.bootstrap)
        .set("socket.timeout.ms", "3000")
        .set("group.id", group_id)
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "earliest")
        .create()
        .context("create consumer")?;
    let mut tpl = TopicPartitionList::new();
    for spec in topics {
        for partition in 0..spec.partitions {
            let offset = match &mode {
                OffsetMode::Beginning => Offset::Beginning,
                OffsetMode::Watermarks(marks) => {
                    let (low, _) = marks
                        .get(&(spec.name.clone(), partition))
                        .copied()
                        .ok_or_else(|| {
                            anyhow!("missing watermark for {}-{partition}", spec.name)
                        })?;
                    Offset::Offset(low)
                }
            };
            tpl.add_partition_offset(&spec.name, partition, offset)?;
        }
    }
    consumer
        .assign(&tpl)
        .context("assign consumer partitions")?;
    let deadline = Instant::now() + timeout;
    let mut consumed = Vec::new();
    let mut seen_offsets = HashSet::new();
    while consumed.len() < expected && Instant::now() < deadline {
        match consumer.poll(Duration::from_millis(200)) {
            Some(Ok(msg)) => {
                let key = msg
                    .key_view::<str>()
                    .and_then(|r| r.ok())
                    .map(str::to_string);
                if let Some(key) = key {
                    if RecordId::decode(&key).is_some() {
                        let marker = (msg.topic().to_string(), msg.partition(), msg.offset());
                        if seen_offsets.insert(marker) {
                            consumed.push(key);
                        }
                    }
                }
            }
            Some(Err(err)) => bail!("consumer error: {err}"),
            None => {}
        }
    }
    Ok(consumed)
}

fn watermarks(
    ctx: &TestContext,
    topics: &[TopicSpec],
) -> Result<BTreeMap<(String, i32), (i64, i64)>> {
    let consumer: BaseConsumer = ClientConfig::new()
        .set("bootstrap.servers", &ctx.bootstrap)
        .set("socket.timeout.ms", "3000")
        .set("group.id", format!("watermarks-{}", ctx.run_id))
        .create()
        .context("create watermark consumer")?;
    let mut out = BTreeMap::new();
    for spec in topics {
        for partition in 0..spec.partitions {
            let pair = consumer.fetch_watermarks(
                &spec.name,
                partition,
                Timeout::After(Duration::from_secs(5)),
            )?;
            out.insert((spec.name.clone(), partition), pair);
        }
    }
    Ok(out)
}

#[derive(Debug)]
struct ExactValidation {
    missing: usize,
    duplicates: usize,
}

fn validate_exact(produced: &[ProducedRecord], consumed: &[String]) -> ExactValidation {
    let expected: HashSet<String> = produced.iter().map(|r| r.id.clone()).collect();
    let actual: HashSet<String> = consumed.iter().cloned().collect();
    ExactValidation {
        missing: expected.difference(&actual).count(),
        duplicates: consumed.len().saturating_sub(actual.len()),
    }
}

#[derive(Debug)]
struct PartitionBalance {
    ok: bool,
    non_empty_ratio: f64,
    min_count: usize,
    max_count: usize,
}

fn validate_partition_balance(
    produced: &[ProducedRecord],
    topics: &[TopicSpec],
) -> PartitionBalance {
    let mut counts = BTreeMap::new();
    for spec in topics {
        for partition in 0..spec.partitions {
            counts.insert((spec.name.clone(), partition), 0usize);
        }
    }
    for record in produced {
        *counts
            .entry((record.topic.clone(), record.partition))
            .or_insert(0) += 1;
    }
    let non_empty = counts.values().filter(|&&n| n > 0).count();
    let min_count = *counts.values().min().unwrap_or(&0);
    let max_count = *counts.values().max().unwrap_or(&0);
    let non_empty_ratio = non_empty as f64 / counts.len().max(1) as f64;
    PartitionBalance {
        ok: non_empty_ratio >= 0.75 && max_count.saturating_sub(min_count) <= 1,
        non_empty_ratio,
        min_count,
        max_count,
    }
}

fn scrape_metrics(ctx: &TestContext) -> Result<BTreeMap<String, f64>> {
    let mut stream = TcpStream::connect(&ctx.metrics_addr)
        .with_context(|| format!("connect metrics endpoint {}", ctx.metrics_addr))?;
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    stream.write_all(b"GET /metrics HTTP/1.1\r\nHost: heimq\r\nConnection: close\r\n\r\n")?;
    let mut body = String::new();
    stream.read_to_string(&mut body)?;
    let body = body.split("\r\n\r\n").nth(1).unwrap_or(&body);
    let mut metrics = BTreeMap::new();
    for line in body.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(name) = parts.next() else { continue };
        let Some(value) = parts.next() else { continue };
        let metric_name = name.split('{').next().unwrap_or(name).to_string();
        let parsed = value.parse::<f64>().unwrap_or_default();
        *metrics.entry(metric_name).or_insert(0.0) += parsed;
    }
    Ok(metrics)
}

fn metric_delta(before: &BTreeMap<String, f64>, after: &BTreeMap<String, f64>, key: &str) -> f64 {
    after.get(key).copied().unwrap_or_default() - before.get(key).copied().unwrap_or_default()
}

fn metric_delta_sum(
    before: &BTreeMap<String, f64>,
    after: &BTreeMap<String, f64>,
    key: &str,
) -> f64 {
    metric_delta(before, after, key)
}

fn mean_window(values: &[f64], skip: usize, take: usize) -> f64 {
    let window: Vec<f64> = values.iter().skip(skip).take(take).copied().collect();
    if window.is_empty() {
        return values.first().copied().unwrap_or_default();
    }
    window.iter().sum::<f64>() / window.len() as f64
}

fn mean_tail(values: &[f64], take: usize) -> f64 {
    let start = values.len().saturating_sub(take);
    let window = &values[start..];
    if window.is_empty() {
        0.0
    } else {
        window.iter().sum::<f64>() / window.len() as f64
    }
}

fn write_artifact(path: &Path, contents: &str) -> Result<()> {
    fs::write(path, contents).with_context(|| format!("write {}", path.display()))
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn metrics_json(metrics: &BTreeMap<String, f64>) -> String {
    let fields: Vec<String> = metrics
        .iter()
        .map(|(key, value)| format!("\"{}\":{}", json_escape(key), value))
        .collect();
    format!("{{{}}}", fields.join(","))
}

fn map_usize_json(map: &BTreeMap<i32, usize>) -> String {
    let fields: Vec<String> = map
        .iter()
        .map(|(key, value)| format!("\"{}\":{}", key, value))
        .collect();
    format!("{{{}}}", fields.join(","))
}

fn option_usize_json(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn watermarks_json(marks: &BTreeMap<(String, i32), (i64, i64)>) -> String {
    let fields: Vec<String> = marks
        .iter()
        .map(|((topic, partition), (low, high))| {
            format!(
                "{{\"topic\":\"{}\",\"partition\":{},\"earliest\":{},\"latest\":{}}}",
                json_escape(topic),
                partition,
                low,
                high
            )
        })
        .collect();
    format!("[{}]", fields.join(","))
}

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

macro_rules! ensure {
    ($condition:expr, $($arg:tt)*) => {
        if !$condition {
            anyhow::bail!($($arg)*);
        }
    };
}

use ensure;
