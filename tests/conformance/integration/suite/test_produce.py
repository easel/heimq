"""Produce and produce/consume round-trip tests.

Ported from integration.rs (`test_rdkafka_produce_*`, `..._roundtrip`,
`..._multiple_topics`). All of these were #[ignore]d in Rust.
"""

import time

from confluent_kafka.admin import AdminClient

from .helpers import consume_n, consumer, payloads, produce_n, produce_sync, producer

# @covers US-001-AC1
# @covers US-001-AC3
# @covers US-013-AC1


def test_rdkafka_simple_produce(bootstrap, topic):
    p = producer(bootstrap)
    for i in range(10):
        produce_sync(p, topic, f"message-{i}".encode(), key=f"key-{i}".encode())


def test_rdkafka_produce_with_key(bootstrap, topic):
    p = producer(bootstrap)
    produce_sync(p, topic, b"my-value", key=b"my-key")


def test_rdkafka_produce_no_key(bootstrap, topic):
    p = producer(bootstrap)
    produce_sync(p, topic, b"value-with-null-key")


def test_rdkafka_produce_empty_value(bootstrap, topic):
    p = producer(bootstrap)
    produce_sync(p, topic, b"", key=b"key")


def test_rdkafka_produce_large_message(bootstrap, topic):
    # 512KB: under librdkafka's 1MB default limit.
    p = producer(bootstrap, **{"message.timeout.ms": 10000})
    produce_sync(p, topic, b"x" * (512 * 1024), key=b"large")


def test_rdkafka_rapid_produce(bootstrap, topic):
    p = producer(bootstrap)
    produce_n(p, topic, 100, "rapid-{}", "k-{}")


# @covers US-001-AC1
# @covers US-001-AC2
# @covers US-001-AC3
def test_rdkafka_produce_consume_roundtrip(bootstrap, topic, group):
    p = producer(bootstrap)
    produce_n(p, topic, 5, "roundtrip-{}", "key-{}")

    c = consumer(bootstrap, group)
    c.subscribe([topic])
    try:
        msgs = consume_n(c, 5)
        assert len(msgs) == 5, f"Should have received 5 messages, got {len(msgs)}"
        assert all(m.value() is not None for m in msgs), "Message should have payload"
    finally:
        c.close()


def test_rdkafka_multi_partition_autocreate_roundtrip(bootstrap_3p, topic, group):
    """With default_partitions=3 and auto-create on, a fresh topic must appear in
    metadata after produce, and every produced message must be consumable.

    Reproduces the kcat smoke-test failure where produce succeeded at the
    delivery layer but a consumer saw "Unknown partition".
    """
    p = producer(bootstrap_3p)
    for k, v in (("k1", "first"), ("k2", "second"), ("k3", "third")):
        produce_sync(p, topic, v.encode(), key=k.encode())

    time.sleep(0.2)

    md = AdminClient({"bootstrap.servers": bootstrap_3p}).list_topics(timeout=10)
    assert topic in md.topics, f"Topic {topic} not present in metadata after produce"

    c = consumer(bootstrap_3p, group)
    c.subscribe([topic])
    try:
        msgs = consume_n(c, 3, timeout=15)
        assert len(msgs) == 3, f"Expected 3 messages across partitions, got {len(msgs)}"
        assert sorted(payloads(msgs)) == ["first", "second", "third"]
    finally:
        c.close()


def test_rdkafka_multiple_topics(bootstrap, topic):
    p = producer(bootstrap)
    for i in range(3):
        produce_sync(p, f"{topic}-{i}", b"test-message", key=b"key")
