"""Shared helpers for the rdkafka-equivalent tests.

The Rust originals used rdkafka (librdkafka). confluent-kafka is the same C
library behind a Python API, so semantics carry over exactly -- including the
delivery-report, commit and seek behaviour the tests assert on.

Every one of those tests was #[ignore]d in Rust ("can segfault in cargo test"),
which is an in-process linking problem. Here the client is a separate process in
its own container, so they simply run.
"""

import time

from confluent_kafka import Consumer, KafkaError, Producer


def producer(bootstrap: str, **cfg) -> Producer:
    conf = {"bootstrap.servers": bootstrap, "message.timeout.ms": 5000}
    conf.update(cfg)
    return Producer(conf)


def consumer(bootstrap: str, group: str, **cfg) -> Consumer:
    conf = {
        "bootstrap.servers": bootstrap,
        "group.id": group,
        "auto.offset.reset": "earliest",
        "enable.auto.commit": False,
    }
    conf.update(cfg)
    return Consumer(conf)


def group_consumer(bootstrap: str, group: str, session_timeout_ms: int = 30000, **cfg):
    return consumer(
        bootstrap,
        group,
        **{
            "session.timeout.ms": session_timeout_ms,
            "heartbeat.interval.ms": 1000,
            **cfg,
        },
    )


def produce_sync(p: Producer, topic: str, value, key=None, partition=None, headers=None):
    """Produce one record and block until the broker acks it.

    Returns the delivered Message, so tests can assert on partition and offset.
    """
    delivered = []

    def cb(err, msg):
        delivered.append((err, msg))

    kwargs = {}
    if key is not None:
        kwargs["key"] = key
    if partition is not None:
        kwargs["partition"] = partition
    if headers is not None:
        kwargs["headers"] = headers

    p.produce(topic, value=value, on_delivery=cb, **kwargs)
    p.flush(15)
    if not delivered:
        raise AssertionError("no delivery report")
    err, msg = delivered[0]
    if err is not None:
        raise AssertionError(f"delivery failed: {err}")
    return msg


def produce_n(p: Producer, topic: str, n: int, value_fmt="msg-{}", key_fmt=None):
    out = []
    for i in range(n):
        key = key_fmt.format(i).encode() if key_fmt else None
        out.append(produce_sync(p, topic, value_fmt.format(i).encode(), key=key))
    return out


def consume_n(c: Consumer, n: int, timeout: float = 10.0):
    """Poll until n messages arrive or the deadline passes. Returns messages."""
    msgs = []
    deadline = time.monotonic() + timeout
    while len(msgs) < n and time.monotonic() < deadline:
        m = c.poll(0.1)
        if m is None:
            continue
        if m.error():
            if m.error().code() == KafkaError._PARTITION_EOF:
                continue
            raise AssertionError(f"consumer error: {m.error()}")
        msgs.append(m)
    return msgs


def drain(c: Consumer, seconds: float = 1.0):
    """Collect whatever arrives within `seconds`."""
    msgs = []
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        m = c.poll(0.1)
        if m is None:
            continue
        if m.error():
            if m.error().code() == KafkaError._PARTITION_EOF:
                continue
            raise AssertionError(f"consumer error: {m.error()}")
        msgs.append(m)
    return msgs


def payloads(msgs) -> list[str]:
    return [m.value().decode() for m in msgs]
