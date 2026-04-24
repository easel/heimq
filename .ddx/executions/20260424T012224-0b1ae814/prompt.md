<bead-review>
  <bead id="heimq-6f69c0d3" iter=1>
    <title>e2e: per-partition ordering under concurrent produce</title>
    <description>
Multiple producers writing concurrently to a multi-partition topic. Each producer tags messages with a monotonic sequence per (producer_id, key). Consumer reads all partitions. For each partition, assert the consumed offsets are strictly increasing and that per-key sequences are monotonic (Kafka only guarantees order within a partition, not across keys/partitions).
In-scope files:
- tests/integration.rs (new fn test_rdkafka_per_partition_ordering)
    </description>
    <acceptance>
`cargo test --test integration test_rdkafka_per_partition_ordering -- --ignored --nocapture` passes and explicitly asserts monotonic offsets per partition.
    </acceptance>
    <labels>area:tests, kind:test, layer:e2e, client:rdkafka, phase:5, tier:p0, topic:multi-partition</labels>
  </bead>

  <governing>
    <note>No governing documents found. Evaluate the diff against the acceptance criteria alone.</note>
  </governing>

  <diff rev="d47c076c24b952c682b9068f1895859fa1ab93c6">
commit d47c076c24b952c682b9068f1895859fa1ab93c6
Author: ddx-land-coordinator <coordinator@ddx.local>
Date:   Thu Apr 23 21:22:24 2026 -0400

    chore: add execution evidence [20260424T011955-]

diff --git a/.ddx/executions/20260424T011955-7db92b00/manifest.json b/.ddx/executions/20260424T011955-7db92b00/manifest.json
new file mode 100644
index 0000000..44c4512
--- /dev/null
+++ b/.ddx/executions/20260424T011955-7db92b00/manifest.json
@@ -0,0 +1,42 @@
+{
+  "attempt_id": "20260424T011955-7db92b00",
+  "bead_id": "heimq-6f69c0d3",
+  "base_rev": "9e850343c6ad1a2ad10275eb1a699a32c826dcb7",
+  "created_at": "2026-04-24T01:19:59.388595634Z",
+  "requested": {
+    "harness": "claude",
+    "prompt": "synthesized"
+  },
+  "bead": {
+    "id": "heimq-6f69c0d3",
+    "title": "e2e: per-partition ordering under concurrent produce",
+    "description": "Multiple producers writing concurrently to a multi-partition topic. Each producer tags messages with a monotonic sequence per (producer_id, key). Consumer reads all partitions. For each partition, assert the consumed offsets are strictly increasing and that per-key sequences are monotonic (Kafka only guarantees order within a partition, not across keys/partitions).\nIn-scope files:\n- tests/integration.rs (new fn test_rdkafka_per_partition_ordering)",
+    "acceptance": "`cargo test --test integration test_rdkafka_per_partition_ordering -- --ignored --nocapture` passes and explicitly asserts monotonic offsets per partition.",
+    "parent": "heimq-700e47b2",
+    "labels": [
+      "area:tests",
+      "kind:test",
+      "layer:e2e",
+      "client:rdkafka",
+      "phase:5",
+      "tier:p0",
+      "topic:multi-partition"
+    ],
+    "metadata": {
+      "claimed-at": "2026-04-24T01:19:55Z",
+      "claimed-machine": "eitri",
+      "claimed-pid": "196235",
+      "execute-loop-heartbeat-at": "2026-04-24T01:19:55.498641551Z",
+      "spec-id": "API-001"
+    }
+  },
+  "paths": {
+    "dir": ".ddx/executions/20260424T011955-7db92b00",
+    "prompt": ".ddx/executions/20260424T011955-7db92b00/prompt.md",
+    "manifest": ".ddx/executions/20260424T011955-7db92b00/manifest.json",
+    "result": ".ddx/executions/20260424T011955-7db92b00/result.json",
+    "checks": ".ddx/executions/20260424T011955-7db92b00/checks.json",
+    "usage": ".ddx/executions/20260424T011955-7db92b00/usage.json",
+    "worktree": "tmp/ddx-exec-wt/.execute-bead-wt-heimq-6f69c0d3-20260424T011955-7db92b00"
+  }
+}
\ No newline at end of file
diff --git a/.ddx/executions/20260424T011955-7db92b00/result.json b/.ddx/executions/20260424T011955-7db92b00/result.json
new file mode 100644
index 0000000..50c48cf
--- /dev/null
+++ b/.ddx/executions/20260424T011955-7db92b00/result.json
@@ -0,0 +1,22 @@
+{
+  "bead_id": "heimq-6f69c0d3",
+  "attempt_id": "20260424T011955-7db92b00",
+  "base_rev": "9e850343c6ad1a2ad10275eb1a699a32c826dcb7",
+  "result_rev": "4a001dda43314559678c008a86cec5254cf4fc68",
+  "outcome": "task_succeeded",
+  "status": "success",
+  "detail": "success",
+  "harness": "claude",
+  "session_id": "eb-92a2cc02",
+  "duration_ms": 134919,
+  "tokens": 7511,
+  "cost_usd": 0.7302117500000002,
+  "exit_code": 0,
+  "execution_dir": ".ddx/executions/20260424T011955-7db92b00",
+  "prompt_file": ".ddx/executions/20260424T011955-7db92b00/prompt.md",
+  "manifest_file": ".ddx/executions/20260424T011955-7db92b00/manifest.json",
+  "result_file": ".ddx/executions/20260424T011955-7db92b00/result.json",
+  "usage_file": ".ddx/executions/20260424T011955-7db92b00/usage.json",
+  "started_at": "2026-04-24T01:19:59.388832594Z",
+  "finished_at": "2026-04-24T01:22:14.30834904Z"
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
## Review: heimq-6f69c0d3 iter 1

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
