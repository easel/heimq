---
ddx:
  id: FEAT-007
  depends_on:
    - helix.prd
  review:
    self_hash: e947608aa0633732bde029cdf025cb8875286da1fed7bf9480548917157390ad
    deps:
      helix.prd: debc0a32007f0c42db51e82f47848c7b988c3f67f6f97171069170492f9b5b95
    reviewed_at: "2026-07-14T06:48:37Z"
---
# Feature Specification: FEAT-007 — Durable Offset Backend (Postgres)

**Feature ID**: FEAT-007
**Status**: Implemented (backfilled)
**Priority**: P1
**Owner**: heimq core
**Covered PRD Subsystem(s)**: Durable offset backend (FEAT-007)
**Covered PRD Requirements**: FR-11 (opt-in Postgres offset store) — PRD P1 #1
**Cross-Subsystem Rationale**: None — single subsystem.

> **Backfill provenance**: this specification was reconstructed on
> 2026-06-11 from implementation evidence, after the feature shipped
> without a governing frame artifact. Primary evidence:
> [AR-2026-04-26-storage-backends](../../06-iterate/alignment-reviews/AR-2026-04-26-storage-backends.md)
> (alignment review of the "Pluggable storage backends" epic,
> heimq-2136e295); source: `src/storage/offset_store.rs` (trait),
> `src/storage/postgres_offsets.rs` (Postgres impl),
> `src/storage/dispatch.rs` (URL-scheme dispatch), `src/config.rs`
> (`--storage-offsets` / `HEIMQ_STORAGE_OFFSETS`); tests:
> `tests/postgres_offsets.rs`; CI: `.github/workflows/stress-matrix.yml`
> + `scripts/stress-matrix.sh`. Statements below are confirmed against
> that evidence unless explicitly marked *(inferred)*.

## Overview

