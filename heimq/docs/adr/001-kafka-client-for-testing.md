# ADR-001: Kafka Client Library for Integration Testing

**Status**: Accepted
**Date**: 2026-01-30

## Context

heimq needs integration tests that validate Kafka protocol compatibility. We evaluated several Rust Kafka client libraries:

| Client | Pure Rust | Async | Consumer Groups | Protocol Config |
|--------|-----------|-------|-----------------|-----------------|
| rdkafka | No (C FFI) | Yes | Yes | Full control |
| rskafka | Yes | Yes | No | Auto-negotiate |
| samsa | Yes | Yes | Yes | Auto-negotiate |
| kafka-rust | Yes | No | Yes | Limited |

## Decision

**Use rdkafka with `cmake-build` feature for integration testing.**

### Rationale

1. **Battle-tested**: 909K downloads/month, production-proven
2. **Complete feature coverage**: Consumer groups, transactions, all compression codecs
3. **Build from source**: `cmake-build` feature compiles librdkafka from source, avoiding system dependency version drift
4. **Better debugging**: Mature error messages help identify heimq protocol bugs

### Mitigations

- **Firewall to dev-dependencies**: rdkafka is ONLY in `[dev-dependencies]`, never in the server binary
- **Build from source**: No system librdkafka required, reproducible builds
- **Keep legacy tests**: kafka crate (pure Rust) validates v0/v1 message format compatibility

## Alternatives Considered

### rskafka (Rejected)
- No consumer group support
- No protocol version control
- InfluxDB abandoned it

### samsa (Not Chosen)
- Pure Rust with consumer groups
- Only ~500 downloads/month (vs rdkafka's 909K)
- Small community, uncertain maintenance

### CLI tools only (Partial)
- kcat/rpk cover ~70-80% of testing needs
- Lack programmatic assertions and fine-grained control
- Good complement to, not replacement for, library tests

## Consequences

### Positive
- Comprehensive protocol testing
- Consumer group validation
- Reproducible builds via cmake-build

### Negative
- First build takes ~2-3 min (librdkafka compilation)
- Requires cmake and C compiler for development
- Larger dev dependency footprint

### Neutral
- Server binary remains pure Rust with no C dependencies

## Implementation

```toml
[dev-dependencies]
# FIREWALLED: dev-dependency only, never in server binary
rdkafka = { version = "0.36", features = ["cmake-build", "tokio"] }

# Legacy protocol testing (pure Rust)
kafka = { version = "0.10", default-features = false, features = ["gzip", "snappy"] }
```

## References

- [Spike: rskafka Evaluation](../spikes/rskafka-evaluation/SPIKE.md)
- [rdkafka cmake-build docs](https://docs.rs/rdkafka/latest/rdkafka/#cmake-build)
- [librdkafka configuration](https://github.com/confluentinc/librdkafka/blob/master/CONFIGURATION.md)
