"""InitProducerId against a transactional id with an open transaction.

Producer A opens a transaction and produces without committing. A second
InitProducerId for the same transactional.id must not succeed immediately: the
coordinator has to abort A's in-flight transaction first, and Kafka signals that
by returning CONCURRENT_TRANSACTIONS (51) until it has.

This is sent as a raw frame on purpose. Every client library retries 51
internally -- librdkafka's init_transactions() loops on it -- so a workload built
on a client can never observe the broker's actual first answer. That is why the
Rust harness, which drove everything through rdkafka, could not see this.
"""

from confluent_kafka import Producer

from ..observation import Observation, error_code
from ..raw_protocol import RawKafkaClient
from ..targets import Target
from .common import create_topic

NAME = "concurrent_transactions"
TOPIC = "parity-concurrent-txn"
TXN_ID = "parity-concurrent-txn-id"


def run(target: Target) -> list[Observation]:
    bootstrap = target.bootstrap_servers
    create_topic(bootstrap, TOPIC, partitions=1)

    # Producer A: open a transaction, produce, do not commit. The produce is
    # what registers the partition with the coordinator, making the transaction
    # genuinely in-flight rather than merely begun client-side.
    producer_a = Producer(
        {
            "bootstrap.servers": bootstrap,
            "socket.timeout.ms": 15000,
            "transactional.id": TXN_ID,
            "transaction.timeout.ms": 30000,
        }
    )
    producer_a.init_transactions(10)
    producer_a.begin_transaction()
    producer_a.produce(TOPIC, key=b"open", value=b"open")
    producer_a.flush(10)

    # One raw InitProducerId for the same id. Observe whatever the broker says
    # first, with no retry.
    with RawKafkaClient(bootstrap) as client:
        client.api_versions()
        code, _, _ = client.init_producer_id_raw(transactional_id=TXN_ID, timeout_ms=30_000)

    try:
        producer_a.abort_transaction(10)
    except Exception:  # noqa: BLE001 - A may already be fenced; teardown only
        pass

    return [
        Observation(
            workload=NAME, step=0, event=error_code(api="InitProducerId", code=code)
        )
    ]
