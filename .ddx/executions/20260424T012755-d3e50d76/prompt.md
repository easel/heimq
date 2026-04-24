<bead-review>
  <bead id="heimq-885505ce" iter=1>
    <title>e2e: explicit partition targeting</title>
    <description>
rdkafka FutureProducer allows specifying a partition in FutureRecord::partition(). Produce to partition 2 of a 4-partition topic, consume only partition 2 via assign() rather than subscribe(), assert the message is present. Ensures heimq honors explicit partition selection bypassing the partitioner.
In-scope files:
- tests/integration.rs (new fn test_rdkafka_explicit_partition_target)
    </description>
    <acceptance>
`cargo test --test integration test_rdkafka_explicit_partition_target -- --ignored --nocapture` passes and asserts the message appears only on the targeted partition.
    </acceptance>
    <labels>area:tests, kind:test, layer:e2e, client:rdkafka, phase:5, tier:p0, topic:partition-selection</labels>
  </bead>

  <governing>
    <note>No governing documents found. Evaluate the diff against the acceptance criteria alone.</note>
  </governing>

  <diff rev="01077c934d99155fda49d8f47498e5e8809419cd">
commit 01077c934d99155fda49d8f47498e5e8809419cd
Author: ddx-land-coordinator <coordinator@ddx.local>
Date:   Thu Apr 23 21:27:54 2026 -0400

    chore: add execution evidence [20260424T012530-]

diff --git a/.ddx/executions/20260424T012530-4bbb7d7e/manifest.json b/.ddx/executions/20260424T012530-4bbb7d7e/manifest.json
new file mode 100644
index 0000000..0c18cf2
--- /dev/null
+++ b/.ddx/executions/20260424T012530-4bbb7d7e/manifest.json
@@ -0,0 +1,42 @@
+{
+  "attempt_id": "20260424T012530-4bbb7d7e",
+  "bead_id": "heimq-885505ce",
+  "base_rev": "ef55c30aa1205eeec0884762c571883bf9283d93",
+  "created_at": "2026-04-24T01:25:34.34428515Z",
+  "requested": {
+    "harness": "claude",
+    "prompt": "synthesized"
+  },
+  "bead": {
+    "id": "heimq-885505ce",
+    "title": "e2e: explicit partition targeting",
+    "description": "rdkafka FutureProducer allows specifying a partition in FutureRecord::partition(). Produce to partition 2 of a 4-partition topic, consume only partition 2 via assign() rather than subscribe(), assert the message is present. Ensures heimq honors explicit partition selection bypassing the partitioner.\nIn-scope files:\n- tests/integration.rs (new fn test_rdkafka_explicit_partition_target)",
+    "acceptance": "`cargo test --test integration test_rdkafka_explicit_partition_target -- --ignored --nocapture` passes and asserts the message appears only on the targeted partition.",
+    "parent": "heimq-700e47b2",
+    "labels": [
+      "area:tests",
+      "kind:test",
+      "layer:e2e",
+      "client:rdkafka",
+      "phase:5",
+      "tier:p0",
+      "topic:partition-selection"
+    ],
+    "metadata": {
+      "claimed-at": "2026-04-24T01:25:30Z",
+      "claimed-machine": "eitri",
+      "claimed-pid": "196235",
+      "execute-loop-heartbeat-at": "2026-04-24T01:25:30.497867517Z",
+      "spec-id": "API-001"
+    }
+  },
+  "paths": {
+    "dir": ".ddx/executions/20260424T012530-4bbb7d7e",
+    "prompt": ".ddx/executions/20260424T012530-4bbb7d7e/prompt.md",
+    "manifest": ".ddx/executions/20260424T012530-4bbb7d7e/manifest.json",
+    "result": ".ddx/executions/20260424T012530-4bbb7d7e/result.json",
+    "checks": ".ddx/executions/20260424T012530-4bbb7d7e/checks.json",
+    "usage": ".ddx/executions/20260424T012530-4bbb7d7e/usage.json",
+    "worktree": "tmp/ddx-exec-wt/.execute-bead-wt-heimq-885505ce-20260424T012530-4bbb7d7e"
+  }
+}
\ No newline at end of file
diff --git a/.ddx/executions/20260424T012530-4bbb7d7e/result.json b/.ddx/executions/20260424T012530-4bbb7d7e/result.json
new file mode 100644
index 0000000..5579616
--- /dev/null
+++ b/.ddx/executions/20260424T012530-4bbb7d7e/result.json
@@ -0,0 +1,22 @@
+{
+  "bead_id": "heimq-885505ce",
+  "attempt_id": "20260424T012530-4bbb7d7e",
+  "base_rev": "ef55c30aa1205eeec0884762c571883bf9283d93",
+  "result_rev": "1df4a6cdbb729b0cf023fd495f2951ee71e36ec8",
+  "outcome": "task_succeeded",
+  "status": "success",
+  "detail": "success",
+  "harness": "claude",
+  "session_id": "eb-422890d5",
+  "duration_ms": 130241,
+  "tokens": 5541,
+  "cost_usd": 0.7135445,
+  "exit_code": 0,
+  "execution_dir": ".ddx/executions/20260424T012530-4bbb7d7e",
+  "prompt_file": ".ddx/executions/20260424T012530-4bbb7d7e/prompt.md",
+  "manifest_file": ".ddx/executions/20260424T012530-4bbb7d7e/manifest.json",
+  "result_file": ".ddx/executions/20260424T012530-4bbb7d7e/result.json",
+  "usage_file": ".ddx/executions/20260424T012530-4bbb7d7e/usage.json",
+  "started_at": "2026-04-24T01:25:34.344541816Z",
+  "finished_at": "2026-04-24T01:27:44.585755792Z"
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
## Review: heimq-885505ce iter 1

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
