"""Broker targets and readiness probing."""

import os
import time
from dataclasses import dataclass

from confluent_kafka.admin import AdminClient


@dataclass
class Target:
    name: str
    bootstrap_servers: str


def from_env() -> tuple[Target, list[Target]]:
    """Return (heimq, oracles). Kafka is canonical; Redpanda is a second oracle."""
    heimq = Target("heimq", os.environ["HEIMQ_BOOTSTRAP"])
    oracles = [
        Target("kafka", os.environ["KAFKA_BOOTSTRAP"]),
        Target("redpanda", os.environ["REDPANDA_BOOTSTRAP"]),
    ]
    return heimq, oracles


def wait_ready(target: Target, attempts: int = 60) -> None:
    """Poll metadata until the broker answers.

    KRaft startup lags the log line the old harness waited on, so retry rather
    than fail on the first refused probe.
    """
    admin = AdminClient({"bootstrap.servers": target.bootstrap_servers})
    last: Exception | None = None
    for _ in range(attempts):
        try:
            admin.list_topics(timeout=5)
            return
        except Exception as e:  # noqa: BLE001 - probe, any failure is a retry
            last = e
            time.sleep(1)
    raise RuntimeError(f"{target.name} not ready at {target.bootstrap_servers}: {last}")
