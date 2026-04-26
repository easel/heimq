# Design: Differential Parity Harness [HARNESS-001]

**Design ID**: HARNESS-001
**Feature**: FEAT-003
**Status**: Draft
**Spec Authority**: FEAT-003-differential-parity-testing.md, US-005

---

## Overview

This document specifies the architecture of `tests/parity/`, a Rust-native
harness that drives identical Kafka client workloads against heimq and
Redpanda, normalizes non-determinism, and reports a structured diff. Zero
diffs on the gating workload constitutes CI success.

---

## Container Orchestration Decision

**Decision: testcontainers (via `testcontainers` + `testcontainers-modules` crates)**

Rationale:

- The harness runs as `cargo test --test parity`; containers are lifecycle-managed
  by the Rust test runtime and torn down automatically on pass or fail.
- No external Docker Compose CLI dependency; works on any host with the Docker
  daemon available (including standard GitHub Actions runners).
- `testcontainers-modules` ships a first-class Redpanda module; no custom image
  management needed.
- The existing `scripts/compatibility-test.sh` uses bare `docker run` and is
  unmaintained; the new harness replaces that pattern for parity work.

docker-compose rejected because:

- Requires separate CLI installation in CI.
- Lifecycle is external to the test process; cleanup on panic is unreliable.
- No additive benefit for a single-container Redpanda dependency.

---

## Module Layout

```
tests/parity/
├── main.rs           -- harness entry: boot containers, run workloads, report
├── broker.rs         -- BrokerTarget enum, rdkafka ClientConfig builders
├── driver.rs         -- WorkloadDriver trait, sequential runner
├── workloads/
│   ├── mod.rs
│   ├── produce_fetch.rs   -- initial workload: produce/fetch round-trip
│   └── consumer_group.rs  -- initial workload: consumer group lifecycle
├── normalize.rs      -- field-level normalization transforms
├── diff.rs           -- diff engine, JSONL serializer
├── exemptions.rs     -- exemption registry loader
└── exemptions.toml   -- known-divergence declarations (checked into source)
```

The harness is a Rust integration test binary (`[[test]]` in `Cargo.toml`
with `harness = false`) so it controls its own `main` and async runtime.

---

## Workload Driver Shape

```rust
/// A recorded observable from one broker run.
pub struct Observation {
    pub workload: &'static str,
    pub step: u32,
    pub event: ObservationEvent,
}

pub enum ObservationEvent {
    RecordConsumed { key: Option<Bytes>, value: Option<Bytes>,
                     headers: Vec<(String, Bytes)>, partition: i32, offset: i64 },
    ErrorCode      { api: &'static str, code: i16 },
    GroupState     { group_id: String, state: String, member_count: usize },
    TxnOutcome     { committed: bool, records_visible: bool },
}

/// A workload drives identical operations against one BrokerTarget and
/// returns a time-ordered list of Observations.
#[async_trait]
pub trait WorkloadDriver: Send + Sync {
    fn name(&self) -> &'static str;
    async fn run(&self, target: &BrokerTarget) -> anyhow::Result<Vec<Observation>>;
}
```

Each workload is called twice — once against heimq, once against Redpanda —
producing two `Vec<Observation>` lists that are then normalized and diffed.

### Initial Workloads

**produce_fetch_roundtrip** (`workloads/produce_fetch.rs`)

1. Create topic (N=1 partition).
2. Produce 20 records with known key/value/header content via rdkafka.
3. Consume from offset 0 until all 20 records are received.
4. Emit one `RecordConsumed` observation per record.
5. Produce a batch with snappy compression; consume; verify payloads match.

**consumer_group_lifecycle** (`workloads/consumer_group.rs`)

1. Create topic (N=3 partitions).
2. Start two consumers in the same group; allow rebalance to complete.
3. Commit offsets via `OffsetCommit`.
4. Fetch committed offsets via `OffsetFetch`; emit `GroupState` observation.
5. Have one consumer leave; wait for rebalance; emit `GroupState` observation.

**Expansion path** (not in scope until FEAT-002 lands):

- `idempotent_producer.rs`: InitProducerId, produce with `enable.idempotence=true`,
  verify no duplicate records on retry.
- `transactions.rs`: begin txn, produce, commit; begin txn, produce, abort;
  verify visibility under `isolation.level=read_committed`.

These workloads are listed in `exemptions.toml` under `future` and are blocked
on FEAT-002 acceptance.

---

## Normalization Layer

Normalization is applied to `Vec<Observation>` before diffing. The layer is
pure and deterministic: given the same raw observation list, it always produces
the same normalized list.

Fields normalized:

| Field / Pattern          | Transform                                      |
|--------------------------|------------------------------------------------|
| broker_id in metadata    | Replace with `0`                               |
| cluster_id               | Replace with `"<cluster_id>"`                 |
| leader_epoch             | Replace with `0`                               |
| log_append_time          | Replace with `0` when `CreateTime` semantics not in use |
| producer_id              | Replace with sequential index `[pid:N]`        |
| producer_epoch           | Replace with `0`                               |
| correlation_id           | Replace with sequential index per connection   |
| member_id (group proto)  | Replace with `"<member_id:N>"`                |
| generation_id            | Replace with sequential index                  |
| record offset            | Preserved (offset is observable semantics)     |
| partition assignment     | Sorted by partition id before comparison       |

