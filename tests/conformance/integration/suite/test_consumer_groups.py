"""Consumer group, offset and seek tests.

Ported from integration.rs (`test_rdkafka_consumer_group_*`, `..._seek_*`,
`..._group_*`, `..._auto_*`, `..._manual_commit_*`). All were #[ignore]d in Rust.
"""

import time

from confluent_kafka import OFFSET_BEGINNING, OFFSET_END, OFFSET_INVALID, TopicPartition

from .helpers import (
    consume_n,
    consumer,
    drain,
    group_consumer,
    payloads,
    produce_n,
    produce_sync,
    producer,
)

# @covers US-002-AC1 US-002-AC3


def test_rdkafka_consumer_group_join(bootstrap, topic, group):
    p = producer(bootstrap)
    produce_n(p, topic, 5, "join-test-{}", "key")

    c = consumer(bootstrap, group)
    c.subscribe([topic])
    try:
        msgs = consume_n(c, 5)
        assert len(msgs) == 5, "Consumer should have joined group and received all messages"
    finally:
        c.close()


# @covers US-002-AC4
def test_rdkafka_consumer_group_offset_commit(bootstrap, topic, group):
    p = producer(bootstrap)
    produce_n(p, topic, 10, "offset-test-{}", "key")

    # First consumer: consume 5 and commit the fifth.
    c1 = consumer(bootstrap, group)
    c1.subscribe([topic])
    try:
        msgs = consume_n(c1, 5)
        assert len(msgs) == 5, "First consumer should receive 5 messages"
        c1.commit(message=msgs[-1], asynchronous=False)
    finally:
        c1.close()

    # Second consumer: must start at offset 5 or later.
    c2 = consumer(bootstrap, group)
    c2.subscribe([topic])
    try:
        msgs = consume_n(c2, 5)
        assert len(msgs) == 5, "Second consumer should receive remaining 5 messages"
        assert msgs[0].offset() >= 5, (
            f"Second consumer should start at offset 5 or later, got {msgs[0].offset()}"
        )
    finally:
        c2.close()


# @covers US-002-AC4
def test_rdkafka_consumer_group_manual_offset_fetch(bootstrap, topic, group):
    p = producer(bootstrap)
    produce_n(p, topic, 5, "fetch-test-{}", "key")

    c = consumer(bootstrap, group)
    c.subscribe([topic])
    try:
        msgs = consume_n(c, 5)
        assert len(msgs) == 5, "Should receive all messages"
        last = msgs[-1]

        c.commit(
            offsets=[TopicPartition(last.topic(), last.partition(), last.offset() + 1)],
            asynchronous=False,
        )
        committed = c.committed(
            [TopicPartition(last.topic(), last.partition())], timeout=5
        )
        assert committed, "Should have committed offset for partition"
        assert committed[0].offset == last.offset() + 1
    finally:
        c.close()


def test_rdkafka_multiple_consumers_same_group(bootstrap, topic, group):
    p = producer(bootstrap)
    produce_n(p, topic, 20, "multi-consumer-{}", "key-{}")

    c1 = group_consumer(bootstrap, group)
    c2 = group_consumer(bootstrap, group)
    c1.subscribe([topic])
    c2.subscribe([topic])
    try:
        total = 0
        deadline = time.monotonic() + 15
        while total < 20 and time.monotonic() < deadline:
            for c in (c1, c2):
                m = c.poll(0.05)
                if m is not None and not m.error():
                    total += 1
        assert total == 20, f"Should receive all 20 messages between both consumers, got {total}"
    finally:
        c1.close()
        c2.close()


# @covers US-002-AC1
def test_rdkafka_consumer_group_lifecycle(bootstrap, topic, group):
    p = producer(bootstrap)
    produce_n(p, topic, 5, "lifecycle-batch1-{}", "key")

    # Consumer joins, consumes, commits each message, then leaves.
    c1 = consumer(bootstrap, group)
    c1.subscribe([topic])
    try:
        msgs = consume_n(c1, 5)
        assert len(msgs) == 5, "Lifecycle step 2: should consume 5 messages"
        for m in msgs:
            c1.commit(message=m, asynchronous=False)
    finally:
        c1.close()

    produce_n(p, topic, 5, "lifecycle-batch2-{}", "key")

    # New consumer in the same group resumes from the committed offset.
    c2 = consumer(bootstrap, group)
    c2.subscribe([topic])
    try:
        msgs = consume_n(c2, 5)
        assert len(msgs) == 5, "Lifecycle step 4: should consume 5 new messages"
        for payload in payloads(msgs):
            assert "batch2" in payload, f"Should receive batch2 messages, got: {payload}"
    finally:
        c2.close()


