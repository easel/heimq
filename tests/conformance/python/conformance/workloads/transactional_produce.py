"""Transactional produce + read_committed consume (US-004-AC6).

Commits one batch of 3 records, aborts another. A read_committed consumer
should see only the 3 committed records.

Ported from parity/workloads/transactional_produce.rs.
"""

import time

from confluent_kafka import Consumer, OFFSET_BEGINNING, Producer, TopicPartition

from ..observation import Observation, record_consumed
from ..targets import Target
from .common import create_topic

NAME = "transactional_produce_roundtrip"
TOPIC = "parity-txn-produce"
TXN_ID = "parity-txn-test"
COMMITTED = 3

# @covers US-004-AC6
# @covers US-005-AC7


def run(target: Target) -> list[Observation]:
    bootstrap = target.bootstrap_servers
    create_topic(bootstrap, TOPIC, partitions=1)

    producer = Producer(
        {
            "bootstrap.servers": bootstrap,
            "socket.timeout.ms": 15000,
            "transactional.id": TXN_ID,
            "transaction.timeout.ms": 30000,
        }
    )
    producer.init_transactions(10)

    # Transaction 1: commit 3 records.
    producer.begin_transaction()
    for i in range(COMMITTED):
        producer.produce(
            TOPIC, key=f"commit-key-{i}".encode(), value=f"commit-val-{i}".encode()
        )
    producer.flush(10)
    producer.commit_transaction(10)

    # Transaction 2: abort 3 records.
    producer.begin_transaction()
    for i in range(COMMITTED):
        producer.produce(
            TOPIC, key=f"abort-key-{i}".encode(), value=f"abort-val-{i}".encode()
        )
    producer.flush(10)
    producer.abort_transaction(10)

    consumer = Consumer(
        {
            "bootstrap.servers": bootstrap,
            "socket.timeout.ms": 15000,
            "group.id": "parity-txn-rc-probe",
            "enable.auto.commit": False,
            "isolation.level": "read_committed",
        }
    )
    consumer.assign([TopicPartition(TOPIC, 0, OFFSET_BEGINNING)])

    observations: list[Observation] = []
    try:
        deadline = time.monotonic() + 30
        while len(observations) < COMMITTED:
            if time.monotonic() > deadline:
                raise RuntimeError(
                    f"{NAME}: timed out, got {len(observations)} "
                    "read_committed records"
                )
            msg = consumer.poll(0.5)
            if msg is None:
                continue
            if msg.error():
                raise RuntimeError(f"consumer error: {msg.error()}")
            observations.append(
                Observation(
                    workload=NAME,
                    step=len(observations),
                    event=record_consumed(
                        key=msg.key(),
                        value=msg.value(),
                        partition=msg.partition(),
                        offset=msg.offset(),
                    ),
                )
            )
    finally:
        consumer.close()

    return observations
