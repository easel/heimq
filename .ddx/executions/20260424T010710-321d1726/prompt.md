<bead-review>
  <bead id="heimq-990ad721" iter=1>
    <title>e2e: multiple independent consumer groups on same topic</title>
    <description>
Produce N messages to topic T. Two consumer groups G1 and G2 each subscribe to T independently. Assert each group receives the full message set, and that committing in G1 does not affect G2's position (committed offsets are group-scoped, not topic-scoped).
In-scope files:
- tests/integration.rs (new fn test_rdkafka_independent_consumer_groups)
    </description>
    <acceptance>
`cargo test --test integration test_rdkafka_independent_consumer_groups -- --ignored --nocapture` passes. Assertions: both groups consume all N messages; after G1 commits offset k, G2's OffsetFetch for the same topic/partition is unaffected.
    </acceptance>
    <labels>area:tests, kind:test, layer:e2e, client:rdkafka, phase:5, tier:p0, topic:group-correctness</labels>
  </bead>

  <governing>
    <note>No governing documents found. Evaluate the diff against the acceptance criteria alone.</note>
  </governing>

  <diff rev="0499855f5df8b197d85d632374fa31851fcf1eba">
commit 0499855f5df8b197d85d632374fa31851fcf1eba
Author: ddx-land-coordinator <coordinator@ddx.local>
Date:   Thu Apr 23 21:07:09 2026 -0400

    chore: add execution evidence [20260424T010511-]

diff --git a/.ddx/executions/20260424T010511-44c454f6/manifest.json b/.ddx/executions/20260424T010511-44c454f6/manifest.json
new file mode 100644
index 0000000..954facc
--- /dev/null
+++ b/.ddx/executions/20260424T010511-44c454f6/manifest.json
@@ -0,0 +1,42 @@
+{
+  "attempt_id": "20260424T010511-44c454f6",
+  "bead_id": "heimq-990ad721",
+  "base_rev": "6c6540926cfe1af6877ae9133453998fa9926423",
+  "created_at": "2026-04-24T01:05:15.614996749Z",
+  "requested": {
+    "harness": "claude",
+    "prompt": "synthesized"
+  },
+  "bead": {
+    "id": "heimq-990ad721",
+    "title": "e2e: multiple independent consumer groups on same topic",
+    "description": "Produce N messages to topic T. Two consumer groups G1 and G2 each subscribe to T independently. Assert each group receives the full message set, and that committing in G1 does not affect G2's position (committed offsets are group-scoped, not topic-scoped).\nIn-scope files:\n- tests/integration.rs (new fn test_rdkafka_independent_consumer_groups)",
+    "acceptance": "`cargo test --test integration test_rdkafka_independent_consumer_groups -- --ignored --nocapture` passes. Assertions: both groups consume all N messages; after G1 commits offset k, G2's OffsetFetch for the same topic/partition is unaffected.",
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
+      "claimed-at": "2026-04-24T01:05:11Z",
+      "claimed-machine": "eitri",
+      "claimed-pid": "196235",
+      "execute-loop-heartbeat-at": "2026-04-24T01:05:11.469996254Z",
+      "spec-id": "API-001"
+    }
+  },
+  "paths": {
+    "dir": ".ddx/executions/20260424T010511-44c454f6",
+    "prompt": ".ddx/executions/20260424T010511-44c454f6/prompt.md",
+    "manifest": ".ddx/executions/20260424T010511-44c454f6/manifest.json",
+    "result": ".ddx/executions/20260424T010511-44c454f6/result.json",
+    "checks": ".ddx/executions/20260424T010511-44c454f6/checks.json",
+    "usage": ".ddx/executions/20260424T010511-44c454f6/usage.json",
+    "worktree": "tmp/ddx-exec-wt/.execute-bead-wt-heimq-990ad721-20260424T010511-44c454f6"
+  }
+}
\ No newline at end of file
diff --git a/.ddx/executions/20260424T010511-44c454f6/result.json b/.ddx/executions/20260424T010511-44c454f6/result.json
new file mode 100644
index 0000000..4b2d85f
--- /dev/null
+++ b/.ddx/executions/20260424T010511-44c454f6/result.json
@@ -0,0 +1,22 @@
+{
+  "bead_id": "heimq-990ad721",
+  "attempt_id": "20260424T010511-44c454f6",
+  "base_rev": "6c6540926cfe1af6877ae9133453998fa9926423",
+  "result_rev": "7db27f602f382149328150334d06cc0b7eb76da6",
+  "outcome": "task_succeeded",
+  "status": "success",
+  "detail": "success",
+  "harness": "claude",
+  "session_id": "eb-8d3e0ab3",
+  "duration_ms": 104311,
+  "tokens": 5199,
+  "cost_usd": 0.82141175,
+  "exit_code": 0,
+  "execution_dir": ".ddx/executions/20260424T010511-44c454f6",
+  "prompt_file": ".ddx/executions/20260424T010511-44c454f6/prompt.md",
+  "manifest_file": ".ddx/executions/20260424T010511-44c454f6/manifest.json",
+  "result_file": ".ddx/executions/20260424T010511-44c454f6/result.json",
+  "usage_file": ".ddx/executions/20260424T010511-44c454f6/usage.json",
+  "started_at": "2026-04-24T01:05:15.615257415Z",
+  "finished_at": "2026-04-24T01:06:59.926873331Z"
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
## Review: heimq-990ad721 iter 1

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
