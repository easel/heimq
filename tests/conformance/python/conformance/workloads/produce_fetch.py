"""Produce/fetch round-trip (SD-003 §Initial Workloads).

Produces N records with deterministic key/value, then consumes them all from
offset 0 using a direct partition assignment (no consumer group). Emits one
RecordConsumed observation per record.

Ported from parity/workloads/produce_fetch.rs.
"""

import time

from confluent_kafka import Consumer, OFFSET_BEGINNING, Producer, TopicPartition

from ..observation import Observation, record_consumed
from ..targets import Target
from .common import create_topic

NAME = "produce_fetch_roundtrip"
TOPIC = "parity-produce-fetch"
N = 10

# @covers US-001-AC4 US-005-AC4 US-005-AC5


def run(target: Target) -> list[Observation]:
    bootstrap = target.bootstrap_servers
    create_topic(bootstrap, TOPIC, partitions=1)

    producer = Producer(
        {"bootstrap.servers": bootstrap, "socket.timeout.ms": 15000}
    )
    for i in range(N):
        producer.produce(TOPIC, key=f"key-{i}".encode(), value=f"val-{i}".encode())
    remaining = producer.flush(15)
    if remaining:
        raise RuntimeError(f"{NAME}: {remaining} messages undelivered after flush")

    consumer = Consumer(
        {
            "bootstrap.servers": bootstrap,
            "socket.timeout.ms": 15000,
            "group.id": "parity-produce-fetch-probe",
            "enable.auto.commit": False,
        }
    )
    consumer.assign([TopicPartition(TOPIC, 0, OFFSET_BEGINNING)])

    observations: list[Observation] = []
    try:
        deadline = time.monotonic() + 30
        while len(observations) < N:
            if time.monotonic() > deadline:
                raise RuntimeError(
                    f"{NAME}: timed out after consuming only "
                    f"{len(observations)}/{N} records"
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
                        timestamp=0,  # normalized away
                    ),
                )
            )
    finally:
        consumer.close()

    return observations
