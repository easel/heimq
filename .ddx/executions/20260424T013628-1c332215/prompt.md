<bead-review>
  <bead id="heimq-4ac4601c" iter=1>
    <title>Introduce RecordBatchView abstraction for structured log writes</title>
    <description>
The log trait needs to accept parsed batch metadata so backends can implement retention by timestamp, compaction by key, header-aware filtering, etc. Today produce handler (src/handler/produce.rs) and fetch handler (src/handler/fetch.rs) pass raw record-batch bytes through to Storage::append.

Add a RecordBatchView type that carries: producer_id, producer_epoch, base_offset (placeholder), base_timestamp, max_timestamp, is_transactional, is_control, compression codec, record count, borrowed iterator over records (offset_delta, timestamp_delta, key, value, headers), and the original &amp;[u8] for fast-path pass-through.

No decoding work beyond what produce handler already does via kafka_protocol::records — we wrap that result.

In-scope files:
- src/storage/record_batch_view.rs (new)
- src/storage/mod.rs (re-export)
Out of scope:
- Calling it from handler/produce.rs (separate bead)
- Changing storage API (separate bead)
    </description>
    <acceptance>
`cargo test --lib storage::record_batch_view` passes with tests that construct a view from a kafka_protocol RecordBatch and iterate records. Existing tests unaffected.
    </acceptance>
    <labels>area:storage, kind:feat, phase:backends, step:2-batch-view</labels>
  </bead>

  <governing>
    <note>No governing documents found. Evaluate the diff against the acceptance criteria alone.</note>
  </governing>

  <diff rev="81895cd85c97746cdaea920705eb08926da2cd8e">
commit 81895cd85c97746cdaea920705eb08926da2cd8e
Author: ddx-land-coordinator <coordinator@ddx.local>
Date:   Thu Apr 23 21:36:21 2026 -0400

    chore: add execution evidence [20260424T013155-]

diff --git a/.ddx/executions/20260424T013155-e283543d/manifest.json b/.ddx/executions/20260424T013155-e283543d/manifest.json
new file mode 100644
index 0000000..af3fdd8
--- /dev/null
+++ b/.ddx/executions/20260424T013155-e283543d/manifest.json
@@ -0,0 +1,38 @@
+{
+  "attempt_id": "20260424T013155-e283543d",
+  "bead_id": "heimq-4ac4601c",
+  "base_rev": "b2232f418332cd743cc18a1e90fb347e6ff2d114",
+  "created_at": "2026-04-24T01:32:04.154988747Z",
+  "requested": {
+    "harness": "claude",
+    "prompt": "synthesized"
+  },
+  "bead": {
+    "id": "heimq-4ac4601c",
+    "title": "Introduce RecordBatchView abstraction for structured log writes",
+    "description": "The log trait needs to accept parsed batch metadata so backends can implement retention by timestamp, compaction by key, header-aware filtering, etc. Today produce handler (src/handler/produce.rs) and fetch handler (src/handler/fetch.rs) pass raw record-batch bytes through to Storage::append.\n\nAdd a RecordBatchView type that carries: producer_id, producer_epoch, base_offset (placeholder), base_timestamp, max_timestamp, is_transactional, is_control, compression codec, record count, borrowed iterator over records (offset_delta, timestamp_delta, key, value, headers), and the original \u0026[u8] for fast-path pass-through.\n\nNo decoding work beyond what produce handler already does via kafka_protocol::records — we wrap that result.\n\nIn-scope files:\n- src/storage/record_batch_view.rs (new)\n- src/storage/mod.rs (re-export)\nOut of scope:\n- Calling it from handler/produce.rs (separate bead)\n- Changing storage API (separate bead)",
+    "acceptance": "`cargo test --lib storage::record_batch_view` passes with tests that construct a view from a kafka_protocol RecordBatch and iterate records. Existing tests unaffected.",
+    "parent": "heimq-2136e295",
+    "labels": [
+      "area:storage",
+      "kind:feat",
+      "phase:backends",
+      "step:2-batch-view"
+    ],
+    "metadata": {
+      "claimed-at": "2026-04-24T01:31:55Z",
+      "claimed-machine": "eitri",
+      "claimed-pid": "196235",
+      "execute-loop-heartbeat-at": "2026-04-24T01:31:55.3735474Z"
+    }
+  },
+  "paths": {
+    "dir": ".ddx/executions/20260424T013155-e283543d",
+    "prompt": ".ddx/executions/20260424T013155-e283543d/prompt.md",
+    "manifest": ".ddx/executions/20260424T013155-e283543d/manifest.json",
+    "result": ".ddx/executions/20260424T013155-e283543d/result.json",
+    "checks": ".ddx/executions/20260424T013155-e283543d/checks.json",
+    "usage": ".ddx/executions/20260424T013155-e283543d/usage.json",
+    "worktree": "tmp/ddx-exec-wt/.execute-bead-wt-heimq-4ac4601c-20260424T013155-e283543d"
+  }
+}
\ No newline at end of file
diff --git a/.ddx/executions/20260424T013155-e283543d/result.json b/.ddx/executions/20260424T013155-e283543d/result.json
new file mode 100644
index 0000000..06e3ab2
--- /dev/null
+++ b/.ddx/executions/20260424T013155-e283543d/result.json
@@ -0,0 +1,22 @@
+{
+  "bead_id": "heimq-4ac4601c",
+  "attempt_id": "20260424T013155-e283543d",
+  "base_rev": "b2232f418332cd743cc18a1e90fb347e6ff2d114",
+  "result_rev": "dbb6f00b5a8b3e905fbe9b426e02f366003c9d5e",
+  "outcome": "task_succeeded",
+  "status": "success",
+  "detail": "success",
+  "harness": "claude",
+  "session_id": "eb-e99a4a0a",
+  "duration_ms": 248309,
+  "tokens": 15229,
+  "cost_usd": 1.7096347499999998,
+  "exit_code": 0,
+  "execution_dir": ".ddx/executions/20260424T013155-e283543d",
+  "prompt_file": ".ddx/executions/20260424T013155-e283543d/prompt.md",
+  "manifest_file": ".ddx/executions/20260424T013155-e283543d/manifest.json",
+  "result_file": ".ddx/executions/20260424T013155-e283543d/result.json",
+  "usage_file": ".ddx/executions/20260424T013155-e283543d/usage.json",
+  "started_at": "2026-04-24T01:32:04.155249747Z",
+  "finished_at": "2026-04-24T01:36:12.464910534Z"
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
## Review: heimq-4ac4601c iter 1

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
