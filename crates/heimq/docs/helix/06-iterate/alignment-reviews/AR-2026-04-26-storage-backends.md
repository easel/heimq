# Alignment Review: storage-backends

**Review Date**: 2026-04-26
**Scope**: pluggable storage (LogBackend / OffsetStore / GroupCoordinatorBackend traits, BackendCapabilities, URL-scheme dispatch, Postgres OffsetStore feature flag, `compute_supported_apis` intersection)
**Status**: complete
**Review Bead**: heimq-c79880b4
**Parent**: heimq-b33b5a3e (alignment review epic)
**Subject Epic**: heimq-2136e295 ("Pluggable storage backends")

## Subject Epic Acceptance Criteria

From heimq-2136e295:

> All child beads closed. heimq starts with default config using three memory backends and all existing tests pass. One durable backend (Postgres OffsetStore) ships behind a feature flag with integration tests. Capability descriptor drives ApiVersions response.

## AC Verification

| AC item | Evidence | Status |
|---------|----------|--------|
| All child beads closed | 10 children of heimq-2136e295 all `closed` (heimq-04def9c7, heimq-20daf864, heimq-24c6cd00, heimq-3af6652c, heimq-4ac4601c, heimq-620861e8, heimq-9e9f257c, heimq-c894fcda, heimq-ed70c613, heimq-fb3e5ca8) | SATISFIED |
| Default config = three memory backends + existing tests pass | `src/storage/dispatch.rs:30-77` (`memory://` for log, offsets, groups); `src/server.rs:35-45` wires all three via dispatch; `cargo test --workspace` reported green by repo-level AR-2026-04-26 | SATISFIED |
| Postgres OffsetStore behind feature flag + integration tests | `src/storage/postgres_offsets.rs` gated by `#[cfg(feature = "backend-postgres")]` (`src/storage/mod.rs:11-12,24-25`); `tests/postgres_offsets.rs` exists and is feature-gated | SATISFIED |
| Capability descriptor drives ApiVersions | `src/protocol/mod.rs::compute_supported_apis` (lines 105-120) consumes `BackendCapabilities + OffsetStoreCapabilities + GroupCoordinatorCapabilities`; `src/server.rs:47` and `src/protocol/router.rs:30` invoke it; result is plumbed into `api_versions::handle` via `Router::advertised_apis` | SATISFIED |

## Implementation Map

- **Trait surface**:
  - `src/storage/mod.rs:64-148` — `LogBackend`, `TopicLog`, `PartitionLog` (with `RecordBatchView` + raw-bytes fast-path; `FetchWait::{Immediate, LongPoll}`)
  - `src/storage/offset_store.rs:58-84` — `OffsetStore` (commit / fetch / fetch_all_for_group / delete_group / capabilities)
  - `src/consumer_group/backend.rs:118-142` — `GroupCoordinatorBackend` (join / sync / heartbeat / leave + capabilities); request/response shaped, not snapshot KV — matches design intent for live coordinator state
- **Capability descriptors**:
  - `src/storage/capabilities.rs:62-106` — `BackendCapabilities` (durability, atomic_append, retention, compression, max sizes, fetch_wait, transactions, idempotent_producer, headers, timestamps, truncate)
  - `src/storage/offset_store.rs:28-37` — `OffsetStoreCapabilities`
  - `src/consumer_group/backend.rs:20-31` — `GroupCoordinatorCapabilities`
- **URL-scheme dispatch**: `src/storage/dispatch.rs` — `dispatch_log_backend`, `dispatch_offset_store`, `dispatch_group_coordinator`. Currently only `memory://` (and `postgres://` when feature-enabled for offsets). Unknown schemes produce a `HeimqError::Config` with the supported-scheme list (fail-fast at startup).
- **ApiVersions intersection**: `src/protocol/mod.rs:75-120`. Per-API gate — Produce/Fetch/ListOffsets/Metadata/CreateTopics/DeleteTopics gated on log; OffsetCommit/Fetch on offset store; FindCoordinator + group APIs on group coordinator; ApiVersions itself always advertised. Tests cover memory default, missing-coord, missing-offset-store, and the explicit "compaction=false must not drop unrelated APIs" pin.
- **Call-time rejection** (heimq-620861e8): `src/handler/produce.rs:35-67` enforces `max_message_bytes` / `max_batch_bytes` returning `MESSAGE_TOO_LARGE` (10), and `caps.transactions=false` returning `INVALID_TXN_STATE` (48). Fetch handler collapses long-poll to immediate when `caps.fetch_wait=false`.

## Findings

### Aligned

| Area | Evidence |
|------|----------|
| Trait shape matches design intent | Log trait is structured (RecordBatchView + raw-bytes fast-path), not byte-opaque (`storage/mod.rs:121-148`); GroupCoordinator is request/response shaped (`consumer_group/backend.rs:118-142`); per-API gating, not global meet (`protocol/mod.rs::capability_gate`) |
| Postgres OffsetStore is a clean trait impl | `src/storage/postgres_offsets.rs:69-175` implements `OffsetStore` only — no leakage into consumer-group code; schema-init is idempotent (`CREATE … IF NOT EXISTS`) and `parse_postgres_url` validates schema names against DDL-injection (`postgres_offsets.rs:202-210`) |
| Dispatch fail-fast | Unknown scheme → `HeimqError::Config` at startup (`dispatch.rs:31-35,47-57,71-76`); covered by tests at `dispatch.rs:115-139` |
| Per-API capability gate is explicit and tested | `protocol/mod.rs:75-86` (`capability_gate`); pinned by `compaction_false_does_not_leak_compaction_specific_apis` (lines 207-220) |

