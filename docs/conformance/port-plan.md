# Porting the conformance suite out of Rust

## Why

The Kafka-facing test suite lived in Rust as `[dev-dependencies]` of the
`heimq` crate. Three consequences:

1. `cargo test`, `cargo check --all-targets`, and `cargo clippy --all-targets`
   compile librdkafka (a C library) even when the change under test is unrelated.
2. The parity harness starts heimq via an in-process `heimq::test_support::TestServer`
   while starting Kafka and Redpanda as containers. heimq is therefore exercised
   through a different code path than the oracles it is compared against.
3. A conformance suite that links the implementation it is testing cannot detect
   a class of bug where the server and the test agree because they share code.

The suite talks to brokers over a wire protocol. Nothing about it needs to be Rust.

## Target shape

```
tests/conformance/
  docker-compose.yml   heimq + apache/kafka:3.9.0 + redpanda:v25.1.1 + runners
  exemptions.toml      moved verbatim; the contract does not change
  python/              confluent-kafka runner
  go/                  franz-go runner (independent protocol implementation)
```

Every broker, including heimq, runs as a container started from its normal
entrypoint. Runners are containers on the same compose network, so brokers are
reachable by service DNS (`heimq:9092`) and the bridge-IP discovery in
`parity/broker.rs` disappears.

heimq is configured entirely by `HEIMQ_*` environment variables and the release
image's `ENTRYPOINT` is the `heimq` binary, so the compose service runs the same
CLI that ships. No embedded mode.

## Contract (frozen before any porting)

Both runners emit the schemas the Rust harness already uses, so output is
comparable across runners and against history:

- `Observation` — `{workload, step, event}` where `event` is one of
  `RecordConsumed`, `ErrorCode`, `GroupState`, `TxnOutcome`.
- `DiffRecord` — `{workload, oracle, step, field, heimq_value, oracle_value,
  divergence, exemption}`, one JSONL line per diverging field.
- `exemptions.toml` — `{id, field, scope, oracle, reason, prd_ref, status}`,
  matched by `field` + `scope` + `oracle` + `status == "active"`.

heimq is diffed against each oracle independently. Kafka is canonical; Redpanda
is a second independent implementation.

## Stages

Each stage ended green before the next began. rdkafka left `Cargo.toml` only at
S8, once nothing in Rust depended on it.

All stages are complete. Kept as a record of what moved where.

| # | Stage | Source | Outcome |
|---|-------|--------|---------|
| S1 | Compose topology + harness core | `parity/{main,diff,driver,normalize,exemptions}.rs` | done |
| S2 | 7 parity workloads, Python | `parity/workloads/*.rs` | done, 14/14 |
| S3 | Same workloads, franz-go | ditto | done; JSONL identical to S2 |
| S4 | Independent client oracles | `compat.rs` (564 loc) | done; kcat had never run |
| S5 | heimq behavioural suite | `integration.rs` (4,541 loc) | done; 42 `#[ignore]`d tests now run |
| S6 | Benchmarks | `bench_baseline.rs`, `parity/bench.rs` | done |
| S7 | Helm/k8s e2e | `helm_memory_e2e.rs` (1,339 loc) | done, 8/8 cases |
| S8 | **Delete** | see below | done; `cargo test` builds no broker client |

An eighth workload, `concurrent_transactions`, was added during S3: franz-go
found heimq returning success from `InitProducerId` for a transactional id whose
previous producer still had an open transaction, where Kafka and Redpanda both
return `CONCURRENT_TRANSACTIONS` (51). Every client library retries that code
internally, so no librdkafka-based test could have seen it. Fixed in
`transaction_state.rs`.

## S8 deletions

```
crates/heimq/tests/{parity,integration.rs,compat.rs,bench_baseline.rs,helm_memory_e2e.rs}
crates/heimq/Cargo.toml   rdkafka, testcontainers, kafka, toml dev-deps
.cargo/config.toml        CPATH shim, only needed by librdkafka
vendor/librdkafka-shim/   curl.h stub, only needed by librdkafka
.github/workflows/helm-memory-e2e.yml   cmake, only needed by librdkafka
```

Result: zero C compilation in the Rust build; `cargo test` builds no broker client.

## How the risks actually landed

- **S4** was mis-sized. The legacy-format concern belonged to `integration.rs`,
  not `compat.rs`, whose oracles were already standalone Go/JS programs. It
  needed Dockerfiles, not rewrites.
- **S5** legacy tests went to `kafka-python-ng` at `api_version=(0, 10, 0)`,
  which still emits the v0/v1 MessageSet. No hand-rolled frames were needed.
  Isolation moved from a broker per test to a topic per test, plus four extra
  broker services for broker-level config (auto-create off; 2, 3, 4 partitions).
- **S7** was the opposite of cheap. Scenarios cannot share a broker, and
  scenario B needed the producer warmed before its plateau window.
- The exemptions did survive unchanged: the redpanda `record.offset` shift is
  still recorded and still exempted, byte-identical under both runners.

## Left behind

- `heimq`'s `IncrementalAlterConfigs` handler answers with a default, error-free
  response when it cannot decode a request, so a malformed alter is
  indistinguishable from success. v1 decodes fine, so nothing depends on this
  today.
- `heimq::test_support::DiffRecord` no longer has a caller.
