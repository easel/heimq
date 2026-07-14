<bead-review>
  <bead id="heimq-1cdbc19d" iter=1>
    <title>Cite parity-harness acceptance coverage</title>
    <description>
Resolve the FEAT-003 portion of AR13-05. Map US-005-AC1 through AC7 to the Python and Go differential runners, normalization, exemption validation, CI entrypoint, and activated idempotent/transaction workloads. Add exact @covers comments at executable functions or tests. Add focused unit tests only where an AC is not exercised. In scope: tests/conformance/python/conformance/, tests/conformance/go/, tests/conformance/run.sh, tests/conformance/exemptions.toml. Out of scope: broker implementation.
    </description>
    <acceptance>
1. The HELIX alignment checker reports no uncited AC for US-005. 2. python3 -m compileall -q tests/conformance/python/conformance passes. 3. cd tests/conformance/go &amp;&amp; go test ./... passes. 4. Exemption validation still rejects empty PRD references and duplicate IDs.
    </acceptance>
    <labels>helix, area:testing, kind:traceability</labels>
  </bead>

  <changed-files>
    <file>tests/conformance/go/main.go</file>
    <file>tests/conformance/go/observation.go</file>
    <file>tests/conformance/go/observation_test.go</file>
    <file>tests/conformance/go/workloads.go</file>
    <file>tests/conformance/python/conformance/diff.py</file>
    <file>tests/conformance/python/conformance/exemptions.py</file>
    <file>tests/conformance/python/conformance/main.py</file>
    <file>tests/conformance/python/conformance/normalize.py</file>
    <file>tests/conformance/python/conformance/workloads/__init__.py</file>
    <file>tests/conformance/python/conformance/workloads/idempotent_produce.py</file>
    <file>tests/conformance/python/conformance/workloads/transactional_produce.py</file>
    <file>tests/conformance/run.sh</file>
  </changed-files>

  <governing>
    <ref id="FEAT-003" path="crates/heimq/docs/helix/01-frame/features/FEAT-003-differential-parity-testing.md" title="Feature Specification: FEAT-003 — Differential Parity Testing vs Redpanda/Kafka">
      <content>
<untrusted-data>
---
ddx:
  id: FEAT-003
  depends_on:
    - helix.prd
    - FEAT-001
    - FEAT-002
  review:
    self_hash: d134a5740222f1b59ecda81b318c7b28de9968bba9f640bb6280310c1e906233
    deps:
      FEAT-001: 7133c264bc364ec4535c1d6b6187a90c9ba66d1fa3df30731ade260c2e092479
      FEAT-002: 164350929c7bbc09a589f3cd1a80b685e88cce1054445fe5373aec566464636f
      helix.prd: 96f0479e307f2c240e8f47b69fff510164d0b9eda132abb22cc4a860932984fe
    reviewed_at: "2026-07-14T05:12:26Z"
---
# Feature Specification: FEAT-003 — Differential Parity Testing vs Redpanda/Kafka

**Feature ID**: FEAT-003
**Status**: Specified
**Priority**: P0
**Owner**: heimq core
**Covered PRD Subsystem(s)**: Differential parity (FEAT-003)
**Covered PRD Requirements**: FR-8 (differential parity testing) — PRD P0 #5
**Cross-Subsystem Rationale**: None — single subsystem.

## Overview

A test harness drives the same client-level workload against heimq and
against a real Redpanda (or Kafka) instance and asserts that observable
behavior is equivalent for in-scope APIs. This is how heimq's PRD claim
of "behaves identically to Kafka/Redpanda" is verified rather than
asserted.

## Ideal Future State

Every protocol-touching change is gated by a reproducible diff harness:
the same client-level workload runs against heimq and a Redpanda
container, observable outputs are normalized and diffed, zero behavioral
diffs is the passing condition, and CI fails on regressions — so parity
with Redpanda is observable rather than asserted.

## Problem Statement

- **Current situation**: `scripts/compatibility-test.sh` runs against
  Redpanda; per-test parity is ad-hoc and not enforced as a diff. Test
  plan Phase 4 ("Baseline Parity") is in flight but incomplete.
- **Pain points**: Divergences from Redpanda may pass contract tests yet
  fail in production tools (Kafka Connect, Debezium, etc.) because the
  contract test only asserts what we thought to assert.
- **Desired outcome**: A reproducible diff harness reports zero
  behavioral diffs between heimq and Redpanda for in-scope APIs at a
  gating workload, and CI fails on regressions.

## Requirements

### Functional Requirements

- **FR-01** — A harness runs the same client workload (driver script + identical
  client config) against heimq and against a Redpanda container.
- **FR-02** — The harness records observable client outputs: records consumed
  (key, value, headers, partition, offset), error codes returned, group
  state observed via client API, transactional outcomes (committed vs
  aborted record visibility under each isolation level).
