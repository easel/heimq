"""Partitioning, fetch behaviour, headers, compression and transaction tests.

Ported from integration.rs. All were #[ignore]d in Rust except the two
transactional ones.
"""

import threading
import time

import pytest
from confluent_kafka import OFFSET_BEGINNING, Consumer, KafkaException, TopicPartition

from .helpers import (
    consume_n,
    consumer,
    drain,
    payloads,
    produce_n,
    produce_sync,
    producer,
)


def test_rdkafka_multi_partition_roundtrip(bootstrap_3p, topic, group):
    N = 60
    p = producer(bootstrap_3p)
    produced = [
        produce_sync(p, topic, f"mp-{i}".encode(), key=f"key-{i}".encode()) for i in range(N)
    ]
    partitions = {m.partition() for m in produced}
    assert len(partitions) > 1, f"messages should span partitions, got {partitions}"

    c = consumer(bootstrap_3p, group)
    c.subscribe([topic])
    try:
        msgs = consume_n(c, N, timeout=20)
        assert len(msgs) == N
        assert sorted(payloads(msgs)) == sorted(f"mp-{i}" for i in range(N))
    finally:
        c.close()


def test_rdkafka_per_partition_ordering(bootstrap_3p, topic, group):
    """Within a partition, offsets increase monotonically, and each key's
    sequence is delivered in produce order."""
    KEYS = ["ka", "kb", "kc"]
    PER_KEY = 20
    p = producer(bootstrap_3p)

    key_partition: dict[str, int] = {}
    for seq in range(PER_KEY):
        for k in KEYS:  # interleave keys
            m = produce_sync(p, topic, f"{k}:{seq}".encode(), key=k.encode())
            prev = key_partition.setdefault(k, m.partition())
            assert prev == m.partition(), f"key {k} must route to one partition"

    c = consumer(bootstrap_3p, group)
    c.subscribe([topic])
    try:
        msgs = consume_n(c, PER_KEY * len(KEYS), timeout=30)
        assert len(msgs) == PER_KEY * len(KEYS)

        # Offsets strictly increase within each partition.
        by_partition: dict[int, list[int]] = {}
        for m in msgs:
            by_partition.setdefault(m.partition(), []).append(m.offset())
        for part, offsets in by_partition.items():
            assert offsets == sorted(offsets), f"partition {part} out of order: {offsets}"
            assert len(set(offsets)) == len(offsets), f"partition {part} has duplicates"

        # Each key's sequence numbers arrive in produce order.
        by_key: dict[str, list[int]] = {}
        for m in msgs:
            k, seq = m.value().decode().split(":")
            by_key.setdefault(k, []).append(int(seq))
        for k, seqs in by_key.items():
            assert seqs == list(range(PER_KEY)), f"key {k} out of order: {seqs}"
    finally:
        c.close()


def test_rdkafka_keyed_partitioner_determinism(bootstrap_3p, topic):
    """The same key must always land on the same partition."""
    p = producer(bootstrap_3p)
    first: dict[str, int] = {}
    for i in range(30):
        k = f"key-{i}"
        first[k] = produce_sync(p, topic, b"v", key=k.encode()).partition()

    for k, part in first.items():
        again = produce_sync(p, topic, b"v2", key=k.encode()).partition()
        assert again == part, f"key {k} moved from partition {part} to {again}"

    assert len(set(first.values())) > 1, "keys should not all hash to one partition"


def test_rdkafka_explicit_partition_target(bootstrap_4p, topic, group):
    """An explicit partition on produce must be honoured by the broker."""
    TARGET = 2
    p = producer(bootstrap_4p)
    m = produce_sync(p, topic, b"targeted", key=b"ignored-for-routing", partition=TARGET)
    assert m.partition() == TARGET, (
        f"broker must honor the explicit partition override: wrote to {m.partition()}"
    )

    c = consumer(bootstrap_4p, group)
    c.assign([TopicPartition(topic, TARGET, OFFSET_BEGINNING)])
    try:
        msgs = consume_n(c, 1)
        assert len(msgs) == 1
        assert msgs[0].partition() == TARGET
        assert msgs[0].value() == b"targeted"
    finally:
        c.close()


def test_rdkafka_fetch_long_poll(bootstrap, topic, group):
    """With fetch.min.bytes unmet, a poll waits rather than spinning; once data
    arrives it is delivered promptly."""
    p = producer(bootstrap)
    produce_sync(p, topic, b"init")

    c = consumer(
        bootstrap,
        group,
        **{
            "fetch.min.bytes": 1024,
            "fetch.wait.max.ms": 500,
            "fetch.queue.backoff.ms": 50,
        },
    )
    c.subscribe([topic])
    try:
        assert consume_n(c, 1), "failed to drain init message before measurement"

        # Empty topic: poll returns nothing, and does not hang past the timeout.
        start = time.monotonic()
        m = c.poll(0.5)
        elapsed = time.monotonic() - start
        assert m is None or m.error(), "no message should arrive on an empty log"
        assert elapsed < 5.0, f"poll hung for {elapsed:.2f}s"

        # Data written mid-wait must be delivered.
        produce_n(p, topic, 20, "longpoll-{}")
        got = consume_n(c, 20, timeout=15)
        assert len(got) == 20
    finally:
        c.close()


