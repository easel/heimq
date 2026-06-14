//! Performance baseline for heimq's hot path (produce / fetch), driven by a real
//! librdkafka client against an in-process heimq broker.
//!
//! Ignored by default (it is a benchmark, not a correctness gate). Run it and
//! record the numbers in `benches/BASELINE.md`:
//!
//! ```sh
//! cargo test -p heimq --release --test bench_baseline -- --ignored --nocapture
//! ```
//!
//! Knobs via env: BENCH_RECORDS (default 100000), BENCH_RECORD_SIZE (default 256),
//! BENCH_LATENCY_SAMPLES (default 2000).

use heimq::test_support::TestServer;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::producer::{BaseProducer, BaseRecord, FutureProducer, FutureRecord, Producer};
use rdkafka::util::Timeout;
use rdkafka::{Offset, TopicPartitionList};
use std::time::{Duration, Instant};

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

#[test]
#[ignore = "performance baseline; run explicitly with --ignored"]
fn bench_baseline_produce_fetch() {
    let records = env_usize("BENCH_RECORDS", 100_000);
    let record_size = env_usize("BENCH_RECORD_SIZE", 256);
    let latency_samples = env_usize("BENCH_LATENCY_SAMPLES", 2_000);
    let topic = "bench-baseline";

    let server = TestServer::start();
    let bootstrap = server.bootstrap_servers();
    let payload = vec![b'x'; record_size];
    let total_bytes = (records * record_size) as f64;

    // ── Produce throughput (pipelined BaseProducer) ──────────────────────────
    let producer: BaseProducer = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap)
        .set("queue.buffering.max.messages", "1000000")
        .set("batch.num.messages", "10000")
        .set("linger.ms", "5")
        .set("acks", "1")
        .create()
        .expect("producer");

    let start = Instant::now();
    for i in 0..records {
        let key = (i as u64).to_le_bytes();
        loop {
            match producer.send(BaseRecord::to(topic).payload(&payload).key(&key[..])) {
                Ok(()) => break,
                Err((rdkafka::error::KafkaError::MessageProduction(
                    rdkafka::types::RDKafkaErrorCode::QueueFull,
                ), _)) => {
                    producer.poll(Duration::from_millis(5));
                }
                Err((e, _)) => panic!("produce error: {e}"),
            }
        }
    }
    producer.flush(Duration::from_secs(60)).expect("flush");
    let produce_elapsed = start.elapsed();
    let produce_tps = records as f64 / produce_elapsed.as_secs_f64();
    let produce_mbps = total_bytes / produce_elapsed.as_secs_f64() / 1_048_576.0;

    // ── Consume throughput (direct partition assign, no group rebalance) ──────
    let consumer: BaseConsumer = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap)
        .set("group.id", "bench-baseline")
        .set("enable.auto.commit", "false")
        .set("fetch.min.bytes", "1")
        .create()
        .expect("consumer");
    let mut tpl = TopicPartitionList::new();
    tpl.add_partition_offset(topic, 0, Offset::Beginning).unwrap();
    consumer.assign(&tpl).unwrap();

    let mut consumed = 0usize;
    let start = Instant::now();
    while consumed < records {
        match consumer.poll(Duration::from_secs(10)) {
            Some(Ok(_)) => consumed += 1,
            Some(Err(e)) => panic!("consume error: {e}"),
            None => {
                if start.elapsed() > Duration::from_secs(120) {
                    panic!("consume timed out at {consumed}/{records}");
                }
            }
        }
    }
    let consume_elapsed = start.elapsed();
    let consume_tps = records as f64 / consume_elapsed.as_secs_f64();
    let consume_mbps = total_bytes / consume_elapsed.as_secs_f64() / 1_048_576.0;

    // ── Produce-ack latency (single in-flight message, p50/p99) ──────────────
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let fproducer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap)
        .set("acks", "1")
        .create()
        .expect("future producer");
    let mut lat: Vec<Duration> = Vec::with_capacity(latency_samples);
    rt.block_on(async {
        for i in 0..latency_samples {
            let key = (i as u64).to_le_bytes();
            let t0 = Instant::now();
            fproducer
                .send(
                    FutureRecord::to(topic).payload(&payload).key(&key[..]),
                    Timeout::After(Duration::from_secs(5)),
                )
                .await
                .expect("send");
            lat.push(t0.elapsed());
        }
    });
    lat.sort();
    let p50 = lat[latency_samples / 2];
    let p99 = lat[(latency_samples * 99 / 100).min(latency_samples - 1)];

    eprintln!("=== heimq perf baseline ===");
    eprintln!("config: records={records} record_size={record_size}B latency_samples={latency_samples}");
    eprintln!(
        "produce_throughput: {:.0} msgs/s  {:.1} MB/s  ({:.2}s)",
        produce_tps, produce_mbps, produce_elapsed.as_secs_f64()
    );
    eprintln!(
        "consume_throughput: {:.0} msgs/s  {:.1} MB/s  ({:.2}s)",
        consume_tps, consume_mbps, consume_elapsed.as_secs_f64()
    );
    eprintln!(
        "produce_ack_latency: p50={:.3}ms  p99={:.3}ms",
        p50.as_secs_f64() * 1000.0,
        p99.as_secs_f64() * 1000.0
    );

    assert_eq!(consumed, records, "must consume every produced record");
}
