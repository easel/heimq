---
ddx:
  id: TEST-HELM-MEMORY-E2E
  kind: test-plan
  status: draft
  owner: heimq
  updated: 2026-06-17
  review:
    self_hash: 2c04b297a085eb18a2208dbd92b4098eb89fece4386dd5dde362de874817f17c
    deps: {}
    reviewed_at: "2026-07-14T06:48:37Z"
---

# Helm Fixed-Memory E2E Plan

## Purpose

Demonstrate that `heimq` behaves predictably when deployed through the Helm
chart as a single memory-only broker with a fixed record-byte budget and a
fixed Kubernetes container memory limit.

The test must prove the behavior that local contract tests currently exercise
inside the process also holds through the production deployment path:

1. load distributes across `1`, `10`, and `100` topics;
2. process memory remains bounded during sustained load and after retention
   reclamation;
3. consumers receive every non-expired record before the retention boundary;
4. producers are backpressured when `retention.ms` protects unexpired data and
   the broker record-byte cap is exhausted;
5. producers are not backpressured for `retention.bytes` topics, and oldest
   records are dropped once the per-topic byte window is exceeded.

## Existing Evidence

- `cargo test -p heimq --test contract memory_bound_unexpired_data_backpressures_produce`
  proves direct-server backpressure returns `KAFKA_STORAGE_ERROR` for unexpired
  data under a broker memory cap.
- `cargo test -p heimq --test contract memory_bound_expired_data_is_reclaimed_for_later_produce_fetch_flow`
  proves expired batches are reclaimed before later appends.
- `cargo test -p heimq --test contract memory_bound_per_topic_retention_bytes_trims_consumer_visible_log`
  proves `retention.bytes` advances the earliest offset.
- CI benchmark and stress jobs exercise the direct binary, not the Helm chart.
- `helm-chart` currently validates render/lint only; it does not install the
  chart or run Kafka traffic.

## Deployment Under Test

- Cluster: local `kind` cluster unless `HEIMQ_E2E_USE_CURRENT_CONTEXT=1`.
- Image: locally built `heimq:helm-memory-e2e` loaded into `kind`, or
  `HEIMQ_E2E_IMAGE` when supplied.
- Chart: `charts/heimq`.
- Release namespace: `heimq-e2e`.
- Release name: `heimq`.
- `fullnameOverride=heimq`, so harness commands can use `svc/heimq` and
  `deploy/heimq`.
- Broker mode: memory-only.
- Helm values:
  - `image.repository=heimq`
  - `image.tag=helm-memory-e2e`
  - `image.pullPolicy=IfNotPresent`
  - `replicaCount=1`
  - `heimq.host=0.0.0.0`
  - `heimq.advertisedHost=""`
  - `heimq.memoryOnly=true`
  - `heimq.maxMemoryBytes=8388608`
  - `heimq.autoCreateTopics=false`
  - `heimq.metrics.enabled=true`
  - `resources.requests.memory=64Mi`
  - `resources.limits.memory=128Mi`
  - `resources.requests.cpu=250m`
  - `resources.limits.cpu=1000m`

The broker record-byte cap and the Kubernetes memory limit are separate
budgets. The report must show both: broker-retained record bytes and pod memory
usage.

`heimq` currently supports an advertised host override but not an advertised
port override. The harness must therefore port-forward the service with a fixed
local port matching the broker listener:

```bash
kubectl -n heimq-e2e port-forward svc/heimq 9092:9092
```

The test bootstrap address is always `127.0.0.1:9092`.

The `heimq.host=0.0.0.0` and empty `advertisedHost` values are part of the
test contract. They make the broker advertise `127.0.0.1:9092`, matching the
fixed local port-forward.

Local preflight requirements:

- local port `9092` is free;
- local port `9093` is free;
- the container cgroup is v2 and exposes `/sys/fs/cgroup/memory.current`;
- Docker, `kind`, `kubectl`, `helm`, and `cargo` are available.

The cgroup-v2 preflight is checked against the host or running kind node before
scenario execution, then confirmed inside the pod after install.

When `HEIMQ_E2E_USE_CURRENT_CONTEXT=1` targets a non-kind cluster,
`HEIMQ_E2E_IMAGE` is required and must refer to an image the cluster can pull.