def test_rdkafka_empty_topic_poll_timeout(bootstrap, topic, group):
    """An existing-but-empty log yields an empty FetchResponse, not an error."""
    p = producer(bootstrap)
    produce_sync(p, topic, b"init")

    c = consumer(bootstrap, group)
    c.subscribe([topic])
    try:
        assert consume_n(c, 1), "failed to drain init message before measurement"

        start = time.monotonic()
        m = c.poll(0.5)
        elapsed = time.monotonic() - start
        assert m is None or m.error(), "empty log must not yield a message"
        assert elapsed < 5.0, f"poll on empty topic took {elapsed:.2f}s"
    finally:
        c.close()


def test_rdkafka_produce_no_autocreate_errors(bootstrap_noac, topic):
    """With auto-create disabled, producing to a missing topic must surface an
    error on the delivery report -- not hang, not silently succeed."""
    p = producer(bootstrap_noac, **{"message.timeout.ms": 5000})
    with pytest.raises(AssertionError, match="delivery failed"):
        produce_sync(p, topic, b"should-fail")


def test_rdkafka_auto_create_toggle(bootstrap, bootstrap_noac, topic):
    """The same produce succeeds where auto-create is on and fails where it is off."""
    ok = producer(bootstrap)
    produce_sync(ok, topic, b"created")  # auto-create on: succeeds

    nope = producer(bootstrap_noac, **{"message.timeout.ms": 5000})
    with pytest.raises(AssertionError, match="delivery failed"):
        produce_sync(nope, f"{topic}-absent", b"should-fail")


def test_rdkafka_oversized_message(bootstrap, topic, group):
    """A 4 MiB payload -- 4x the conventional 1 MiB broker default -- must
    round-trip intact rather than hang or truncate."""
    SIZE = 4 * 1024 * 1024
    payload = b"z" * SIZE

    p = producer(
        bootstrap,
        **{
            "message.timeout.ms": 15000,
            "message.max.bytes": 16777216,
            "queue.buffering.max.kbytes": 32768,
        },
    )
    produce_sync(p, topic, payload, key=b"oversized")

    c = consumer(
        bootstrap,
        group,
        **{
            "fetch.message.max.bytes": 16777216,
            "fetch.max.bytes": 16777216,
            "receive.message.max.bytes": 16778240,  # fetch.max.bytes + 1024
        },
    )
    c.subscribe([topic])
    try:
        msgs = consume_n(c, 1, timeout=30)
        assert len(msgs) == 1, "oversized message must be delivered"
        assert len(msgs[0].value()) == SIZE, "oversized message must round-trip intact"
        assert msgs[0].value() == payload
    finally:
        c.close()


def test_rdkafka_concurrent_producers(bootstrap, topic, group):
    PRODUCERS, PER_PRODUCER = 4, 25
    errors: list[BaseException] = []

    def run(pid: int) -> None:
        try:
            p = producer(bootstrap)
            for i in range(PER_PRODUCER):
                produce_sync(p, topic, f"p{pid}-m{i}".encode(), key=f"p{pid}".encode())
        except BaseException as e:  # noqa: BLE001
            errors.append(e)

    threads = [threading.Thread(target=run, args=(i,)) for i in range(PRODUCERS)]
    for t in threads:
        t.start()
    for t in threads:
        t.join()
    assert not errors, f"producer thread failed: {errors}"

    c = consumer(bootstrap, group)
    c.subscribe([topic])
    try:
        total = PRODUCERS * PER_PRODUCER
        msgs = consume_n(c, total, timeout=30)
        assert len(msgs) == total
        assert len(set(payloads(msgs))) == total, "every message must be distinct"
    finally:
        c.close()


def test_rdkafka_produce_consume_soak(bootstrap, topic, group):
    N = 500
    p = producer(bootstrap)
    produce_n(p, topic, N, "soak-{}", "k-{}")

    c = consumer(bootstrap, group)
    c.subscribe([topic])
    try:
        msgs = consume_n(c, N, timeout=60)
        assert len(msgs) == N, f"expected {N} messages, got {len(msgs)}"
        assert payloads(msgs) == [f"soak-{i}" for i in range(N)], "order must be preserved"
        offsets = [m.offset() for m in msgs]
        assert offsets == sorted(offsets), "offsets must increase monotonically"
    finally:
        c.close()


def test_rdkafka_record_headers_roundtrip(bootstrap, topic, group):
    expected = [
        ("trace-id", b"abc-123"),
        ("content-type", b"application/octet-stream"),
        ("x-binary", bytes([0x00, 0x01, 0x02, 0xFF])),
    ]
    p = producer(bootstrap)
    produce_sync(
        p, topic, b"headers-roundtrip-payload", key=b"headers-key", headers=expected
    )

    c = consumer(bootstrap, group)
    c.subscribe([topic])
    try:
        msgs = consume_n(c, 1)
        assert len(msgs) == 1, "did not receive message within timeout"
        m = msgs[0]
        assert m.value() == b"headers-roundtrip-payload"

        got = m.headers()
        assert got is not None, "consumed message had no headers"
        assert len(got) == len(expected), f"header count mismatch: {got}"
        for (gk, gv), (ek, ev) in zip(got, expected):
            assert gk == ek
            assert gv == ev
    finally:
        c.close()


