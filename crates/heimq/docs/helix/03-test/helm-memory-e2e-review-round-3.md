# Helm Fixed-Memory E2E Plan Review: Round 3

## Verdict

REQUEST_CHANGES.

## Accepted Findings

- Scenario A2 tried to retain `10000` records under an `8MiB` cap while the
  plan's own planning estimate allowed only about `5461` records.
- The authoritative Helm values list did not include the locally built image
  repository and tag.
- Scenario C did not define concrete topic, partition, record, payload, and
  retention timing.
- Scenario E did not define concrete produce volume or expected offset window.
- Cgroup memory should remain an absolute ceiling, while
  `heimq_memory_log_bytes` is the primary plateau metric.
- Topic configs must be applied with IncrementalAlterConfigs; CreateTopics
  configs are not currently persisted by heimq.
- Metrics readiness needs its own `/metrics` polling gate because the chart
  readiness probe only checks the Kafka port.

## Resolution

The plan was revised to reduce Scenario A2 to `4000` retained records, wire the
local image into Helm values, give Scenarios C and E concrete workloads and
offset expectations, make retained bytes the primary plateau trend assertion,
require IncrementalAlterConfigs for topic config, and add an explicit metrics
readiness poll.