Normalization rules are encoded as a table of `NormRule` values and applied
via a single-pass visitor over `Observation` fields. New rules are added to
the table; no branching logic in caller code.

---

## Structured Diff Format

The diff engine compares two normalized `Vec<Observation>` lists
(same workload, different target) and emits a JSONL stream — one object per
diverging field.

Each diff record:

```jsonc
{
  "workload":       "produce_fetch_roundtrip",
  "step":           3,
  "field":          "record.value",
  "heimq_value":    "<base64>",
  "redpanda_value": "<base64>",
  "divergence":     "value_mismatch",   // "value_mismatch" | "missing_in_heimq" | "extra_in_heimq"
  "exemption":      null                // exemption id string or null
}
```

The diff output path is `target/parity/<timestamp>-<workload>.jsonl`.
A summary line is printed to stdout:

```
[PASS] produce_fetch_roundtrip: 0 diffs
[FAIL] consumer_group_lifecycle: 2 diffs (0 exempt, 2 unmatched)
```

Exit code: `0` when all unmatched diffs are zero; `1` otherwise.

---

## Known-Divergence Registry

`tests/parity/exemptions.toml` is the single source of truth for intentional
behavioral differences. Each entry must cite a PRD reference.

```toml
[[exemption]]
id        = "cluster-id-nondeterministic"
field     = "cluster_id"
scope     = "all"
reason    = "heimq generates a fresh cluster_id on each start; clients do not use this for routing"
prd_ref   = "PRD non-goal #1 (no persistence of broker identity across restart)"
status    = "active"

[[exemption]]
id        = "txn-state-loss-on-restart"
field     = "txn_outcome.records_visible"
scope     = "workloads/transactions"
reason    = "In-memory transactional state is not persisted; PRD explicitly allows loss on restart"
prd_ref   = "PRD non-goal #1"
status    = "future"   # workload not yet implemented
```

`status = "future"` entries are loaded but not enforced until the referenced
workload is active. The diff engine flags an exemption as `stale` if it
matches zero observations in a run, surfacing dead entries for cleanup.

---

## Container Configuration

```rust
pub struct BrokerTarget {
    pub kind: BrokerKind,        // Heimq | Redpanda
    pub bootstrap_servers: String,
}

pub enum BrokerKind { Heimq, Redpanda }
```

Redpanda container spec:

- Image: `docker.redpanda.com/redpandadata/redpanda:latest` (pinned to a
  digest in `tests/parity/redpanda-image.lock` to prevent flake from
  upstream changes).
- Exposed port: `9092` (mapped to ephemeral host port by testcontainers).
- Start command: `redpanda start --smp 1 --memory 512M --overprovisioned
  --kafka-addr 0.0.0.0:9092 --advertise-kafka-addr localhost:<port>`
- Ready signal: TCP accept on the mapped port (testcontainers default health check).

heimq target:

- Started as a spawned subprocess via `heimq::test_support::TestServer::start()`
  on an ephemeral port (reuses the subprocess fixture already used in
  `tests/integration.rs`). Alternative constructors:
  `TestServer::start_with_auto_create(bool)` and
  `TestServer::start_with_partitions(bool, i32)`.
- The subprocess is launched via `std::process::Command` using the
  `CARGO_BIN_EXE_heimq` binary. Port allocation uses an atomic counter
  starting at 19092. `Drop` kills the child process on teardown.
- The `test-support` feature flag (combined with `cfg(any(test, feature = "test-support"))`) gates this module.

---

## CI Integration

A GitHub Actions job `parity` is added in a follow-on issue (FEAT-003 build
phase). It runs on pull requests that touch `src/protocol/` or `src/handler/`:

```yaml
jobs:
  parity:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --test parity -- --nocapture
```

Docker is available by default on `ubuntu-latest` runners. Docker-in-Docker
is not required because testcontainers uses the host Docker socket.

---

## Quality Gates

- `cargo clippy -p heimq --tests` passes on `tests/parity/`.
- `cargo fmt --check` passes.
- The JSONL diff output schema is validated by a unit test in `diff.rs`.
- The exemption loader validates that each `exemption.id` is unique and
  `prd_ref` is non-empty at test startup (fail-fast, not silently ignored).

---

## Open Questions (resolved for this design pass)

| Question | Resolution |
|---|---|
| testcontainers vs docker-compose | testcontainers (see §Container Decision) |
| Normalization of offsets | Offsets are preserved — they are observable semantics |
| Diff granularity | Per-field within a step (not per-message blob) |
| Known-divergence format | TOML file checked into source tree |
| Where to store diff output | `target/parity/` (gitignored) |
