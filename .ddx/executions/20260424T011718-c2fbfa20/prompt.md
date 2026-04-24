<bead-review>
  <bead id="heimq-1597cf54" iter=1>
    <title>e2e: auto-commit with configured interval</title>
    <description>
Exercise the enable.auto.commit=true code path with an explicit auto.commit.interval.ms (e.g. 200ms). Consumer reads K messages, waits &gt;interval, closes; fresh consumer in same group should resume after the auto-committed offset. Complements the manual-commit test above.
In-scope files:
- tests/integration.rs (new fn test_rdkafka_auto_commit_interval)
    </description>
    <acceptance>
`cargo test --test integration test_rdkafka_auto_commit_interval -- --ignored --nocapture` passes and asserts resume position matches auto-committed offset (within one batch of the last consumed message).
    </acceptance>
    <labels>area:tests, kind:test, layer:e2e, client:rdkafka, phase:5, tier:p0, topic:offsets</labels>
  </bead>

  <governing>
    <note>No governing documents found. Evaluate the diff against the acceptance criteria alone.</note>
  </governing>

  <diff rev="ff0e8a50ecf44a2acc73674037f6b97c8e543682">
commit ff0e8a50ecf44a2acc73674037f6b97c8e543682
Author: ddx-land-coordinator <coordinator@ddx.local>
Date:   Thu Apr 23 21:17:17 2026 -0400

    chore: add execution evidence [20260424T011358-]

diff --git a/.ddx/executions/20260424T011358-fca690ea/manifest.json b/.ddx/executions/20260424T011358-fca690ea/manifest.json
new file mode 100644
index 0000000..f3ff643
--- /dev/null
+++ b/.ddx/executions/20260424T011358-fca690ea/manifest.json
@@ -0,0 +1,42 @@
+{
+  "attempt_id": "20260424T011358-fca690ea",
+  "bead_id": "heimq-1597cf54",
+  "base_rev": "fb448cc517defd9358b10cc98eb17659772de2f9",
+  "created_at": "2026-04-24T01:14:02.339244106Z",
+  "requested": {
+    "harness": "claude",
+    "prompt": "synthesized"
+  },
+  "bead": {
+    "id": "heimq-1597cf54",
+    "title": "e2e: auto-commit with configured interval",
+    "description": "Exercise the enable.auto.commit=true code path with an explicit auto.commit.interval.ms (e.g. 200ms). Consumer reads K messages, waits \u003einterval, closes; fresh consumer in same group should resume after the auto-committed offset. Complements the manual-commit test above.\nIn-scope files:\n- tests/integration.rs (new fn test_rdkafka_auto_commit_interval)",
+    "acceptance": "`cargo test --test integration test_rdkafka_auto_commit_interval -- --ignored --nocapture` passes and asserts resume position matches auto-committed offset (within one batch of the last consumed message).",
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
+      "claimed-at": "2026-04-24T01:13:58Z",
+      "claimed-machine": "eitri",
+      "claimed-pid": "196235",
+      "execute-loop-heartbeat-at": "2026-04-24T01:13:58.345017796Z",
+      "spec-id": "API-001"
+    }
+  },
+  "paths": {
+    "dir": ".ddx/executions/20260424T011358-fca690ea",
+    "prompt": ".ddx/executions/20260424T011358-fca690ea/prompt.md",
+    "manifest": ".ddx/executions/20260424T011358-fca690ea/manifest.json",
+    "result": ".ddx/executions/20260424T011358-fca690ea/result.json",
+    "checks": ".ddx/executions/20260424T011358-fca690ea/checks.json",
+    "usage": ".ddx/executions/20260424T011358-fca690ea/usage.json",
+    "worktree": "tmp/ddx-exec-wt/.execute-bead-wt-heimq-1597cf54-20260424T011358-fca690ea"
+  }
+}
\ No newline at end of file
diff --git a/.ddx/executions/20260424T011358-fca690ea/result.json b/.ddx/executions/20260424T011358-fca690ea/result.json
new file mode 100644
index 0000000..2810eb7
--- /dev/null
+++ b/.ddx/executions/20260424T011358-fca690ea/result.json
@@ -0,0 +1,22 @@
+{
+  "bead_id": "heimq-1597cf54",
+  "attempt_id": "20260424T011358-fca690ea",
+  "base_rev": "fb448cc517defd9358b10cc98eb17659772de2f9",
+  "result_rev": "fa040cbf9d3b3009cf7abad6988faa5d4681c5b9",
+  "outcome": "task_succeeded",
+  "status": "success",
+  "detail": "success",
+  "harness": "claude",
+  "session_id": "eb-ef57f014",
+  "duration_ms": 184778,
+  "tokens": 9981,
+  "cost_usd": 0.8973752500000001,
+  "exit_code": 0,
+  "execution_dir": ".ddx/executions/20260424T011358-fca690ea",
+  "prompt_file": ".ddx/executions/20260424T011358-fca690ea/prompt.md",
+  "manifest_file": ".ddx/executions/20260424T011358-fca690ea/manifest.json",
+  "result_file": ".ddx/executions/20260424T011358-fca690ea/result.json",
+  "usage_file": ".ddx/executions/20260424T011358-fca690ea/usage.json",
+  "started_at": "2026-04-24T01:14:02.339509236Z",
+  "finished_at": "2026-04-24T01:17:07.11841663Z"
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
## Review: heimq-1597cf54 iter 1

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
