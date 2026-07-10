"""Conformance runner.

heimq is diffed against each reference broker independently. Apache Kafka is the
canonical oracle; Redpanda is retained as a second, independent implementation.

Port of parity/main.rs. Exits non-zero if any workload has unmatched diffs.
"""

import json
import os
import sys
from datetime import datetime, timezone
from pathlib import Path

from . import diff as diff_mod
from . import normalize, targets, workloads
from .exemptions import load as load_exemptions

EXEMPTIONS_PATH = Path("/conformance/exemptions.toml")


def main() -> int:
    heimq, oracles = targets.from_env()
    exemptions = load_exemptions(EXEMPTIONS_PATH)

    print("waiting for brokers...", flush=True)
    for t in [heimq, *oracles]:
        targets.wait_ready(t)
    print("brokers ready", flush=True)

    out_dir = Path(os.environ.get("OUT_DIR", "/out"))
    out_dir.mkdir(parents=True, exist_ok=True)

    any_fail = False
    selected = workloads.all()

    for w in selected:
        heimq_obs = normalize.normalize(w.run(heimq))

        for oracle in oracles:
            oracle_obs = normalize.normalize(w.run(oracle))
            diffs = diff_mod.diff(w.NAME, oracle.name, heimq_obs, oracle_obs, exemptions)
            unmatched = [d for d in diffs if d.exemption is None]

            ts = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
            path = out_dir / f"{ts}-{oracle.name}-{w.NAME}.jsonl"
            with path.open("w") as f:
                for d in diffs:
                    f.write(json.dumps(d.to_json()) + "\n")

            if not unmatched:
                print(f"[PASS] {w.NAME} vs {oracle.name}: 0 diffs", flush=True)
            else:
                print(
                    f"[FAIL] {w.NAME} vs {oracle.name}: "
                    f"{len(unmatched)} unmatched diffs",
                    flush=True,
                )
                for d in unmatched:
                    print(f"  {json.dumps(d.to_json())}", flush=True)
                any_fail = True

    if not selected:
        print("[PASS] scaffolding: containers up, client connected, 0 workloads")

    if any_fail:
        print("one or more parity workloads had unmatched diffs", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
