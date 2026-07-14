---
ddx:
  id: TRAIT-001
  type: contract
  activity: design
  status: draft
  version: 0.1.0
  depends_on:
    - helix.prd
    - ADR-006
    - ADR-007
  review:
    self_hash: 0f7508ff4e3da65c8f84f465ea2b2c7db82797016f5333240148cdcaa7e7cc23
    deps:
      ADR-006: 881bf5e99cfef0f38fec536b48898ee1d3bf40b1be8bd2fbfc036f48aabfc385
      ADR-007: e299f6b474affd654a4a833dfc4a85eb8a2fccf52674517a1e207ec426177541
      helix.prd: debc0a32007f0c42db51e82f47848c7b988c3f67f6f97171069170492f9b5b95
    reviewed_at: "2026-07-14T06:48:37Z"
---

# TRAIT-001: Backend Trait Families

**Contract ID**: TRAIT-001  **Type**: Library (Rust trait surface)
**Version**: 0.1.0  **Status**: Draft  **Related**: ADR-006, ADR-007, API-001

---

## Purpose

Defines the four trait families that form the engine/backend seam in
`heimq-broker`: Log (`LogBackend`/`TopicLog`/`PartitionLog`), `OffsetStore`,
`GroupCoordinatorBackend`, and `ClusterView` (NEW). Any crate embedding the
engine implements one or more families; the engine enforces no durability or
tenancy policy — those are impl properties declared via capabilities.

---

## Scope and Boundaries

In scope: normative method signatures, capability structs, durability-boundary
semantics (ADR-006), complete-work-before-ack requirement, per-request
principal context threading, per-family conformance suite obligations.

Out of scope: internal backend implementations; wire framing (WIRE-001,
CODEC-001); SASL/TLS; fjord/pqueue/niflheim product semantics.

Owning system: `heimq-broker` crate (`crates/heimq-broker/src/storage/`, `crates/heimq-broker/src/consumer_group/`).

---

## Normative Surface

### Cross-Cutting Rules

**Principal context** (IMPLEMENTED; bead heimq-43770932):
Every trait method serving a client request MUST accept a `RequestContext`
carrying principal, tenant, and client-id fields. The engine passes context but
enforces none of its contents — tenancy, quota, and RBAC are the impl's
responsibility. `heimq-broker::RequestContext` is constructible by embedders;
context-aware backend methods (`*_with_context`) are threaded through request
handlers while legacy methods remain as compatibility shims. SASL/TLS/auth
themselves remain embedder responsibilities (out of scope below) — this rule
covers only the context threading.

**Capability-gated advertisement**: A backend family absent or returning
`name == "unknown"` causes `compute_supported_apis`
(`mod.rs` under `crates/heimq/src/protocol/`) to filter all APIs gated on that family from the
ApiVersions response. Gating is per-API — a missing family does not remove
unrelated APIs.

**FEAT-002 gating**: `BackendCapabilities::idempotent_producer` and
`::transactions` (both `false` in `minimal()`) are independently gateable and
MUST remain `false` at IP-001 Gate A.

**Durability boundary** (ADR-006): An in-memory impl presents as a fresh broker
on restart. Clients receive `UNKNOWN_PRODUCER_ID` and re-initialize via
`InitProducerId`. Durable impls declare what survives via `survives_restart`.

---

### Family 1 — Log (`LogBackend` / `TopicLog` / `PartitionLog`)

Provenance: `crates/heimq-broker/src/storage/mod.rs:64–148`

Existing baseline signatures (verbatim):

