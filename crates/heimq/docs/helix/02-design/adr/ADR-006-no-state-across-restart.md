---
ddx:
  id: ADR-006
  status: accepted
  review:
    self_hash: 881bf5e99cfef0f38fec536b48898ee1d3bf40b1be8bd2fbfc036f48aabfc385
    deps: {}
    reviewed_at: "2026-06-22T21:30:26Z"
---

# ADR-006: No Idempotent-Producer / Transaction State Across Restart

| Date | Status | Deciders | Related | Confidence |
|------|--------|----------|---------|------------|
| 2026-06-11 | Accepted | heimq maintainers | FEAT-002 | High |

> Extracted from PRD §Resolved Decisions during the 2026-06-11 alignment pass.

## Context

FEAT-002 (core Kafka semantics) covers idempotent producers and transactions. The PRD ([../../01-frame/prd.md](../../01-frame/prd.md)) needed a fixed answer to whether producer-ID and transaction state survive a broker restart, since heimq is an in-memory broker. See [FEAT-002](../../01-frame/features/FEAT-002-core-kafka-semantics.md).

## Decision

Idempotent producer / transaction state across restart: not retained. heimq is in-memory; on restart the broker presents as a fresh broker. This matches what real Kafka does when its log is lost (e.g., disk wipe / fresh broker): clients receive UNKNOWN_PRODUCER_ID and re-initialize via InitProducerId. Modern librdkafka and the Java client handle this transparently. No heimq-specific recovery behavior — Kafka spec applies.

## Alternatives

No alternatives were recorded in the PRD; the implied alternative — persisting producer/transaction state across restart — contradicts heimq's in-memory design and was not pursued.

## Consequences

| Type | Impact |
|------|--------|
| Positive | No heimq-specific recovery code; behavior matches a real Kafka broker with a lost log, which modern clients already handle transparently. |
| Negative | Clients pinned to very old versions that mishandle UNKNOWN_PRODUCER_ID may fail after a heimq restart. |
| Neutral | Restart semantics are defined entirely by the Kafka spec, not by heimq. |

## References

- [PRD §Resolved Decisions](../../01-frame/prd.md)
- [FEAT-002 core Kafka semantics](../../01-frame/features/FEAT-002-core-kafka-semantics.md)
