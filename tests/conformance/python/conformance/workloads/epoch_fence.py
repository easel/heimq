"""Zombie-producer fencing (fault injection).

Producer A opens a transaction and produces (without committing). Producer B,
sharing the same transactional.id, initialises transactions -- which fences
producer A by bumping the producer epoch. Producer A's subsequent commit must
then fail on every broker. heimq enforces this via INVALID_PRODUCER_EPOCH (47)
in its EndTxn handler; the differential assertion here is the observable outcome
(the zombie's commit is rejected), normalized across brokers since the client
surfaces a translated transaction error rather than the raw code.

Ported from parity/workloads/epoch_fence.rs.
"""

from confluent_kafka import KafkaException, Producer

from ..observation import Observation, error_code
from ..targets import Target
from .common import create_topic

NAME = "epoch_fence"
TOPIC = "parity-epoch-fence"
TXN_ID = "parity-epoch-fence-txn"


def _txn_producer(bootstrap: str) -> Producer:
    return Producer(
        {
            "bootstrap.servers": bootstrap,
            "socket.timeout.ms": 15000,
            "transactional.id": TXN_ID,
            "transaction.timeout.ms": 30000,
        }
    )


def run(target: Target) -> list[Observation]:
    bootstrap = target.bootstrap_servers
    create_topic(bootstrap, TOPIC, partitions=1)

    # Producer A: open a transaction and produce, but do not commit.
    producer_a = _txn_producer(bootstrap)
    producer_a.init_transactions(10)
    producer_a.begin_transaction()
    producer_a.produce(TOPIC, key=b"zombie", value=b"zombie")
    producer_a.flush(10)

    # Producer B: same transactional.id -- init_transactions fences producer A.
    producer_b = _txn_producer(bootstrap)
    producer_b.init_transactions(10)

    # Producer A is now a zombie; its commit must be rejected on every broker.
    try:
        producer_a.commit_transaction(10)
        fenced = False
    except KafkaException:
        fenced = True

    return [
        Observation(
            workload=NAME,
            step=0,
            # 1 = fenced (commit rejected), 0 = not fenced. Every broker must fence.
            event=error_code(api="EndTxn", code=1 if fenced else 0),
        )
    ]