```rust
// crates/heimq-broker/src/storage/mod.rs:64
pub trait LogBackend: Send + Sync {
    fn create_topic(&self, name: &str, num_partitions: i32) -> Result<Arc<dyn TopicLog>>;
    fn delete_topic(&self, name: &str) -> Result<()>;
    fn list_topics(&self) -> Vec<String>;
    fn topic(&self, name: &str) -> Option<Arc<dyn TopicLog>>;
    fn capabilities(&self) -> &BackendCapabilities;
    fn get_or_create_topic(&self, name: &str, num_partitions: i32) -> Arc<dyn TopicLog>;
    fn get_all_topic_metadata(&self) -> Vec<(String, i32)>;
    // Narrow accessors replacing fn config(&self) -> &Config (removed in seam heimq-5c4ad46f)
    fn default_num_partitions(&self) -> i32;
    fn auto_create_topics(&self) -> bool;
    fn append(&self, topic_name: &str, partition: i32, records: &[u8]) -> Result<(i64, i64)>;
    fn append_with_context(&self, ctx: &RequestContext, topic_name: &str, partition: i32, records: &[u8]) -> Result<(i64, i64)>;
    fn append_async_with_context_and_retention_policy<'a>(
        &'a self,
        ctx: &'a RequestContext,
        topic_name: &'a str,
        partition: i32,
        records: &'a [u8],
        retention: Option<RetentionPolicy>,
    ) -> AppendFuture<'a>;
    fn fetch(&self, topic_name: &str, partition: i32, offset: i64, max_bytes: i32) -> Result<(Vec<u8>, i64)>;
    fn fetch_with_context(&self, ctx: &RequestContext, topic_name: &str, partition: i32, offset: i64, max_bytes: i32) -> Result<(Vec<u8>, i64)>;
    fn high_watermark(&self, topic_name: &str, partition: i32) -> Result<i64>;
    fn log_start_offset(&self, topic_name: &str, partition: i32) -> Result<i64>;
}

// crates/heimq-broker/src/storage/mod.rs:112
pub trait TopicLog: Send + Sync {
    fn name(&self) -> &str;
    fn num_partitions(&self) -> i32;
    fn partition(&self, id: i32) -> Result<Arc<dyn PartitionLog>>;
    fn config(&self) -> &TopicConfig;
}

// crates/heimq-broker/src/storage/mod.rs:121
pub trait PartitionLog: Send + Sync {
    fn id(&self) -> i32;
    fn append(&self, view: &RecordBatchView<'_>, raw_bytes: Option<&[u8]>) -> Result<(i64, i64)>;
    fn read(&self, offset: i64, max_bytes: usize, wait: FetchWait) -> Result<(Vec<u8>, i64)>;
    fn log_start_offset(&self) -> i64;
    fn high_watermark(&self) -> i64;
    fn truncate_before(&self, offset: i64) -> Result<()>;
}
```

Normative rules:
- `create_topic` MUST be idempotent for identical arguments.
- Read-side methods (`fetch`, `read`, `high_watermark`, `log_start_offset`) are
  OPTIONAL for produce-only impls; `fetch_wait: false` MUST be declared in
  capabilities and Fetch (api_key 1) MUST NOT be advertised.
- **NEW — complete-work-before-ack**: `PartitionLog::append` MUST NOT resolve
  until the impl's durability requirement AND all pre-ack product work are done.
  An impl MAY perform product work (example: niflheim enqueues materialization)
  before returning. The broker engine MUST NOT send a Produce response before
  `append` returns.
- **Async pre-ack work — RESOLVED** (bead heimq-77965f97): `LogBackend` and
  `PartitionLog` now expose `append_async` returning an `AppendFuture`
  (`crates/heimq-broker/src/storage/mod.rs`), documented to "return a future that
  resolves when the append's offset assignment is complete." Synchronous backends
  inherit the default (`ready(self.append(...))`); deferred backends (niflheim,
  awaiting a WAL write + materialization enqueue before the produce ack) override
  it so complete-work-before-ack is expressible for *genuinely async* pre-ack
  work. `produce::append_records_async` carries the future through the produce
  path. This gap is closed.
- **Record-decode exposure — RESOLVED** (bead heimq-0123b72d): an impl reads produced
  records via `RecordBatchView`/`RecordView`
  (`crates/heimq-broker/src/storage/record_batch_view.rs`). `RecordView::headers()`
  exposes `(&str, Option<&[u8]>)`, so a consumer requiring zero direct
  `kafka-protocol` dependency (niflheim) can iterate records — key, value,
  headers, timestamp — over engine/std/bytes types only.
- `append` returns `(base_offset, log_end_offset)`; both MUST be monotonically
  increasing within a partition.

---

### Family 2 — `OffsetStore`

Provenance: `crates/heimq-broker/src/storage/offset_store.rs:58–84`

```rust
// crates/heimq-broker/src/storage/offset_store.rs:58
pub trait OffsetStore: Send + Sync {
    // Returns Result<()> since seam heimq-ec30673c: completion semantics require
    // the impl to complete all durability work before returning and signal failure.
    fn commit(&self, group_id: &str, topic: &str, partition: i32,
              offset: i64, leader_epoch: i32, metadata: Option<String>) -> Result<()>;
    fn fetch(&self, group_id: &str, topic: &str, partition: i32) -> Option<CommittedOffset>;
    fn fetch_all_for_group(&self, group_id: &str) -> HashMap<(String, i32), CommittedOffset>;
    fn delete_group(&self, group_id: &str);
    fn capabilities(&self) -> &OffsetStoreCapabilities;
}
```

