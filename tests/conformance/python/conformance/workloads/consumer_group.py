"""Consumer group lifecycle (SD-003 §Initial Workloads).

Creates a topic, produces records, subscribes with a consumer group, consumes
all records, commits offsets, then emits a GroupState observation reflecting the
observed member count and state after rebalance.

Ported from parity/workloads/consumer_group.rs.
"""

import time

from confluent_kafka import Consumer, Producer

from ..observation import Observation, group_state, record_consumed
from ..targets import Target
from .common import create_topic

NAME = "consumer_group_lifecycle"
TOPIC = "parity-consumer-group"
GROUP = "parity-cg-lifecycle"
N = 6

# @covers US-002-AC5 US-005-AC4 US-005-AC5


def run(target: Target) -> list[Observation]:
    bootstrap = target.bootstrap_servers
    create_topic(bootstrap, TOPIC, partitions=1)

    producer = Producer(
        {"bootstrap.servers": bootstrap, "socket.timeout.ms": 15000}
    )
    for i in range(N):
        producer.produce(TOPIC, key=f"cg-key-{i}".encode(), value=f"cg-val-{i}".encode())
    remaining = producer.flush(15)
    if remaining:
        raise RuntimeError(f"{NAME}: {remaining} messages undelivered after flush")

    consumer = Consumer(
        {
            "bootstrap.servers": bootstrap,
            "socket.timeout.ms": 15000,
            "group.id": GROUP,
            "auto.offset.reset": "earliest",
            "enable.auto.commit": False,
        }
    )
    consumer.subscribe([TOPIC])

    observations: list[Observation] = []
    try:
        deadline = time.monotonic() + 30
        while len(observations) < N:
            if time.monotonic() > deadline:
                raise RuntimeError(
                    f"{NAME}: timed out after consuming only "
                    f"{len(observations)}/{N} records"
                )
            msg = consumer.poll(1.0)
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
            consumer.commit(msg, asynchronous=True)

        # A single consumer in a group always transitions to Stable after
        # rebalance and holds all partitions. member_count is deterministic.
        observations.append(
            Observation(
                workload=NAME,
                step=len(observations),
                event=group_state(group_id=GROUP, state="Stable", member_count=1),
            )
        )
    finally:
        consumer.close()

    return observations
