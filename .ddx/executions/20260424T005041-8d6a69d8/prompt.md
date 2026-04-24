<bead-review>
  <bead id="heimq-f17da917" iter=1>
    <title>e2e: single-group delivery integrity on multi-partition topic</title>
    <description>
Create a multi-partition topic (&gt;=3 partitions), produce N messages (&gt;=300) with distinct keys, start a consumer group with &gt;=2 members, assert every produced message is consumed exactly once across the group and that no two members claim the same partition concurrently. Supersedes the earlier 'exactly-once' framing — Kafka groups do not guarantee exactly-once; assertion is no gaps + no duplicate ownership.
In-scope files:
- tests/integration.rs (new #[tokio::test] fn test_rdkafka_group_multi_partition_delivery)
Out of scope:
- tests/contract.rs — contract layer is covered
- Any src/ changes — this should work against current broker behavior
    </description>
    <acceptance>
`cargo test --test integration test_rdkafka_group_multi_partition_delivery -- --ignored --nocapture` passes. Test asserts: (a) set of consumed (partition, offset) pairs == set of produced, (b) no partition assigned to two members at the same time during steady state.
    </acceptance>
    <labels>area:tests, kind:test, layer:e2e, client:rdkafka, phase:5, tier:p0, topic:group-correctness</labels>
  </bead>

  <governing>
    <note>No governing documents found. Evaluate the diff against the acceptance criteria alone.</note>
  </governing>

  <diff rev="b90b9b434b7f113cd0bfcde74c59ba6f1b9b3a5a">
commit b90b9b434b7f113cd0bfcde74c59ba6f1b9b3a5a
Author: ddx-land-coordinator <coordinator@ddx.local>
Date:   Thu Apr 23 20:50:01 2026 -0400

    chore: add execution evidence [20260424T003805-]

diff --git a/.ddx/executions/20260424T003805-bcbf14f1/manifest.json b/.ddx/executions/20260424T003805-bcbf14f1/manifest.json
new file mode 100644
index 0000000..d949dc6
--- /dev/null
+++ b/.ddx/executions/20260424T003805-bcbf14f1/manifest.json
@@ -0,0 +1,84 @@
+{
+  "attempt_id": "20260424T003805-bcbf14f1",
+  "bead_id": "heimq-f17da917",
+  "base_rev": "bc9b5845b123abbddbf94887b17b9524c85073fa",
+  "created_at": "2026-04-24T00:38:40.038067465Z",
+  "requested": {
+    "harness": "claude",
+    "prompt": "synthesized"
+  },
+  "bead": {
+    "id": "heimq-f17da917",
+    "title": "e2e: single-group delivery integrity on multi-partition topic",
+    "description": "Create a multi-partition topic (\u003e=3 partitions), produce N messages (\u003e=300) with distinct keys, start a consumer group with \u003e=2 members, assert every produced message is consumed exactly once across the group and that no two members claim the same partition concurrently. Supersedes the earlier 'exactly-once' framing — Kafka groups do not guarantee exactly-once; assertion is no gaps + no duplicate ownership.\nIn-scope files:\n- tests/integration.rs (new #[tokio::test] fn test_rdkafka_group_multi_partition_delivery)\nOut of scope:\n- tests/contract.rs — contract layer is covered\n- Any src/ changes — this should work against current broker behavior",
+    "acceptance": "`cargo test --test integration test_rdkafka_group_multi_partition_delivery -- --ignored --nocapture` passes. Test asserts: (a) set of consumed (partition, offset) pairs == set of produced, (b) no partition assigned to two members at the same time during steady state.",
+    "parent": "heimq-700e47b2",
+    "labels": [
+      "area:tests",
+      "kind:test",
+      "layer:e2e",
+      "client:rdkafka",
+      "phase:5",
+      "tier:p0",
+      "topic:group-correctness"
+    ],
+    "metadata": {
+      "claimed-at": "2026-04-24T00:38:05Z",
+      "claimed-machine": "eitri",
+      "claimed-pid": "196235",
+      "events": [
+        {
+          "actor": "ddx",
+          "body": "tier=cheap harness= model= probe=no viable provider\nno viable harness found",
+          "created_at": "2026-04-24T00:37:37.219406093Z",
+          "kind": "tier-attempt",
+          "source": "ddx agent execute-loop",
+          "summary": "skipped"
+        },
+        {
+          "actor": "ddx",
+          "body": "tier=standard harness= model= probe=no viable provider\nno viable harness found",
+          "created_at": "2026-04-24T00:37:37.345453475Z",
+          "kind": "tier-attempt",
+          "source": "ddx agent execute-loop",
+          "summary": "skipped"
+        },
+        {
+          "actor": "ddx",
+          "body": "tier=smart harness= model= probe=no viable provider\nno viable harness found",
+          "created_at": "2026-04-24T00:37:37.474746939Z",
+          "kind": "tier-attempt",
+          "source": "ddx agent execute-loop",
+          "summary": "skipped"
+        },
+        {
+          "actor": "ddx",
+          "body": "{\"tiers_attempted\":[{\"tier\":\"cheap\",\"status\":\"skipped\",\"cost_usd\":0,\"duration_ms\":0},{\"tier\":\"standard\",\"status\":\"skipped\",\"cost_usd\":0,\"duration_ms\":0},{\"tier\":\"smart\",\"status\":\"skipped\",\"cost_usd\":0,\"duration_ms\":0}],\"winning_tier\":\"exhausted\",\"total_cost_usd\":0,\"wasted_cost_usd\":0}",
+          "created_at": "2026-04-24T00:37:37.47941377Z",
+          "kind": "escalation-summary",
+          "source": "ddx agent execute-loop",
+          "summary": "winning_tier=exhausted attempts=3 total_cost_usd=0.0000 wasted_cost_usd=0.0000"
+        },
+        {
+          "actor": "ddx",
+          "body": "execute-loop: all tiers exhausted — no viable provider found",
+          "created_at": "2026-04-24T00:37:37.487393224Z",
+          "kind": "execute-bead",
+          "source": "ddx agent execute-loop",
+          "summary": "execution_failed"
+        }
+      ],
+      "execute-loop-heartbeat-at": "2026-04-24T00:38:05.325310572Z",
+      "spec-id": "API-001"
+    }
+  },
+  "paths": {
+    "dir": ".ddx/executions/20260424T003805-bcbf14f1",
+    "prompt": ".ddx/executions/20260424T003805-bcbf14f1/prompt.md",
+    "manifest": ".ddx/executions/20260424T003805-bcbf14f1/manifest.json",
+    "result": ".ddx/executions/20260424T003805-bcbf14f1/result.json",
+    "checks": ".ddx/executions/20260424T003805-bcbf14f1/checks.json",
+    "usage": ".ddx/executions/20260424T003805-bcbf14f1/usage.json",
+    "worktree": "tmp/ddx-exec-wt/.execute-bead-wt-heimq-f17da917-20260424T003805-bcbf14f1"
+  }
+}
\ No newline at end of file
diff --git a/.ddx/executions/20260424T003805-bcbf14f1/result.json b/.ddx/executions/20260424T003805-bcbf14f1/result.json
new file mode 100644
index 0000000..53dcc8e
--- /dev/null
+++ b/.ddx/executions/20260424T003805-bcbf14f1/result.json
@@ -0,0 +1,22 @@
+{
+  "bead_id": "heimq-f17da917",
+  "attempt_id": "20260424T003805-bcbf14f1",
+  "base_rev": "bc9b5845b123abbddbf94887b17b9524c85073fa",
+  "result_rev": "ab6b355e62189426eb261318d53d27e71187b503",
+  "outcome": "task_succeeded",
+  "status": "success",
+  "detail": "success",
+  "harness": "claude",
+  "session_id": "eb-0a56af29",
+  "duration_ms": 670703,
+  "tokens": 39625,
+  "cost_usd": 3.297269,
+  "exit_code": 0,
+  "execution_dir": ".ddx/executions/20260424T003805-bcbf14f1",
+  "prompt_file": ".ddx/executions/20260424T003805-bcbf14f1/prompt.md",
+  "manifest_file": ".ddx/executions/20260424T003805-bcbf14f1/manifest.json",
+  "result_file": ".ddx/executions/20260424T003805-bcbf14f1/result.json",
+  "usage_file": ".ddx/executions/20260424T003805-bcbf14f1/usage.json",
+  "started_at": "2026-04-24T00:38:40.038406048Z",
+  "finished_at": "2026-04-24T00:49:50.742278423Z"
+}
\ No newline at end of file
  </diff>

  <instructions>
You are reviewing a bead implementation against its acceptance criteria.

## Your task

Examine the diff and each acceptance-criteria (AC) item. For each item assign one grade:

- **APPROVE** — fully and correctly implemented; cite the specific file path and line that proves it.
- **REQUEST_CHANGES** — partially implemented or has fixable minor issues.
- **BLOCK** — not implemented, incorrectly implemented, or the diff is insufficient to evaluate.

Overall verdict rule:
- All items APPROVE → **APPROVE**
- Any item BLOCK → **BLOCK**
- Otherwise → **REQUEST_CHANGES**

## Required output format

Respond with a structured review using exactly this layout (replace placeholder text):

---
## Review: heimq-f17da917 iter 1

### Verdict: APPROVE | REQUEST_CHANGES | BLOCK

### AC Grades

| # | Item | Grade | Evidence |
|---|------|-------|----------|
| 1 | &lt;AC item text, max 60 chars&gt; | APPROVE | path/to/file.go:42 — brief note |
| 2 | &lt;AC item text, max 60 chars&gt; | BLOCK   | — not found in diff |

### Summary

&lt;1–3 sentences on overall implementation quality and any recurring theme in findings.&gt;

### Findings

&lt;Bullet list of REQUEST_CHANGES and BLOCK findings. Each finding must name the specific file, function, or test that is missing or wrong — specific enough for the next agent to act on without re-reading the entire diff. Omit this section entirely if verdict is APPROVE.&gt;
  </instructions>
</bead-review>
