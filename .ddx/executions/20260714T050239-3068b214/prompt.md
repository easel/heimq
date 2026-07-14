<bead-review>
  <bead id="heimq-ed5504bc" iter=1>
    <title>Cite durable-offset and engine-conformance acceptance coverage</title>
    <description>
Resolve the remaining AR13-05 coverage for US-015 and the new engine stories created for FR-12/FR-13. Add exact @covers citations to Postgres offset tests, stress matrix, heimq-testkit suites, RequestContext tests, handler codec/dispatch tests, and deferred-ack tests. Add focused tests if any new criterion is not exercised. In scope: crates/heimq-testkit/, crates/heimq-broker/, crates/heimq-handlers/, crates/heimq/tests/postgres_offsets.rs, crates/heimq/scripts/stress-matrix.sh. Out of scope: Niflheim code.
    </description>
    <acceptance>
1. The HELIX alignment checker reports no uncited AC for US-015, US-016, or US-017. 2. cargo test -p heimq-broker -p heimq-handlers -p heimq-testkit passes. 3. cargo test -p heimq --test postgres_offsets --features backend-postgres passes or self-skips only for its documented missing external Postgres URL.
    </acceptance>
    <labels>helix, area:testing, kind:traceability, ac-quality:needs-refinement</labels>
  </bead>

  <changed-files>
    <file>crates/heimq-broker/src/context.rs</file>
    <file>crates/heimq-broker/src/produce.rs</file>
    <file>crates/heimq-handlers/src/api_versions.rs</file>
    <file>crates/heimq-handlers/src/codec.rs</file>
    <file>crates/heimq-handlers/src/error.rs</file>
    <file>crates/heimq-testkit/tests/fixture_backend_conformance.rs</file>
    <file>crates/heimq-testkit/tests/memory_backend_conformance.rs</file>
    <file>crates/heimq-testkit/tests/postgres_offset_store_conformance.rs</file>
    <file>crates/heimq/scripts/stress-matrix.sh</file>
    <file>crates/heimq/tests/postgres_offsets.rs</file>
  </changed-files>

  <governing>
    <ref id="FEAT-007" path="crates/heimq/docs/helix/01-frame/features/FEAT-007-durable-offset-backend.md" title="Feature Specification: FEAT-007 — Durable Offset Backend (Postgres)">
      <content>
<untrusted-data>
---
ddx:
  id: FEAT-007
  depends_on:
    - helix.prd
  review:
    self_hash: e947608aa0633732bde029cdf025cb8875286da1fed7bf9480548917157390ad
    deps:
      helix.prd: 236574e8f31d3847bb8269d538fe07d0a47376aa7d7e75c30dca783e479ad4ab
    reviewed_at: "2026-06-22T21:30:26Z"
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
</untrusted-data>
      </content>
    </ref>
  </governing>

  <diff rev="71f8b8fcce76c15bb2b66723019a62c2ebeffe4a">
<untrusted-data>
diff --git a/crates/heimq-broker/src/context.rs b/crates/heimq-broker/src/context.rs
index e5aaf72..3aa36a0 100644
--- a/crates/heimq-broker/src/context.rs
+++ b/crates/heimq-broker/src/context.rs
@@ -42,3 +42,31 @@ impl RequestContext {
         }
     }
 }
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    #[test]
+    // @covers US-016-AC4 US-017-AC1
+    fn request_context_carries_identity_without_policy_decisions() {
+        let ctx = RequestContext::new(
+            Some("principal-a".to_string()),
+            Some("tenant-a".to_string()),
+            Some("client-a".to_string()),
+        );
+
+        assert_eq!(ctx.principal.as_deref(), Some("principal-a"));
+        assert_eq!(ctx.tenant.as_deref(), Some("tenant-a"));
+        assert_eq!(ctx.client_id.as_deref(), Some("client-a"));
+    }
+
+    #[test]
+    // @covers US-016-AC4
+    fn anonymous_context_has_no_embedder_policy_identity() {
+        assert_eq!(RequestContext::anonymous(), RequestContext::ANONYMOUS);
+        assert!(RequestContext::ANONYMOUS.principal.is_none());
+        assert!(RequestContext::ANONYMOUS.tenant.is_none());
+        assert!(RequestContext::ANONYMOUS.client_id.is_none());
+    }
+}
diff --git a/crates/heimq-broker/src/produce.rs b/crates/heimq-broker/src/produce.rs
index 1c2ac84..2075553 100644
--- a/crates/heimq-broker/src/produce.rs
+++ b/crates/heimq-broker/src/produce.rs
@@ -596,6 +596,7 @@ mod tests {
     }
 
     #[test]