@pytest.mark.parametrize("codec", ["gzip", "snappy", "lz4", "zstd"])
def test_rdkafka_compression_codecs(bootstrap, topic, group, codec):
    """Each codec must round-trip exactly. linger/batch settings force a real
    batch so the codec actually engages."""
    N = 20
    p = producer(
        bootstrap,
        **{"compression.type": codec, "linger.ms": 50, "batch.num.messages": N},
    )
    produce_n(p, topic, N, f"{codec}" + "-{}")

    c = consumer(bootstrap, group)
    c.subscribe([topic])
    try:
        msgs = consume_n(c, N, timeout=20)
        assert len(msgs) == N, f"{codec}: expected {N}, got {len(msgs)}"
        assert payloads(msgs) == [f"{codec}-{i}" for i in range(N)]
    finally:
        c.close()


def test_rdkafka_pause_resume_partitions(bootstrap_2p, topic, group):
    """Pause partition 0: only partition 1 messages arrive. Resume it: the
    partition 0 backlog drains."""
    p = producer(bootstrap_2p)
    for part in (0, 1):
        for i in range(5):
            produce_sync(p, topic, f"p{part}-{i}".encode(), partition=part)

    c = consumer(bootstrap_2p, group)
    c.assign([TopicPartition(topic, 0, OFFSET_BEGINNING), TopicPartition(topic, 1, OFFSET_BEGINNING)])
    try:
        c.pause([TopicPartition(topic, 0)])

        got = consume_n(c, 5, timeout=15)
        assert len(got) == 5
        assert all(m.partition() == 1 for m in got), (
            f"paused partition 0 must deliver nothing, got {[m.partition() for m in got]}"
        )
        assert not drain(c, 1.0), "nothing more should arrive while partition 0 is paused"

        c.resume([TopicPartition(topic, 0)])
        drained = consume_n(c, 5, timeout=15)
        assert len(drained) == 5, "partition 0 backlog must drain after resume"
        assert all(m.partition() == 0 for m in drained)
    finally:
        c.close()


def _txn_producer(bootstrap: str, txn_id: str):
    return producer(
        bootstrap,
        **{
            "transactional.id": txn_id,
            "enable.idempotence": True,
            "acks": "all",
            "message.timeout.ms": 10000,
        },
    )


def test_rdkafka_transactional_produce_commit(bootstrap, topic, group):
    p = _txn_producer(bootstrap, f"txn-commit-{topic}")
    p.init_transactions(10)
    p.begin_transaction()
    for i in range(5):
        p.produce(topic, value=f"txn-commit-{i}".encode())
    p.flush(10)
    p.commit_transaction(10)

    c = consumer(bootstrap, group, **{"isolation.level": "read_committed"})
    c.subscribe([topic])
    try:
        msgs = consume_n(c, 5, timeout=20)
        assert len(msgs) == 5, "committed transactional messages must be visible"
        assert payloads(msgs) == [f"txn-commit-{i}" for i in range(5)]
    finally:
        c.close()


def test_rdkafka_transactional_produce_abort(bootstrap, topic, group):
    p = _txn_producer(bootstrap, f"txn-abort-{topic}")
    p.init_transactions(10)
    p.begin_transaction()
    for i in range(5):
        p.produce(topic, value=f"txn-abort-{i}".encode())
    p.flush(10)
    p.abort_transaction(10)

    c = consumer(bootstrap, group, **{"isolation.level": "read_committed"})
    c.subscribe([topic])
    try:
        assert not drain(c, 3.0), "aborted transactional messages must not be visible"
    finally:
        c.close()


def test_rdkafka_batched_producer_mid_batch_seek(bootstrap, topic, group):
    """Regression: seeking to an offset in the middle of a produced batch must
    deliver exactly the records from that offset on."""
    N = 100
    p = producer(
        bootstrap,
        **{"message.timeout.ms": 10000, "linger.ms": 50, "batch.num.messages": 100},
    )
    produced = [produce_sync(p, topic, f"batched-{i}".encode()) for i in range(N)]

    mid = produced[N // 2]
    c = consumer(bootstrap, group)
    c.assign([TopicPartition(topic, mid.partition(), mid.offset())])
    try:
        msgs = consume_n(c, N // 2, timeout=20)
        assert len(msgs) == N // 2, f"expected {N // 2} from mid-batch offset, got {len(msgs)}"
        assert msgs[0].offset() == mid.offset()
        assert msgs[0].value() == mid.value()
        assert payloads(msgs) == [f"batched-{i}" for i in range(N // 2, N)]
    finally:
        c.close()
