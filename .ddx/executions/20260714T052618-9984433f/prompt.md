<bead-review>
  <bead id="heimq-86f66ee2" iter=1>
    <title>Cite wire, group, idempotence, and transaction acceptance coverage</title>
    <description>
Resolve the core portion of AR13-05. Audit US-001 through US-004 and US-013 criterion-by-criterion against contract, integration, handler, and parity tests. Add exact canonical @covers US-NNN-ACm comments only where the named test exercises that behavior; replace generic @covers US-004 comments with exact IDs. Add or strengthen tests for any criterion that lacks an exercising test. In scope: crates/heimq/tests/contract.rs, tests/conformance/integration/suite/, tests/conformance/go/workloads.go, tests/conformance/python/conformance/workloads/, and focused handler tests. Out of scope: benchmark and ecosystem stories.
    </description>
    <acceptance>
1. The HELIX alignment checker reports no uncited AC for US-001, US-002, US-003, US-004, or US-013. 2. cargo test -p heimq --test contract passes. 3. pytest tests/conformance/integration/suite --collect-only succeeds when Python dependencies are available, and every added citation names a test that asserts the cited behavior.
    </acceptance>
    <labels>helix, area:testing, kind:traceability, ac-quality:needs-refinement</labels>
  </bead>

  <changed-files>
    <file>crates/heimq/tests/contract.rs</file>
    <file>tests/conformance/go/workloads.go</file>
    <file>tests/conformance/integration/suite/test_consumer_groups.py</file>
    <file>tests/conformance/python/conformance/workloads/produce_fetch.py</file>
  </changed-files>

  <governing>
    <ref id="FEAT-002" path="crates/heimq/docs/helix/01-frame/features/FEAT-002-core-kafka-semantics.md" title="Feature Specification: FEAT-002 — Core Kafka Semantics (Groups, Transactions, Idempotency)">
      <content>
<untrusted-data>
---
ddx:
  id: FEAT-002
  depends_on:
    - helix.prd
    - FEAT-001
    - FEAT-006
  review:
    self_hash: 164350929c7bbc09a589f3cd1a80b685e88cce1054445fe5373aec566464636f
    deps:
      FEAT-001: 7133c264bc364ec4535c1d6b6187a90c9ba66d1fa3df30731ade260c2e092479
      FEAT-006: e59d3b8965ebd35b4bbe9c5302f4218432ad83ec27691989dfd4c345ac2ae004
      helix.prd: 96f0479e307f2c240e8f47b69fff510164d0b9eda132abb22cc4a860932984fe
    reviewed_at: "2026-07-14T05:12:26Z"
---
# Feature Specification: FEAT-002 — Core Kafka Semantics (Groups, Transactions, Idempotency)

**Feature ID**: FEAT-002
**Status**: Specified
**Priority**: P0
**Owner**: heimq core
**Covered PRD Subsystem(s)**: Core semantics (FEAT-002)
**Covered PRD Requirements**: FR-5 (consumer groups), FR-6 (idempotent producers), FR-7 (transactional groups / EOS) — PRD P0 #2, #3, #4
**Cross-Subsystem Rationale**: None — single subsystem.

## Overview

heimq correctly implements the Kafka semantic surface that production
services depend on: consumer groups, idempotent producers, and transactional
producers (EOS). This addresses PRD goal 2 (the semantic surface that
production services rely on works correctly).

## Ideal Future State

An idempotent or transactional Kafka client, and consumer groups of any
membership, run unchanged against heimq and observe the same semantics
they would against Kafka/Redpanda for single-coordinator deployments: a
group of N members sees no gaps and no duplicate ownership across
rebalances and resumes from committed offsets; retried produces are
de-duplicated; `read_committed` consumers observe only committed records.

## Problem Statement

- **Current situation**: Consumer groups are implemented (per `API-001`).
  Idempotent producers and transactions are explicitly excluded as `R3` in
  the current contract.
- **Pain points**: Services using `enable.idempotence=true` (often the
  rdkafka default) and services using transactional sinks (Flink, Kafka
  Streams, Debezium) cannot run against heimq today.
- **Desired outcome**: An idempotent or transactional Kafka client runs
  unchanged against heimq and observes the same semantics it would against
  Kafka/Redpanda for single-coordinator deployments.

## Functional Areas

