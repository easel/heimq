"""Idempotent produce round-trip (US-003-AC5).

Produces N records with enable.idempotence=true, then consumes from offset 0.
The broker must correctly handle InitProducerId and sequence tracking. Emits
observations identical in shape to produce_fetch so the diff engine can compare.

Ported from parity/workloads/idempotent_produce.rs.
"""

import time

from confluent_kafka import Consumer, OFFSET_BEGINNING, Producer, TopicPartition

from ..observation import Observation, record_consumed
from ..targets import Target
from .common import create_topic

NAME = "idempotent_produce_roundtrip"
TOPIC = "parity-idempotent-produce"
N = 10

# @covers US-003-AC5


def run(target: Target) -> list[Observation]:
    bootstrap = target.bootstrap_servers
    create_topic(bootstrap, TOPIC, partitions=1)

    producer = Producer(
        {
            "bootstrap.servers": bootstrap,
            "socket.timeout.ms": 15000,
            "enable.idempotence": True,
        }
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
            "group.id": "parity-idempotent-probe",
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
                    ),
                )
            )
    finally:
        consumer.close()

    return observations
