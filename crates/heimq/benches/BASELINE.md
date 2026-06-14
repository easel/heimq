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

## Caveats

These are indicative single-run numbers on a developer machine over loopback, not
isolated-hardware results — treat them as an order-of-magnitude regression anchor,
not a published spec. Latency is client→broker→ack round-trip (the full protocol
path), not end-to-end produce→consume. The broker is single-node, in-memory, and
non-replicated by design, so these figures reflect protocol/transport efficiency,
not durable-write performance.