- **FR-03** — The harness normalizes non-determinism (broker ids, host-specific
  timestamps, monotonic ids) before diffing.
- **FR-04** — The harness reports zero diffs as success and prints a structured
  diff on failure (target field, heimq value, redpanda value, request
  correlation_id).
- **FR-05** — Workloads cover produce/fetch and consumer groups as gating workloads
  at FEAT-003 acceptance; idempotent producers and transactions are added
  once FEAT-002 is accepted (see US-005 acceptance criteria and
  solution-designs/SD-003-differential-parity-testing.md § Expansion path).
- **FR-06** — The harness is runnable locally (developer machine with Docker) and in
  CI.

### Non-Functional Requirements

- **Determinism**: Diff harness must be deterministic — runs of the same
  workload produce the same diff (after normalization).
- **Performance**: Harness completes its gating workload in under 10
  minutes on CI hardware.
- **Reliability**: Less than 1% flake rate on the gating workload.

## User Stories

- [US-005 — Run the same workload against heimq and Redpanda and diff](../user-stories/US-005-differential-parity-harness.md)

## Edge Cases and Error Handling

- **Redpanda container fails to start**: harness exits with a clear,
  non-flake error; CI marks the run as `error` not `fail`.
- **Workload non-determinism slips through**: harness logs the
  normalization rules used so divergences can be triaged.
- **Intentional diffs (e.g., loss-on-restart)**: encoded as known
  exemptions in the harness with a reference to the PRD non-goal.

## Success Metrics

- Zero unmatched diffs at the gating workload across in-scope APIs.
- Differential harness gates CI for protocol-touching changes.

## Constraints and Assumptions

- Redpanda is an acceptable proxy for Kafka behavior for the in-scope APIs.
- Docker-in-Docker (or privileged docker) is available in CI.

## Dependencies

- **Other features**: FEAT-001 and FEAT-002 (defines the in-scope APIs).
- **External services**: Redpanda container; optionally Kafka container.
- **PRD requirements**: P0 #5.

## Out of Scope

- Performance / throughput diffs (handled by FEAT-004).
- Diffs for out-of-scope APIs (security, share groups, admin
  reassignment, etc.).
</untrusted-data>
      </content>
    </ref>
  </governing>

  <diff rev="7c578c3fc09de97706ae23256db4e7807c030fe9">
<untrusted-data>
diff --git a/tests/conformance/go/main.go b/tests/conformance/go/main.go
index 1579d38..bf8b9bf 100644
--- a/tests/conformance/go/main.go
+++ b/tests/conformance/go/main.go
@@ -51,6 +51,8 @@ func mustEnv(k string) string {
 	return v
 }
 
