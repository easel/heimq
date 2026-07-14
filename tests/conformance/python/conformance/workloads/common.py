"""Helpers shared by workloads."""

import time

from confluent_kafka.admin import AdminClient, NewTopic


def create_topic(bootstrap: str, topic: str, *, partitions: int = 1) -> None:
    """Create a topic, tolerating one that already exists."""
    admin = AdminClient(
        {"bootstrap.servers": bootstrap, "socket.timeout.ms": 15000}
    )
    futures = admin.create_topics(
        [NewTopic(topic, num_partitions=partitions, replication_factor=1)],
        request_timeout=15,
    )
    for name, fut in futures.items():
        try:
            fut.result()
        except Exception as e:  # noqa: BLE001
            if "TOPIC_ALREADY_EXISTS" in str(e) or "already exists" in str(e):
                continue
            raise RuntimeError(f"create_topic({name}) failed: {e}") from e

    # CreateTopics completion does not guarantee that every broker has observed
    # the elected leader yet. Raw Produce requests do not have the metadata
    # refresh/retry loop supplied by a normal producer, so wait until the topic
    # is actually writable before those workloads begin.
    deadline = time.monotonic() + 15
    last_state = "metadata unavailable"
    while time.monotonic() < deadline:
        try:
            metadata = admin.list_topics(topic=topic, timeout=5)
            topic_metadata = metadata.topics.get(topic)
            if topic_metadata is None:
                last_state = "topic missing from metadata"
            elif topic_metadata.error is not None:
                last_state = str(topic_metadata.error)
            elif len(topic_metadata.partitions) != partitions:
                last_state = (
                    f"expected {partitions} partitions, "
                    f"found {len(topic_metadata.partitions)}"
                )
            elif all(
                partition.error is None and partition.leader >= 0
                for partition in topic_metadata.partitions.values()
            ):
                return
            else:
                last_state = "one or more partitions have no elected leader"
        except Exception as e:  # noqa: BLE001
            last_state = str(e)
        time.sleep(0.1)

    raise RuntimeError(f"topic {topic} did not become writable: {last_state}")
