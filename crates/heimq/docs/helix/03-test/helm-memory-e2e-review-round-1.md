# Helm Fixed-Memory E2E Plan Review: Round 1

## Verdict

BLOCK.

## Accepted Findings

- The plan did not define the required metrics contract. The chart exposes a
  metrics port, but the server does not currently start a Prometheus exporter.
- The plan did not specify how topics are created or how per-topic
  `retention.ms` and `retention.bytes` configs are applied before records are
  produced.
- The external client path was ambiguous. `heimq` advertises a host override but
  not a port override, so the local port-forward must bind `127.0.0.1:9092`.
- Scenario E was not deterministic against the current append path. The global
  memory cap can reject an append before the async sweeper trims
  `retention.bytes`.
- Memory plateau measurement lacked a primary source of truth, cadence, warmup,
  trend formula, and failure conditions.
- Producer retry/timeout settings were not pinned, so retriable storage-full
  errors could be hidden by the client.
- The report wording risked overclaiming persistent durability even though the
  suite proves in-memory accepted-record visibility and retention semantics.
- Scenario A sizing incorrectly implied consumed records free retained memory.

## Resolution

The plan was revised to make metrics implementation a prerequisite, pin the
port-forward shape, disable topic auto-creation, create topics through the
harness before producing, apply topic configs through IncrementalAlterConfigs,
require append-path `retention.bytes` trimming before storage-full responses,
define cgroup memory sampling, and narrow the report vocabulary to
accepted-record durability.
