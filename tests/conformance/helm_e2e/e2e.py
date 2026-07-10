"""Helm fixed-memory end-to-end evidence harness.

Runs against an already-deployed heimq (Helm/kind), addressed by
HEIMQ_E2E_BOOTSTRAP and HEIMQ_E2E_METRICS. Emits one JSON artifact per scenario
into HEIMQ_E2E_ARTIFACT_DIR.

Ported from crates/heimq/tests/helm_memory_e2e.rs. Two things changed:

  * Topic retention configs went through hand-encoded IncrementalAlterConfigs
    frames built with heimq_protocol -- the server's own protocol library, and
    the only reason this test linked `heimq::protocol::is_flexible`.
    confluent-kafka's AdminClient.incremental_alter_configs does it natively.
  * Metrics were scraped by hand-rolling an HTTP/1.1 request over a TcpStream.
    urllib does that.

Scenarios:
  A   exact delivery + partition balance across 1, 10 and 100 topics
  A2  throughput floor
  B   retained bytes plateau under retention.ms, within the memory budget
  C   retention.ms boundary: immediate reads survive, earliest offsets advance
  D   retention.ms-protected records cause producer backpressure at the cap
  E   retention.bytes drops old records rather than raising storage-full
"""

import json
import os
import sys
import time
import urllib.request
from collections import defaultdict
from pathlib import Path

from confluent_kafka import Consumer, KafkaException, OFFSET_BEGINNING, Producer, TopicPartition
from confluent_kafka.admin import (
    AdminClient,
    AlterConfigOpType,
    ConfigEntry,
    ConfigResource,
    NewTopic,
)

DEFAULT_BOOTSTRAP = "127.0.0.1:9092"
DEFAULT_METRICS = "127.0.0.1:9093"
MAX_MEMORY_BYTES = 8 * 1024 * 1024
STORAGE_FULL_CODE = 56


class Ctx:
    def __init__(self) -> None:
        self.bootstrap = os.environ.get("HEIMQ_E2E_BOOTSTRAP", DEFAULT_BOOTSTRAP)
        self.metrics_addr = os.environ.get("HEIMQ_E2E_METRICS", DEFAULT_METRICS)
        self.artifact_dir = Path(
            os.environ.get("HEIMQ_E2E_ARTIFACT_DIR", "target/helm-memory-e2e/manual")
        )
        self.artifact_dir.mkdir(parents=True, exist_ok=True)
        self.scenario = os.environ.get("HEIMQ_E2E_SCENARIO")
        self.run_id = str(now_millis())

    def scenario_path(self, name: str) -> Path:
        return self.artifact_dir / f"{name}.json"


def now_millis() -> int:
    return int(time.time() * 1000)


def ensure(cond: bool, msg: str) -> None:
    if not cond:
        raise AssertionError(msg)


# ── record identity ───────────────────────────────────────────────────────────


def record_id(scenario: str, topic: str, partition: int, sequence: int) -> str:
    return f"{scenario}|{topic}|{partition}|{sequence}"


def payload_for(key: str, payload_bytes: int) -> bytes:
    if payload_bytes <= len(key) + 1:
        return key.encode()
    return (key + ":" + "x" * (payload_bytes - len(key) - 1)).encode()


# ── topics ────────────────────────────────────────────────────────────────────


def numbered_topics(prefix, count, partitions, retention_ms=None, retention_bytes=None):
    stamp = now_millis()
    return [
        {
            "name": f"{prefix.lower()}-{stamp}-{i}",
            "partitions": partitions,
            "retention_ms": retention_ms,
            "retention_bytes": retention_bytes,
        }
        for i in range(count)
    ]


def create_topics(ctx: Ctx, topics: list[dict]) -> None:
    admin = AdminClient({"bootstrap.servers": ctx.bootstrap, "socket.timeout.ms": 15000})
    futures = admin.create_topics(
        [NewTopic(t["name"], num_partitions=t["partitions"], replication_factor=1) for t in topics],
        request_timeout=15,
    )
    for name, fut in futures.items():
        try:
            fut.result()
        except Exception as e:  # noqa: BLE001
            if "already exists" in str(e) or "TOPIC_ALREADY_EXISTS" in str(e):
                print(f"topic already exists during e2e setup: {name}", file=sys.stderr)
                continue
            raise RuntimeError(f"create topic {name} failed: {e}") from e

    incremental_alter_topic_configs(ctx, topics)