+// @covers US-005-AC1
+// @covers US-005-AC3
 func run() error {
 	ctx := context.Background()
 
diff --git a/tests/conformance/go/observation.go b/tests/conformance/go/observation.go
index 96df8bd..b898f91 100644
--- a/tests/conformance/go/observation.go
+++ b/tests/conformance/go/observation.go
@@ -16,6 +16,7 @@ type Observation struct {
 	Event    map[string]any
 }
 
+// @covers US-005-AC2
 func recordConsumed(key, value []byte, partition int32, offset int64) map[string]any {
 	return map[string]any{
 		"type":      "RecordConsumed",
@@ -55,6 +56,7 @@ type DiffRecord struct {
 
 // comparedFields lists, per event type, the (event key, diff field name) pairs.
 // Order matches the Python runner.
+// @covers US-005-AC1
 var comparedFields = map[string][][2]string{
 	"RecordConsumed": {
 		{"key", "record.key"},
@@ -98,6 +100,7 @@ type Exemptions struct{ Entries []Exemption }
 
 // find matches on field, active status, scope ("all" or the workload), and
 // oracle ("all" or the reference broker).
+// @covers US-005-AC6
 func (e *Exemptions) find(field, workload, oracle string) *string {
 	for i := range e.Entries {
 		x := &e.Entries[i]
@@ -110,6 +113,7 @@ func (e *Exemptions) find(field, workload, oracle string) *string {
 	return nil
 }
 
+// @covers US-005-AC6
 func loadExemptions(path string) (*Exemptions, error) {
 	var doc struct {
 		Exemption []Exemption `toml:"exemption"`
@@ -137,6 +141,9 @@ func loadExemptions(path string) (*Exemptions, error) {
 }
 
 // diff compares two normalized observation streams: heimq against one oracle.
+// @covers US-005-AC1
+// @covers US-005-AC3
+// @covers US-005-AC6
 func diff(workload, oracle string, heimq, oracleObs []Observation, ex *Exemptions) []DiffRecord {
 	var out []DiffRecord
 	shared := min(len(heimq), len(oracleObs))
diff --git a/tests/conformance/go/observation_test.go b/tests/conformance/go/observation_test.go
new file mode 100644
index 0000000..54556de
--- /dev/null
+++ b/tests/conformance/go/observation_test.go
@@ -0,0 +1,97 @@
+package main
+
+import (
+	"os"
+	"path/filepath"
+	"testing"
+)
+
+// @covers US-005-AC1
+// @covers US-005-AC3
+func TestDiffReportsStructuredMismatchAndZeroDiffSuccess(t *testing.T) {
+	ex := &Exemptions{}
+	heimq := []Observation{{
+		Workload: "produce_fetch_roundtrip",
+		Step:     0,
+		Event:    recordConsumed([]byte("k"), []byte("heimq"), 0, 0),
+	}}
+	oracle := []Observation{{
+		Workload: "produce_fetch_roundtrip",
+		Step:     0,
+		Event:    recordConsumed([]byte("k"), []byte("oracle"), 0, 0),
+	}}
+
+	diffs := diff("produce_fetch_roundtrip", "redpanda", heimq, oracle, ex)
+	if len(diffs) != 1 {
+		t.Fatalf("expected one structured diff, got %d", len(diffs))
+	}
+	if diffs[0].Workload != "produce_fetch_roundtrip" ||
+		diffs[0].Oracle != "redpanda" ||
+		diffs[0].Step != 0 ||
+		diffs[0].Field != "record.value" ||
+		diffs[0].HeimqValue != "heimq" ||
+		diffs[0].OracleValue != "oracle" ||
+		diffs[0].Divergence != "value_mismatch" {
+		t.Fatalf("unexpected diff record: %#v", diffs[0])
+	}
+
+	diffs = diff("produce_fetch_roundtrip", "redpanda", heimq, heimq, ex)
+	if len(diffs) != 0 {
+		t.Fatalf("expected zero diffs for matching observations, got %#v", diffs)
+	}
+}
+
+// @covers US-005-AC6
+func TestLoadExemptionsRejectsDuplicateIDs(t *testing.T) {
+	path := writeTOML(t, `
+[[exemption]]
+id = "dup"
+field = "record.offset"
+scope = "all"
+oracle = "all"
+reason = "first"
+prd_ref = "PRD non-goal #1"
+status = "active"
+
+[[exemption]]
+id = "dup"
+field = "record.value"
+scope = "all"
+oracle = "all"
+reason = "second"
+prd_ref = "PRD non-goal #1"
+status = "active"
+`)
+
+	if _, err := loadExemptions(path); err == nil {
+		t.Fatal("expected duplicate exemption ID to be rejected")
+	}
+}
+
+// @covers US-005-AC6
+func TestLoadExemptionsRejectsEmptyPRDRef(t *testing.T) {
+	path := writeTOML(t, `
+[[exemption]]
+id = "missing-prd"
+field = "record.offset"
+scope = "all"
+oracle = "all"
+reason = "must cite the governing PRD non-goal"
+prd_ref = ""
+status = "active"
+`)
+
+	if _, err := loadExemptions(path); err == nil {
+		t.Fatal("expected empty prd_ref to be rejected")
+	}
+}
+
+func writeTOML(t *testing.T, contents string) string {
+	t.Helper()
+
+	path := filepath.Join(t.TempDir(), "exemptions.toml")
+	if err := os.WriteFile(path, []byte(contents), 0o644); err != nil {
+		t.Fatalf("write exemptions.toml: %v", err)
+	}
+	return path
+}
diff --git a/tests/conformance/go/workloads.go b/tests/conformance/go/workloads.go
index a9be0c8..71bab45 100644
--- a/tests/conformance/go/workloads.go
+++ b/tests/conformance/go/workloads.go
@@ -17,6 +17,8 @@ type Workload struct {
 }
 
 // workloads in the same execution order as the Rust harness's workloads::all().
+// @covers US-005-AC4
+// @covers US-005-AC7
 func allWorkloads() []Workload {
 	return []Workload{
 		{"produce_fetch_roundtrip", produceFetch},
@@ -136,6 +138,7 @@ func produceFetch(ctx context.Context, bootstrap string) ([]Observation, error)
 
 // ── idempotent_produce_roundtrip ──────────────────────────────────────────────
 // @covers US-003-AC5
+// @covers US-005-AC7
 
 func idempotentProduce(ctx context.Context, bootstrap string) ([]Observation, error) {
 	const topic, n = "parity-idempotent-produce", 10
@@ -216,6 +219,7 @@ func consumerGroup(ctx context.Context, bootstrap string) ([]Observation, error)
 
 // ── transactional_produce_roundtrip ───────────────────────────────────────────
 // @covers US-004-AC6
+// @covers US-005-AC7
 
 func transactionalProduce(ctx context.Context, bootstrap string) ([]Observation, error) {
 	const topic, txnID = "parity-txn-produce", "parity-txn-test"
diff --git a/tests/conformance/python/conformance/diff.py b/tests/conformance/python/conformance/diff.py
index 27ddafb..6c2b612 100644
--- a/tests/conformance/python/conformance/diff.py
+++ b/tests/conformance/python/conformance/diff.py
@@ -40,6 +40,9 @@ def _to_json(value: Any) -> Any:
     return value
 
 
+# @covers US-005-AC1
+# @covers US-005-AC3
+# @covers US-005-AC6
 def diff(
     workload: str,
     oracle: str,
diff --git a/tests/conformance/python/conformance/exemptions.py b/tests/conformance/python/conformance/exemptions.py
index 109072f..6ff58d9 100644
--- a/tests/conformance/python/conformance/exemptions.py
+++ b/tests/conformance/python/conformance/exemptions.py
@@ -25,6 +25,7 @@ class Exemptions:
     def __init__(self, entries: list[Exemption]):
         self.entries = entries
 
+    # @covers US-005-AC6
     def find(self, field: str, workload: str, oracle: str) -> str | None:
         for e in self.entries:
             if (
@@ -37,6 +38,7 @@ class Exemptions:
         return None
 
 
+# @covers US-005-AC6
 def load(path: Path) -> Exemptions:
     if not path.exists():
         return Exemptions([])
diff --git a/tests/conformance/python/conformance/main.py b/tests/conformance/python/conformance/main.py
index 4823bc3..ac026d3 100644
--- a/tests/conformance/python/conformance/main.py
+++ b/tests/conformance/python/conformance/main.py
@@ -19,6 +19,8 @@ from .exemptions import load as load_exemptions
 EXEMPTIONS_PATH = Path("/conformance/exemptions.toml")
 
 
+# @covers US-005-AC1
+# @covers US-005-AC3
 def main() -> int:
     heimq, oracles = targets.from_env()
     exemptions = load_exemptions(EXEMPTIONS_PATH)
diff --git a/tests/conformance/python/conformance/normalize.py b/tests/conformance/python/conformance/normalize.py
index 5f949b1..e8f3a18 100644
--- a/tests/conformance/python/conformance/normalize.py
+++ b/tests/conformance/python/conformance/normalize.py
@@ -8,5 +8,6 @@ timestamps, producer_id, member_id, generation_id) lands here.
 from .observation import Observation
 
 
+# @covers US-005-AC2
 def normalize(observations: list[Observation]) -> list[Observation]:
     return observations
diff --git a/tests/conformance/python/conformance/workloads/__init__.py b/tests/conformance/python/conformance/workloads/__init__.py
index e421719..0e1219b 100644
--- a/tests/conformance/python/conformance/workloads/__init__.py
+++ b/tests/conformance/python/conformance/workloads/__init__.py
@@ -14,6 +14,8 @@ from . import (
 )
 
 
+# @covers US-005-AC4
+# @covers US-005-AC7
 def all() -> list[ModuleType]:
     """Every workload, in execution order. Each module exposes NAME and run()."""
     return [
diff --git a/tests/conformance/python/conformance/workloads/idempotent_produce.py b/tests/conformance/python/conformance/workloads/idempotent_produce.py
index 48465eff..dfffbf5 100644
--- a/tests/conformance/python/conformance/workloads/idempotent_produce.py
+++ b/tests/conformance/python/conformance/workloads/idempotent_produce.py
@@ -20,6 +20,7 @@ TOPIC = "parity-idempotent-produce"
 N = 10
 
 # @covers US-003-AC5
+# @covers US-005-AC7
 
 
 def run(target: Target) -> list[Observation]:
diff --git a/tests/conformance/python/conformance/workloads/transactional_produce.py b/tests/conformance/python/conformance/workloads/transactional_produce.py
index 5ba7030..71a22cc 100644
--- a/tests/conformance/python/conformance/workloads/transactional_produce.py
+++ b/tests/conformance/python/conformance/workloads/transactional_produce.py
@@ -20,6 +20,7 @@ TXN_ID = "parity-txn-test"
 COMMITTED = 3
 
 # @covers US-004-AC6
+# @covers US-005-AC7
 
 
 def run(target: Target) -> list[Observation]:
diff --git a/tests/conformance/run.sh b/tests/conformance/run.sh
index 61b905b..e9ae8bc 100755
--- a/tests/conformance/run.sh
+++ b/tests/conformance/run.sh
@@ -30,6 +30,8 @@ status=0
 for runner in "${runners[@]}"; do
   echo "═══ conformance runner: ${runner} ═══"
   cleanup
+  # @covers US-005-AC1
+  # @covers US-005-AC5
   if ! docker compose up --build --abort-on-container-exit \
       --exit-code-from "$runner" "$runner"; then
     status=1
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