# @covers US-002-AC3
def test_rdkafka_consumer_rebalance_on_new_member(bootstrap, topic, group):
    p = producer(bootstrap)
    produce_n(p, topic, 10, "rebalance-{}", "key-{}")

    c1 = group_consumer(bootstrap, group)
    c1.subscribe([topic])
    total = 0
    for _ in range(5):
        m = c1.poll(0.5)
        if m is not None and not m.error():
            total += 1

    c2 = group_consumer(bootstrap, group)
    c2.subscribe([topic])
    try:
        deadline = time.monotonic() + 15
        while total < 10 and time.monotonic() < deadline:
            for c in (c1, c2):
                m = c.poll(0.1)
                if m is not None and not m.error():
                    total += 1
        assert total >= 10, f"Should receive all messages even after rebalance, got {total}"
    finally:
        c1.close()
        c2.close()


def test_rdkafka_consumer_group_empty_topic(bootstrap, topic, group):
    """An empty topic must time out, not error."""
    c = consumer(bootstrap, group)
    c.subscribe([topic])
    try:
        m = c.poll(0.5)
        if m is not None and not m.error():
            raise AssertionError("Should not receive message from empty topic")
    finally:
        c.close()


def test_rdkafka_consumer_seek_to_beginning(bootstrap, topic, group):
    p = producer(bootstrap)
    produce_n(p, topic, 10, "seek-{}", "key")

    c = consumer(bootstrap, group)
    c.subscribe([topic])
    try:
        msgs = consume_n(c, 10)
        assert len(msgs) == 10, "Should receive all 10 messages initially"
        for m in msgs:
            c.commit(message=m, asynchronous=False)

        for tp in c.assignment():
            c.seek(TopicPartition(tp.topic, tp.partition, OFFSET_BEGINNING))

        again = consume_n(c, 10)
        assert len(again) == 10, "Should re-receive all 10 messages after seek to beginning"
    finally:
        c.close()


def test_rdkafka_seek_to_end(bootstrap, topic, group):
    p = producer(bootstrap)
    produce_n(p, topic, 10, "initial-{}", "key")

    c = consumer(bootstrap, group)
    c.subscribe([topic])
    try:
        msgs = consume_n(c, 10)
        assert len(msgs) == 10, "Should receive all 10 initial messages"

        assignment = c.assignment()
        assert assignment, "consumer must have an assignment before seeking"
        for tp in assignment:
            c.seek(TopicPartition(tp.topic, tp.partition, OFFSET_END))

        # No old message may be re-delivered.
        stale = drain(c, 0.5)
        assert not stale, f"Unexpected message after seek to end: {payloads(stale)}"

        # librdkafka discards the first Fetch response after a seek; let the
        # server's long-poll (capped at 500ms) expire before producing, or the
        # post-seek messages are skipped.
        time.sleep(0.2)

        produce_n(p, topic, 5, "after-seek-{}", "key")

        got = consume_n(c, 5, timeout=15)
        assert len(got) == 5, "Should receive exactly the 5 post-seek messages"
        for m in got:
            assert m.value().decode().startswith("after-seek-")
            assert m.offset() >= 10, f"offset {m.offset()} < 10"
    finally:
        c.close()


def test_rdkafka_seek_to_offset(bootstrap, topic, group):
    p = producer(bootstrap)
    produced = [
        (produce_sync(p, topic, f"seek-off-{i}".encode(), key=b"key").offset(), f"seek-off-{i}")
        for i in range(10)
    ]

    c = consumer(bootstrap, group)
    c.subscribe([topic])
    try:
        msgs = consume_n(c, 10)
        assert len(msgs) == 10

        target_offset, target_payload = produced[5]
        tp = c.assignment()[0]
        c.seek(TopicPartition(tp.topic, tp.partition, target_offset))

        got = consume_n(c, 5)
        assert len(got) == 5, f"Should receive 5 messages from offset {target_offset}"
        assert got[0].offset() == target_offset
        assert got[0].value().decode() == target_payload
    finally:
        c.close()


