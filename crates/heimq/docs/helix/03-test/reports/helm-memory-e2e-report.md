---
ddx:
  id: TEST-HELM-MEMORY-E2E-REPORT
  kind: test-report
  status: complete
  owner: heimq
  updated: 2026-06-17
  review:
    self_hash: 034f36919001d5cbe9dd4a56c85342a7ac774d16e2622fdc7de778c97fcbfac1
    deps: {}
    reviewed_at: "2026-07-14T06:48:37Z"
---

# Helm Fixed-Memory E2E Report

## Verdict

PASS. The Helm-deployed, memory-only `heimq` broker met the accepted-record
durability checks and the fixed-budget performance napkin math on both a local
kind run and the on-demand GitHub Actions deployment run.

Primary evidence is GitHub Actions run
`27724888367`, `helm-memory-e2e`, commit
`bcb24cec3f65591a30e1b18c929d0ea562a3686f`, completed successfully on
2026-06-17 in `11m23s`:

- Run URL: `https://github.com/easel/heimq/actions/runs/27724888367`
- Artifact: `helm-memory-e2e-27724888367`, artifact id `7710028973`, `81540`
  bytes
- Downloaded copy: `/tmp/heimq-gh-27724888367/helm-memory-e2e-27724888367/20260617T225335Z`

Corroborating local evidence:

- Command: `HEIMQ_E2E_SKIP_IMAGE_BUILD=1 scripts/helm-memory-e2e.sh`
- Artifact: `target/helm-memory-e2e/20260617T223334Z`

The first GitHub run, `27724520093`, failed before scenario execution because
the Docker builder pinned Rust `1.85.1` while the lockfile required Rust
`1.88+`. That was captured as `heimq-256b3613`, fixed by aligning the
toolchain and Docker builder to Rust `1.88.0`, and closed after the passing run
above.

## Environment

Both runs installed the chart as a single pod with:

- `heimq.memoryOnly=true`
- `heimq.maxMemoryBytes=8388608`
- `heimq.autoCreateTopics=false`
- `heimq.metrics.enabled=true`
- Kubernetes memory request `64Mi`
- Kubernetes memory limit `128Mi`
- CPU request `250m`
- CPU limit `1000m`

GitHub runner context:

- Linux `runnervm1li68`, kernel `6.17.0-1018-azure`, `x86_64`
- Docker Engine `28.0.4`, linux/amd64
- kubectl client `v1.31.0`
- Helm `v4.2.1`
- Cargo `1.88.0`
- kind cluster API endpoint `https://127.0.0.1:39741`

Local context:

- Linux `eitri`, kernel `7.0.11-orbstack-00360-gc9bc4d96ac70`, `aarch64`
- Docker client `29.2.1`, server `29.4.0`, linux/arm64, OrbStack
- kubectl client `v1.36.1`
- Helm `v4.2.0`
- Cargo `1.92.0`

## Scenario Metrics

GitHub Actions run `27724888367`:

| Scenario | Purpose | Result | Key Metrics |
| --- | --- | --- | --- |
| A, 1 topic | load balance | PASS | `800/800` acked, `800` consumed, `0` missing, `0` duplicates, non-empty partition ratio `1.0`, partition records `200..200`, retained bytes `498290` |
| A, 10 topics | load balance | PASS | `1600/1600` acked, `1600` consumed, `0` missing, `0` duplicates, non-empty partition ratio `1.0`, partition records `40..40`, retained bytes `1002100` |
| A, 100 topics | load balance | PASS | `2000/2000` acked, `2000` consumed, `0` missing, `0` duplicates, non-empty partition ratio `1.0`, partition records `10..10`, retained bytes `1256800` |
| A2 | tunnel-bounded throughput | PASS | `4000/4000` acked, `4000` consumed, `0` missing, `0` duplicates, `33ms`, `119141.5387 records/sec`, `62.460978 MiB/sec`, retained bytes `2238780` |
| B | retention.ms plateau | PASS | `950` produced, `0` errors, `19` retained-byte samples, first post-warmup mean `116850`, last mean `123000`, final retained bytes `92250`, reclaimed bytes `491890`, storage-full delta `0` |
| C | retention.ms boundary | PASS | `300` acked, `300` immediately consumed, `0` missing, `0` duplicates, latest offsets unchanged, all partitions advanced `earliest` from `0` to `50` while `latest` stayed `50`, reclaimed bytes `184470` |
| D | retention.ms backpressure | PASS | first storage-full attempt `1995`, `1995` records accepted before storage-full, `17` Kafka error-code `56` storage errors, `16` post-full errors, `1995` consumed, `0` missing, `0` duplicates, final retained bytes `8387865` |
| E | retention.bytes trimming | PASS | `4000/4000` acked, `0` storage-full errors, retained expected `2420`, `2420` consumed, `0` missing, `0` duplicates, every topic watermark `158..400`, reclaimed bytes `3410120`, final retained bytes `5224780` |

Local run `20260617T223334Z` matched the same verdicts. Its A2 throughput was
`10290.4270 records/sec` and `5.394845 MiB/sec`; scenario D again accepted
`1995` records before storage-full and returned `17` error-code `56` storage
errors; scenario E again acknowledged all `4000` records without storage-full
and retained exactly `2420`.

## Durability Checks

Accepted-record durability means every record acknowledged before a retention
boundary or storage-full boundary must be visible to the consumer exactly once.

