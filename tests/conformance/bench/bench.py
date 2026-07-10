"""Throughput and latency benchmark.

Replaces two Rust benchmarks that could only be run by hand:

  * bench_baseline.rs   -- #[ignore]d, heimq alone, produce/consume throughput
                           plus produce latency percentiles.
  * parity/bench.rs     -- behind BENCH_COMPARE, heimq vs kafka vs redpanda.

Both drove librdkafka; confluent-kafka is the same C library, so the numbers
remain comparable with anything recorded in benches/BASELINE.md.

Knobs (same names as the Rust originals):
  BENCH_RECORDS         default 100000
  BENCH_RECORD_SIZE     default 256
  BENCH_LATENCY_SAMPLES default 2000
  BENCH_TARGETS         comma-separated name=host:port; default is heimq only.
"""

import os
import statistics
import sys
import time

from confluent_kafka import Consumer, OFFSET_BEGINNING, Producer, TopicPartition
from confluent_kafka.admin import AdminClient, NewTopic


def env_int(key: str, default: int) -> int:
    try:
        return int(os.environ.get(key, default))
    except ValueError:
        return default


RECORDS = env_int("BENCH_RECORDS", 100_000)
RECORD_SIZE = env_int("BENCH_RECORD_SIZE", 256)
LATENCY_SAMPLES = env_int("BENCH_LATENCY_SAMPLES", 2_000)


def wait_ready(bootstrap: str, attempts: int = 60) -> None:
    admin = AdminClient({"bootstrap.servers": bootstrap})
    for _ in range(attempts):
        try:
            admin.list_topics(timeout=5)
            return
        except Exception:  # noqa: BLE001
            time.sleep(1)
    raise RuntimeError(f"broker not ready: {bootstrap}")


def ensure_topic(bootstrap: str, topic: str) -> None:
    admin = AdminClient({"bootstrap.servers": bootstrap})
    for _, fut in admin.create_topics(
        [NewTopic(topic, num_partitions=1, replication_factor=1)]
    ).items():
        try:
            fut.result()
        except Exception as e:  # noqa: BLE001
            if "already exists" not in str(e) and "TOPIC_ALREADY_EXISTS" not in str(e):
                raise


def bench_target(name: str, bootstrap: str) -> dict:
    wait_ready(bootstrap)
    topic = f"bench-{name}"
    ensure_topic(bootstrap, topic)

    payload = b"x" * RECORD_SIZE
    total_bytes = RECORDS * RECORD_SIZE

    # ── produce ───────────────────────────────────────────────────────────
    p = Producer(
        {
            "bootstrap.servers": bootstrap,
            "queue.buffering.max.messages": 1_000_000,
            "linger.ms": 5,
        }
    )
    latencies: list[float] = []
    sample_every = max(1, RECORDS // LATENCY_SAMPLES) if LATENCY_SAMPLES else 0

    start = time.perf_counter()
    for i in range(RECORDS):
        key = i.to_bytes(8, "little")
        if sample_every and i % sample_every == 0:
            sent = time.perf_counter()

            def cb(err, _msg, sent=sent):
                if err is None:
                    latencies.append((time.perf_counter() - sent) * 1000.0)

            p.produce(topic, value=payload, key=key, on_delivery=cb)
        else:
            p.produce(topic, value=payload, key=key)
        if i % 10_000 == 0:
            p.poll(0)
    p.flush(120)
    produce_elapsed = time.perf_counter() - start

    # ── consume ───────────────────────────────────────────────────────────
    c = Consumer(
        {
            "bootstrap.servers": bootstrap,
            "group.id": f"bench-{name}-probe",
            "enable.auto.commit": False,
            "fetch.min.bytes": env_int("BENCH_CONSUMER_FETCH_MIN_BYTES", 1),
            "fetch.wait.max.ms": env_int("BENCH_CONSUMER_FETCH_WAIT_MAX_MS", 500),
        }
    )
    c.assign([TopicPartition(topic, 0, OFFSET_BEGINNING)])
    consumed = 0
    start = time.perf_counter()
    deadline = start + 300
    while consumed < RECORDS and time.perf_counter() < deadline:
        for m in c.consume(num_messages=1000, timeout=1.0):
            if not m.error():
                consumed += 1
    consume_elapsed = time.perf_counter() - start
    c.close()

    if consumed != RECORDS:
        raise RuntimeError(f"{name}: consumed {consumed}/{RECORDS}")

    result = {
        "target": name,
        "produce_tps": RECORDS / produce_elapsed,
        "produce_mbps": total_bytes / produce_elapsed / 1_048_576.0,
        "consume_tps": RECORDS / consume_elapsed,
    }
    if latencies:
        latencies.sort()
        result["p50_ms"] = statistics.median(latencies)
        result["p99_ms"] = latencies[min(len(latencies) - 1, int(len(latencies) * 0.99))]
    return result


def main() -> int:
    spec = os.environ.get("BENCH_TARGETS") or f"heimq={os.environ['HEIMQ_BOOTSTRAP']}"
    targets = [t.split("=", 1) for t in spec.split(",")]

    print(f"records={RECORDS} record_size={RECORD_SIZE}B", flush=True)
    results = [bench_target(name, boot) for name, boot in targets]

    width = max(len(r["target"]) for r in results)
    for r in results:
        line = (
            f"{r['target']:<{width}}  "
            f"produce {r['produce_tps']:>10,.0f} msg/s ({r['produce_mbps']:>6.1f} MiB/s)  "
            f"consume {r['consume_tps']:>10,.0f} msg/s"
        )
        if "p50_ms" in r:
            line += f"  p50 {r['p50_ms']:.2f}ms  p99 {r['p99_ms']:.2f}ms"
        print(line, flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
