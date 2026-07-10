# Conformance suite

Everything that talks to heimq over the Kafka wire protocol. It runs in
containers and needs nothing on the host but Docker — no Rust toolchain, no
librdkafka, no Go, Node or kcat.

```bash
./run.sh                    # parity: heimq vs Kafka + Redpanda (python + go)
./compat/run.sh             # independent client oracles against heimq
./integration/run.sh        # heimq's own behavioural suite
./bench/run.sh [compare]    # throughput
./helm_e2e/run.sh           # fixed-memory evidence harness
```

## Why it isn't Rust

The suite used to live in `crates/heimq/tests/` as `[dev-dependencies]` of the
`heimq` crate. That had three costs:

1. **It linked the code it was testing.** `parity/raw_protocol.rs` built its
   test frames with `heimq_protocol`, the server's own protocol library. A bug
   in that encoder was invisible: both sides of the comparison shared it.
   `helm_memory_e2e.rs` imported `heimq::protocol::is_flexible` for the same
   reason.
2. **It tested a different server than it shipped.** The parity harness compared
   an in-process `test_support::TestServer` against containerized oracles. Every
   broker now runs from the same `ENTRYPOINT` the release image uses.
3. **It didn't run.** 42 of `integration.rs`'s 61 tests were `#[ignore]`d
   ("can segfault in cargo test"), and nothing in CI passed `--ignored`.
   `compat.rs` returned `Ok` when `go`, `node` or `kcat` was absent from `PATH`,
   and kcat is not on the GitHub runner image — so that oracle reported green
   without ever executing. Its kcat invocation was also wrong (options after
   `-G` are parsed as topic names), which nothing had noticed.

Building librdkafka into the test binary bought two executing tests.

## Layout

| Path | What |
|------|------|
| `run.sh`, `docker-compose.yml` | parity: 8 workloads × 2 oracles × 2 clients |
| `python/` | confluent-kafka runner (wraps librdkafka) |
| `go/` | franz-go runner (independent implementation) |
| `exemptions.toml` | known, justified divergences from an oracle |
| `compat/` | franz-go, sarama, KafkaJS, kcat against heimq alone |
| `integration/` | pytest suite; legacy v0/v1 via kafka-python-ng |
| `bench/` | throughput, optionally comparative |
| `helm_e2e/` | fixed-memory scenarios against a Helm/kind deploy |

## Contract

Both parity runners emit the same schemas, so their output is directly
comparable — and is compared:

- `Observation` — `{workload, step, event}`, event one of `RecordConsumed`,
  `ErrorCode`, `GroupState`, `TxnOutcome`.
- `DiffRecord` — one JSONL line per diverging field, into `target/conformance/`.
- `exemptions.toml` — matched by `field` + `scope` + `oracle` + `status`.

heimq is diffed against each oracle independently. Apache Kafka is canonical;
Redpanda is a second independent implementation.

## Adding a workload

Add a module to `python/conformance/workloads/` and `go/workloads.go`, register
it in both, and keep the two in the same order. A workload that only one runner
implements will show up as an observation-count mismatch, which is the point.

Two runners exist so a client bug cannot hide behind agreement with itself.
`confluent-kafka` is librdkafka; `franz-go` shares no code with it. That is not
theoretical: franz-go caught heimq returning success from `InitProducerId` where
Kafka returns `CONCURRENT_TRANSACTIONS`, because librdkafka retries that code
internally and no librdkafka-based test can observe the broker's first answer.