Each scenario runs against a fresh Helm install and a fresh pod. This is
mandatory because deleting topics does not prove the broker's retained-byte
accounting returns to baseline. The orchestration script uninstalls the release,
reinstalls it with the same fixed budgets, waits for readiness, confirms
`http://127.0.0.1:9093/metrics` returns HTTP 200, confirms
`heimq_memory_log_bytes == 0`, and then runs one scenario.

## Harness Shape

Add a repository-owned Helm e2e harness:

- `crates/heimq/tests/helm_memory_e2e.rs`: Rust test binary that connects to an
  externally supplied Kafka bootstrap address and executes deterministic
  producer/consumer scenarios through `rdkafka`.
- `scripts/helm-memory-e2e.sh`: orchestration script that builds the image,
  creates or reuses the cluster, installs the chart, port-forwards Kafka and
  metrics endpoints, runs the Rust harness, samples pod memory, writes raw
  artifacts, and cleans up unless `HEIMQ_E2E_KEEP_CLUSTER=1`.
- `.github/workflows/helm-memory-e2e.yml`: manual and pull-request capable
  workflow. It may start as non-blocking if runtime is too high, but it must be
  runnable on demand.
- `crates/heimq/docs/helix/03-test/reports/helm-memory-e2e-report.md`: final
  report with command output, scenario metrics, durability verdicts, and
  hardware napkin math.

The harness should use one external port-forwarded bootstrap endpoint rather
than running the clients inside the cluster. That keeps the test close to the
existing Rust client tests and avoids building a second client image.

Topic setup is harness-owned:

- set `heimq.autoCreateTopics=false`;
- create every topic through Kafka `CreateTopics` before producing;
- apply topic configs through `IncrementalAlterConfigs` immediately after
  creation and assert success. heimq does not currently persist CreateTopics
  configs into `ConfigStore`;
- fail fast if any topic config acknowledgement is not success;
- use unique scenario-prefixed topic names so offset and retention state cannot
  leak between scenarios.

Producer configuration is fixed for all scenarios unless a scenario states an
override:

- `acks=all`
- `enable.idempotence=false`
- `retries=0`
- `message.timeout.ms=3000`
- `socket.timeout.ms=3000`
- `request.timeout.ms=3000`
- `linger.ms=0`
- `batch.num.messages=1`
- `queue.buffering.max.messages=1000`

The harness counts delivery-report outcomes, not only `send()` return values.
Scenario D records the first broker error code surfaced through the delivery
report and treats `KAFKA_STORAGE_ERROR` / code `56` as the required
backpressure signal.

Throughput measurement uses a separate producer profile so correctness checks
are not latency-bound by single-record batches:

- `acks=all`
- `enable.idempotence=false`
- `retries=0`
- `message.timeout.ms=5000`
- `linger.ms=5`
- `batch.num.messages=100`
- `queue.buffering.max.messages=10000`

Consumer configuration is fixed for exact-delivery scenarios:

- use unique group IDs per scenario;
- `enable.auto.commit=false`;
- `auto.offset.reset=earliest`;
- use manual assignment to every intended topic partition after topic creation;
- consume from `Offset::Beginning`;
- record identity is `(scenario, topic, partition, sequence)` encoded in both
  key and payload;
- producers explicitly set the target partition to the `partition` encoded in
  the record identity. The partition for sequence `n` is `n % partitions`;
- the validator compares the acknowledged record-id set to the consumed
  record-id set and reports missing and duplicate IDs.

Port-forward supervision:

- start Kafka and metrics port-forward processes before each scenario;
- poll both local ports before producing;
- if either process exits during a scenario, fail the scenario as
  `port_forward_lost` and include the process output in artifacts;
- tear down port-forwards after the final metrics scrape for the scenario.

## Metrics To Capture

Raw artifacts are written under `target/helm-memory-e2e/<timestamp>/`:

- Helm release values and rendered manifest.
- Pod description and pod events.
- Pod memory samples from the container cgroup:
  `kubectl exec deploy/heimq -c heimq -- cat /sys/fs/cgroup/memory.current`.
  `kubectl top` may be captured as supplemental evidence only.
- Broker metrics scrapes from `http://127.0.0.1:9093/metrics` after a fixed
  metrics port-forward:
  `kubectl -n heimq-e2e port-forward svc/heimq 9093:9093`.
