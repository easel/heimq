---
ddx:
  id: ADR-004
  status: accepted
---

# ADR-004: OpenMessaging Benchmark Version Targeting

| Date | Status | Deciders | Related | Confidence |
|------|--------|----------|---------|------------|
| 2026-06-11 | Accepted | heimq maintainers | FEAT-004 | High |

> Extracted from PRD §Resolved Decisions during the 2026-06-11 alignment pass.

## Context

FEAT-004 (benchmark conformance) requires heimq to run under the OpenMessaging Benchmark Kafka driver. The PRD ([../../01-frame/prd.md](../../01-frame/prd.md)) needed a fixed answer to which OpenMessaging Benchmark version the bench harness builds against, so that conformance results are reproducible. See [FEAT-004](../../01-frame/features/FEAT-004-benchmark-conformance.md).

## Decision

OpenMessaging Benchmark version: target the latest released driver. The bench harness pins to that release; bumps are tracked as ordinary maintenance.

## Alternatives

No alternatives were recorded in the PRD; the implied alternative — tracking an unpinned/floating driver version — is rejected because unreproducible harness builds would make benchmark regressions unattributable.

## Consequences

| Type | Impact |
|------|--------|
| Positive | Reproducible benchmark runs; clear upgrade path via ordinary maintenance bumps. |
| Negative | The pin can lag upstream releases until a maintenance bump lands. |
| Neutral | Version bumps are routine work items, not architecture changes. |

## References

- [PRD §Resolved Decisions](../../01-frame/prd.md)
- [FEAT-004 benchmark conformance](../../01-frame/features/FEAT-004-benchmark-conformance.md)
