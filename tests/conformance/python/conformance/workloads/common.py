"""Helpers shared by workloads."""

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