- Per-scenario JSON result:
  - topics;
  - partitions per topic;
  - attempted produce count;
  - acknowledged produce count;
  - accepted record-batch bytes total;
  - max observed accepted record-batch bytes;
  - produce errors by Kafka error code;
  - consumed record count;
  - duplicate count;
  - missing record count;
  - earliest and latest offsets sampled after each retention phase;
  - elapsed wall time;
  - approximate throughput records/sec and MiB/sec.
- Hardware/context artifacts:
  - host or GitHub runner label;
  - CPU model and logical CPU count when available;
  - Kubernetes node allocatable CPU and memory;
  - container runtime;
  - configured pod requests and limits.

Every scenario captures a scenario-local metrics baseline before producing and
a final metrics scrape after the last consume/offset check. Scenario B also
captures interval samples. Counter assertions use final-minus-baseline deltas
for that scenario only.

The broker must expose these Prometheus metrics before the Helm e2e suite can
claim success:

- `heimq_memory_log_bytes` gauge, no labels: `MemoryLog::total_bytes()`.
- `heimq_memory_log_messages_total` counter, no labels: records accepted by
  `MemoryLog::append`.
- `heimq_storage_full_errors_total` counter, no labels: `StorageFull` returns
  from append admission.
- `heimq_retention_reclaimed_bytes_total` counter, label
  `reason="retention_ms|retention_bytes"`: bytes reclaimed by expiry or
  per-topic byte trimming.

Implementation requirements:

- reclamation code must produce separate byte counts for expiry and size trim
  before metrics are emitted. The current aggregate `reclaim_topic` style is not
  sufficient for the labeled metric contract;
- the implementation may introduce a small return type such as
  `{ expired_bytes, size_trim_bytes }` for topic reclamation, or split the
  calls internally, but it must avoid double-counting bytes that satisfy both
  predicates;
- the HTTP endpoint must be `/metrics` on `Config.metrics_port`;
- the metrics server must bind `0.0.0.0:Config.metrics_port` so the Kubernetes
  Service and port-forward can reach it;
- metrics start only when `Config.metrics=true`;
- the exporter is Prometheus text format using the repository's existing
  `metrics` and `metrics-exporter-prometheus` dependencies;
- all metrics above must be scrapeable before Scenario A starts;
- the report must include raw metric samples and parsed peak/final values.
- `reason="retention_ms"` increments on both time-based sweeper reclamation and
  append-path expired-data reclamation;
- `reason="retention_bytes"` increments on sweeper size trimming and the new
  append-path size trimming.

## Scenarios

### Scenario A: Topic Fan-Out And Delivery

Run three harness-directed fan-out cases with `topics={1,10,100}` and a fixed
per-record payload size.
Each case gets a fresh Helm install and fresh pod.
Consumption does not release retained memory, so each case must keep total
retained bytes safely below the `8MiB` broker cap.

| Topics | Partitions/Topic | Records/Topic | Payload Bytes | Expected Records |
|---:|---:|---:|---:|---:|
| 1 | 4 | 800 | 512 | 800 |
| 10 | 4 | 160 | 512 | 1600 |
| 100 | 2 | 20 | 512 | 2000 |

Acceptance:

- every produced record is acknowledged;
- every acknowledged record is consumed exactly once before retention expiry;
- missing records = `0`;
- duplicates = `0`;
- each topic receives the configured count;
- at least 75% of all intended partitions are non-empty;
- per-partition acknowledged counts match the explicit round-robin target
  distribution: each partition receives either `floor(records/partitions)` or
  `ceil(records/partitions)` records per topic.

### Scenario A2: Throughput Sample

Run a separate `10` topic case using the throughput producer profile, `4`
partitions per topic, `400` records per topic, and `512B` payloads. The
retained workload is `4000` records, below the `8MiB` cap at the conservative
`1.5KiB` planning estimate.

Acceptance:

- acknowledged records = `4000`;
- missing records = `0`;
- duplicates = `0`;
- measured producer throughput is at least `1000 records/sec` and
  `0.49 MiB/sec` payload throughput;
- pod memory stays below `128Mi`;
- broker retained record bytes stay at or below `8MiB`.