def test_rdkafka_group_multi_partition_delivery(bootstrap_3p, topic, group):
    """300 keyed messages across 3 partitions must all be delivered exactly once."""
    p = producer(bootstrap_3p)
    produced = set()
    for i in range(300):
        m = produce_sync(p, topic, f"val-{i}".encode(), key=f"key-{i:06d}".encode())
        produced.add((m.partition(), m.offset()))

    assert len(produced) == 300, "each produced message must have a unique (partition, offset)"
    partitions = {pt for pt, _ in produced}
    assert len(partitions) >= 3, f"expected messages to span all 3 partitions, got {partitions}"

    c = consumer(bootstrap_3p, group)
    c.subscribe([topic])
    try:
        msgs = consume_n(c, 300, timeout=30)
        assert len(msgs) == 300, f"expected 300 delivered, got {len(msgs)}"
        delivered = {(m.partition(), m.offset()) for m in msgs}
        assert delivered == produced, "delivered set must equal produced set"
    finally:
        c.close()


def test_rdkafka_group_rebalance_on_graceful_leave(bootstrap_3p, topic, group):
    """After consumer 2 leaves gracefully, consumer 1 must own all 3 partitions."""
    p = producer(bootstrap_3p)
    produce_n(p, topic, 9, "leave-{}", "key-{}")

    c1 = group_consumer(bootstrap_3p, group)
    c2 = group_consumer(bootstrap_3p, group)
    c1.subscribe([topic])
    c2.subscribe([topic])

    for _ in range(40):
        c1.poll(0.05)
        c2.poll(0.05)

    c2.close()  # graceful LeaveGroup

    try:
        deadline = time.monotonic() + 30
        owned = set()
        while time.monotonic() < deadline:
            c1.poll(0.1)
            owned = {tp.partition for tp in c1.assignment()}
            if owned == {0, 1, 2}:
                break
        assert owned == {0, 1, 2}, f"consumer 1 should own all partitions, owns {owned}"
    finally:
        c1.close()


def test_rdkafka_group_rebalance_on_session_timeout(bootstrap_3p, topic, group):
    """When consumer 2 stops polling, its session expires and consumer 1 takes over."""
    p = producer(bootstrap_3p)
    produce_n(p, topic, 9, "timeout-{}", "key-{}")

    c1 = group_consumer(bootstrap_3p, group)
    c2 = group_consumer(
        bootstrap_3p,
        group,
        session_timeout_ms=6000,
        **{"max.poll.interval.ms": 10000, "heartbeat.interval.ms": 1000},
    )
    c1.subscribe([topic])
    c2.subscribe([topic])

    for _ in range(40):
        c1.poll(0.05)
        c2.poll(0.05)

    # Stop pumping c2 but keep it open: closing would send an immediate
    # LeaveGroup, which is the graceful path, not the watchdog path.
    try:
        deadline = time.monotonic() + 60
        owned = set()
        while time.monotonic() < deadline:
            c1.poll(0.2)
            owned = {tp.partition for tp in c1.assignment()}
            if owned == {0, 1, 2}:
                break
        assert owned == {0, 1, 2}, (
            f"consumer 1 should take over all partitions after c2's session times out, owns {owned}"
        )
    finally:
        c1.close()
        c2.close()


# @covers US-002-AC4
def test_rdkafka_group_resume_from_committed(bootstrap, topic, group):
    """A new consumer with auto.offset.reset=latest must still resume from the
    committed offset. If the commit were ignored it would see nothing."""
    TOTAL, COMMIT_AFTER = 10, 4
    p = producer(bootstrap)
    produce_n(p, topic, TOTAL, "resume-test-{}", "key")

    a = consumer(bootstrap, group)
    a.subscribe([topic])
    try:
        msgs = consume_n(a, COMMIT_AFTER)
        assert len(msgs) == COMMIT_AFTER
        last = msgs[-1]
        a.commit(
            offsets=[TopicPartition(last.topic(), last.partition(), last.offset() + 1)],
            asynchronous=False,
        )
    finally:
        a.close()

    b = consumer(bootstrap, group, **{"auto.offset.reset": "latest"})
    b.subscribe([topic])
    try:
        tail = consume_n(b, TOTAL - COMMIT_AFTER, timeout=15)
        extra = drain(b, 1.0)
        assert not extra, f"unexpected extra messages: {payloads(extra)}"
        assert len(tail) == TOTAL - COMMIT_AFTER, (
            f"Consumer B should resume from committed offset and see only the "
            f"{TOTAL - COMMIT_AFTER} uncommitted tail messages, got {len(tail)}"
        )
        assert tail[0].value().decode() == f"resume-test-{COMMIT_AFTER}"
        assert tail[-1].value().decode() == f"resume-test-{TOTAL - 1}"
    finally:
        b.close()


