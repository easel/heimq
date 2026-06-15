//! Comparative throughput benchmark: identical produce/consume load against
//! heimq, Apache Kafka, and Redpanda (the same three brokers the parity harness
//! boots). Gated by BENCH_COMPARE=1. Reuses a real librdkafka client so the only
//! variable is the broker.
//!
//! Caveat: heimq is reached in-process over loopback; Kafka/Redpanda are reached
//! over the Docker bridge. That asymmetry favours heimq slightly on latency, but
//! is immaterial to the throughput question if the gap is large.

use crate::broker::BrokerTarget;
use anyhow::Result;
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::client::DefaultClientContext;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::producer::{BaseProducer, BaseRecord, Producer};
use rdkafka::{Offset, TopicPartitionList};
use std::time::{Duration, Instant};

/// (produce_msgs_per_s, consume_msgs_per_s) for one broker.
fn bench_one(
    rt: &tokio::runtime::Runtime,
    target: &BrokerTarget,
    topic: &str,
    records: usize,
    record_size: usize,
) -> Result<(f64, f64)> {
    let bootstrap = target.bootstrap_servers.as_str();

    let admin: AdminClient<DefaultClientContext> = rdkafka::ClientConfig::new()
        .set("bootstrap.servers", bootstrap)
        .set("socket.timeout.ms", "20000")
        .create()?;
    rt.block_on(admin.create_topics(
        &[NewTopic::new(topic, 1, TopicReplication::Fixed(1))],
        &AdminOptions::new().request_timeout(Some(Duration::from_secs(20))),
    ))?;
    std::thread::sleep(Duration::from_millis(500)); // topic propagation

    let payload = vec![b'x'; record_size];

    // ── Produce ──────────────────────────────────────────────────────────────
    let producer: BaseProducer = rdkafka::ClientConfig::new()
        .set("bootstrap.servers", bootstrap)
        .set("queue.buffering.max.messages", "1000000")
        .set("batch.num.messages", "10000")
        .set("linger.ms", "5")
        .set("acks", "1")
        .create()?;
    let start = Instant::now();
    for i in 0..records {
        let key = (i as u64).to_le_bytes();
        loop {
            match producer.send(BaseRecord::to(topic).payload(&payload).key(&key[..])) {
                Ok(()) => break,
                Err((
                    rdkafka::error::KafkaError::MessageProduction(
                        rdkafka::types::RDKafkaErrorCode::QueueFull,
                    ),
                    _,
                )) => producer.poll(Duration::from_millis(5)),
                Err((e, _)) => anyhow::bail!("produce: {e}"),
            }
        }
    }
    producer.flush(Duration::from_secs(60))?;
    let produce_tps = records as f64 / start.elapsed().as_secs_f64();

    // ── Consume (direct partition assign, no group rebalance) ────────────────
    let consumer: BaseConsumer = rdkafka::ClientConfig::new()
        .set("bootstrap.servers", bootstrap)
        .set("group.id", "bench-compare")
        .set("enable.auto.commit", "false")
        .create()?;
    let mut tpl = TopicPartitionList::new();
    tpl.add_partition_offset(topic, 0, Offset::Beginning)?;
    consumer.assign(&tpl)?;
    let mut got = 0usize;
    let start = Instant::now();
    while got < records {
        match consumer.poll(Duration::from_secs(10)) {
            Some(Ok(_)) => got += 1,
            Some(Err(e)) => anyhow::bail!("consume: {e}"),
            None => {
                if start.elapsed() > Duration::from_secs(180) {
                    anyhow::bail!("consume timed out at {got}/{records}");
                }
            }
        }
    }
    let consume_tps = records as f64 / start.elapsed().as_secs_f64();

    Ok((produce_tps, consume_tps))
}

pub fn run_compare(heimq: &str, kafka: &str, redpanda: &str) -> Result<()> {
    let records: usize = std::env::var("BENCH_RECORDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(200_000);
    let size: usize = 256;
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    let targets = [
        (
            "heimq",
            BrokerTarget {
                bootstrap_servers: heimq.to_string(),
            },
        ),
        (
            "kafka",
            BrokerTarget {
                bootstrap_servers: kafka.to_string(),
            },
        ),
        (
            "redpanda",
            BrokerTarget {
                bootstrap_servers: redpanda.to_string(),
            },
        ),
    ];

    let mut results = Vec::new();
    for (name, target) in &targets {
        let topic = format!("bench-cmp-{name}");
        let (p, c) = bench_one(&rt, target, &topic, records, size)?;
        eprintln!("{name}: produce {p:.0} msgs/s  consume {c:.0} msgs/s");
        results.push((*name, p, c));
    }

    let (_, hp, hc) = results[0];
    println!("\n=== throughput: {records} x {size}B, acks=1, 1 partition ===");
    println!("{:<10} {:>14} {:>14}", "broker", "produce/s", "consume/s");
    for (name, p, c) in &results {
        println!("{:<10} {:>14.0} {:>14.0}", name, p, c);
    }
    println!();
    for (name, p, c) in results.iter().skip(1) {
        println!(
            "heimq vs {:<8} produce {:.2}x   consume {:.2}x",
            name,
            hp / p,
            hc / c
        );
    }
    Ok(())
}
