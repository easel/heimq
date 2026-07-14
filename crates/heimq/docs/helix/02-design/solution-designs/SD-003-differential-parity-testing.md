---
ddx:
  id: SD-003
  type: solution-design
  activity: design
  status: approved
  depends_on:
    - helix.prd
    - FEAT-003
    - US-005
  review:
    self_hash: a415d412e130ef7e35646deeba1ce0fd5a3e84b8d86104a7ae7b734212eab843
    deps:
      FEAT-003: d134a5740222f1b59ecda81b318c7b28de9968bba9f640bb6280310c1e906233
      US-005: 168616bd29809066f578398a7c34ba64f23005e237d58eac1381dde83a4ef2fd
      helix.prd: 96f0479e307f2c240e8f47b69fff510164d0b9eda132abb22cc4a860932984fe
    reviewed_at: "2026-07-14T05:12:26Z"
---

# Design: Differential Parity Harness [SD-003]

**Design ID**: SD-003 (formerly HARNESS-001)
**Feature**: FEAT-003
**Status**: Approved (implementation in progress — scaffolding shipped, workloads pending)
**Spec Authority**: FEAT-003-differential-parity-testing.md, US-005

---

## Scope

This document specifies the architecture of `tests/parity/`, a Rust-native
harness that drives identical Kafka client workloads against heimq and
Redpanda, normalizes non-determinism, and reports a structured diff. Zero
diffs on the gating workload constitutes CI success.

---

## Requirements Mapping

FEAT-003 functional requirements map to harness capabilities as follows
(requirement text lives in FEAT-003; it is not restated here):

| FEAT-003 FR | Technical Capability | Design Element / Module |
|-------------|----------------------|-------------------------|
| FR 1 (same workload, both brokers) | `WorkloadDriver` run twice per workload, once per `BrokerTarget` | `driver.rs`, `broker.rs` |
| FR 2 (record observable outputs) | `Observation` / `ObservationEvent` variants (`RecordConsumed`, `ErrorCode`, `GroupState`, `TxnOutcome`) | `driver.rs` |
| FR 3 (normalize non-determinism) | Pure, deterministic `NormRule` table | `normalize.rs` (§ Normalization Layer) |
| FR 4 (zero diffs = success; structured diff on failure) | JSONL diff records, exit code 0/1 | `diff.rs` (§ Structured Diff Format) |
| FR 5 (gating workloads; txn after FEAT-002) | produce_fetch + consumer_group workloads; expansion path | `workloads/` (§ Initial Workloads) |
| FR 6 (runnable locally and in CI) | testcontainers + host Docker socket; GitHub Actions job | § Solution Approaches, § CI Integration |

---

## Solution Approaches

### Container Orchestration Decision

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

## System Decomposition

### Module Layout

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

## Domain Model: Workload Driver Shape

```rust
/// A recorded observable from one broker run.
pub struct Observation {
    pub workload: &'static str,
    pub step: u32,
    pub event: ObservationEvent,
}

pub enum ObservationEvent {
    RecordConsumed { key: Option<Bytes>, value: Option<Bytes>,
                     headers: Vec<(String, Bytes)>, partition: i32, offset: i64,
                     timestamp: i64 },
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
| record CreateTime ts     | Replace with sequential per-step index `[ts:N]` when `CreateTime` semantics in use; otherwise preserve |
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

- Image: `docker.redpanda.com/redpandadata/redpanda:v25.1.1` (specific
  version tag; bump via a dependency-update PR when a new Redpanda release
  is validated).
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
phase). It runs on pull requests that touch `heimq/src/protocol/`,
`heimq/src/handler/`, or `heimq/tests/parity/`:

```yaml
name: parity

on:
  pull_request:
    paths:
      - 'heimq/src/protocol/**'
      - 'heimq/src/handler/**'
      - 'heimq/tests/parity/**'
      - 'heimq/Cargo.toml'

defaults:
  run:
    working-directory: heimq

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

## Traceability

| Requirement | Design Element | Verification |
|-------------|----------------|--------------|
| FEAT-003 FR 1–2 | `WorkloadDriver` + `Observation` (§ Domain Model) | Workload runs against both targets in `cargo test --test parity` |
| FEAT-003 FR 3 | `NormRule` table (§ Normalization Layer) | Deterministic normalization; rules logged for triage |
| FEAT-003 FR 4 | Diff engine + JSONL schema (§ Structured Diff Format) | JSONL schema validated by unit test in `diff.rs`; exit code gates CI |
| FEAT-003 FR 5 | § Initial Workloads + § Expansion path | Gating workloads at FEAT-003 acceptance; txn workloads blocked on FEAT-002 |
| FEAT-003 FR 6 | § CI Integration | GitHub Actions `parity` job (follow-on issue) |
| FEAT-003 edge case: intentional diffs | § Known-Divergence Registry (`exemptions.toml`) | Exemption loader fail-fast validation (§ Quality Gates) |

## Open Questions (resolved for this design pass)

| Question | Resolution |
|---|---|
| testcontainers vs docker-compose | testcontainers (see §Container Decision) |
| Normalization of offsets | Offsets are preserved — they are observable semantics |
| Diff granularity | Per-field within a step (not per-message blob) |
| Known-divergence format | TOML file checked into source tree |
| Where to store diff output | `target/parity/` (gitignored) |
