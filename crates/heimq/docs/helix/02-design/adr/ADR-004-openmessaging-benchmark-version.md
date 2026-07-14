---
ddx:
  id: ADR-004
  status: accepted
  review:
    self_hash: 0ef955788f215b561be68b17a67b6bb3517c562404d9f3c5d4572705eed383c7
    deps: {}
    reviewed_at: "2026-07-14T06:48:37Z"
---

# ADR-004: OpenMessaging Benchmark Version Targeting

| Date | Status | Deciders | Related | Confidence |
|------|--------|----------|---------|------------|
| 2026-06-11 | Accepted | heimq maintainers | FEAT-004 | High |

> Extracted from PRD §Resolved Decisions during the 2026-06-11 alignment pass.

## Context

FEAT-004 (benchmark conformance) requires heimq to run under the OpenMessaging Benchmark Kafka driver. The PRD ([../../01-frame/prd.md](../../01-frame/prd.md)) needed a fixed answer to which OpenMessaging Benchmark version the bench harness builds against, so that conformance results are reproducible. See [FEAT-004](../../01-frame/features/FEAT-004-benchmark-conformance.md).

## Decision

OpenMessaging Benchmark version: pin the latest released upstream tag available from `openmessaging/benchmark`, `jms`, at commit `c0e51b8b86a3b0ff50b935152d6e600602a7f0a0`. The bench harness checks out that commit; bumps are tracked as ordinary maintenance.

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