| Area | User question or job | Feature responsibility |
|------|----------------------|------------------------|
| Consumer groups | Read a partitioned topic as a group with no gaps or duplicate ownership; resume from committed offsets | Group lifecycle (join / sync / heartbeat / leave / coordinator) and offset commit/fetch per Kafka spec for a single coordinator |
| Idempotent producers | Produce with retries without introducing duplicates | producerId/epoch issuance and `(producerId, epoch, partition)` sequence tracking with standard duplicate / out-of-order error semantics |
| Transactions (EOS) | Commit or abort writes atomically; consume only committed records under `read_committed` | Single-coordinator transaction state machine, transaction markers, transactional offset commits, isolation-level visibility |

## Requirements

### Functional Requirements by Area

#### Consumer groups

- **CG-01** — JoinGroup / SyncGroup / Heartbeat / LeaveGroup / FindCoordinator behave
  per Kafka spec for a single coordinator.
- **CG-02** — OffsetCommit / OffsetFetch persist (in-memory by default) and return
  committed offsets to subsequent group members.
- **CG-03** — A group of N members reading a partitioned topic sees no record gaps
  and no duplicate ownership across rebalance events (join, leave,
  session-timeout expiry).
- **CG-04** — Members resume from committed offsets after restart.

#### Idempotent producers

- **IP-01** — `InitProducerId` returns a producerId / epoch.
- **IP-02** — The broker tracks `(producerId, epoch, partition)` sequence numbers.
- **IP-03** — Duplicate sequence numbers are de-duplicated or return
  `DUPLICATE_SEQUENCE_NUMBER` per Kafka spec; out-of-order returns
  `OUT_OF_ORDER_SEQUENCE_NUMBER`.
- **IP-04** — A producer with `enable.idempotence=true` running unchanged against
  heimq sees the same observable behavior as against Redpanda for a
  produce-with-retry workload.

#### Transactions (EOS)

- **TX-01** — `InitProducerId` with a `transactional.id` returns a fenced producerId
  and bumps the epoch on re-init.
- **TX-02** — `AddPartitionsToTxn`, `AddOffsetsToTxn`, `EndTxn`, `WriteTxnMarkers`,
  `TxnOffsetCommit` operate on a single-coordinator transaction state
  machine consistent with Kafka.
- **TX-03** — `read_committed` consumers do not observe records from open or aborted
  transactions; `read_uncommitted` consumers do.
- **TX-04** — A transactional producer + read_committed consumer running unchanged
  against heimq sees the same observable behavior as against Redpanda for
  commit and abort flows.

### Non-Functional Requirements

- **Reliability**: All P0 acceptance scenarios in the PRD for groups,
  idempotency, and transactions pass against heimq and against Redpanda.
- **Determinism**: Differential parity reports zero behavioral diffs for
  these flows.
- **Compatibility**: Loss of in-memory state on restart is acceptable; the
  client re-initializes (re-issues `InitProducerId`, rejoins group) and
  recovers per Kafka spec for a fresh broker.

## User Stories

- [US-002 — Consumer group rebalance correctness](../user-stories/US-002-consumer-group-rebalance.md)
- [US-003 — Idempotent producer dedup](../user-stories/US-003-idempotent-producer.md)
- [US-004 — Transactional commit and abort](../user-stories/US-004-transactional-commit-abort.md)

## Edge Cases and Error Handling

- **Producer fenced**: a stale epoch using a transactional id returns
  `INVALID_PRODUCER_EPOCH`.
- **Transaction timeout**: `transaction.timeout.ms` enforced; expired
  transactions are aborted.
- **Sequence wrap**: handled per Kafka spec.
- **Group rebalance during transaction**: consumer commits via
  `TxnOffsetCommit` and observes the standard Kafka semantics.
- **Restart-during-transaction / restart-with-active-producer-id**:
  in-memory state loss is acceptable. heimq presents as a fresh broker;
  clients receive `UNKNOWN_PRODUCER_ID` and re-initialize via
  `InitProducerId` per Kafka spec. This is the same behavior real Kafka
  exhibits when its log is lost (fresh broker / disk wipe), so no
  heimq-specific client handling is needed — modern librdkafka and the
  Java client recover transparently.

## Success Metrics

- All FEAT-002 acceptance test sketches in the PRD pass.
- Differential parity (FEAT-003) reports zero diffs for groups,
  idempotency, and transactions in the gating workload.

## Constraints and Assumptions

- Single transaction coordinator (no replicated transaction log).
- Loss-on-restart is acceptable; fresh-broker semantics are observed by
  clients on reconnect.

## Dependencies

