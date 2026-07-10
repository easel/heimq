"""Legacy protocol tests.

The Rust originals used the pure-Rust `kafka` crate to emit the v0/v1 message
format. kafka-python-ng is the equivalent here: pure Python, and with
api_version below (0, 11) it produces the legacy MessageSet rather than a v2
RecordBatch. confluent-kafka cannot emit these formats at all.

Ported from crates/heimq/tests/integration.rs (the `legacy_*` tests, which were
the only integration tests that actually ran under `cargo test` -- the rdkafka
ones were all #[ignore]d).
"""

import threading

import pytest
from kafka import KafkaConsumer, KafkaProducer

LEGACY_API = (0, 10, 0)


def legacy_producer(bootstrap: str) -> KafkaProducer:
    return KafkaProducer(
        bootstrap_servers=bootstrap,
        api_version=LEGACY_API,
        acks=1,
        retries=0,
        max_request_size=8 * 1024 * 1024,
    )


def send(producer: KafkaProducer, topic: str, value: bytes, key: bytes | None = None):
    """Send and block on the broker's ack, so a rejection surfaces as an error."""
    return producer.send(topic, value=value, key=key).get(timeout=10)


def topic_names(bootstrap: str) -> set[str]:
    c = KafkaConsumer(bootstrap_servers=bootstrap, api_version=LEGACY_API)
    try:
        return set(c.topics())
    finally:
        c.close()


def test_metadata_fetch(bootstrap):
    # Should connect and fetch metadata without error.
    assert isinstance(topic_names(bootstrap), set)


def test_legacy_produce(bootstrap, topic):
    p = legacy_producer(bootstrap)
    try:
        for i in range(10):
            send(p, topic, f"message-{i}".encode())
    finally:
        p.close()


def test_legacy_produce_with_key(bootstrap, topic):
    p = legacy_producer(bootstrap)
    try:
        send(p, topic, b"my-value", key=b"my-key")
    finally:
        p.close()


def test_legacy_empty_message(bootstrap, topic):
    p = legacy_producer(bootstrap)
    try:
        send(p, topic, b"")
    finally:
        p.close()


def test_legacy_multiple_topics(bootstrap, topic):
    topics = [f"{topic}-{i}" for i in range(5)]
    p = legacy_producer(bootstrap)
    try:
        for t in topics:
            send(p, t, f"msg-to-{t}".encode())
    finally:
        p.close()

    names = topic_names(bootstrap)
    for t in topics:
        assert t in names, f"Should have topic {t}"


def test_legacy_large_batch_produce(bootstrap, topic):
    p = legacy_producer(bootstrap)
    try:
        for i in range(150):
            send(p, topic, f"batch-message-{i:05d}".encode())
    finally:
        p.close()
    assert topic in topic_names(bootstrap), "Topic should exist after producing"


def test_legacy_binary_payload_produce(bootstrap, topic):
    payloads = [
        bytes([0x00, 0x01, 0x02, 0xFF, 0xFE, 0xFD]),  # raw bytes
        bytes([0x00, 0x00, 0x00, 0x00]),  # all nulls
        bytes(range(256)),  # every byte value
        bytes([0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]),  # PNG header
        bytes([0x1F, 0x8B, 0x08, 0x00]),  # gzip header
        bytes([0xEF, 0xBB, 0xBF]) + b"hello",  # UTF-8 BOM + hello
        bytes([0x80, 0x81, 0x82, 0x83]),  # invalid UTF-8
    ]
    p = legacy_producer(bootstrap)
    try:
        for payload in payloads:
            send(p, topic, payload)
    finally:
        p.close()
    assert topic in topic_names(bootstrap), "Topic should exist"


def test_legacy_empty_topic_name_handling(bootstrap):
    """An empty topic name must be rejected or accepted -- never hang or crash."""
    p = legacy_producer(bootstrap)
    try:
        try:
            send(p, "", b"x")
        except Exception:  # noqa: BLE001 - either outcome is valid
            pass
    finally:
        p.close()
    # The broker must still answer afterwards.
    assert isinstance(topic_names(bootstrap), set)


