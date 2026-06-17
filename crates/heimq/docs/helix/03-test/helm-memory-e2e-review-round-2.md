# Helm Fixed-Memory E2E Plan Review: Round 2

## Verdict

BLOCK.

## Accepted Findings

- Unique topic names do not reset retained memory. Each scenario needs a fresh
  Helm release or an independently verified retained-byte baseline.
- Scenario D must use the fixed `8MiB` broker cap and concrete produce sizing,
  not a vague "small enough to fill" cap.
- Consumer validation must define record identity, assignment, group behavior,
  offset reset, and duplicate/missing detection.
- Append-path `retention.bytes` trimming needs an explicit ownership change
  because per-topic config currently lives outside `MemoryLog::append`.
- Correctness fan-out and throughput measurement must be separated.
- The partition-distribution denominator must be all intended partitions.
- Helm resource names must be pinned with either release name or
  `fullnameOverride`.
- Scenario B needs a bounded produce rate and windowed plateau calculation.
- Cgroup sampling assumes cgroup v2 and must be a preflight requirement.
- The bead list should be enumerated before execution.

## Resolution

The plan was revised to use one fresh Helm install per scenario, pin the release
name and `fullnameOverride`, define cgroup-v2 preflight, define exact producer
and consumer configurations, use fixed record identities, size Scenario D under
the fixed cap, require append admission to see effective per-topic retention
config, bound Scenario B load, compare windowed means, and enumerate the DDx
beads to create.
