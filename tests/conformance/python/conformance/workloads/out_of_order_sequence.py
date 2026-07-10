"""Hand-crafted out-of-order idempotent producer sequence.

Ported from parity/workloads/out_of_order_sequence.rs.
"""

from ..observation import Observation, error_code
from ..raw_protocol import RawKafkaClient
from ..targets import Target
from .common import create_topic

NAME = "out_of_order_sequence"
TOPIC = "parity-out-of-order-sequence"


def run(target: Target) -> list[Observation]:
    bootstrap = target.bootstrap_servers
    create_topic(bootstrap, TOPIC, partitions=1)

    with RawKafkaClient(bootstrap) as client:
        client.api_versions()
        producer_id, producer_epoch = client.init_producer_id()

        first = client.produce_sequence(TOPIC, producer_id, producer_epoch, 0)
        if first != 0:
            raise RuntimeError(f"initial Produce returned error {first}")

        out_of_order = client.produce_sequence(TOPIC, producer_id, producer_epoch, 5)

    return [
        Observation(
            workload=NAME, step=0, event=error_code(api="Produce", code=out_of_order)
        )
    ]
