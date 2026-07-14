"""Field-level normalization to remove non-determinism before diffing.

Rules follow SD-003 §Normalization Layer. Currently a pass-through, matching the
Rust harness; future NormRule expansion (broker_id, cluster_id, leader_epoch,
timestamps, producer_id, member_id, generation_id) lands here.
"""

from .observation import Observation


# @covers US-005-AC2
def normalize(observations: list[Observation]) -> list[Observation]:
    return observations