def test_legacy_metadata_refresh_after_topic_creation(bootstrap, topic):
    before = topic_names(bootstrap)
    new_topics = [f"{topic}-refresh-{i}" for i in range(3)]
    assert not (before & set(new_topics))

    p = legacy_producer(bootstrap)
    try:
        for t in new_topics:
            send(p, t, b"init")
    finally:
        p.close()

    after = topic_names(bootstrap)
    for t in new_topics:
        assert t in after, f"Should see topic {t} after refresh"


def test_legacy_various_message_formats(bootstrap, topic):
    keyed = [
        (b"key1", b"value1"),
        (b"key2", b""),  # empty value
        (b"key3", b"value-for-key3"),
        (b"key-with-unicode", b"val"),
        (b"k", b"large-value-placeholder"),
    ]
    p = legacy_producer(bootstrap)
    try:
        for k, v in keyed:
            send(p, topic, v, key=k)
        send(p, topic, b"value-with-null-key")  # null key
    finally:
        p.close()
    assert topic in topic_names(bootstrap), "Topic should exist"


def test_legacy_produce_many_small_messages(bootstrap, topic):
    p = legacy_producer(bootstrap)
    try:
        for i in range(100):
            send(p, topic, f"small-{i}".encode())
    finally:
        p.close()
    assert topic in topic_names(bootstrap), "Topic should exist"


def test_legacy_produce_varying_sizes(bootstrap, topic):
    p = legacy_producer(bootstrap)
    try:
        for size in (1, 10, 100, 1000, 10000, 50000):
            send(p, topic, b"x" * size)
    finally:
        p.close()
    assert topic in topic_names(bootstrap), "Topic should exist"


def test_legacy_sequential_produce(bootstrap, topic):
    p = legacy_producer(bootstrap)
    try:
        for i in range(50):
            send(p, topic, f"{i:08d}".encode())  # zero-padded, easy to parse
    finally:
        p.close()
    assert topic in topic_names(bootstrap), "Topic should exist"


def test_legacy_concurrent_producers(bootstrap, topic):
    errors: list[BaseException] = []

    def produce(producer_id: int) -> None:
        try:
            p = legacy_producer(bootstrap)
            try:
                for i in range(20):
                    send(p, topic, f"producer-{producer_id}-msg-{i}".encode())
            finally:
                p.close()
        except BaseException as e:  # noqa: BLE001 - reported on the main thread
            errors.append(e)

    threads = [threading.Thread(target=produce, args=(i,)) for i in range(3)]
    for t in threads:
        t.start()
    for t in threads:
        t.join()

    assert not errors, f"producer thread failed: {errors}"
    assert topic in topic_names(bootstrap), "Topic should exist after concurrent production"


@pytest.mark.parametrize(
    "suffix",
    ["with-dashes", "with_underscores", "with.dots", "WithCamelCase", "123numbers"],
)
def test_legacy_special_characters_in_topic_name(bootstrap, topic, suffix):
    name = f"{topic}-{suffix}"
    p = legacy_producer(bootstrap)
    try:
        send(p, name, b"test")
    finally:
        p.close()
    assert name in topic_names(bootstrap)


def test_legacy_rapid_metadata_refresh(bootstrap):
    for _ in range(10):
        assert isinstance(topic_names(bootstrap), set)


def test_legacy_keyed_messages_produce(bootstrap, topic):
    pairs = [
        (b"user:1", b"alice"),
        (b"user:2", b"bob"),
        (b"user:3", b"charlie"),
        (b"order:100", b"pending"),
        (b"order:101", b"completed"),
    ]
    p = legacy_producer(bootstrap)
    try:
        for k, v in pairs:
            send(p, topic, v, key=k)
    finally:
        p.close()
    assert topic in topic_names(bootstrap), "Topic should exist"