Normative rules:
- Durability is an impl property declared in `OffsetStoreCapabilities`
  (`crates/heimq-broker/src/storage/offset_store.rs:28`): `durability: Durability`,
  `survives_restart: bool`.
- After `commit` returns, a subsequent `fetch` for the same key MUST return the
  committed value (read-your-writes within a process).
- On restart, `survives_restart: false` impl MUST return `None` for all
  `fetch` calls. `survives_restart: true` impl MUST reconstruct state before
  accepting requests.
- `delete_group` MUST remove all partition entries for the group atomically.

---

### Family 3 — `GroupCoordinatorBackend`

Provenance: `crates/heimq-broker/src/consumer_group/backend.rs:123–142`

```rust
// crates/heimq-broker/src/consumer_group/backend.rs:123
pub trait GroupCoordinatorBackend: Send + Sync {
    fn join_group(&self, req: JoinRequest) -> JoinResult;
    fn sync_group(&self, req: SyncRequest) -> SyncResult;
    fn heartbeat(&self, group_id: &str, generation_id: i32, member_id: &str) -> HeartbeatResult;
    fn leave_group(&self, group_id: &str, member_ids: &[String]) -> LeaveResult;
    fn capabilities(&self) -> &GroupCoordinatorCapabilities;
}
```

Normative rules:
- At most one `GroupCoordinatorBackend` is active per broker process
  (single-coordinator semantics).
- Protocol errors are returned as Kafka error codes in result DTOs, not as
  Rust `Err` — so handlers map them to wire responses without information loss.
- `join_group` with empty `req.member_id` MUST return `MEMBER_ID_REQUIRED`
  (79) with a freshly minted id; MUST NOT add the member.
- Stale `generation_id` in heartbeat or sync_group MUST return
  `ILLEGAL_GENERATION` (22).
- Durable group state (`survives_restart: true`) is an impl property.

**Manager-above-trait contract (ADR — join/sync hold semantics)**: The trait
methods are sync and return immediately. Rebalance hold-until-all-joined
behaviour (blocking `sync_group` until all members have called it) is
implemented by the coordinator manager layer above the trait
(`ConsumerGroupManager`), not by the trait implementation. A durable or
multi-node backend implementing this trait MUST NOT assume the caller holds
a long-lived connection open — the manager owns the retry loop and will
call `sync_group` again for each arriving member. This design keeps the
trait surface simple and testable without async complexity. If a future
backend requires genuine async state holds, a `GroupCoordinatorBackendV2`
trait with `async fn` methods should be introduced rather than complicating
the current synchronous surface.

Key DTOs are normative: `JoinRequest`, `JoinResult`, `JoinMember`,
`SyncRequest`, `SyncResult`, `HeartbeatResult`, `LeaveResult`
(verbatim `crates/heimq-broker/src/consumer_group/backend.rs:53–116`). Field semantics match the
Kafka protocol.

`GroupCoordinatorCapabilities` (`crates/heimq-broker/src/consumer_group/backend.rs:19`):
`name`, `version`, `durability`, `survives_restart`, `multi_node: bool`.

---

### Family 4 — `ClusterView` (NEW)

No source exists today. Proposed interface — **proposed, finalized in Slice 1**:

```rust
// proposed — finalized in Slice 1
#[derive(Debug, Clone)]
pub struct BrokerInfo { pub node_id: i32, pub host: String, pub port: u16 }

#[derive(Debug, Clone)]
pub struct PartitionLeader {
    pub topic: String, pub partition: i32,
    pub leader_id: i32, pub leader_epoch: i32,
}

#[derive(Debug)]
pub enum ClusterViewError {
    NotLeaderOrFollower { topic: String, partition: i32 },
    NotCoordinator { group_id: String },
}

pub trait ClusterView: Send + Sync {
    /// This broker's own identity.
    fn self_broker(&self) -> BrokerInfo;
    /// All known brokers (includes self).
    fn brokers(&self) -> Vec<BrokerInfo>;
    /// Leader for (topic, partition); Err on stale/unknown.
    fn partition_leader(&self, topic: &str, partition: i32)
        -> Result<BrokerInfo, ClusterViewError>;
    /// Coordinator for group_id; Err when not this node.
    fn find_coordinator(&self, group_id: &str)
        -> Result<BrokerInfo, ClusterViewError>;
}
```

