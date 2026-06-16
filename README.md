# heimq

heimq is a lightweight, single-node Kafka-compatible broker written in Rust.
It is built for local development, CI, test fixtures, and small ephemeral
deployments where Kafka protocol compatibility matters more than distributed
durability.

heimq is not a Kafka cluster replacement. It does not provide replication,
controller quorum, durable multi-broker recovery, or Kafka's full operational
surface. Its current bias is a small binary, memory-first behavior, fast startup,
and deterministic testability.

## Quick start

Build and run heimq from source:

```bash
cargo build -p heimq --release
./target/release/heimq --memory-only --host 0.0.0.0 --port 9092
```

Produce and consume with standard Kafka CLI tools:

```bash
printf 'hello heimq\n' | kafka-console-producer.sh \
  --bootstrap-server localhost:9092 \
  --topic heimq-smoke

kafka-console-consumer.sh \
  --bootstrap-server localhost:9092 \
  --topic heimq-smoke \
  --from-beginning \
  --max-messages 1
```

You can also run the checked-in smoke script, which starts heimq and drives a
basic producer/consumer flow:

```bash
just bench-smoke
```

## Docker

Release images are published to `ghcr.io/easel/heimq`.

```bash
docker run --rm -p 9092:9092 \
  -e HEIMQ_HOST=0.0.0.0 \
  -e HEIMQ_MEMORY_ONLY=true \
  ghcr.io/easel/heimq:v0.1.0
```

Build and smoke-test the image locally:

```bash
just docker-build
./scripts/docker-image-smoke.sh heimq:local
```

The Docker release workflow is [.github/workflows/docker-image.yml](.github/workflows/docker-image.yml).
It publishes stable tag aliases for release tags and runs a Kafka client smoke
test before pushing.

## Helm

The Helm chart lives in [charts/heimq](charts/heimq). It installs one
memory-only heimq broker by default.

Install from the local chart:

```bash
helm install heimq charts/heimq \
  --set image.repository=ghcr.io/easel/heimq \
  --set image.tag=v0.1.0
```

Install the published OCI chart:

```bash
helm install heimq oci://ghcr.io/easel/charts/heimq \
  --version 0.1.0
```

Validate the chart locally:

```bash
just helm-check
```

The chart publish workflow is [.github/workflows/helm-chart.yml](.github/workflows/helm-chart.yml).
Chart usage details are in [charts/heimq/README.md](charts/heimq/README.md).

## Release artifacts

Tagged releases use the version contract documented in
[docs/release/release-infrastructure-plan.md](docs/release/release-infrastructure-plan.md):

- Git tag: `vX.Y.Z`
- Binary version: `X.Y.Z`
- Docker tags: `vX.Y.Z`, `X.Y.Z`, `X.Y`, `X`, and `latest` for stable tags
- Helm chart: `version: X.Y.Z`, `appVersion: X.Y.Z`

Binary releases are built by
[.github/workflows/release-binaries.yml](.github/workflows/release-binaries.yml)
and attached to GitHub Releases as `heimq-linux-x86_64` plus
`heimq-linux-x86_64.sha256`.

Docker images are published to `ghcr.io/easel/heimq`. Helm charts are published
to `oci://ghcr.io/easel/charts/heimq`.

Operational release notes and recovery steps are in
[docs/release/runbook.md](docs/release/runbook.md).

## Development

The repository pins the Rust toolchain in [rust-toolchain.toml](rust-toolchain.toml).
Install Rust, Docker, Helm, and `just`, then run:

```bash
just --list
just ci
```

Common commands:

```bash
just fmt          # cargo fmt --all -- --check
just clippy       # cargo clippy --workspace --all-targets -- -D warnings
just test         # cargo test --workspace --all-targets
just docker-build # docker build -t heimq:local .
just helm-check   # helm lint + helm template
just bench-smoke  # local produce/consume benchmark smoke
just parity       # Docker-backed parity tests
```

The main CI workflow is [.github/workflows/test.yml](.github/workflows/test.yml).
Additional compatibility and performance workflows include:

- [.github/workflows/bench-smoke.yml](.github/workflows/bench-smoke.yml)
- [.github/workflows/stress-matrix.yml](.github/workflows/stress-matrix.yml)
- [.github/workflows/consumer-matrix.yml](.github/workflows/consumer-matrix.yml)
- [.github/workflows/parity.yml](.github/workflows/parity.yml)
- [.github/workflows/postgres-conformance.yml](.github/workflows/postgres-conformance.yml)
- [.github/workflows/helm-chart.yml](.github/workflows/helm-chart.yml)

Use `just release-check` before tagging a release.

## Tests and benchmarks

Run the full workspace test suite:

```bash
cargo test --workspace --all-targets
```

Run the compatibility script:

```bash
./scripts/compatibility-test.sh
```

Run the local benchmark smoke:

```bash
just bench-smoke
```

Run OpenMessaging Benchmark smoke infrastructure:

```bash
./scripts/bench/run-omb.sh scripts/bench/openmessaging/workload-smoke.yaml
```

Benchmark and stress artifacts live under [scripts/bench](scripts/bench).

## Compatibility and limits

heimq implements a practical subset of Kafka protocol behavior for a
single-node broker. It supports common produce, fetch, metadata, topic,
consumer-group, offset, transaction, config, and log-description flows used by
the checked-in compatibility suites.

Current limits and non-goals:

- Single-node only; no broker replication or distributed controller quorum.
- Memory-first operation; persistence and external storage backends are not a
  substitute for Kafka's durability model.
- Kafka admin and broker operations are implemented only where required by the
  compatibility and conformance suites.
- Security protocols such as SASL/TLS are not enabled by default.
- Compatibility is validated by tests, not by a promise that every Kafka client
  edge case behaves identically to Apache Kafka.

More implementation detail is available in [crates/heimq/README.md](crates/heimq/README.md).