+    // @covers US-016-AC4 US-017-AC1
     fn produce_append_core_threads_request_context_to_backend() {
         let storage = TestStorage::new();
         let records = record_batch_header_bytes(1, -1, 0);
@@ -619,6 +620,7 @@ mod tests {
     }
 
     #[test]
+    // @covers US-017-AC2 US-017-AC3
     fn produce_append_async_waits_on_deferred_backend_without_sync_append() {
         let state = Arc::new(DeferredAppendState::new());
         let storage = DeferredStorage::new(state.clone());
diff --git a/crates/heimq-handlers/src/api_versions.rs b/crates/heimq-handlers/src/api_versions.rs
index 9c961db..56c4c09 100644
--- a/crates/heimq-handlers/src/api_versions.rs
+++ b/crates/heimq-handlers/src/api_versions.rs
@@ -22,3 +22,52 @@ pub fn handle(_api_version: i16, apis: &[(i16, i16, i16)]) -> ApiVersionsRespons
 
     response
 }
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    fn keys(response: &ApiVersionsResponse) -> Vec<i16> {
+        response.api_keys.iter().map(|api| api.api_key).collect()
+    }
+
+    #[test]
+    // @covers US-016-AC2
+    fn api_versions_omits_apis_missing_from_capability_gated_input() {
+        let advertised = [(0, 0, 11), (1, 0, 12), (18, 0, 3)];
+
+        let response = handle(3, &advertised);
+        let keys = keys(&response);
+
+        assert!(keys.contains(&0), "Produce remains advertised");
+        assert!(keys.contains(&1), "Fetch remains advertised");
+        assert!(keys.contains(&18), "ApiVersions remains advertised");
+        assert!(
+            !keys.contains(&8) && !keys.contains(&9),
+            "Offset APIs omitted by capability computation must not be reintroduced"
+        );
+        assert!(
+            !keys.contains(&10) && !keys.contains(&11),
+            "Group APIs omitted by capability computation must not be reintroduced"
+        );
+    }
+
+    #[test]
+    // @covers US-016-AC3
+    fn api_versions_preserves_unrelated_apis_when_one_family_is_omitted() {
+        let advertised = [(0, 0, 11), (1, 0, 12), (8, 0, 9), (9, 0, 9), (18, 0, 3)];
+
+        let response = handle(3, &advertised);
+        let keys = keys(&response);
+
+        for key in [0, 1, 8, 9, 18] {
+            assert!(keys.contains(&key), "api key {key} must remain advertised");
+        }
+        for key in [10, 11, 12, 13, 14] {
+            assert!(
+                !keys.contains(&key),
+                "missing group-family key {key} must stay omitted"
+            );
+        }
+    }
+}
diff --git a/crates/heimq-handlers/src/codec.rs b/crates/heimq-handlers/src/codec.rs
index 080509e..9a96607 100644
--- a/crates/heimq-handlers/src/codec.rs
+++ b/crates/heimq-handlers/src/codec.rs
@@ -279,6 +279,7 @@ mod tests {
     }
 
     #[test]