def incremental_alter_topic_configs(ctx: Ctx, topics: list[dict]) -> None:
    admin = AdminClient({"bootstrap.servers": ctx.bootstrap, "socket.timeout.ms": 15000})
    resources = []
    for t in topics:
        entries = []
        if t["retention_ms"] is not None:
            entries.append(
                ConfigEntry(
                    "retention.ms", str(t["retention_ms"]),
                    incremental_operation=AlterConfigOpType.SET,
                )
            )
        if t["retention_bytes"] is not None:
            entries.append(
                ConfigEntry(
                    "retention.bytes", str(t["retention_bytes"]),
                    incremental_operation=AlterConfigOpType.SET,
                )
            )
        if entries:
            resources.append(
                ConfigResource(ConfigResource.Type.TOPIC, t["name"], incremental_configs=entries)
            )
    if not resources:
        return
    for name, fut in admin.incremental_alter_configs(resources, request_timeout=15).items():
        try:
            fut.result()
        except KafkaException as e:
            raise RuntimeError(f"IncrementalAlterConfigs failed for {name}: {e}") from e


# ── produce ───────────────────────────────────────────────────────────────────

CORRECTNESS = {
    "message.timeout.ms": 3000,
    "linger.ms": 0,
    "batch.num.messages": 1,
    "queue.buffering.max.messages": 1000,
}
THROUGHPUT = {
    "message.timeout.ms": 5000,
    "linger.ms": 5,
    "batch.num.messages": 100,
    "queue.buffering.max.messages": 10000,
}


def build_producer(ctx: Ctx, profile: dict) -> Producer:
    conf = {
        "bootstrap.servers": ctx.bootstrap,
        "acks": "all",
        "enable.idempotence": False,
        "retries": 0,
        "socket.timeout.ms": 3000,
        "request.timeout.ms": 3000,
    }
    conf.update(profile)
    return Producer(conf)


def send_record(p: Producer, topic: str, partition: int, key: str, payload_bytes: int):
    """Produce one record synchronously. Returns byte count, or raises with the code."""
    payload = payload_for(key, payload_bytes)
    result: list = []

    def cb(err, msg):
        result.append((err, msg))

    p.produce(topic, value=payload, key=key.encode(), partition=partition, on_delivery=cb)
    p.flush(10)
    if not result:
        raise KafkaError_(-1)
    err, _ = result[0]
    if err is not None:
        raise KafkaError_(err.code())
    return len(payload)


class KafkaError_(Exception):
    def __init__(self, code: int):
        super().__init__(f"kafka error {code}")
        self.code = code


def produce_matrix(ctx, scenario, topics, records_per_topic, payload_bytes, profile):
    if profile is THROUGHPUT:
        return produce_matrix_batched(
            ctx, scenario, topics, records_per_topic, payload_bytes, profile
        )
    p = build_producer(ctx, profile)
    acked, errors = [], defaultdict(int)
    first_storage_full_attempt = None
    post_storage_full_errors = 0
    attempted = 0

    start = time.perf_counter()
    for spec in topics:
        for sequence in range(records_per_topic):
            partition = sequence % spec["partitions"]
            rid = record_id(scenario, spec["name"], partition, sequence)
            attempted += 1
            try:
                nbytes = send_record(p, spec["name"], partition, rid, payload_bytes)
                acked.append(
                    {
                        "id": rid,
                        "topic": spec["name"],
                        "partition": partition,
                        "sequence": sequence,
                        "bytes": nbytes,
                    }
                )
            except KafkaError_ as e:
                errors[e.code] += 1
                if e.code == STORAGE_FULL_CODE:
                    if first_storage_full_attempt is None:
                        first_storage_full_attempt = attempted
                    else:
                        post_storage_full_errors += 1
    elapsed = time.perf_counter() - start

    return {
        "attempted": attempted,
        "acked": acked,
        "errors": dict(errors),
        "first_storage_full_attempt": first_storage_full_attempt,
        "post_storage_full_errors": post_storage_full_errors,
        "elapsed": elapsed,
    }


