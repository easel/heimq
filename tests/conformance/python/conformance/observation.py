"""Observation model shared by every workload.

Mirrors the `Observation`/`ObservationEvent` schema the Rust harness emitted, so
JSONL output stays comparable across runners and against history.
"""

from dataclasses import dataclass, field
from typing import Any


@dataclass
class Observation:
    workload: str
    step: int
    event: dict[str, Any]


def record_consumed(
    *,
    key: bytes | None,
    value: bytes | None,
    partition: int,
    offset: int,
    headers: list[tuple[str, bytes]] | None = None,
    timestamp: int = 0,
) -> dict[str, Any]:
    return {
        "type": "RecordConsumed",
        "key": key,
        "value": value,
        "headers": headers or [],
        "partition": partition,
        "offset": offset,
        "timestamp": timestamp,
    }


def error_code(*, api: str, code: int) -> dict[str, Any]:
    return {"type": "ErrorCode", "api": api, "code": code}


def group_state(*, group_id: str, state: str, member_count: int) -> dict[str, Any]:
    return {
        "type": "GroupState",
        "group_id": group_id,
        "state": state,
        "member_count": member_count,
    }


def txn_outcome(*, committed: bool, records_visible: bool) -> dict[str, Any]:
    return {
        "type": "TxnOutcome",
        "committed": committed,
        "records_visible": records_visible,
    }


@dataclass
class DiffRecord:
    workload: str
    oracle: str
    step: int
    field: str
    heimq_value: Any
    oracle_value: Any
    divergence: str
    exemption: str | None = None

    def to_json(self) -> dict[str, Any]:
        return {
            "workload": self.workload,
            "oracle": self.oracle,
            "step": self.step,
            "field": self.field,
            "heimq_value": self.heimq_value,
            "oracle_value": self.oracle_value,
            "divergence": self.divergence,
            "exemption": self.exemption,
        }