Because the client path uses `kubectl port-forward`, throughput is reported as
tunnel-bounded end-to-end throughput. The hardware napkin math may use it only
as a lower-bound smoke signal, not as an isolated broker-capacity benchmark.
The `1000 records/sec` threshold is normative for the local report run on the
developer/CI hardware named in the report. In GitHub pull-request runs it is
report-only unless `HEIMQ_E2E_ENFORCE_THROUGHPUT=1`.

### Scenario B: Memory Plateau Under Retention

Run sustained produce/fetch against `10` topics for `20s`, with a `5s` warmup
excluded from trend calculations. Set scenario topic `retention.ms=3000`, so
the run crosses multiple retention windows and the fixed `500ms` sweeper tick.
Limit aggregate scenario produce rate to `500 records/sec` at `512B` payloads,
scheduled evenly across the `10` topics, so no more than about `2.25MiB` of
estimated retained data is unexpired at a time, below the fixed `8MiB` cap.

Sample every `1s`:

- cgroup memory from `/sys/fs/cgroup/memory.current`;
- `heimq_memory_log_bytes`;
- `heimq_retention_reclaimed_bytes_total`.

Acceptance:

- pod memory stays below `128Mi`;
- broker retained record bytes stay at or below `heimq.maxMemoryBytes`;
- after warmup, the mean of the last 5 samples is not more than 15% above the
  mean of the first 5 post-warmup samples for retained bytes;
- after warmup, the mean of the last 5 cgroup-memory samples is not more than
  `16MiB` above the mean of the first 5 post-warmup cgroup-memory samples;
- cgroup memory is also evaluated as an absolute ceiling because allocator
  retention can keep RSS/cgroup usage high after broker bytes are reclaimed;
- storage-full errors remain `0`;
- any pod restart, `OOMKilled` state, missing cgroup sample, or missing metrics
  sample fails the scenario.

### Scenario C: `retention.ms` Delivery Boundary

Use `3` topics, `2` partitions per topic, `100` records per topic, `512B`
payloads, and `retention.ms=5000`. Produce all records, consume immediately
from the beginning, then wait `6500ms` to cross the retention boundary and
allow at least one `500ms` sweeper tick.

Acceptance:

- acknowledged records = `300`;
- immediate consumer receives every acknowledged record exactly once within
  `2000ms` of the final produce acknowledgement. If this deadline is missed,
  the harness reports `immediate_consume_timeout` separately from missing broker
  records;
- after retention expiry, poll for up to `3000ms` for sweeper-only reclamation
  to advance earliest offsets. Do not append to the measured topics during this
  phase;
- latest offsets remain equal to the produced count per partition;
- missing immediate records = `0`;
- duplicate immediate records = `0`.

### Scenario D: `retention.ms` Backpressure

Use a topic with `retention.ms=60000`, the fixed `8MiB` broker cap, `1`
partition, `4096B` payloads, and up to `4096` produce attempts. The test stops
only after the first delivery report with Kafka storage error `56` plus `16`
additional produce attempts, or after all attempts complete.

Scenario D uses a scenario-specific chart override `heimq.retentionMs=1000`
and topic `retention.ms=60000`. Produce enough records to exceed `6MiB`, wait
`1500ms`, then continue producing until the first storage-full delivery report.
This makes the scenario sensitive to effective per-topic retention: if append
admission incorrectly uses only the broker default, expired records can be
reclaimed and the required storage-full response may not occur.

Acceptance:

- initial produces succeed until the cap is reached;
- at least `256` records are accepted before the first storage-full response;
- fewer than `4096` records are accepted before the first storage-full response;
- at the first storage-full response, `heimq_memory_log_bytes` is greater than
  or equal to `maxMemoryBytes - max_observed_accepted_batch_bytes` and less than
  or equal to `maxMemoryBytes`;
- the sum of accepted record-batch bytes before the first storage-full response
  is greater than or equal to `maxMemoryBytes - max_observed_accepted_batch_bytes`;
- later produce attempts fail with retriable Kafka storage error
  `KAFKA_STORAGE_ERROR` / code `56`;
- all `16` post-first-error attempts fail with code `56`;
- the producer records at least one backpressure error;
- records accepted before the first storage-full response remain consumable;
- broker retained record bytes stay at or below the cap.
- `heimq_retention_reclaimed_bytes_total{reason="retention_ms"}` delta remains
  `0` before the first storage-full response.

### Scenario E: `retention.bytes` Drop Without Backpressure