- **Other features**: FEAT-001 (wire protocol), FEAT-006 (flexible-version
  codec — modern transactional APIs are flexible-only:
  InitProducerId v2+, EndTxn v3+, AddPartitionsToTxn v3+,
  TxnOffsetCommit v3+), FEAT-003 (parity tests).
- **PRD requirements**: P0 #2, #3, #4.

## Out of Scope

- Multi-coordinator transaction log replication.
- Persistence of producer-id / transaction state across restart (acceptable
  per PRD non-goal #1).
</untrusted-data>
      </content>
    </ref>
  </governing>

  <diff rev="4a683f32625f3f517a753a61f96c368aab6e53f6">
<untrusted-data>
diff --git a/crates/heimq/tests/contract.rs b/crates/heimq/tests/contract.rs
index 6fc866a..12a6f6a 100644
--- a/crates/heimq/tests/contract.rs
+++ b/crates/heimq/tests/contract.rs
@@ -1175,7 +1175,7 @@ fn contract_init_producer_id_returns_valid_id() {
     );
 }
 
-// @covers US-003-AC4
+// @covers US-003-AC2 US-003-AC4
 #[test]
 fn contract_out_of_order_sequence_returns_error() {
     let server = TestServer::start();
@@ -1274,7 +1274,7 @@ fn contract_init_producer_id_with_transactional_id() {
     );
 }
 