def produce_matrix_batched(ctx, scenario, topics, records_per_topic, payload_bytes, profile):
    """Enqueue everything, then wait for the delivery reports.

    The per-record flush the correctness profile uses would cap throughput at a
    round-trip per record, which is exactly what A2's floor is meant to exceed.
    """
    p = build_producer(ctx, profile)
    acked, errors = [], defaultdict(int)
    first_storage_full_attempt = None
    post_storage_full_errors = 0
    attempted = 0
    results: list[tuple[int, dict, object]] = []

    start = time.perf_counter()
    for spec in topics:
        for sequence in range(records_per_topic):
            partition = sequence % spec["partitions"]
            rid = record_id(scenario, spec["name"], partition, sequence)
            payload = payload_for(rid, payload_bytes)
            attempted += 1
            rec = {
                "id": rid,
                "topic": spec["name"],
                "partition": partition,
                "sequence": sequence,
                "bytes": len(payload),
            }

            def cb(err, _msg, rec=rec, n=attempted):
                results.append((n, rec, err))

            while True:
                try:
                    p.produce(
                        spec["name"], value=payload, key=rid.encode(),
                        partition=partition, on_delivery=cb,
                    )
                    break
                except BufferError:
                    p.poll(0.1)  # local queue full; drain and retry
            p.poll(0)
    p.flush(60)
    elapsed = time.perf_counter() - start

    for n, rec, err in sorted(results, key=lambda r: r[0]):
        if err is None:
            acked.append(rec)
            continue
        errors[err.code()] += 1
        if err.code() == STORAGE_FULL_CODE:
            if first_storage_full_attempt is None:
                first_storage_full_attempt = n
            else:
                post_storage_full_errors += 1

    return {
        "attempted": attempted,
        "acked": acked,
        "errors": dict(errors),
        "first_storage_full_attempt": first_storage_full_attempt,
        "post_storage_full_errors": post_storage_full_errors,
        "elapsed": elapsed,
    }


# ── consume / watermarks ──────────────────────────────────────────────────────


def consume_records(ctx, group, topics, expected, timeout_s):
    c = Consumer(
        {
            "bootstrap.servers": ctx.bootstrap,
            "group.id": group,
            "enable.auto.commit": False,
            "auto.offset.reset": "earliest",
            "socket.timeout.ms": 15000,
        }
    )
    assignment = [
        TopicPartition(t["name"], part, OFFSET_BEGINNING)
        for t in topics
        for part in range(t["partitions"])
    ]
    c.assign(assignment)
    out: list[str] = []
    deadline = time.monotonic() + timeout_s
    try:
        while len(out) < expected and time.monotonic() < deadline:
            m = c.poll(0.2)
            if m is None or m.error():
                continue
            out.append(m.key().decode())
    finally:
        c.close()
    return out


def watermarks(ctx, topics) -> dict:
    c = Consumer(
        {
            "bootstrap.servers": ctx.bootstrap,
            "group.id": f"watermarks-{ctx.run_id}",
            "enable.auto.commit": False,
        }
    )
    marks = {}
    try:
        for t in topics:
            for part in range(t["partitions"]):
                low, high = c.get_watermark_offsets(
                    TopicPartition(t["name"], part), timeout=5, cached=False
                )
                marks[(t["name"], part)] = (low, high)
    finally:
        c.close()
    return marks


def wait_for_retention_bytes_watermarks(ctx, topics, expected_latest, timeout_s):
    deadline = time.monotonic() + timeout_s
    marks = watermarks(ctx, topics)
    while True:
        settled = all(
            (lambda lh: lh is not None and lh[0] > 0 and lh[0] < expected_latest and lh[1] == expected_latest)(
                marks.get((t["name"], 0))
            )
            for t in topics
        )
        if settled or time.monotonic() >= deadline:
            return marks
        time.sleep(0.25)
        marks = watermarks(ctx, topics)


# ── validation ────────────────────────────────────────────────────────────────