def test_rdkafka_independent_consumer_groups(bootstrap, topic):
    """Offsets are scoped per group: G1's commit must not move G2."""
    N = 8
    g1, g2 = "indep-g1", "indep-g2"
    p = producer(bootstrap)
    produce_n(p, topic, N, "indep-{}", "key")

    c1 = consumer(bootstrap, g1)
    c1.subscribe([topic])
    try:
        m1 = consume_n(c1, N)
        assert len(m1) == N, "Group 1 should see all messages"
        fourth = m1[3]
        c1.commit(
            offsets=[TopicPartition(fourth.topic(), fourth.partition(), fourth.offset() + 1)],
            asynchronous=False,
        )
    finally:
        c1.close()

    c2 = consumer(bootstrap, g2)
    c2.subscribe([topic])
    try:
        m2 = consume_n(c2, N)
        assert len(m2) == N, "Group 2 should see all messages, unaffected by G1's commit"

        g2_committed = c2.committed(
            [TopicPartition(topic, fourth.partition())], timeout=5
        )[0].offset
        assert g2_committed != fourth.offset() + 1, (
            "G2's committed offset must not reflect G1's commit"
        )
    finally:
        c2.close()


def test_rdkafka_auto_offset_reset_earliest_vs_latest(bootstrap, topic):
    """earliest sees pre-existing messages; latest sees only subsequent ones."""
    M = 5
    p = producer(bootstrap)
    produce_n(p, topic, M, "pre-{}", "key")

    c1 = consumer(bootstrap, "aor-earliest-grp")
    c1.subscribe([topic])
    try:
        msgs = consume_n(c1, M)
        assert len(msgs) == M, "earliest must receive all pre-existing messages"
        assert payloads(msgs) == [f"pre-{i}" for i in range(M)]
    finally:
        c1.close()

    c2 = consumer(bootstrap, "aor-latest-grp", **{"auto.offset.reset": "latest"})
    c2.subscribe([topic])
    try:
        # Let C2 join and set its position to latest before any new produce.
        stale = drain(c2, 3.0)
        assert not stale, f"latest must not see pre-existing messages, got {payloads(stale)}"

        produce_n(p, topic, M, "post-{}", "key")
        got = consume_n(c2, M, timeout=15)
        assert len(got) == M, "latest must see messages produced after it joined"
        assert payloads(got) == [f"post-{i}" for i in range(M)]
    finally:
        c2.close()


def test_rdkafka_manual_commit_offset_roundtrip(bootstrap, topic, group):
    """What consumer A commits, a fresh consumer B must read back via OffsetFetch."""
    K = 5
    p = producer(bootstrap)
    produce_n(p, topic, K, "manual-{}", "key")

    a = consumer(bootstrap, group)
    a.subscribe([topic])
    try:
        msgs = consume_n(a, K)
        assert len(msgs) == K
        for m in msgs:
            a.commit(message=m, asynchronous=False)
        last = msgs[-1]
        next_offset = last.offset() + 1
        a.commit(
            offsets=[TopicPartition(last.topic(), last.partition(), next_offset)],
            asynchronous=False,
        )
    finally:
        a.close()

    b = consumer(bootstrap, group)
    try:
        committed = b.committed(
            [TopicPartition(topic, last.partition(), OFFSET_INVALID)], timeout=5
        )
        assert committed[0].offset == next_offset, (
            f"B should observe A's committed offset {next_offset}, got {committed[0].offset}"
        )
    finally:
        b.close()


def test_rdkafka_auto_commit_interval(bootstrap, topic, group):
    """The background auto-commit thread must commit the stored offset."""
    K, INTERVAL_MS = 5, 500
    p = producer(bootstrap)
    produce_n(p, topic, 10, "autocommit-{}", "key")

    a = consumer(
        bootstrap,
        group,
        **{
            "enable.auto.commit": True,
            "auto.commit.interval.ms": INTERVAL_MS,
            "enable.auto.offset.store": False,
        },
    )
    a.subscribe([topic])
    try:
        msgs = consume_n(a, K)
        assert len(msgs) == K
        a.store_offsets(message=msgs[-1])
        # Poll past the interval so the background committer runs.
        deadline = time.monotonic() + (INTERVAL_MS / 1000.0) * 6
        while time.monotonic() < deadline:
            a.poll(0.1)
    finally:
        a.close()

    b = consumer(bootstrap, group)
    b.subscribe([topic])
    try:
        msgs = consume_n(b, 10 - K, timeout=15)
        assert msgs, "B should resume after the auto-committed offset"
        assert msgs[0].offset() >= K, (
            f"B should resume at or after offset {K}, got {msgs[0].offset()}"
        )
    finally:
        b.close()