-// @covers US-004-AC2 US-004-AC9
+// @covers US-004-AC2
 #[test]
 fn contract_add_partitions_to_txn_basic() {
     use heimq_protocol::messages::add_partitions_to_txn_request::{
@@ -1320,7 +1320,7 @@ fn contract_add_partitions_to_txn_basic() {
     );
 }
 
-// @covers US-004 EndTxn basic flow
+// @covers US-004-AC9
 #[test]
 fn contract_end_txn_basic() {
     use heimq_protocol::messages::end_txn_request::EndTxnRequest;
@@ -1351,7 +1351,7 @@ fn contract_end_txn_basic() {
     assert_eq!(end_resp.error_code, 0, "EndTxn commit should succeed");
 }
 
-// @covers US-004 AddOffsetsToTxn basic flow
+// @covers US-004-AC8
 #[test]
 fn contract_add_offsets_to_txn_basic() {
     use heimq_protocol::messages::add_offsets_to_txn_request::AddOffsetsToTxnRequest;
@@ -1383,7 +1383,7 @@ fn contract_add_offsets_to_txn_basic() {
     assert_eq!(resp.error_code, 0, "AddOffsetsToTxn should succeed");
 }
 
-// @covers US-004 TxnOffsetCommit basic flow
+// @covers US-004-AC11
 #[test]
 fn contract_txn_offset_commit_basic() {
     use heimq_protocol::messages::add_offsets_to_txn_request::AddOffsetsToTxnRequest;
@@ -1448,7 +1448,7 @@ fn contract_txn_offset_commit_basic() {
     );
 }
 
-// @covers US-004 WriteTxnMarkers basic flow
+// @covers US-004-AC10
 #[test]
 fn contract_write_txn_markers_basic() {
     use heimq_protocol::messages::init_producer_id_request::InitProducerIdRequest;
@@ -1487,7 +1487,7 @@ fn contract_write_txn_markers_basic() {
     );
 }
 
-// @covers US-004-AC6
+// @covers US-004-AC3
 #[test]
 fn contract_transactional_produce_commit_fetch() {
     use heimq_protocol::messages::add_partitions_to_txn_request::{
@@ -1626,7 +1626,7 @@ fn contract_transactional_produce_commit_fetch() {
     assert_eq!(records[0].value, Some(bytes::Bytes::from("txn-val")));
 }
 
-// @covers US-003-AC3 (duplicate retry is de-duplicated)
+// @covers US-003-AC2 US-003-AC3
 #[test]
 fn contract_duplicate_sequence_is_deduplicated() {
     use heimq_protocol::messages::init_producer_id_request::InitProducerIdRequest;
@@ -1682,7 +1682,7 @@ fn contract_duplicate_sequence_is_deduplicated() {
     );
 }
 
-// @covers US-004 (stale producer epoch fencing)
+// @covers US-004-AC7
 #[test]
 fn contract_stale_epoch_returns_invalid_producer_epoch() {
     use heimq_protocol::messages::add_partitions_to_txn_request::{
@@ -1735,7 +1735,7 @@ fn contract_stale_epoch_returns_invalid_producer_epoch() {
     );
 }
 
-// @covers US-004 (aborted transaction invisible to read_committed)
+// @covers US-004-AC4
 #[test]
 fn contract_aborted_transaction_invisible_to_read_committed() {
     use heimq_protocol::messages::add_partitions_to_txn_request::{
@@ -1868,7 +1868,7 @@ fn contract_aborted_transaction_invisible_to_read_committed() {
     );
 }
 
-// @covers US-004
+// @covers US-004-AC5
 #[test]
 fn contract_read_uncommitted_observes_aborted_records() {
     use heimq_protocol::messages::add_partitions_to_txn_request::{
diff --git a/tests/conformance/go/workloads.go b/tests/conformance/go/workloads.go
index ca59b61..a9be0c8 100644
--- a/tests/conformance/go/workloads.go
+++ b/tests/conformance/go/workloads.go
@@ -121,7 +121,7 @@ func produceN(ctx context.Context, bootstrap, topic, keyPrefix, valPrefix string
 }
 
 // ── produce_fetch_roundtrip ───────────────────────────────────────────────────
-// @covers US-001-AC4 US-005-AC4 US-005-AC5
+// @covers US-001-AC4 US-005-AC4 US-005-AC5 US-013-AC4
 
 func produceFetch(ctx context.Context, bootstrap string) ([]Observation, error) {
 	const topic, n = "parity-produce-fetch", 10
diff --git a/tests/conformance/integration/suite/test_consumer_groups.py b/tests/conformance/integration/suite/test_consumer_groups.py
index 44c91ef..99f7dba 100644
--- a/tests/conformance/integration/suite/test_consumer_groups.py
+++ b/tests/conformance/integration/suite/test_consumer_groups.py
@@ -88,6 +88,7 @@ def test_rdkafka_consumer_group_manual_offset_fetch(bootstrap, topic, group):
         c.close()
 
 
+# @covers US-002-AC1
 def test_rdkafka_multiple_consumers_same_group(bootstrap, topic, group):
     p = producer(bootstrap)
     produce_n(p, topic, 20, "multi-consumer-{}", "key-{}")
@@ -110,7 +111,7 @@ def test_rdkafka_multiple_consumers_same_group(bootstrap, topic, group):
         c2.close()
 
 
-# @covers US-002-AC1
+# @covers US-002-AC1 US-002-AC4
 def test_rdkafka_consumer_group_lifecycle(bootstrap, topic, group):
     p = producer(bootstrap)
     produce_n(p, topic, 5, "lifecycle-batch1-{}", "key")
@@ -261,6 +262,7 @@ def test_rdkafka_seek_to_offset(bootstrap, topic, group):
         c.close()
 
 
+# @covers US-002-AC1
 def test_rdkafka_group_multi_partition_delivery(bootstrap_3p, topic, group):
     """300 keyed messages across 3 partitions must all be delivered exactly once."""
     p = producer(bootstrap_3p)
@@ -284,6 +286,7 @@ def test_rdkafka_group_multi_partition_delivery(bootstrap_3p, topic, group):
         c.close()
 
 
+# @covers US-002-AC2
 def test_rdkafka_group_rebalance_on_graceful_leave(bootstrap_3p, topic, group):
     """After consumer 2 leaves gracefully, consumer 1 must own all 3 partitions."""
     p = producer(bootstrap_3p)
@@ -313,6 +316,7 @@ def test_rdkafka_group_rebalance_on_graceful_leave(bootstrap_3p, topic, group):
         c1.close()
 
 
+# @covers US-002-AC2
 def test_rdkafka_group_rebalance_on_session_timeout(bootstrap_3p, topic, group):
     """When consumer 2 stops polling, its session expires and consumer 1 takes over."""
     p = producer(bootstrap_3p)
diff --git a/tests/conformance/python/conformance/workloads/produce_fetch.py b/tests/conformance/python/conformance/workloads/produce_fetch.py
index 7c458ba..e3612c4 100644
--- a/tests/conformance/python/conformance/workloads/produce_fetch.py
+++ b/tests/conformance/python/conformance/workloads/produce_fetch.py
@@ -19,7 +19,7 @@ NAME = "produce_fetch_roundtrip"
 TOPIC = "parity-produce-fetch"
 N = 10
 
-# @covers US-001-AC4 US-005-AC4 US-005-AC5
+# @covers US-001-AC4 US-005-AC4 US-005-AC5 US-013-AC4
 
 
 def run(target: Target) -> list[Observation]:
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
