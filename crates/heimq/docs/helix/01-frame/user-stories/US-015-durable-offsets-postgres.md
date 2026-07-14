---
ddx:
  id: US-015
  depends_on:
    - FEAT-007
  review:
    self_hash: 2e5fa6adc59b1e69ed0b8be84483707fe98b10907be6beefc9c39b7b62d14174
    deps:
      FEAT-007: e947608aa0633732bde029cdf025cb8875286da1fed7bf9480548917157390ad
    reviewed_at: "2026-07-14T06:48:37Z"
---
# US-015 — Durable offsets via Postgres

**Feature**: FEAT-007 — Durable Offset Backend (Postgres)
**Feature Requirements**: DO-01, DO-02, DO-03, DO-04
**PRD Requirements**: FR-11
**Priority**: P1
**Status**: Implemented (backfilled)

> **Backfill provenance**: reconstructed 2026-06-11 from existing tests
> and CI (`tests/postgres_offsets.rs`, `scripts/stress-matrix.sh`,
> `.github/workflows/stress-matrix.yml`) and
> [AR-2026-04-26-storage-backends](../../06-iterate/alignment-reviews/AR-2026-04-26-storage-backends.md).
> Acceptance criteria are derived from what the shipped tests assert,
> not invented; gaps are marked.

## Story

**As a** platform / CI maintainer (PRD secondary persona),
**I want** to point heimq's offset store at Postgres via
`HEIMQ_STORAGE_OFFSETS=postgres://…`,
**So that** consumer groups in longer-lived dev/CI environments resume
from their committed offsets after a broker restart instead of
re-reading or skipping data.

## Context

heimq is in-memory by default and loses all state on restart (PRD
non-goal #1). For CI environments where the broker outlives a single
test run — deploys, crashes, container reschedules — losing committed
offsets forces consumers back to auto-offset-reset behavior. This story
exercises FEAT-007 DO-01 through DO-04, tracing to PRD FR-11 / P1 #1:
opt-in durability for offsets only, default unchanged.

## Walkthrough

1. Maintainer provisions a Postgres instance and starts heimq with
   `HEIMQ_STORAGE_OFFSETS=postgres://user:pw@host/db?schema=ci` (or
   `--storage-offsets …`).
2. heimq connects at startup, idempotently creates the schema/table,
   and serves the normal Kafka API surface; misconfiguration fails the
   start instead of failing later commits.
3. A consumer group commits offsets through the standard
   `OffsetCommit` API.
4. The broker process restarts (same Postgres URL/schema).
5. The consumer's `OffsetFetch` returns the offsets committed before
   the restart; the group resumes from its committed position.
6. Other environments that set nothing continue to get the in-memory
   store with unchanged behavior.

## Acceptance Criteria

- [x] **US-015-AC1** — Given a heimq binary built with
  `backend-postgres` and a reachable Postgres, when the broker is
  started with `--storage-offsets postgres://…` /
  `HEIMQ_STORAGE_OFFSETS=postgres://…`, then it starts successfully and
  `OffsetCommit` requests succeed (error_code 0) against the Postgres
  store.
- [x] **US-015-AC2** — Given an offset committed via `OffsetCommit`
  while the Postgres backend is active, when the broker process is
  killed and restarted against the same Postgres URL and schema, then
  `OffsetFetch` returns that committed offset (error_code 0, same
  offset value).
- [x] **US-015-AC3** — Given no offset-storage configuration, when the
  broker starts, then the offset backend defaults to `memory://` and
  the default-config test suite passes unchanged (durable backend not
  required for correctness).
- [x] **US-015-AC4** — Given the same client workload (produce, consume
  with consumer groups, offset commit/fetch), when it is run against
  both the memory-only and pg-offsets backend configurations, then it
  passes on both — same client-observable `OffsetCommit`/`OffsetFetch`
  semantics either backend.

## Edge Cases

From FEAT-007 § Edge Cases and Error Handling:

- **Commit fails against Postgres (KNOWN OPEN GAP, AR Q5)**: the client
  still receives a Kafka success response; the error is only logged
  (`src/storage/postgres_offsets.rs:89-95`). Recorded, not resolved.
- **`leader_epoch`**: not persisted; `OffsetFetch` from Postgres always
  reports leader_epoch 0 (AR Q6).
- **Postgres unreachable / unknown scheme / feature not compiled**:
  startup fails fast with a config or storage error.
- **Concurrent CI runs**: isolated via per-run `?schema=` parameter
  (the integration test uses a unique schema per run,
  `tests/postgres_offsets.rs:155-159`).

## Test Scenarios

Existing tests/CI covering each AC (backfill — these already ship):

| Scenario | AC ID | Existing test / CI | Notes |
|----------|-------|--------------------|-------|
| Start with Postgres offsets, commit succeeds | US-015-AC1 | `tests/postgres_offsets.rs::postgres_offset_store_survives_restart` (Phase 1, lines 164-190) | Requires `--features backend-postgres` and `HEIMQ_PG_TEST_URL`; self-skips otherwise |
| Committed offset survives restart | US-015-AC2 | `tests/postgres_offsets.rs::postgres_offset_store_survives_restart` (Phase 2, lines 193-211) | Kills and respawns the broker against the same URL+schema; asserts offset 1234 returned |
| Default remains `memory://`, suite unchanged | US-015-AC3 | `src/config.rs` config tests (default assertion at line 144); `cargo test --workspace` on default config (AR AC table row 2); `scripts/stress-matrix.sh` `memory-only` row | |
| Same workload passes on both backends | US-015-AC4 | `scripts/stress-matrix.sh` (`memory-only` + `pg-offsets` rows, same kcat-stress workload) via `.github/workflows/stress-matrix.yml` | CI job is `continue-on-error` (advisory, not a hard gate); pg row self-skips when `POSTGRES_URL` unset. No assertion-level diff of responses between backends exists — parity is "same workload passes", plus the AR Q6 leader-epoch caveat |

No AC is fully untested; AC4's coverage is the weakest (advisory CI,
workload-level rather than response-diff parity) and is flagged as such
above.

## Dependencies

- **Stories**: US-002 (consumer-group offset commit/fetch path must
  work for either backend to be exercised).
- **Feature Spec**: FEAT-007
- **Feature Requirements**: DO-01, DO-02, DO-03, DO-04
- **PRD Requirements**: FR-11
- **External**: Postgres (CI uses `postgres:16` service container);
  `backend-postgres` cargo feature; kcat for the stress-matrix
  workload.

## Out of Scope

Per FEAT-007 § Out of Scope:

- Durable logs or group state — offsets only.
- Surfacing Postgres commit failures to clients (AR Q5 follow-up, not
  this story).
- Leader-epoch persistence (AR Q6).
