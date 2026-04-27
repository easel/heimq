# Kafka Perf-Test Bench Harness

Runs `kafka-producer-perf-test` and `kafka-consumer-perf-test` against
heimq using Docker — no JVM required on the host.

## Prerequisites

- Linux with Docker Engine (the harness uses `docker run --network host` to
  reach heimq on `127.0.0.1:$HEIMQ_PORT`; Docker Desktop on macOS/Windows does
  not route host networking the same way and the readiness loop will time out)
- Rust toolchain (to build heimq), or set `HEIMQ_BINARY` to a pre-built binary

## Quick Start

```bash
# From the repo root:
./scripts/bench/kafka-perf-test.sh
```

This builds heimq, starts it on port 19092, runs the legacy producer
profile (10 000 records, 1 KiB each), then the consumer profile, and
exits 0 if both tools report no errors.

## Profiles

Profiles live in `scripts/bench/profiles/`. Each is a shell fragment
sourced by `kafka-perf-test.sh` that sets the parameters for one run.

| File | Description |
|---|---|
| `producer-legacy.conf` | 10 000 × 1 KiB, acks=1, throughput unlimited, no idempotence |
| `consumer-legacy.conf` | Consume all records from the producer topic, single thread |

Idempotent and transactional profiles are **not yet included** — they
depend on FEAT-006 (flex versions) and FEAT-002 (transactions) and will
be added in a follow-up bead.

## Usage

```bash
# Both producer and consumer (default):
./scripts/bench/kafka-perf-test.sh

# Producer only:
./scripts/bench/kafka-perf-test.sh --producer

# Consumer only (requires a prior producer run to populate the topic):
./scripts/bench/kafka-perf-test.sh --consumer

# Custom profile:
./scripts/bench/kafka-perf-test.sh --producer --profile ./profiles/producer-legacy.conf
```

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `HEIMQ_BINARY` | _(build from source)_ | Path to a pre-built heimq binary |
| `HEIMQ_PORT` | `19092` | Port heimq listens on (avoids conflicts with port 9092) |
| `KAFKA_IMAGE` | `confluentinc/cp-kafka:7.6.1` | Docker image supplying the perf tools |

## Pass/Fail Criteria

The harness asserts:
1. The tool process exits 0.
2. No lines matching `error` or `exception` (case-insensitive) appear
   in stdout or stderr.

Wall-clock budget: ≤ 30 minutes (CI gate).
