"""Workload registry. Order matches the Rust harness's workloads::all()."""

from types import ModuleType

from . import (
    consumer_group,
    duplicate_sequence,
    epoch_fence,
    idempotent_produce,
    out_of_order_sequence,
    produce_fetch,
    transactional_produce,
)


def all() -> list[ModuleType]:
    """Every workload, in execution order. Each module exposes NAME and run()."""
    return [
        produce_fetch,
        consumer_group,
        idempotent_produce,
        transactional_produce,
        epoch_fence,
        duplicate_sequence,
        out_of_order_sequence,
    ]