heimq is in-memory by default and permitted to lose state on restart
(PRD non-goal #1). FEAT-007 is the one opt-in durability slice the PRD
allows: a Postgres-backed `OffsetStore` so committed consumer-group
offsets survive a broker restart even though message logs do not.
Covers PRD FR-11 / P1 #1.

## Ideal Future State

A CI maintainer or developer who wants offsets to outlive broker
restarts sets `HEIMQ_STORAGE_OFFSETS=postgres://…` (or passes
`--storage-offsets`) and nothing else changes: clients use the same
`OffsetCommit` / `OffsetFetch` APIs, the broker starts fail-fast if the
backend is misconfigured, and a restarted broker resumes consumer groups
from the offsets committed before the restart. Everyone else keeps the
in-memory default and is unaffected.

## Problem Statement

- **Current situation** (at the time the epic was framed): all broker
  state, including committed consumer-group offsets, lived in memory; a
  broker restart lost every committed offset, so any consumer resumed
  from auto-offset-reset policy rather than its committed position.
- **Pain points**: longer-lived dev/CI environments that restart the
  broker (deploys, crashes, container reschedules) force consumers to
  re-read or skip data; mock-style brokers offer no remedy.
- **Desired outcome**: an opt-in durable offset store such that
  committed offsets survive broker restart, without making durability a
  default or a correctness dependency for any in-scope feature.

## Requirements

### Functional Requirements

- **DO-01** — **Opt-in activation.** The offset backend is selected by
  URL scheme via `HEIMQ_STORAGE_OFFSETS` / `--storage-offsets`
  (`src/config.rs:69-70`). `postgres://…` selects the Postgres-backed
  store when the binary is built with the `backend-postgres` feature
  (`src/storage/mod.rs:11-12,24-25`); an unrecognized scheme fails fast
  at startup with a config error listing supported schemes
  (`src/storage/dispatch.rs`, per AR § Findings "Dispatch fail-fast").
  An optional `?schema=<name>` URL parameter isolates the table into a
  named Postgres schema; schema initialization is idempotent
  (`src/storage/postgres_offsets.rs:30-66`).
- **DO-02** — **Restart survival.** Offsets committed through the Kafka
  `OffsetCommit` API while the Postgres backend is active are readable
  via `OffsetFetch` after a full broker process restart pointed at the
  same Postgres database/schema. The backend declares
  `survives_restart: true`, durability `WalFsync`
  (`src/storage/postgres_offsets.rs:15-20`).
- **DO-03** — **In-memory default unchanged.** The default offset
  backend remains `memory://` (`src/config.rs:69`); with no
  configuration, behavior is identical to pre-FEAT-007 heimq, and
  correctness of all in-scope features (FR-1..FR-10) does not depend on
  the durable backend (PRD non-goal #1).
- **DO-04** — **Backend parity.** Client-observable `OffsetCommit` /
  `OffsetFetch` semantics are the same regardless of backend: the
  handlers operate on the `OffsetStore` trait only
  (`src/handler/offset_commit.rs`, `src/handler/offset_fetch.rs`), and
  the Postgres impl is a clean trait implementation with no leakage into
  consumer-group code (AR § Findings "Postgres OffsetStore is a clean
  trait impl"). The same client workload passes against the memory-only
  and pg-offsets configurations (`scripts/stress-matrix.sh` rows). Known
  parity caveats are recorded under Edge Cases below.

### Non-Functional Requirements

- **Reliability**: misconfiguration (unknown scheme, unreachable
  Postgres) fails at startup, not at first commit
  (`src/storage/dispatch.rs`; `PostgresOffsetStore::connect` returns
  `HeimqError::Storage`, `src/storage/postgres_offsets.rs:35-46`).
- **Security**: `?schema=` names are validated against DDL injection
  before interpolation into `CREATE SCHEMA` statements
  (`postgres_offsets.rs:202-210` per AR § Findings).
- **Performance**: no specific target was framed for this slice
  *(inferred — no performance requirement found in the epic AC or AR)*;
  the stress-matrix workload (5 topics x 10k msgs) completing is the
  only load evidence.

## User Stories

- [US-015 — Durable offsets via Postgres](../user-stories/US-015-durable-offsets-postgres.md)

## Edge Cases and Error Handling

- **Commit fails against Postgres (KNOWN OPEN GAP — AR Q5)**:
  `OffsetStore::commit` is infallible at the trait level, so
  `PostgresOffsetStore::commit` swallows DB errors via
  `tracing::error!` and the client receives a normal Kafka success
  response for a commit that was not persisted
  (`src/storage/postgres_offsets.rs:89-95`). Severity rated
  medium (durability semantics) in
  [AR-2026-04-26-storage-backends](../../06-iterate/alignment-reviews/AR-2026-04-26-storage-backends.md)
  Q5, with a suggested follow-up (make the trait return `Result<()>`
  and map failures to a Kafka error code). **Recorded, not resolved** —
  this backfill does not change the behavior.
- **`leader_epoch` not persisted (AR Q6)**: the v1 Postgres schema has
  no leader-epoch column; `commit` discards it and `fetch` always
  returns `leader_epoch: 0` (`src/storage/postgres_offsets.rs:69-120`).
  Known v1 limitation; a parity caveat against DO-04 for clients that
  read the epoch back.
- **Postgres unreachable at startup**: `connect` returns a storage
  error and the broker does not start (fail-fast, DO-01).
- **Binary built without `backend-postgres`**: `postgres://` is an
  unrecognized scheme and startup fails with the supported-scheme list
  (`src/storage/mod.rs:11-12`; AR Q4 notes the test-side cfg wrinkle).
- **Broker restart with active groups**: message logs and group state
  are still in-memory and lost; only committed offsets survive. This is
  by design (PRD non-goal #1, ADR-006 for producer/txn state).

## Success Metrics

- The restart-survival integration test
  (`tests/postgres_offsets.rs::postgres_offset_store_survives_restart`)
  passes when run with `--features backend-postgres` against a real
  Postgres.
- The backend-matrix CI job (`stress-matrix.yml`) runs the same kcat
  workload green on both the memory-only and pg-offsets rows (advisory:
  the job is `continue-on-error`).
- Default-config test suite remains green with zero configuration
  changes (DO-03).

## Constraints and Assumptions

- Single-node broker; the Postgres store holds offsets only — logs and
  group-coordinator state remain in-memory (epic scope per AR).
- A reachable Postgres instance is the operator's responsibility; heimq
  only creates its schema/table idempotently.
- Built behind the `backend-postgres` cargo feature; the default build
  has no Postgres dependency at runtime.

## Dependencies

- **Other features**: none at the trait-consumer level — `OffsetStore`
  is consumed only by the `OffsetCommit`/`OffsetFetch` handlers
  (`src/handler/offset_commit.rs`, `src/handler/offset_fetch.rs`).
  Verified 2026-06-11: no transactional/group-commit path
  (`TxnOffsetCommit`) exists in `src/handler/`, so FEAT-002 is **not** a
  dependency of this slice. The capability descriptor feeds
  `compute_supported_apis` gating from FEAT-001's ApiVersions surface
  (AR § Implementation Map).
- **External services**: Postgres (tested against `postgres:16` in CI).
- **PRD requirements**: FR-11 / P1 #1.

## Out of Scope

- Durable message logs or durable group-coordinator state (only the
  offset subsystem is durable; PRD non-goal #1 stands for everything
  else).
- Making the durable backend the default, or required for correctness
  of any in-scope feature.
- Fallible commit error reporting to clients (AR Q5 — recorded above as
  a known gap, deliberately not resolved here).
- Leader-epoch persistence/fencing (AR Q6 — known v1 limitation).
- Other durable backends (e.g., SQLite, files); only Postgres shipped.
