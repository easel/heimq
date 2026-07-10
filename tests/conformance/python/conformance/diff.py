"""Diff two normalized observation streams: heimq against one reference broker.

Port of the Rust `diff.rs`. One DiffRecord per diverging field.

Deliberate divergence from the Rust original: it matched (RecordConsumed,
RecordConsumed), (GroupState, GroupState) and (ErrorCode, ErrorCode), letting
two identical TxnOutcome events fall through to the event-type catch-all and
report a spurious mismatch. That was unreachable (no workload emitted one).
Here TxnOutcome is compared field-wise like every other event.
"""

from typing import Any

from .exemptions import Exemptions
from .observation import DiffRecord, Observation

# Fields compared per event type, as (observation key, diff field name).
COMPARED_FIELDS: dict[str, list[tuple[str, str]]] = {
    "RecordConsumed": [
        ("key", "record.key"),
        ("value", "record.value"),
        ("partition", "record.partition"),
        ("offset", "record.offset"),
    ],
    "ErrorCode": [("api", "error_code.api"), ("code", "error_code.code")],
    "GroupState": [
        ("state", "group_state.state"),
        ("member_count", "group_state.member_count"),
    ],
    "TxnOutcome": [
        ("committed", "txn_outcome.committed"),
        ("records_visible", "txn_outcome.records_visible"),
    ],
}


def _to_json(value: Any) -> Any:
    if isinstance(value, bytes):
        return value.decode("utf-8", errors="replace")
    return value


def diff(
    workload: str,
    oracle: str,
    heimq: list[Observation],
    oracle_obs: list[Observation],
    exemptions: Exemptions,
) -> list[DiffRecord]:
    diffs: list[DiffRecord] = []
    shared = min(len(heimq), len(oracle_obs))

    for obs in heimq[shared:]:
        diffs.append(
            DiffRecord(
                workload=workload,
                oracle=oracle,
                step=obs.step,
                field="observation",
                heimq_value=obs.event["type"],
                oracle_value=None,
                divergence="extra_in_heimq",
            )
        )
    for obs in oracle_obs[shared:]:
        diffs.append(
            DiffRecord(
                workload=workload,
                oracle=oracle,
                step=obs.step,
                field="observation",
                heimq_value=None,
                oracle_value=obs.event["type"],
                divergence="missing_in_heimq",
            )
        )

    for h, r in zip(heimq, oracle_obs):
        _diff_events(workload, oracle, h.step, h.event, r.event, exemptions, diffs)

    return diffs


def _diff_events(
    workload: str,
    oracle: str,
    step: int,
    h: dict[str, Any],
    r: dict[str, Any],
    exemptions: Exemptions,
    out: list[DiffRecord],
) -> None:
    if h["type"] != r["type"]:
        out.append(
            DiffRecord(
                workload=workload,
                oracle=oracle,
                step=step,
                field="event_type",
                heimq_value=h["type"],
                oracle_value=r["type"],
                divergence="value_mismatch",
            )
        )
        return

    for key, field_name in COMPARED_FIELDS[h["type"]]:
        if h[key] == r[key]:
            continue
        out.append(
            DiffRecord(
                workload=workload,
                oracle=oracle,
                step=step,
                field=field_name,
                heimq_value=_to_json(h[key]),
                oracle_value=_to_json(r[key]),
                divergence="value_mismatch",
                exemption=exemptions.find(field_name, workload, oracle),
            )
        )
