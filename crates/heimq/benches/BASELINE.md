# heimq Performance Baseline

Reproducible produce/fetch throughput and latency for heimq's hot path, measured
with a real librdkafka client (`rdkafka` crate) against an in-process, memory-only
heimq broker. This file is the regression reference: re-run the command and
compare.

## Reproduce

```sh
cargo test -p heimq --release --test bench_baseline -- --ignored --nocapture
```

Knobs (env): `BENCH_RECORDS` (default 100000), `BENCH_RECORD_SIZE` (default 256),
`BENCH_LATENCY_SAMPLES` (default 2000). The bench source is
`crates/heimq/tests/bench_baseline.rs`.

## Recorded baseline

| Metric | Value |
|--------|-------|
| Produce throughput | **543,851 msgs/s** · 132.8 MB/s |
| Consume throughput | **177,790 msgs/s** · 43.4 MB/s |
| Produce-ack latency p50 | **6.52 ms** |
| Produce-ack latency p99 | **8.57 ms** |

- **heimq commit:** `934a894` (broker state benchmarked)
- **Workload:** 100,000 records × 256 B, single topic / single partition, `acks=1`
- **Produce:** pipelined `BaseProducer` (batch 10k, linger 5ms) to flush
- **Consume:** `BaseConsumer` with direct partition assign (no group rebalance)
- **Latency:** single in-flight message, 2,000 samples, `FutureProducer` send→ack
- **Environment:** aarch64 Linux under OrbStack; in-process broker over loopback;
  release build

## Head-to-head vs Apache Kafka and Redpanda

Identical librdkafka workload (200,000 × 256 B, acks=1, single partition) against
all three brokers via the parity harness's 3-broker boot. Reproduce:

```sh
PARITY_TESTS=1 BENCH_COMPARE=1 cargo test -p heimq --release --test parity -- --nocapture
```

| broker | produce msgs/s | consume msgs/s |
|--------|---------------:|---------------:|
| **heimq** | **2,304,980** | **357,691** |
| Apache Kafka 3.9.0 | 825,042 | 68,855 |
| Redpanda v25.1.1 | 477,020 | 97,893 |

heimq vs Kafka: **2.79× produce, 5.19× consume**. vs Redpanda: **4.83× produce,
3.65× consume**. heimq leads both on both paths.

The produce figure is post-optimization: removing a double full-batch decode from
the produce hot path (`RecordBatchHeader::peek` instead of two
`RecordBatchView::from_bytes`) raised heimq produce ~3.8× (601k → 2.30M msgs/s).

Caveat: heimq is reached in-process over loopback; Kafka/Redpanda over the Docker
bridge — a small latency edge to heimq, immaterial at these ratios. Container
numbers are noisy run-to-run (Redpanda produce was 2.27M in one run); heimq's
self-relative ~3.8× produce gain is the stable, reproducible result.

## Caveats

These are indicative single-run numbers on a developer machine over loopback, not
isolated-hardware results — treat them as an order-of-magnitude regression anchor,
not a published spec. Latency is client→broker→ack round-trip (the full protocol
path), not end-to-end produce→consume. The broker is single-node, in-memory, and
non-replicated by design, so these figures reflect protocol/transport efficiency,
not durable-write performance.
