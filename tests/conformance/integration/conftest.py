"""Fixtures for the heimq integration suite.

The Rust original started a fresh in-process `TestServer` per test. Here every
test talks to the shipped `heimq` binary in a container over TCP, and isolation
comes from per-test topic names instead of per-test brokers.

Two brokers, because auto-create cannot be both on and off in one process:
`bootstrap` has topic auto-creation enabled, `bootstrap_noac` has it disabled.
"""

import os
import time
import uuid

import pytest
from confluent_kafka.admin import AdminClient


def _wait_ready(bootstrap: str, attempts: int = 60) -> None:
    admin = AdminClient({"bootstrap.servers": bootstrap})
    last: Exception | None = None
    for _ in range(attempts):
        try:
            admin.list_topics(timeout=5)
            return
        except Exception as e:  # noqa: BLE001 - probe; any failure is a retry
            last = e
            time.sleep(1)
    raise RuntimeError(f"broker not ready at {bootstrap}: {last}")


@pytest.fixture(scope="session")
def bootstrap() -> str:
    b = os.environ["HEIMQ_BOOTSTRAP"]
    _wait_ready(b)
    return b


@pytest.fixture(scope="session")
def bootstrap_noac() -> str:
    """A heimq with HEIMQ_AUTO_CREATE_TOPICS=false."""
    b = os.environ["HEIMQ_NOAC_BOOTSTRAP"]
    _wait_ready(b)
    return b


@pytest.fixture
def topic() -> str:
    """A topic name unique to this test, standing in for a fresh broker."""
    return f"it-{uuid.uuid4().hex[:12]}"


@pytest.fixture(scope="session")
def bootstrap_3p() -> str:
    """A heimq with HEIMQ_DEFAULT_PARTITIONS=3."""
    b = os.environ["HEIMQ_3P_BOOTSTRAP"]
    _wait_ready(b)
    return b


@pytest.fixture
def group() -> str:
    """A consumer group unique to this test."""
    return f"grp-{uuid.uuid4().hex[:12]}"


@pytest.fixture(scope="session")
def bootstrap_2p() -> str:
    """A heimq with HEIMQ_DEFAULT_PARTITIONS=2."""
    b = os.environ["HEIMQ_2P_BOOTSTRAP"]
    _wait_ready(b)
    return b


@pytest.fixture(scope="session")
def bootstrap_4p() -> str:
    """A heimq with HEIMQ_DEFAULT_PARTITIONS=4."""
    b = os.environ["HEIMQ_4P_BOOTSTRAP"]
    _wait_ready(b)
    return b
