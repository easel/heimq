# Porting the conformance suite out of Rust

## Why

The Kafka-facing test suite is written in Rust as `[dev-dependencies]` of the
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

Each stage ends green before the next begins. rdkafka leaves `Cargo.toml` only
at S6, once nothing in Rust depends on it.

| # | Stage | Source | Gate |
|---|-------|--------|------|
| S1 | Compose topology + harness core (observation, diff, normalize, exemptions, runner) | `parity/{main,diff,driver,normalize,exemptions}.rs` | all three brokers reachable from a runner container |
| S2 | 8 parity workloads, Python | `parity/workloads/*.rs` (~800 loc) | parity green vs kafka **and** redpanda |
| S3 | Same 8 workloads, franz-go | ditto | green, and JSONL identical to S2 |
| S4 | Legacy message-format compat | `compat.rs` (564 loc) | green |
| S5 | heimq behavioural suite | `integration.rs` (~4,500 loc) | green |
| S6 | Benchmarks | `bench_baseline.rs`, `parity/bench.rs` (348 loc) | runs, numbers comparable |
| S7 | Helm/k8s e2e | `helm_memory_e2e.rs` (1,339 loc) | green |
| S8 | **Delete** | see below | CI green on all suites |

## S8 deletions

```
crates/heimq/tests/{parity,integration.rs,compat.rs,bench_baseline.rs,helm_memory_e2e.rs}
crates/heimq/Cargo.toml   rdkafka, testcontainers, kafka, toml dev-deps
.cargo/config.toml        CPATH shim, only needed by librdkafka
vendor/librdkafka-shim/   curl.h stub, only needed by librdkafka
.github/workflows/helm-memory-e2e.yml   cmake, only needed by librdkafka
```

Result: zero C compilation in the Rust build; `cargo test` builds no broker client.

## Known risks

- **S4** legacy v0–v2 message formats are produced today by the pure-Rust `kafka`
  crate. `confluent-kafka` cannot emit v0/v1 batches, so this stage needs
  hand-rolled frames over a socket. Sized as the hardest per-line stage.
- **S5** is the bulk of the code (~4,500 loc) and currently uses the in-process
  `TestServer`; each test becomes a container round-trip. Expect it to be slower
  in wall-clock and to need a shared fixture rather than per-test servers.
- **S7** shells out to `helm`/`kubectl` already; the Rust wrapper adds little.
  Likely the cheapest stage per line.
- Redpanda and Kafka disagree in places today; those disagreements are recorded
  in `exemptions.toml` and must survive the port unchanged, or the port has
  changed behaviour rather than relocating it.
