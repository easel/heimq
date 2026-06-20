# Helm Fixed-Memory E2E Plan Review: Round 4

## Verdict

REQUEST_CHANGES.

## Accepted Findings

- Scenario B needed a process-memory steadiness criterion, not only a retained
  broker-byte plateau.
- Scenario D needed to fail if append admission ignored effective per-topic
  `retention.ms`.
- Scenario E needed exact retained-suffix delivery, not only "some suffix."
- Scenario A cases needed explicit per-case fresh installs.
- Producer partitioning needed to be explicit because the record identity
  contains partition.
- Scenario-local metric baselines and final scrapes needed to be specified.
- `retention_ms` reclaim counters needed explicit wiring on both sweeper and
  append-path reclamation paths.
- Metrics must bind `0.0.0.0`, and port-forwards need supervision.

## Resolution

The plan was revised to use explicit producer partition routing, scenario-local
metric scrape baselines, exact retained-suffix comparison, per-case fresh
installs, a per-topic-retention-sensitive backpressure setup, cgroup-memory
bounded-drift criteria, explicit `retention_ms` counter wiring, supervised
port-forwards, and qualified tunnel-bounded throughput reporting.