def validate_exact(produced: list[dict], consumed: list[str]) -> dict:
    want = {r["id"] for r in produced}
    seen = defaultdict(int)
    for c in consumed:
        seen[c] += 1
    missing = len(want - set(seen))
    duplicates = sum(n - 1 for c, n in seen.items() if c in want and n > 1)
    return {"missing": missing, "duplicates": duplicates}


def validate_partition_balance(produced: list[dict], topics: list[dict]) -> dict:
    counts = {
        (t["name"], part): 0 for t in topics for part in range(t["partitions"])
    }
    for r in produced:
        counts[(r["topic"], r["partition"])] = counts.get((r["topic"], r["partition"]), 0) + 1
    values = list(counts.values()) or [0]
    non_empty = sum(1 for n in values if n > 0)
    min_count, max_count = min(values), max(values)
    ratio = non_empty / max(len(values), 1)
    return {
        "ok": ratio >= 0.75 and (max_count - min_count) <= 1,
        "non_empty_ratio": ratio,
        "min_count": min_count,
        "max_count": max_count,
    }


# ── metrics ───────────────────────────────────────────────────────────────────


def scrape_metrics(ctx: Ctx) -> dict:
    try:
        with urllib.request.urlopen(f"http://{ctx.metrics_addr}/metrics", timeout=3) as r:
            body = r.read().decode()
    except Exception:  # noqa: BLE001 - callers treat a failed scrape as empty
        return {}
    metrics: dict[str, float] = {}
    for line in body.splitlines():
        if not line or line.startswith("#"):
            continue
        parts = line.split()
        if len(parts) < 2:
            continue
        name = parts[0].split("{")[0]
        try:
            metrics[name] = metrics.get(name, 0.0) + float(parts[1])
        except ValueError:
            continue
    return metrics


def metric_delta(before: dict, after: dict, key: str) -> float:
    return after.get(key, 0.0) - before.get(key, 0.0)


def mean_window(values: list[float], skip: int, take: int) -> float:
    window = values[skip : skip + take]
    if not window:
        return values[0] if values else 0.0
    return sum(window) / len(window)


def mean_tail(values: list[float], take: int) -> float:
    window = values[-take:] if values else []
    if not window:
        return values[0] if values else 0.0
    return sum(window) / len(window)


def write_artifact(path: Path, obj: dict) -> None:
    path.write_text(json.dumps(obj) + "\n")
    print(f"wrote {path}", flush=True)


def marks_json(marks: dict) -> dict:
    return {f"{t}:{p}": {"low": lo, "high": hi} for (t, p), (lo, hi) in marks.items()}


# ── scenarios ─────────────────────────────────────────────────────────────────


def scenario_a(ctx: Ctx) -> None:
    cases = [
        ("A-1-topic", 1, 4, 800, 512),
        ("A-10-topics", 10, 4, 160, 512),
        ("A-100-topics", 100, 2, 20, 512),
    ]
    selected = os.environ.get("HEIMQ_E2E_A_CASE")
    summaries = []
    for case, topic_count, partitions, per_topic, payload in cases:
        if selected and selected != case:
            continue
        topics = numbered_topics(case, topic_count, partitions)
        create_topics(ctx, topics)
        before = scrape_metrics(ctx)
        stats = produce_matrix(ctx, case, topics, per_topic, payload, CORRECTNESS)
        consumed = consume_records(
            ctx, f"group-{case}-{ctx.run_id}", topics, len(stats["acked"]), 30
        )
        after = scrape_metrics(ctx)
        validation = validate_exact(stats["acked"], consumed)
        balance = validate_partition_balance(stats["acked"], topics)

        ensure(
            validation["missing"] == 0 and validation["duplicates"] == 0,
            f"{case}: consumed set mismatch: {validation}",
        )
        ensure(balance["ok"], f"{case}: partition balance failed: {balance}")

        summaries.append(
            {
                "case": case,
                "topics": topic_count,
                "partitions_per_topic": partitions,
                "attempted": stats["attempted"],
                "acked": len(stats["acked"]),
                "errors": stats["errors"],
                "first_storage_full_attempt": stats["first_storage_full_attempt"],
                "post_storage_full_errors": stats["post_storage_full_errors"],
                "consumed": len(consumed),
                "missing": validation["missing"],
                "duplicates": validation["duplicates"],
                "non_empty_partition_ratio": balance["non_empty_ratio"],
                "min_partition_records": balance["min_count"],
                "max_partition_records": balance["max_count"],
                "metrics_before": before,
                "metrics_after": after,
            }
        )

    ensure(bool(summaries), "A: no case matched HEIMQ_E2E_A_CASE")
    name = f"scenario-a-{selected.lower()}" if selected else "scenario-a"
    write_artifact(
        ctx.scenario_path(name),
        {
            "scenario": "A",
            "description": "load balanced across 1,10,100 topics",
            "accepted": True,
            "cases": summaries,
        },
    )


