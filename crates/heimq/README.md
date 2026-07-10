# heimq

A fast, lightweight, single-node Kafka-compatible API server written in Rust.

## Overview

heimq implements the Apache Kafka wire protocol, providing a drop-in replacement for Kafka in scenarios where:
- Single-node operation is sufficient
- Speed and simplicity are prioritized over durability
- Low latency is critical
- Resource efficiency matters

**Design Philosophy**: Transport, simplicity, and speed at the expense of durability. Data loss is acceptable.

## Features

- **Full Kafka Protocol Compatibility**: Works with standard Kafka clients (librdkafka, kafka-python, franz-go, etc.)
- **Single Binary**: No JVM, no ZooKeeper, no external dependencies
- **Sub-millisecond Latency**: Memory-first storage with optional persistence
- **Minimal Resource Usage**: Designed for efficiency

## Supported APIs

| API | Key | Status | Description |
|-----|-----|--------|-------------|
| ApiVersions | 18 | ✅ | Protocol version negotiation |
| Metadata | 3 | ✅ | Topic/partition discovery |
| Produce | 0 | ✅ | Message ingestion |
| Fetch | 1 | ✅ | Message consumption |
| CreateTopics | 19 | ✅ | Topic management |
| DeleteTopics | 20 | ✅ | Topic deletion |
| ListOffsets | 2 | ✅ | Offset queries |
| FindCoordinator | 10 | ✅ | Consumer group coordination |
| JoinGroup | 11 | ✅ | Consumer group membership |
| SyncGroup | 14 | ✅ | Partition assignment |
| Heartbeat | 12 | ✅ | Group liveness |
| LeaveGroup | 13 | ✅ | Group departure |
| OffsetFetch | 9 | ✅ | Committed offset retrieval |
| OffsetCommit | 8 | ✅ | Offset persistence |

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                        heimq                            │
├─────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐ │
│  │   Network   │  │   Protocol  │  │     Storage     │ │
│  │   (tokio)   │──│   Handler   │──│   (in-memory)   │ │
│  │             │  │             │  │                 │ │
│  └─────────────┘  └─────────────┘  └─────────────────┘ │
│         │                │                   │         │
│         ▼                ▼                   ▼         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐ │
│  │ Connection  │  │   Request   │  │     Topic       │ │
│  │   Manager   │  │   Router    │  │   Partitions    │ │
│  └─────────────┘  └─────────────┘  └─────────────────┘ │
├─────────────────────────────────────────────────────────┤
│                  Consumer Groups                        │
│  ┌─────────────────────────────────────────────────┐   │
│  │  Coordinator │ Membership │ Offset Store        │   │
│  └─────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

### Components

1. **Network Layer (tokio)**: Async TCP server handling Kafka binary protocol
2. **Protocol Handler**: Decodes requests, routes to handlers, encodes responses
3. **Storage Engine**: In-memory log segments with optional WAL
4. **Consumer Groups**: Simplified coordinator for group membership and offsets

### Storage Design

```
Memory Layout:
┌────────────────────────────────────────┐
│ Topic: "orders"                        │
├────────────────────────────────────────┤
│ Partition 0: [Segment 0] [Segment 1]   │
│ Partition 1: [Segment 0]               │
│ Partition 2: [Segment 0] [Segment 1]   │
└────────────────────────────────────────┘

Segment Structure:
┌─────────────────────────────────────────┐
│ base_offset: i64                        │
│ records: Vec<RecordBatch>               │
│ index: BTreeMap<offset, position>       │
└─────────────────────────────────────────┘
```

## Building

```bash
cargo build --release
```

## Release Artifacts

Tagged releases publish Linux x86_64 binary artifacts from the
`release-binaries` GitHub Actions workflow. Download the `heimq-linux-x86_64`
artifact and its `.sha256` file from the workflow run for the tag, then run:

```bash
sha256sum -c heimq-linux-x86_64.sha256
chmod +x heimq-linux-x86_64
./heimq-linux-x86_64 --help
```

## Running

```bash
# Start server on default port (9092)
./target/release/heimq

# Custom configuration
./target/release/heimq --port 9092 --data-dir /tmp/heimq

# In-memory only (fastest, no persistence)
./target/release/heimq --memory-only
```

### Docker

Tagged releases publish `ghcr.io/easel/heimq:<tag>` from the `docker-image`
GitHub Actions workflow. The image exposes the Kafka listener on port 9092 and
uses the heimq binary as its entrypoint.

```bash
docker run --rm -p 9092:9092 ghcr.io/easel/heimq:v0.1.0 --memory-only
```

### Helm

The chart installs one memory-only heimq broker by default behind a ClusterIP
Service on port 9092.

```bash
helm install heimq charts/heimq \
  --set image.repository=ghcr.io/easel/heimq \
  --set image.tag=v0.1.0

helm upgrade --install heimq charts/heimq \
  --set image.tag=v0.1.0 \
  --set service.type=LoadBalancer
```

## Configuration

| Flag | Default | Description |
|------|---------|-------------|
| `--port` | 9092 | Listen port |
| `--host` | 0.0.0.0 | Bind address |
| `--data-dir` | ./data | Data directory |
| `--memory-only` | false | Disable persistence |
| `--segment-size` | 1GB | Max segment size |
| `--retention-ms` | 604800000 | Message retention (7 days) |

## Testing

### Integration Tests

Integration tests use two Kafka client libraries:

- **rdkafka** (dev-dependency): Modern protocol testing via librdkafka, built from source with `cmake-build` feature
- **kafka** crate: Legacy protocol (v0/v1) compatibility verification

```bash
# Run all tests
cargo test

# Run integration tests only
cargo test --test integration
```

**Note**: rdkafka requires cmake and a C compiler. On first build, librdkafka is compiled from source (~2-3 min). This is a dev-dependency only and does not affect the server binary.

### Using kafka-python

```python
from kafka import KafkaProducer, KafkaConsumer

producer = KafkaProducer(bootstrap_servers='localhost:9092')
producer.send('test-topic', b'hello world')
producer.flush()

consumer = KafkaConsumer('test-topic', bootstrap_servers='localhost:9092')
for msg in consumer:
    print(msg.value)
```

### Using kcat (CLI testing)

```bash
# Produce
echo "hello" | kcat -b localhost:9092 -t test -P

# Consume
kcat -b localhost:9092 -t test -C -c 10 -e

# Metadata
kcat -b localhost:9092 -L
```

### Compatibility Testing

```bash
# Differential: heimq vs Apache Kafka and Redpanda, in containers
./tests/conformance/run.sh
```

## Performance

Target benchmarks (single node, in-memory mode):
- **Produce latency**: < 100μs p50, < 500μs p99
- **Fetch latency**: < 50μs p50, < 200μs p99
- **Throughput**: > 1M messages/sec (small messages)

## Limitations

By design, heimq makes explicit trade-offs:

1. **Single Node Only**: No replication, no distributed consensus
2. **Durability Not Guaranteed**: Memory-first, persistence is best-effort
3. **Simplified Consumer Groups**: Basic offset storage, no complex rebalancing
4. **No Transactions**: Idempotent producer not supported
5. **No ACLs**: No authentication or authorization

## Inspiration

- [Apache Kafka](https://github.com/apache/kafka) - The original
- [Redpanda](https://github.com/redpanda-data/redpanda) - C++ high-performance implementation
- [WarpStream](https://warpstream.com) - Stateless Kafka-compatible architecture
- [kafka-protocol-rs](https://github.com/tychedelia/kafka-protocol-rs) - Rust protocol library

## License

MIT
