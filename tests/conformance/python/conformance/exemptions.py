"""Load and validate exemptions.toml.

Same schema and matching rules as the Rust harness: an entry matches when the
field matches, status is active, scope is the workload or "all", and oracle is
the reference broker or "all".
"""

import tomllib
from dataclasses import dataclass
from pathlib import Path


@dataclass
class Exemption:
    id: str
    field: str
    scope: str
    oracle: str
    reason: str
    prd_ref: str
    status: str


class Exemptions:
    def __init__(self, entries: list[Exemption]):
        self.entries = entries

    # @covers US-005-AC6
    def find(self, field: str, workload: str, oracle: str) -> str | None:
        for e in self.entries:
            if (
                e.field == field
                and e.status == "active"
                and e.scope in ("all", workload)
                and e.oracle in ("all", oracle)
            ):
                return e.id
        return None


# @covers US-005-AC6
def load(path: Path) -> Exemptions:
    if not path.exists():
        return Exemptions([])

    with path.open("rb") as f:
        doc = tomllib.load(f)

    entries = []
    seen: set[str] = set()
    for raw in doc.get("exemption", []):
        e = Exemption(
            id=raw["id"],
            field=raw["field"],
            scope=raw["scope"],
            oracle=raw.get("oracle", "all"),
            reason=raw["reason"],
            prd_ref=raw["prd_ref"],
            status=raw["status"],
        )
        if e.id in seen:
            raise ValueError(f"Duplicate exemption id: {e.id}")
        seen.add(e.id)
        if not e.prd_ref:
            raise ValueError(f"Exemption '{e.id}' has empty prd_ref")
        entries.append(e)

    return Exemptions(entries)