### Quality Notes (non-blocking)

| # | Concern | Severity | Evidence | Suggested follow-up |
|---|---------|----------|----------|---------------------|
| Q1 | `compute_supported_apis` presence heuristic uses `name != "unknown"`. Any backend that legitimately leaves `name` unset (e.g. test scaffolding, future no-op backend) is silently dropped from advertisement. | low | `src/protocol/mod.rs:88-98` | Replace with an explicit `present` flag on the capability struct, or make `name: Option<&'static str>` with `None` ⇒ absent. |
| Q2 | Capability *flags* (compaction, transactions, idempotent_producer, retention, compression set, etc.) on `BackendCapabilities` do not currently affect `compute_supported_apis` — only `name` does. The static `SUPPORTED_APIS` list contains no compaction-/transaction-specific entries to gate, which is why the test pins this. The descriptor is therefore richer than current consumers require. | informational | `protocol/mod.rs:75-120` vs `storage/capabilities.rs:62-106` | Acceptable as forward-compat surface. Document the asymmetry in `capabilities.rs` so future API additions remember to wire their gates. |
| Q3 | Per-API `capability_gate` keys are a hand-maintained list of API key integers (`0,1,2,3,8,9,10,11,12,13,14,18,19,20`). If a new API is added to `SUPPORTED_APIS` and the matcher is not updated, it silently falls through to `Always`. | low | `src/protocol/mod.rs:75-86` | Either make the gate a property of the entry in `SUPPORTED_APIS` (extend tuple to include gate enum) or assert in a test that every key in `SUPPORTED_APIS` is also present in the `capability_gate` match arms. |
| Q4 | `dispatch::tests::unknown_offset_scheme_errors` uses `postgres://x` as the "unknown" scheme. When the `backend-postgres` feature is enabled, `postgres://` is a *recognized* scheme and the test will attempt a real connection (and fail differently). | low | `src/storage/dispatch.rs:122-126` | Add `#[cfg(not(feature = "backend-postgres"))]` to the test, or pick a scheme that is unrecognized in both feature configurations (`weird://` is already used for log/group — reuse it). |
| Q5 | `OffsetStore::commit` is infallible at the trait level. `PostgresOffsetStore::commit` therefore swallows DB errors via `tracing::error!` and returns `()` — a client whose commit failed will receive a normal Kafka success response. | medium (durability semantics) | `src/storage/postgres_offsets.rs:89-95` | Out of scope for this epic, but worth a follow-up bead: consider returning `Result<()>` from the trait so the OffsetCommit handler can map a backend failure to `OFFSET_METADATA_TOO_LARGE` / `COORDINATOR_LOAD_IN_PROGRESS` / a generic `UNKNOWN_SERVER_ERROR`. |
| Q6 | `PostgresOffsetStore` ignores `leader_epoch` — schema has no column for it; `commit` discards `_leader_epoch`, `fetch` always returns `leader_epoch: 0`. | low | `src/storage/postgres_offsets.rs:69-120` | Document as a known limitation of the v1 Postgres schema. Add a follow-up bead if/when leader-epoch fencing becomes load-bearing. |
| Q7 | Router does not consult `advertised_apis` to reject filtered-out API keys at request time — it dispatches purely on the static `match header.api_key` arm (`router.rs:63-82`). For the current memory + Postgres-offset configurations every API is advertised, so this is latent only. | low | `src/protocol/router.rs:53-85`; `compute_supported_apis` filters but is consumed only by `api_versions::handle` (`router.rs:108`) | Optional hardening: gate the `match` arms with `if self.advertised_apis.iter().any(\|(k,_,_)\| *k == header.api_key)`, or fold ungated keys into the `_ =>` unsupported branch. Not required for any present backend. |

None of the above blocks epic closure.

## Closure Recommendation

**RECOMMEND CLOSE heimq-2136e295.**

All four AC items are satisfied with concrete, test-covered evidence:
- 10/10 child beads closed.
- Three memory backends wired through URL dispatch on default config (`server.rs:35-45`).
- Postgres OffsetStore feature-gated with integration tests (`postgres_offsets.rs`, `tests/postgres_offsets.rs`).
- `compute_supported_apis` per-API gating is implemented and unit-tested for memory-default, missing-coord, missing-offset-store, and compaction-false-doesn't-leak.

Quality findings Q1–Q7 above are minor polish and durability-semantics observations, not AC gaps. Suggested handling:

- Q4 (test cfg gating) and Q3 (gate keyed off `SUPPORTED_APIS`) are cheap and could be folded into existing `heimq-3feeffb3` (epic closure / doc refresh) or filed as separate small `kind:fix` beads.
- Q5 (fallible OffsetStore commit) is a meaningful trait-shape question; deserves its own bead post-closure if/when durable-commit error reporting is prioritized.
- Q1, Q2, Q6, Q7 are forward-compat / latent items — fine to leave on the backlog without immediate beads.

Recommended action sequence for the closure operator:
1. Confirm `cargo test --workspace --all-features` and `cargo test -p heimq --test postgres_offsets --features backend-postgres` are green on the closing revision.
2. Close heimq-2136e295 with reference to this AR.
3. Optionally file follow-ups for Q3, Q4, Q5 as `area:storage`, `kind:fix` / `kind:feat` beads (not blocking).

## Cross-references

- Repo-level AR: `docs/helix/06-iterate/alignment-reviews/AR-2026-04-26-repo.md` (storage-backends row classified ALIGNED; epic closure pending = heimq-3feeffb3).
- Sibling reviews under heimq-b33b5a3e: heimq-600562e8 (consumer-groups), heimq-013aaa35 (e2e Phase 5/6).
