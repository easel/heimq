---
ddx:
  id: helix.parking-lot
  type: registry
  activity: frame
---

# Parking Lot (Deferred / Future Work)

## Purpose

Registry for heimq work that is deliberately deferred or future-scoped so it
stays findable without contaminating current commitments. Seeded by the
2026-06-11 alignment pass with deferrals already recorded in the PRD, feature
specs, and contracts; it contains nothing not already decided elsewhere.

## Policy
- Rejected items do not belong here; close or cancel them instead.
- Active work does not belong here; track it in the Feature Registry and DDx.
- Deferred items must include rationale and revisit trigger.
- Revisit triggers must be objective enough for another agent to evaluate.
- Any parked artifact must set `ddx.parking_lot: true` in its frontmatter.

## Deferred / Future Items

### OpenMessaging Benchmark version bumps
- **Type**: Future
- **Artifact Type**: Other (maintenance)
- **Source**: PRD §Resolved Decisions / ADR-004
- **Rationale**: Ordinary maintenance; the benchmark dependency is pinned and bumps carry no design decisions.
- **Impact if Omitted**: Benchmark tooling drifts behind upstream releases.
- **Dependencies**: None
- **Revisit Trigger**: A new OpenMessaging Benchmark release lands.
- **Target Activity/Milestone**: Maintenance
- **Owner**: heimq maintainers
- **Last Reviewed**: 2026-06-11

### Ecosystem-tool divergences from documented Kafka behavior
- **Type**: Deferred
- **Artifact Type**: Other (compatibility findings)
- **Source**: PRD Risks table; FEAT-005 edge-case policy
- **Rationale**: Per the recorded policy, each divergence is captured here with a reproducer rather than silently emulated.
- **Impact if Omitted**: Tool-specific quirks get emulated ad hoc without traceability.
- **Dependencies**: A reproducer for the observed divergence
- **Revisit Trigger**: An ecosystem tool is observed diverging from documented Kafka behavior, with a reproducer in hand.
- **Target Activity/Milestone**: As encountered
- **Owner**: heimq maintainers
- **Last Reviewed**: 2026-06-11

### Configurable artificial latency / drop injection for chaos-style tests
- **Type**: Future
- **Artifact Type**: Feature Spec
- **Source**: prd.md Nice to Have (P2 #2)
- **Rationale**: PRD P2 nice-to-have; unscheduled.
- **Impact if Omitted**: No chaos-style fault injection in tests; core compatibility goals unaffected.
- **Dependencies**: None recorded
- **Revisit Trigger**: P2 items are scheduled into an active milestone.
- **Target Activity/Milestone**: Unscheduled
- **Owner**: heimq maintainers
- **Last Reviewed**: 2026-06-11

### Per-API version maxima bumps beyond current targets
- **Type**: Future
- **Artifact Type**: Other (maintenance)
- **Source**: API-001 §Version Policy
- **Rationale**: Tracked as maintenance once FEAT-006 lands; current targets are fixed by the API-001 version matrix.
- **Impact if Omitted**: Advertised per-API maxima lag the current Kafka spec over time.
- **Dependencies**: FEAT-006 (flexible-version protocol) landed
- **Revisit Trigger**: FEAT-006 lands and a newer Kafka spec raises a per-API maximum beyond the current targets.
- **Target Activity/Milestone**: Maintenance (post-FEAT-006)
- **Owner**: heimq maintainers
- **Last Reviewed**: 2026-06-11

### Golden traces / Kafka docker baseline (test-plan Phase 4)
- **Type**: Deferred
- **Artifact Type**: Other (test-plan phase)
- **Source**: test-plan Phase 4 + AR-2026-04-26
- **Rationale**: Parity harness compares live behavior, making static golden traces redundant unless byte-level regression pinning is needed.
- **Impact if Omitted**: No Kafka docker target in `scripts/compatibility-test.sh` and no golden request/response fixtures; behavioral regressions are caught by the live Phase 8 harness instead.
- **Dependencies**: Phase 8 differential parity harness (FEAT-003)
- **Revisit Trigger**: Parity harness proves insufficient for byte-level regressions or a no-docker CI environment is needed.
- **Target Activity/Milestone**: Unscheduled
- **Owner**: heimq maintainers
- **Last Reviewed**: 2026-06-11

### TRAIT-002: sequencing/commit seam for leaderless multi-partition writers
- **Type**: Deferred
- **Artifact Type**: Contract (engine trait)
- **Source**: fjord ADR-005 / fjord TD-005 §Heimq seam (2026-06-14)
- **Rationale**: fjord's diskless re-baseline assigns Kafka offsets at a
  metadata-plane commit that atomically sequences **many partitions** carried in
  one multiplexed object — a shape that does not fit
  `PartitionLog::append` (per-partition, returns offsets synchronously from a
  local counter, assumes a single writer owns the partition). fjord's chosen
  path (S1) is to **own sequencing above the heimq log traits** and use them
  only for raw object IO, so heimq needs **no** immediate change and its lossy
  single-node distribution stays free of object-storage/multi-partition-commit
  machinery (charter intact). A heimq-side seam (S2: a `LogBackend`-level atomic
  multi-partition commit + offset-assignment hook, feature/trait-gated off the
  distribution) is recorded as TRAIT-002 for if/when a second consumer needs it.
- **Impact if Omitted**: fjord proceeds via S1 with no heimq change; only the
  shared-seam reuse across consumers is deferred.
- **Dependencies**: A second engine consumer (niflheim/pqueue) needing
  commit-time multi-partition sequencing; fjord SPIKE-001 Workload 0 confirming
  the sequencing path is viable at all.
- **Revisit Trigger**: niflheim or pqueue requires the same sequence-at-commit
  seam, OR fjord finds S1 forces it to fork heimq log internals.
- **Target Activity/Milestone**: Unscheduled (revisit post fjord Phase 3)
- **Owner**: heimq maintainers + fjord
- **Last Reviewed**: 2026-06-14

## Parked Artifacts (Links)

None.
