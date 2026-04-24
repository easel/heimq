<bead-review>
  <bead id="heimq-f1e42e32" iter=1>
    <title>e2e: rebalance on member leave (graceful + session timeout)</title>
    <description>
Two scenarios in one test file: (1) member calls unsubscribe()/close() → LeaveGroup → remaining member takes over all partitions; (2) member stops heartbeating and session timeout expires → coordinator evicts member → remaining member takes over. Existing test_rdkafka_consumer_rebalance_on_new_member covers the join direction; this covers leave.
In-scope files:
- tests/integration.rs (new fns test_rdkafka_group_rebalance_on_graceful_leave, test_rdkafka_group_rebalance_on_session_timeout)
    </description>
    <acceptance>
`cargo test --test integration test_rdkafka_group_rebalance_on -- --ignored --nocapture` runs both tests and asserts final partition assignment maps entirely to the surviving member.
    </acceptance>
    <labels>area:tests, kind:test, layer:e2e, client:rdkafka, phase:5, tier:p0, topic:group-correctness</labels>
  </bead>

  <governing>
    <note>No governing documents found. Evaluate the diff against the acceptance criteria alone.</note>
  </governing>

  <diff rev="6e9dbfddd1e789b8a22c83364da433ecc9c08d31">
commit 6e9dbfddd1e789b8a22c83364da433ecc9c08d31
Author: ddx-land-coordinator <coordinator@ddx.local>
Date:   Thu Apr 23 21:04:36 2026 -0400

    chore: add execution evidence [20260424T005459-]

diff --git a/.ddx/executions/20260424T005459-04bf1b1b/manifest.json b/.ddx/executions/20260424T005459-04bf1b1b/manifest.json
new file mode 100644
index 0000000..594fb6b
--- /dev/null
+++ b/.ddx/executions/20260424T005459-04bf1b1b/manifest.json
@@ -0,0 +1,42 @@
+{
+  "attempt_id": "20260424T005459-04bf1b1b",
+  "bead_id": "heimq-f1e42e32",
+  "base_rev": "57428a84512f277034274229289f4a9dd536fbcf",
+  "created_at": "2026-04-24T00:55:03.611772206Z",
+  "requested": {
+    "harness": "claude",
+    "prompt": "synthesized"
+  },
+  "bead": {
+    "id": "heimq-f1e42e32",
+    "title": "e2e: rebalance on member leave (graceful + session timeout)",
+    "description": "Two scenarios in one test file: (1) member calls unsubscribe()/close() → LeaveGroup → remaining member takes over all partitions; (2) member stops heartbeating and session timeout expires → coordinator evicts member → remaining member takes over. Existing test_rdkafka_consumer_rebalance_on_new_member covers the join direction; this covers leave.\nIn-scope files:\n- tests/integration.rs (new fns test_rdkafka_group_rebalance_on_graceful_leave, test_rdkafka_group_rebalance_on_session_timeout)",
+    "acceptance": "`cargo test --test integration test_rdkafka_group_rebalance_on -- --ignored --nocapture` runs both tests and asserts final partition assignment maps entirely to the surviving member.",
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
+      "claimed-at": "2026-04-24T00:54:59Z",
+      "claimed-machine": "eitri",
+      "claimed-pid": "196235",
+      "execute-loop-heartbeat-at": "2026-04-24T00:54:59.67872117Z",
+      "spec-id": "API-001"
+    }
+  },
+  "paths": {
+    "dir": ".ddx/executions/20260424T005459-04bf1b1b",
+    "prompt": ".ddx/executions/20260424T005459-04bf1b1b/prompt.md",
+    "manifest": ".ddx/executions/20260424T005459-04bf1b1b/manifest.json",
+    "result": ".ddx/executions/20260424T005459-04bf1b1b/result.json",
+    "checks": ".ddx/executions/20260424T005459-04bf1b1b/checks.json",
+    "usage": ".ddx/executions/20260424T005459-04bf1b1b/usage.json",
+    "worktree": "tmp/ddx-exec-wt/.execute-bead-wt-heimq-f1e42e32-20260424T005459-04bf1b1b"
+  }
+}
\ No newline at end of file
diff --git a/.ddx/executions/20260424T005459-04bf1b1b/result.json b/.ddx/executions/20260424T005459-04bf1b1b/result.json
new file mode 100644
index 0000000..ae1d96f
--- /dev/null
+++ b/.ddx/executions/20260424T005459-04bf1b1b/result.json
@@ -0,0 +1,22 @@
+{
+  "bead_id": "heimq-f1e42e32",
+  "attempt_id": "20260424T005459-04bf1b1b",
+  "base_rev": "57428a84512f277034274229289f4a9dd536fbcf",
+  "result_rev": "9bafeb34c3c77a0cda6510aee793d476a5c7fb94",
+  "outcome": "task_succeeded",
+  "status": "success",
+  "detail": "success",
+  "harness": "claude",
+  "session_id": "eb-d7c64f74",
+  "duration_ms": 561692,
+  "tokens": 29527,
+  "cost_usd": 3.3149900000000003,
+  "exit_code": 0,
+  "execution_dir": ".ddx/executions/20260424T005459-04bf1b1b",
+  "prompt_file": ".ddx/executions/20260424T005459-04bf1b1b/prompt.md",
+  "manifest_file": ".ddx/executions/20260424T005459-04bf1b1b/manifest.json",
+  "result_file": ".ddx/executions/20260424T005459-04bf1b1b/result.json",
+  "usage_file": ".ddx/executions/20260424T005459-04bf1b1b/usage.json",
+  "started_at": "2026-04-24T00:55:03.612023997Z",
+  "finished_at": "2026-04-24T01:04:25.304203876Z"
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
## Review: heimq-f1e42e32 iter 1

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