| Check | Evidence | Verdict |
| --- | --- | --- |
| Consumers receive every record across 1, 10, and 100 topic fan-out | Scenario A consumed exactly all acked records for `800`, `1600`, and `2000` attempted records, with `0` missing and `0` duplicates | PASS |
| Consumers receive every record before a `retention.ms` boundary | Scenario C consumed `300/300` immediately, with `0` missing and `0` duplicates | PASS |
| Expired records are no longer visible after the retention boundary | Scenario C latest offsets stayed fixed while earliest offsets advanced from `0` to `50` on every partition | PASS |
| Producers are backpressured when `retention.ms` protects unexpired data and the broker cap is exhausted | Scenario D produced until attempt `1995`, then returned Kafka error code `56` seventeen times with no reclamation | PASS |
| Accepted pre-backpressure records remain durable | Scenario D consumed `1995/1995` accepted records, with `0` missing and `0` duplicates | PASS |
| `retention.bytes` drops excess records without producer backpressure | Scenario E acknowledged `4000/4000`, had `0` storage-full errors, advanced every topic watermark to `158..400`, and consumed the expected retained suffix of `2420` records | PASS |

## Memory And Retention

Broker retained-byte cap: `8388608` bytes.

GitHub retained-byte observations:

- Scenario A peak retained bytes: `1256800`
- Scenario A2 retained bytes: `2238780`
- Scenario B final retained bytes: `92250`
- Scenario C final retained bytes: `0`
- Scenario D final retained bytes: `8387865`, which is `743` bytes below the
  broker cap
- Scenario E final retained bytes: `5224780`, which is `18100` bytes below the
  aggregate `10 * 524288` byte topic retention window

GitHub cgroup memory observations from `/sys/fs/cgroup/memory.current`:

- Scenario B interval samples: `21` samples, min `1593344`, max `2166784`,
  mean `2051705.9`
- Max post-scenario cgroup sample: `12787712` bytes in Scenario D
- `12787712 / 134217728 = 9.5%` of the `128Mi` pod memory limit

Local cgroup memory observations:

- Scenario B interval samples: `20` samples, min `7372800`, max `11812864`,
  mean `10806067.2`
- Max post-scenario cgroup sample: `22024192` bytes in Scenario D
- `22024192 / 134217728 = 16.4%` of the `128Mi` pod memory limit

Scenario B plateau drift used retained broker bytes. The GitHub run moved from
first post-warmup mean `116850` bytes to last mean `123000` bytes, a `6150`
byte delta. That is far below the `16MiB` drift ceiling.

## Performance Napkin Math

Normative thresholds from the plan:

- Pod memory must remain below `128Mi`.
- Broker retained bytes must remain at or below `8MiB`.
- Scenario A2 must exceed `1000 records/sec`.
- Scenario A2 must exceed `0.49 MiB/sec` payload throughput.
- Scenario A2 retained workload is `4000` records.
- Conservative expected max retained records at `1.5KiB` each is
  `8MiB / 1.5KiB ~= 5461` records.

Measured GitHub result:

- A2 throughput: `119141.5387 records/sec`, `119.1x` the record threshold
- A2 payload throughput: `62.460978 MiB/sec`, `127.5x` the payload threshold
- A2 retained bytes per accepted record: `2238780 / 4000 = 559.7` bytes
- A2 retained workload: `4000` records, below the conservative `5461` record
  budget
- Scenario D retained-byte peak: `8387865` bytes, below the `8388608` byte
  broker cap
- Scenario E retained suffix bytes per retained record:
  `5224780 / 2420 = 2159.0` bytes, consistent with `2048B` payloads plus
  record overhead
- Pod memory peak: `12787712` bytes, below the `134217728` byte Kubernetes
  memory limit

Measured local corroboration:

- A2 throughput: `10290.4270 records/sec`, `10.3x` the record threshold
- A2 payload throughput: `5.394845 MiB/sec`, `11.0x` the payload threshold
- Pod memory peak: `22024192` bytes, below the `134217728` byte Kubernetes
  memory limit

The fixed hardware budget therefore holds under both measured environments:
the broker admits work up to the retained-byte cap, rejects protected
unexpired writes when the cap is full, trims byte-retained topics instead of
backpressuring, and keeps process memory well below the Kubernetes limit.

## Commands And Checks

Commands run during implementation and verification:

- `scripts/helm-memory-e2e.sh --help`
- `cargo test -p heimq --test helm_memory_e2e -- --ignored --exact helm_memory_e2e_requires_bootstrap_or_runs_when_bootstrap_set`
- `cargo test -p heimq --test helm_memory_e2e --no-run`
- `cargo fmt --all -- --check`
- `bash -n scripts/helm-memory-e2e.sh`
- `helm lint charts/heimq`
- `helm template heimq charts/heimq`
- `docker build -t heimq:helm-memory-e2e .`
- `HEIMQ_E2E_SKIP_IMAGE_BUILD=1 scripts/helm-memory-e2e.sh`
- `gh workflow run helm-memory-e2e.yml --ref main`
- `gh run watch 27724888367 --interval 30 --exit-status`
- Workspace test hooks triggered by DDx tracker commits; all completed
  successfully.

## Open Issues

No open e2e bug beads remain from this run. The surfaced issues were captured
and closed:

- `heimq-21f7c041`: kind readiness fallback for local OrbStack loopback
- `heimq-78d3a9ee`: A2 throughput producer overhead
- `heimq-a7e494bb`: Scenario B warmup sampling cadence
- `heimq-61652420`: Scenario E retention.bytes sweeper race
- `heimq-256b3613`: Docker builder Rust version mismatch in GitHub Actions