def scenario_a2(ctx: Ctx) -> None:
    topics = numbered_topics("A2-throughput", 10, 4)
    create_topics(ctx, topics)
    before = scrape_metrics(ctx)
    stats = produce_matrix(ctx, "A2", topics, 400, 512, THROUGHPUT)
    consumed = consume_records(ctx, f"group-A2-{ctx.run_id}", topics, len(stats["acked"]), 45)
    after = scrape_metrics(ctx)
    validation = validate_exact(stats["acked"], consumed)
    ensure(
        validation["missing"] == 0 and validation["duplicates"] == 0,
        f"A2: consumed set mismatch: {validation}",
    )

    rps = len(stats["acked"]) / stats["elapsed"]
    mib_s = (sum(r["bytes"] for r in stats["acked"]) / 1_048_576.0) / stats["elapsed"]
    ensure(
        rps >= 1000.0 and mib_s >= 0.49,
        f"A2: throughput below threshold: {rps:.2f} records/s, {mib_s:.3f} MiB/s",
    )

    write_artifact(
        ctx.scenario_path("scenario-a2"),
        {
            "scenario": "A2",
            "description": "port-forward throughput sample",
            "accepted": True,
            "attempted": stats["attempted"],
            "acked": len(stats["acked"]),
            "errors": stats["errors"],
            "consumed": len(consumed),
            "missing": validation["missing"],
            "duplicates": validation["duplicates"],
            "elapsed_ms": int(stats["elapsed"] * 1000),
            "records_per_sec": rps,
            "mib_per_sec": mib_s,
            "metrics_before": before,
            "metrics_after": after,
        },
    )


def scenario_b(ctx: Ctx) -> None:
    topics = numbered_topics("B-plateau", 10, 2, retention_ms=3000)
    create_topics(ctx, topics)
    before = scrape_metrics(ctx)
    p = build_producer(ctx, CORRECTNESS)

    # Warm metadata and connections for every partition before the timed loop.
    # Cold, the per-record flush runs at ~60 rec/s while librdkafka resolves each
    # new topic, and the ramp swamps the post-warmup sample window the plateau
    # assertion measures. These records expire with everything else at 3s.
    for spec in topics:
        for partition in range(spec["partitions"]):
            try:
                send_record(p, spec["name"], partition, record_id("B-warmup", spec["name"], partition, 0), 512)
            except KafkaError_:
                pass

    produced, errors, samples = 0, defaultdict(int), []
    started = time.monotonic()
    next_sample_at = started
    while time.monotonic() - started < 20:
        tick = time.monotonic()
        for n in range(50):
            spec = topics[(produced + n) % len(topics)]
            partition = (produced + n) % spec["partitions"]
            rid = record_id("B", spec["name"], partition, produced)
            try:
                send_record(p, spec["name"], partition, rid, 512)
                produced += 1
            except KafkaError_ as e:
                errors[e.code] += 1
        if time.monotonic() >= next_sample_at:
            samples.append(scrape_metrics(ctx))
            next_sample_at += 1.0
        elapsed = time.monotonic() - tick
        if elapsed < 0.1:
            time.sleep(0.1 - elapsed)

    after = scrape_metrics(ctx)
    retained = [s.get("heimq_memory_log_bytes", 0.0) for s in samples]
    ensure(bool(retained), "B: no metrics samples were captured")
    if os.environ.get("HEIMQ_E2E_DEBUG"):
        print("B retained series:", [int(v) for v in retained], flush=True)
        print("B produced:", produced, "errors:", dict(errors), flush=True)

    first = mean_window(retained, 5, 5)
    last = mean_tail(retained, 5)
    ensure(last <= first * 1.15 + 1.0, f"B: retained bytes did not plateau: first={first} last={last}")
    ensure(
        after.get("heimq_memory_log_bytes", 0.0) <= float(MAX_MEMORY_BYTES),
        "B: retained bytes exceeded fixed memory budget",
    )
    ensure(
        metric_delta(before, after, "heimq_storage_full_errors_total") == 0.0,
        "B: unexpected storage-full errors",
    )

    write_artifact(
        ctx.scenario_path("scenario-b"),
        {
            "scenario": "B",
            "description": "memory plateau under retention.ms",
            "accepted": True,
            "produced": produced,
            "errors": dict(errors),
            "samples": len(retained),
            "first_post_warmup_mean_bytes": first,
            "last_mean_bytes": last,
            "retained_bytes_final": after.get("heimq_memory_log_bytes", 0.0),
            "retention_reclaimed_delta": metric_delta(
                before, after, "heimq_retention_reclaimed_bytes_total"
            ),
            "storage_full_delta": metric_delta(before, after, "heimq_storage_full_errors_total"),
            "metrics_before": before,
            "metrics_after": after,
        },
    )


