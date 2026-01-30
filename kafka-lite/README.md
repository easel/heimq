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

## Running

```bash
# Start server on default port (9092)
./target/release/heimq

# Custom configuration
./target/release/heimq --port 9092 --data-dir /tmp/heimq

# In-memory only (fastest, no persistence)
./target/release/heimq --memory-only
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

### Using librdkafka (rdkafka-rs for Rust tests)

```bash
cargo test --features integration-tests
```

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

### Compatibility Testing

```bash
# Run against real Kafka for comparison
./scripts/compatibility-test.sh
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
