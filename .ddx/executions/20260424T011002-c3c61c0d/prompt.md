<bead-review>
  <bead id="heimq-a77fc237" iter=1>
    <title>e2e: auto.offset.reset earliest vs latest on fresh group</title>
    <description>
Pre-populate topic T with M messages before any consumer exists. Start consumer C1 in new group G1 with auto.offset.reset=earliest — must receive all M messages. Start consumer C2 in new group G2 with auto.offset.reset=latest — must receive zero existing messages but see subsequent produces.
In-scope files:
- tests/integration.rs (new fn test_rdkafka_auto_offset_reset_earliest_vs_latest)
    </description>
    <acceptance>
`cargo test --test integration test_rdkafka_auto_offset_reset_earliest_vs_latest -- --ignored --nocapture` passes with both branches asserted.
    </acceptance>
    <labels>area:tests, kind:test, layer:e2e, client:rdkafka, phase:5, tier:p0, topic:offsets</labels>
  </bead>

  <governing>
    <note>No governing documents found. Evaluate the diff against the acceptance criteria alone.</note>
  </governing>

  <diff rev="e73133ac616dd3d0137869f0b99c7b43627e4046">
commit e73133ac616dd3d0137869f0b99c7b43627e4046
Author: ddx-land-coordinator <coordinator@ddx.local>
Date:   Thu Apr 23 21:09:59 2026 -0400

    chore: add execution evidence [20260424T010741-]

diff --git a/.ddx/executions/20260424T010741-235e307b/manifest.json b/.ddx/executions/20260424T010741-235e307b/manifest.json
new file mode 100644
index 0000000..ce3f151
--- /dev/null
+++ b/.ddx/executions/20260424T010741-235e307b/manifest.json
@@ -0,0 +1,42 @@
+{
+  "attempt_id": "20260424T010741-235e307b",
+  "bead_id": "heimq-a77fc237",
+  "base_rev": "931f235a1a970f442f35002e95b8d2d01cbf497d",
+  "created_at": "2026-04-24T01:07:45.029556881Z",
+  "requested": {
+    "harness": "claude",
+    "prompt": "synthesized"
+  },
+  "bead": {
+    "id": "heimq-a77fc237",
+    "title": "e2e: auto.offset.reset earliest vs latest on fresh group",
+    "description": "Pre-populate topic T with M messages before any consumer exists. Start consumer C1 in new group G1 with auto.offset.reset=earliest — must receive all M messages. Start consumer C2 in new group G2 with auto.offset.reset=latest — must receive zero existing messages but see subsequent produces.\nIn-scope files:\n- tests/integration.rs (new fn test_rdkafka_auto_offset_reset_earliest_vs_latest)",
+    "acceptance": "`cargo test --test integration test_rdkafka_auto_offset_reset_earliest_vs_latest -- --ignored --nocapture` passes with both branches asserted.",
+    "parent": "heimq-700e47b2",
+    "labels": [
+      "area:tests",
+      "kind:test",
+      "layer:e2e",
+      "client:rdkafka",
+      "phase:5",
+      "tier:p0",
+      "topic:offsets"
+    ],
+    "metadata": {
+      "claimed-at": "2026-04-24T01:07:41Z",
+      "claimed-machine": "eitri",
+      "claimed-pid": "196235",
+      "execute-loop-heartbeat-at": "2026-04-24T01:07:41.164748583Z",
+      "spec-id": "API-001"
+    }
+  },
+  "paths": {
+    "dir": ".ddx/executions/20260424T010741-235e307b",
+    "prompt": ".ddx/executions/20260424T010741-235e307b/prompt.md",
+    "manifest": ".ddx/executions/20260424T010741-235e307b/manifest.json",
+    "result": ".ddx/executions/20260424T010741-235e307b/result.json",
+    "checks": ".ddx/executions/20260424T010741-235e307b/checks.json",
+    "usage": ".ddx/executions/20260424T010741-235e307b/usage.json",
+    "worktree": "tmp/ddx-exec-wt/.execute-bead-wt-heimq-a77fc237-20260424T010741-235e307b"
+  }
+}
\ No newline at end of file
diff --git a/.ddx/executions/20260424T010741-235e307b/result.json b/.ddx/executions/20260424T010741-235e307b/result.json
new file mode 100644
index 0000000..a99bff0
--- /dev/null
+++ b/.ddx/executions/20260424T010741-235e307b/result.json
@@ -0,0 +1,22 @@
+{
+  "bead_id": "heimq-a77fc237",
+  "attempt_id": "20260424T010741-235e307b",
+  "base_rev": "931f235a1a970f442f35002e95b8d2d01cbf497d",
+  "result_rev": "05fd9b37688248edc4eefde16654908fabfa4739",
+  "outcome": "task_succeeded",
+  "status": "success",
+  "detail": "success",
+  "harness": "claude",
+  "session_id": "eb-63b19dbe",
+  "duration_ms": 125255,
+  "tokens": 6258,
+  "cost_usd": 0.6899157499999998,
+  "exit_code": 0,
+  "execution_dir": ".ddx/executions/20260424T010741-235e307b",
+  "prompt_file": ".ddx/executions/20260424T010741-235e307b/prompt.md",
+  "manifest_file": ".ddx/executions/20260424T010741-235e307b/manifest.json",
+  "result_file": ".ddx/executions/20260424T010741-235e307b/result.json",
+  "usage_file": ".ddx/executions/20260424T010741-235e307b/usage.json",
+  "started_at": "2026-04-24T01:07:45.029785422Z",
+  "finished_at": "2026-04-24T01:09:50.284972459Z"
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
## Review: heimq-a77fc237 iter 1

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