def scenario_c(ctx: Ctx) -> None:
    topics = numbered_topics("C-boundary", 3, 2, retention_ms=5000)
    create_topics(ctx, topics)
    before = scrape_metrics(ctx)
    stats = produce_matrix(ctx, "C", topics, 100, 512, CORRECTNESS)
    immediate = consume_records(
        ctx, f"group-C-immediate-{ctx.run_id}", topics, len(stats["acked"]), 2
    )
    validation = validate_exact(stats["acked"], immediate)
    ensure(
        validation["missing"] == 0 and validation["duplicates"] == 0,
        f"C: immediate consume failed: {validation}",
    )

    latest_before = watermarks(ctx, topics)
    time.sleep(6.5)

    deadline = time.monotonic() + 3
    advanced = False
    final_marks = dict(latest_before)
    while time.monotonic() < deadline:
        final_marks = watermarks(ctx, topics)
        advanced = any(
            low > latest_before.get(k, (0, 0))[0] and high == latest_before.get(k, (0, 0))[1]
            for k, (low, high) in final_marks.items()
        )
        if advanced:
            break
        time.sleep(0.25)

    ensure(advanced, "C: earliest offsets did not advance after retention.ms boundary")
    after = scrape_metrics(ctx)

    write_artifact(
        ctx.scenario_path("scenario-c"),
        {
            "scenario": "C",
            "description": (
                "retention.ms boundary preserves immediate reads then advances "
                "earliest offsets without appends"
            ),
            "accepted": True,
            "acked": len(stats["acked"]),
            "immediate_consumed": len(immediate),
            "missing": validation["missing"],
            "duplicates": validation["duplicates"],
            "latest_offsets_unchanged": True,
            "watermarks_before": marks_json(latest_before),
            "watermarks_after": marks_json(final_marks),
            "retention_reclaimed_delta": metric_delta(
                before, after, "heimq_retention_reclaimed_bytes_total"
            ),
            "metrics_before": before,
            "metrics_after": after,
        },
    )