Use `10` topics, `1` partition per topic, `retention.bytes=524288`, `400`
records per topic, and `2048B` payloads. Each topic writes comfortably more
than its byte window. With one partition per topic, the summed positive
`retention.bytes` commitments are `5MiB`, below the fixed `8MiB` broker cap.

The implementation must trim `retention.bytes`-bounded topics on the append path
before returning a storage-full response. Relying on the async sweeper is not
deterministic enough for this scenario because the global cap check can run
before the sweeper trims old records.

Implementation ownership:

- append admission must have access to the effective topic retention config
  (`retention.ms`, `retention.bytes`) before deciding whether to return
  `StorageFull`;
- either move cap enforcement into the produce path where `ConfigStore` is
  available, or extend the log append API so the caller passes the effective
  retention policy;
- per-topic `retention.bytes` trim must run before the global cap rejects the
  append;
- append-path trimming must increment
  `heimq_retention_reclaimed_bytes_total{reason="retention_bytes"}`.

Acceptance:

- all produce attempts succeed;
- acknowledged records = `4000`;
- storage-full error count remains `0`;
- earliest offsets advance after the per-topic byte window is exceeded;
- latest offset for each topic is `400`;
- earliest offset for each topic is greater than `0` and less than `400`;
- consumers assigned to `[earliest, latest)` for each topic consume exactly the
  expected retained ID set: every acknowledged ID with sequence in
  `[earliest, latest)` is present once, with missing count `0` and duplicate
  count `0`;
- broker retained record bytes stay at or below the broker cap.

The retained-byte safety rationale depends on `partitions=1`. Actual retained
data is bounded per partition; increasing Scenario E partitions would multiply
the real retained-byte window and must be treated as a different test.

## Performance Napkin Math

The report must compare measured throughput to a conservative single-pod
budget:

- CPU limit: `1` vCPU.
- Memory limit: `128Mi`.
- Broker record-byte cap: `8Mi`.
- Payload: `512B`; use measured retained-byte delta from Scenario A to compute
  effective retained bytes per record. Use `1.5KiB` only as a pre-run planning
  estimate.
- Expected max retained records: `8MiB / 1.5KiB ~= 5461` records.
- Required correctness load: scenario fan-out peak is `2000` records, below the
  retained budget without relying on consumption to free memory.
- Required throughput load: Scenario A2 acknowledges `4000` records and then
  consumes those same record IDs.
- Required throughput: at least `1000 records/sec` and `0.49 MiB/sec` payload
  throughput on Scenario A2.

The report must show tunnel-bounded throughput, retained byte peak, cgroup
memory peak, measured effective retained bytes per record, and whether these
thresholds were met. The report must call these "accepted-record durability"
metrics, not persistent durability metrics.

For the final report, Scenario A2 must meet both throughput thresholds on the
local report run. Pull-request workflow runs may record these values as
telemetry without enforcing them unless `HEIMQ_E2E_ENFORCE_THROUGHPUT=1`.

## DDx Beads

Created beads:

1. `heimq-27a6e68c`: epic for the Helm fixed-memory e2e evidence suite.
2. `heimq-7a75bc97`: metrics exporter.
3. `heimq-b2ffc9c2`: append-path retention admission.
4. `heimq-fd28a041`: Helm memory e2e harness.
5. `heimq-0c8e64ff`: Helm memory e2e workflow.
6. `heimq-0fcafd25`: run the suite and write the final report.

Any defect discovered while implementing or running the suite gets its own DDx
bug bead and must be closed before the final report bead closes.

## Non-Scope

- Multi-broker clustering.
- Persistent log storage durability across pod restart.
- Exactly-once transactions.
- Public CI gate hardening beyond a runnable workflow.
- Kafka/Redpanda differential comparison for these scenarios.

## Stop Condition

The work is complete only when:

1. DDx beads covering the implementation and any surfaced defects are closed.
2. The Helm chart has been installed locally or in CI with the fixed memory
   budget.
3. The Helm e2e suite has produced raw artifacts.
4. At least one on-demand `helm-memory-e2e` GitHub workflow run has completed
   and archived raw artifacts, even if the workflow remains non-blocking for
   pull requests.
5. The final report states whether every accepted-record durability metric
   passed.
6. The final report includes the performance napkin math and measured results.