Normative rules:
- The single-node reference impl MUST return `self_broker()` for both
  `partition_leader` and `find_coordinator` for all inputs. This normativizes
  current `Config`-derived behavior in Metadata and FindCoordinator handlers.
- `NOT_LEADER_OR_FOLLOWER` (error 6) MUST be returned in Produce/Fetch
  responses when `partition_leader` errors.
- `NOT_COORDINATOR` (error 16) MUST be returned in group API responses when
  `find_coordinator` errors.
- Multi-node impls own leader emulation; this contract fixes only the interface
  and error semantics.

---

## Precedence and Compatibility

**Evolution policy**: Additive-only between major versions. Permitted: new
default-implemented methods, new capability fields defaulting to `false`/`None`,
new non-exhaustive enum variants. Breaking changes require a recorded
consumer-matrix-green migration before tagging.

---

## Error Semantics

| Condition | Error | Retry | Recovery |
|-----------|-------|-------|----------|
| Backend absent (name == "unknown") | Family APIs absent from ApiVersions | N/A | Supply named impl |
| `PartitionLog::append` fails | `Err`; broker returns `KAFKA_STORAGE_ERROR` | Client decides | Impl-specific |
| Restart, in-memory impl | `UNKNOWN_PRODUCER_ID` on idempotent/txn produce; offsets empty | Re-initialize | ADR-006: Kafka spec |
| Stale generation | `ILLEGAL_GENERATION` (22) | No; client rejoins | JoinGroup |
| Leader moved | `NOT_LEADER_OR_FOLLOWER` (6) | Yes, after metadata refresh | Metadata |
| Coordinator moved | `NOT_COORDINATOR` (16) | Yes, after FindCoordinator | FindCoordinator |
| Empty member_id | `MEMBER_ID_REQUIRED` (79) + fresh id | Yes, with minted id | Re-issue JoinGroup |

---

## Conformance Suite Obligations

**An implementation is supported only if its per-family suite passes**, including
the three consumer-shaped fixture backends (IP-001 Slice 3).

| Family | Suite | Required executors |
|--------|-------|--------------------|
| Log | `log_append_read_properties` | object-store-shape, WAL-deferred-ack-shape, queue-enqueue-shape |
| OffsetStore | `offset_commit_fetch_restart_matrix` | in-memory + Postgres |
| GroupCoordinatorBackend | `rebalance_trace_conformance` | in-memory reference |
| ClusterView | `cluster_view_routing_conformance` | single-node reference |

Fixture backends (IP-001 Slice 3): (a) object-store-shape `TopicLog`
(manifest-ordered append); (b) WAL-deferred-ack-shape `TopicLog` with deferred
acks + pre-ack work hook; (c) queue-enqueue produce sink (read-side absent,
capability-declared accordingly). Known limit: in-memory conformance cannot
prove durability-boundary semantics.

---

## Examples

Single-node `ClusterView` returns self for all queries (**proposed**):
```rust
fn partition_leader(&self, _t: &str, _p: i32) -> Result<BrokerInfo, ClusterViewError> {
    Ok(self.self_broker())
}
fn find_coordinator(&self, _g: &str) -> Result<BrokerInfo, ClusterViewError> {
    Ok(self.self_broker())
}
```

---

## Validation Checklist

- [x] Normative fields and rules are explicit.
- [x] Compatibility and precedence rules are explicit.
- [x] Error handling is explicit.
- [x] Executable tests derivable: per-family conformance suites; `compute_supported_apis` tests in `mod.rs` under `crates/heimq/src/protocol/`.
- [x] Non-normative notes cannot be mistaken for contract requirements.
- [ ] `ClusterView` trait and DTOs finalized in Slice 1 (currently proposed).
- [x] `RequestContext` principal/tenant/client-id threading implemented in backend traits (bead heimq-43770932).
- [x] `append` async pre-ack seam for genuinely-async product work (bead heimq-77965f97; `append_async`/`AppendFuture` landed on `LogBackend`+`PartitionLog`).
- [x] Record-view iteration free of re-exported `kafka-protocol` types (bead heimq-0123b72d).
- [ ] Consumer-shaped fixture backends compiled and passing (IP-001 Slice 3).