+    // @covers US-017-AC5
     fn test_encode_response_error_mapping() {
         let response = FailingEncode;
         let err = encode_response(1, 0, 0, &response).unwrap_err();
@@ -299,6 +300,7 @@ mod tests {
     }
 
     #[test]
+    // @covers US-017-AC5
     fn test_encode_response_invalid_versions_error() {
         use heimq_protocol::messages::api_versions_response::ApiVersionsResponse;
         use heimq_protocol::messages::create_topics_response::CreateTopicsResponse;
diff --git a/crates/heimq-handlers/src/error.rs b/crates/heimq-handlers/src/error.rs
index 240a244..66e2107 100644
--- a/crates/heimq-handlers/src/error.rs
+++ b/crates/heimq-handlers/src/error.rs
@@ -35,6 +35,7 @@ mod tests {
     use super::*;
 
     #[test]
+    // @covers US-017-AC4
     fn test_error_codes() {
         assert_eq!(HeimqError::TopicNotFound("t".into()).to_error_code(), 3);
         assert_eq!(
diff --git a/crates/heimq-testkit/tests/fixture_backend_conformance.rs b/crates/heimq-testkit/tests/fixture_backend_conformance.rs
index bde0859..49ebb91 100644
--- a/crates/heimq-testkit/tests/fixture_backend_conformance.rs
+++ b/crates/heimq-testkit/tests/fixture_backend_conformance.rs
@@ -16,11 +16,13 @@ use std::sync::Arc;
 // ── WAL-shape ─────────────────────────────────────────────────────────────────
 
 #[test]
+// @covers US-016-AC1
 fn wal_shape_partition_log_suite() {
     suites::partition_log::run_all(&|| Arc::new(WalShapePartitionLog::new(0)) as Arc<_>);
 }
 
 #[test]
+// @covers US-016-AC1
 fn wal_shape_records_in_wal_before_ack() {
     use heimq::storage::{PartitionLog, RecordBatchView};
     use heimq::test_support::encode_record_batch;
@@ -57,6 +59,7 @@ fn wal_shape_records_in_wal_before_ack() {
 // ── Queue-sink-shape ──────────────────────────────────────────────────────────
 
 #[test]
+// @covers US-016-AC1
 fn queue_sink_partition_log_suite() {
     suites::partition_log::run_all(&|| {
         let (log, _rx) = QueueSinkPartitionLog::new(0);
@@ -65,6 +68,7 @@ fn queue_sink_partition_log_suite() {
 }
 
 #[test]
+// @covers US-016-AC1
 fn queue_sink_delivers_to_channel() {
     use heimq::storage::{PartitionLog, RecordBatchView};
     use heimq::test_support::encode_record_batch;
@@ -98,11 +102,13 @@ fn queue_sink_delivers_to_channel() {
 // ── Object-store-shape ────────────────────────────────────────────────────────
 
 #[test]
+// @covers US-016-AC1
 fn object_store_shape_partition_log_suite() {
     suites::partition_log::run_all(&|| Arc::new(ObjectStoreShapePartitionLog::new(0)) as Arc<_>);
 }
 
 #[test]
+// @covers US-016-AC1
 fn object_store_shape_records_in_manifest() {
     use heimq::storage::{PartitionLog, RecordBatchView};
     use heimq::test_support::encode_record_batch;
diff --git a/crates/heimq-testkit/tests/memory_backend_conformance.rs b/crates/heimq-testkit/tests/memory_backend_conformance.rs
index 1e2da15..f398d8e 100644
--- a/crates/heimq-testkit/tests/memory_backend_conformance.rs
+++ b/crates/heimq-testkit/tests/memory_backend_conformance.rs
@@ -12,6 +12,7 @@ use std::sync::Arc;
 // ── LogBackend ────────────────────────────────────────────────────────────────
 
 #[test]
+// @covers US-016-AC1
 fn memory_log_backend_capabilities_name() {
     let cfg = test_config(true);
     let backend = MemoryLog::new(cfg);
@@ -19,6 +20,7 @@ fn memory_log_backend_capabilities_name() {
 }
 
 #[test]
+// @covers US-016-AC1
 fn memory_log_backend_create_and_find_topic() {
     let cfg = test_config(true);
     let backend = MemoryLog::new(cfg);
@@ -28,6 +30,7 @@ fn memory_log_backend_create_and_find_topic() {
 }
 
 #[test]
+// @covers US-016-AC1
 fn memory_log_backend_list_topics() {
     let cfg = test_config(true);
     let backend = MemoryLog::new(cfg);
@@ -35,6 +38,7 @@ fn memory_log_backend_list_topics() {
 }
 
 #[test]
+// @covers US-016-AC1
 fn memory_log_backend_append_and_fetch() {
     let cfg = test_config(true);
     let backend = MemoryLog::new(cfg);
@@ -42,6 +46,7 @@ fn memory_log_backend_append_and_fetch() {
 }
 
 #[test]
+// @covers US-016-AC1
 fn memory_log_backend_high_watermark() {
     let cfg = test_config(true);
     let backend = MemoryLog::new(cfg);
@@ -49,6 +54,7 @@ fn memory_log_backend_high_watermark() {
 }
 
 #[test]
+// @covers US-016-AC1
 fn memory_log_backend_log_start_offset_initial() {
     let cfg = test_config(true);
     let backend = MemoryLog::new(cfg);
@@ -56,6 +62,7 @@ fn memory_log_backend_log_start_offset_initial() {
 }
 
 #[test]
+// @covers US-016-AC1
 fn memory_log_backend_get_or_create_idempotent() {
     let cfg = test_config(true);
     let backend = MemoryLog::new(cfg);
@@ -63,6 +70,7 @@ fn memory_log_backend_get_or_create_idempotent() {
 }
 
 #[test]
+// @covers US-016-AC1
 fn memory_log_backend_delete_topic() {
     let cfg = test_config(true);
     let backend = MemoryLog::new(cfg);
@@ -72,6 +80,7 @@ fn memory_log_backend_delete_topic() {
 // ── PartitionLog ──────────────────────────────────────────────────────────────
 
 #[test]
+// @covers US-016-AC1
 fn memory_partition_log_suite() {
     suites::partition_log::run_all(&|| Arc::new(MemoryPartitionLog::new(0)) as Arc<_>);
 }
@@ -79,6 +88,7 @@ fn memory_partition_log_suite() {
 // ── OffsetStore ───────────────────────────────────────────────────────────────
 
 #[test]
+// @covers US-016-AC1
 fn memory_offset_store_suite() {
     let cfg = test_config(true);
     let manager = ConsumerGroupManager::new(cfg);
@@ -89,6 +99,7 @@ fn memory_offset_store_suite() {
 // ── ClusterView ───────────────────────────────────────────────────────────────
 
 #[test]
+// @covers US-016-AC1
 fn single_node_cluster_view_suite() {
     let cfg = test_config(true);
     let view = SingleNodeClusterView::new(&cfg);
@@ -98,6 +109,7 @@ fn single_node_cluster_view_suite() {
 // ── GroupCoordinatorBackend ───────────────────────────────────────────────────
 
 #[test]
+// @covers US-016-AC1
 fn memory_group_coordinator_suite() {
     let cfg = test_config(true);
     let manager = ConsumerGroupManager::new(cfg);
diff --git a/crates/heimq-testkit/tests/postgres_offset_store_conformance.rs b/crates/heimq-testkit/tests/postgres_offset_store_conformance.rs
index 64acaff..5edf256 100644
--- a/crates/heimq-testkit/tests/postgres_offset_store_conformance.rs
+++ b/crates/heimq-testkit/tests/postgres_offset_store_conformance.rs
@@ -46,6 +46,7 @@ async fn start_postgres() -> (ContainerAsync<GenericImage>, String) {
 }
 
 #[tokio::test]
+// @covers US-016-AC1
 async fn postgres_offset_store_capabilities() {
     if !pg_tests_enabled() {
         return;
@@ -60,6 +61,7 @@ async fn postgres_offset_store_capabilities() {
 }
 
 #[tokio::test]
+// @covers US-016-AC1
 async fn postgres_offset_store_fetch_missing() {
     if !pg_tests_enabled() {
         return;
@@ -74,6 +76,7 @@ async fn postgres_offset_store_fetch_missing() {
 }
 
 #[tokio::test]
+// @covers US-016-AC1
 async fn postgres_offset_store_commit_and_fetch() {
     if !pg_tests_enabled() {
         return;
@@ -88,6 +91,7 @@ async fn postgres_offset_store_commit_and_fetch() {
 }
 
 #[tokio::test]
+// @covers US-016-AC1
 async fn postgres_offset_store_commit_overwrites() {
     if !pg_tests_enabled() {
         return;
@@ -102,6 +106,7 @@ async fn postgres_offset_store_commit_overwrites() {
 }
 
 #[tokio::test]
+// @covers US-016-AC1
 async fn postgres_offset_store_fetch_all_for_group() {
     if !pg_tests_enabled() {
         return;
@@ -116,6 +121,7 @@ async fn postgres_offset_store_fetch_all_for_group() {
 }
 
 #[tokio::test]
+// @covers US-016-AC1
 async fn postgres_offset_store_delete_group() {
     if !pg_tests_enabled() {
         return;
diff --git a/crates/heimq/scripts/stress-matrix.sh b/crates/heimq/scripts/stress-matrix.sh
index 8a3f413..320f5a2 100755
--- a/crates/heimq/scripts/stress-matrix.sh
+++ b/crates/heimq/scripts/stress-matrix.sh
@@ -27,6 +27,7 @@ export PRODUCERS="${PRODUCERS:-1}"
 export CONSUMERS="${CONSUMERS:-1}"
 export GROUPS_PER_TOPIC="${GROUPS_PER_TOPIC:-1}"
 
+# @covers US-015-AC3 US-015-AC4
 # Matrix rows: "label|log|offsets|groups|requires_pg|features"
 ROWS=(
   "memory-only|memory://|memory://|memory://|0|"
diff --git a/crates/heimq/tests/postgres_offsets.rs b/crates/heimq/tests/postgres_offsets.rs
index 023380c..ab05559 100644
--- a/crates/heimq/tests/postgres_offsets.rs
+++ b/crates/heimq/tests/postgres_offsets.rs
@@ -151,6 +151,7 @@ fn unique_suffix() -> String {
 }
 
 #[test]
+// @covers US-015-AC1 US-015-AC2
 fn postgres_offset_store_survives_restart() {
     let base_url = match std::env::var("HEIMQ_PG_TEST_URL") {
         Ok(v) if !v.is_empty() => v,
</untrusted-data>
  </diff>

  <strictness-mode mode="strict">strict — each AC must be anchored to a named Test* function or a diff-touched symbol; file-only evidence is insufficient.</strictness-mode>

  <instructions>
You are reviewing a bead implementation against its acceptance criteria.

## AC-Check Ratification

When an &lt;ac-check&gt; section is present, ratify the mechanical results rather
than re-verifying them independently from the diff:

- result="pass": confirm the evidence is credible. Override to fail only if
  the evidence is fabricated — include judgment_override_reason and a diff
  citation (file:line) in the per_ac evidence string.
- result="fail": mechanically verified failure. Grade as fail and BLOCK unless
  the commit message contains an explicit AC-Waive trailer for this AC.
- result="needs_judgment": adjudicate from the diff. If you cannot determine
  pass/fail without additional bead context from the operator, use
  REQUEST_CLARIFICATION for that AC item.
- result="error": treat as needs_judgment.

Overriding a mechanical grade (pass→fail or fail→pass) requires an explicit
judgment_override_reason note and a concrete diff citation in the evidence.

## Strictness Mode

The &lt;strictness-mode&gt; tag specifies per-bead evidence requirements:

- strict (kind:fix, kind:feat): each AC must be anchored to a named Test*
  function or a diff-touched symbol; file-only evidence is insufficient.
- behavior-light (kind:refactor, kind:chore): build green plus file/symbol
  evidence suffices; test-name match required only when an AC explicitly
  names a Test* function.
- mechanical (kind:doc, kind:mechanical): file presence, renames, or symbol
  evidence only; no test-name or runtime evidence required.

## Verdicts

For each acceptance-criteria (AC) item, decide whether it is implemented
correctly, then assign one overall verdict:

- APPROVE — every AC item is fully and correctly implemented.
- REQUEST_CHANGES — some AC items are partial or have fixable minor issues.
- BLOCK — at least one AC item is not implemented or incorrectly implemented;
  or the diff is insufficient to evaluate.
- REQUEST_CLARIFICATION — you cannot adjudicate one or more needs_judgment AC
  items without operator clarification. Use this ONLY when the item is
  ambiguous even given the full diff. This verdict does NOT block the queue;
  it routes to the operator lane for input.

## Required output format (schema_version: 1)

Respond with EXACTLY one JSON object as your final response, fenced as a single ```json … ``` code block. Do not include any prose outside the fenced block. The JSON must match this schema:

```json
{
  "schema_version": 1,
  "verdict": "APPROVE",
  "summary": "≤300 char human-readable verdict justification",
  "per_ac": [
    { "number": 1, "item": "acceptance criterion text", "grade": "pass", "evidence": "file:line or test evidence" }
  ],
  "findings": [
    { "severity": "info", "summary": "what is wrong or notable", "location": "path/to/file.go:42" }
  ]
}
```

Rules:
- "verdict" must be exactly one of "APPROVE", "REQUEST_CHANGES", "BLOCK", "REQUEST_CLARIFICATION".
- "severity" must be exactly one of "info", "warn", "block".
- Output the JSON object inside ONE fenced ```json … ``` block. No additional prose, no extra fences, no markdown headings.
- Do not echo this template back. Do not write the verdict value anywhere except as the JSON value of the verdict field.
  </instructions>
</bead-review>