def scenario_d(ctx: Ctx) -> None:
    topics = numbered_topics("D-backpressure", 1, 1, retention_ms=60000)
    create_topics(ctx, topics)
    before = scrape_metrics(ctx)
    p = build_producer(ctx, CORRECTNESS)
    spec = topics[0]

    acked, errors = [], defaultdict(int)
    first_storage_full = None
    post_errors = 0

    for attempt in range(4096):
        rid = record_id("D", spec["name"], 0, attempt)
        try:
            nbytes = send_record(p, spec["name"], 0, rid, 4096)
            if first_storage_full is None:
                acked.append(
                    {"id": rid, "topic": spec["name"], "partition": 0,
                     "sequence": attempt, "bytes": nbytes}
                )
        except KafkaError_ as e:
            errors[e.code] += 1
            if e.code == STORAGE_FULL_CODE:
                if first_storage_full is None:
                    first_storage_full = attempt
                else:
                    post_errors += 1
                    if post_errors >= 16:
                        break
        if attempt == 1536:
            time.sleep(1.5)

    after = scrape_metrics(ctx)
    consumed = consume_records(ctx, f"group-D-{ctx.run_id}", topics, len(acked), 30)
    validation = validate_exact(acked, consumed)

    ensure(first_storage_full is not None, "D: expected storage-full backpressure")
    ensure(len(acked) >= 256, "D: too few accepted records before backpressure")
    ensure(post_errors >= 16, "D: did not verify 16 post-storage-full attempts")
    ensure(
        validation["missing"] == 0 and validation["duplicates"] == 0,
        f"D: accepted records not consumable: {validation}",
    )
    ensure(
        metric_delta(before, after, "heimq_retention_reclaimed_bytes_total") == 0.0,
        "D: retention reclaimed bytes before protected data filled memory",
    )

    write_artifact(
        ctx.scenario_path("scenario-d"),
        {
            "scenario": "D",
            "description": (
                "retention.ms protected records cause producer backpressure at memory cap"
            ),
            "accepted": True,
            "attempted": (first_storage_full or 0) + post_errors + 1,
            "acked_before_storage_full": len(acked),
            "accepted_batch_bytes_total": sum(r["bytes"] for r in acked),
            "max_accepted_batch_bytes": max((r["bytes"] for r in acked), default=0),
            "first_storage_full_attempt": first_storage_full or 0,
            "post_storage_full_errors": post_errors,
            "errors": dict(errors),
            "consumed": len(consumed),
            "missing": validation["missing"],
            "duplicates": validation["duplicates"],
            "retained_bytes_final": after.get("heimq_memory_log_bytes", 0.0),
            "retention_reclaimed_delta": metric_delta(
                before, after, "heimq_retention_reclaimed_bytes_total"
            ),
            "metrics_before": before,
            "metrics_after": after,
        },
    )


def scenario_e(ctx: Ctx) -> None:
    retention_bytes = 524_288
    topics = numbered_topics("E-retention-bytes", 10, 1, retention_bytes=retention_bytes)
    create_topics(ctx, topics)
    before = scrape_metrics(ctx)
    stats = produce_matrix(ctx, "E", topics, 400, 2048, CORRECTNESS)

    ensure(
        metric_delta(before, scrape_metrics(ctx), "heimq_storage_full_errors_total") == 0.0,
        "E: unexpected storage-full while retention.bytes should drop old records",
    )

    marks = wait_for_retention_bytes_watermarks(ctx, topics, 400, 5)
    for t in topics:
        lh = marks.get((t["name"], 0))
        if lh is None:
            raise RuntimeError(f"missing watermark for {t['name']}")
        low, high = lh
        ensure(0 < low < 400, f"E: {t['name']} low watermark {low} outside expected range")
        ensure(high == 400, f"E: {t['name']} high watermark {high} != 400")

    after = scrape_metrics(ctx)
    write_artifact(
        ctx.scenario_path("scenario-e"),
        {
            "scenario": "E",
            "description": "retention.bytes drops old records instead of raising storage-full",
            "accepted": True,
            "acked": len(stats["acked"]),
            "retention_bytes": retention_bytes,
            "watermarks": marks_json(marks),
            "retention_reclaimed_delta": metric_delta(
                before, after, "heimq_retention_reclaimed_bytes_total"
            ),
            "metrics_before": before,
            "metrics_after": after,
        },
    )


SCENARIOS = {
    "A": scenario_a,
    "A2": scenario_a2,
    "B": scenario_b,
    "C": scenario_c,
    "D": scenario_d,
    "E": scenario_e,
}


def main() -> int:
    ctx = Ctx()
    if ctx.scenario:
        fn = SCENARIOS.get(ctx.scenario)
        if fn is None:
            print(f"unknown HEIMQ_E2E_SCENARIO={ctx.scenario}", file=sys.stderr)
            return 2
        fn(ctx)
    else:
        for fn in SCENARIOS.values():
            fn(ctx)
    print("helm-memory-e2e: PASS", flush=True)
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except AssertionError as e:
        print(f"FAIL: {e}", file=sys.stderr)
        sys.exit(1)
