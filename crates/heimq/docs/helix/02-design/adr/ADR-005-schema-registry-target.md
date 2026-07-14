---
ddx:
  id: ADR-005
  status: accepted
  review:
    self_hash: 3004cc4ce8a153265d6fb5f44e16679f84b3c5abe7940b2d3bd7e1223afead56
    deps: {}
    reviewed_at: "2026-07-14T06:48:37Z"
---

# ADR-005: Schema Registry Compatibility Target

| Date | Status | Deciders | Related | Confidence |
|------|--------|----------|---------|------------|
| 2026-06-11 | Accepted | heimq maintainers | FEAT-005 | High |

> Extracted from PRD §Resolved Decisions during the 2026-06-11 alignment pass.

## Context

FEAT-005 (ecosystem integrations) involves clients and tooling that use a schema registry. The PRD ([../../01-frame/prd.md](../../01-frame/prd.md)) needed a fixed answer to which schema registry API heimq's ecosystem testing targets. See [FEAT-005](../../01-frame/features/FEAT-005-ecosystem-integrations.md).

## Decision

Schema Registry: target the Confluent Schema Registry API. Apicurio's Confluent-compatibility mode may incidentally pass but is not a separate target.

## Alternatives

| Option | Evaluation |
|--------|------------|
| Apicurio (as a separate target) | Rejected as a target: its Confluent-compatibility mode may incidentally pass, but heimq does not test or guarantee it separately. |
| **Confluent Schema Registry API** | **Selected: the de-facto ecosystem standard clients integrate against.** |

## Consequences

| Type | Impact |
|------|--------|
| Positive | One well-defined registry API to test against; matches what most Kafka clients use. |
| Negative | Apicurio-specific behavior outside Confluent-compatibility mode is unsupported and untested. |
| Neutral | Apicurio in Confluent-compatibility mode may work incidentally without any heimq commitment. |

## References

- [PRD §Resolved Decisions](../../01-frame/prd.md)
- [FEAT-005 ecosystem integrations](../../01-frame/features/FEAT-005-ecosystem-integrations.md)
