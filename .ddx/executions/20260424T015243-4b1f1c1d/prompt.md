<bead-review>
  <bead id="heimq-20daf864" iter=1>
    <title>Extract LogBackend + TopicLog + PartitionLog traits</title>
    <description>
Replace the concrete Storage struct with three traits:

- LogBackend: create_topic, delete_topic, list_topics, topic(name) -&gt; Arc&lt;dyn TopicLog&gt;, capabilities().
- TopicLog: name(), num_partitions(), partition(id) -&gt; Arc&lt;dyn PartitionLog&gt;, config() -&gt; &amp;TopicConfig.
- PartitionLog: append(RecordBatchView, raw_bytes: Option&lt;&amp;[u8]&gt;), read(offset, max_bytes, FetchWait), log_start_offset, high_watermark, truncate_before.

Rename existing Storage/Topic/Partition to MemoryLog/MemoryTopicLog/MemoryPartitionLog and have them implement the traits. Handlers receive Arc&lt;dyn LogBackend&gt; where they currently have Arc&lt;Storage&gt;. Zero behavior change expected.

FetchWait enum: Immediate | LongPoll { min_bytes, max_wait_ms }.

In-scope files:
- src/storage/mod.rs (trait defs + re-exports)
- src/storage/memory.rs (new — rename of current impl)
- src/storage/partition.rs, src/storage/segment.rs, src/storage/topic.rs (updated to implement traits)
- src/handler/*.rs (call-site changes where they hold Arc&lt;Storage&gt;)
- src/server.rs (holds Arc&lt;dyn LogBackend&gt;)
Out of scope:
- OffsetStore and GroupCoordinatorBackend (separate beads)
- URL dispatch (separate bead)
- Any non-memory backend (separate bead)
Depends on: RecordBatchView bead
    </description>
    <acceptance>
All existing tests pass unchanged: `cargo test --workspace` green. `cargo llvm-cov --workspace --all-features --fail-under-lines 100` still passes. `scripts/kcat-stress.sh` with default params still green.
    </acceptance>
    <labels>area:storage, kind:feat, phase:backends, step:3-log-trait</labels>
  </bead>

  <governing>
    <note>No governing documents found. Evaluate the diff against the acceptance criteria alone.</note>
  </governing>

  <diff rev="08ee7af9f664a9582a1b4f80ddd683aed667a549">
commit 08ee7af9f664a9582a1b4f80ddd683aed667a549
Author: ddx-land-coordinator <coordinator@ddx.local>
Date:   Thu Apr 23 21:52:32 2026 -0400

    chore: add execution evidence [20260424T013736-]

diff --git a/.ddx/executions/20260424T013736-cd061fd2/manifest.json b/.ddx/executions/20260424T013736-cd061fd2/manifest.json
new file mode 100644
index 0000000..c062eb6
--- /dev/null
+++ b/.ddx/executions/20260424T013736-cd061fd2/manifest.json
@@ -0,0 +1,38 @@
+{
+  "attempt_id": "20260424T013736-cd061fd2",
+  "bead_id": "heimq-20daf864",
+  "base_rev": "6de081f61a05d7543bba5aa0ee2f9ec9fd99ef05",
+  "created_at": "2026-04-24T01:37:45.573243187Z",
+  "requested": {
+    "harness": "claude",
+    "prompt": "synthesized"
+  },
+  "bead": {
+    "id": "heimq-20daf864",
+    "title": "Extract LogBackend + TopicLog + PartitionLog traits",
+    "description": "Replace the concrete Storage struct with three traits:\n\n- LogBackend: create_topic, delete_topic, list_topics, topic(name) -\u003e Arc\u003cdyn TopicLog\u003e, capabilities().\n- TopicLog: name(), num_partitions(), partition(id) -\u003e Arc\u003cdyn PartitionLog\u003e, config() -\u003e \u0026TopicConfig.\n- PartitionLog: append(RecordBatchView, raw_bytes: Option\u003c\u0026[u8]\u003e), read(offset, max_bytes, FetchWait), log_start_offset, high_watermark, truncate_before.\n\nRename existing Storage/Topic/Partition to MemoryLog/MemoryTopicLog/MemoryPartitionLog and have them implement the traits. Handlers receive Arc\u003cdyn LogBackend\u003e where they currently have Arc\u003cStorage\u003e. Zero behavior change expected.\n\nFetchWait enum: Immediate | LongPoll { min_bytes, max_wait_ms }.\n\nIn-scope files:\n- src/storage/mod.rs (trait defs + re-exports)\n- src/storage/memory.rs (new — rename of current impl)\n- src/storage/partition.rs, src/storage/segment.rs, src/storage/topic.rs (updated to implement traits)\n- src/handler/*.rs (call-site changes where they hold Arc\u003cStorage\u003e)\n- src/server.rs (holds Arc\u003cdyn LogBackend\u003e)\nOut of scope:\n- OffsetStore and GroupCoordinatorBackend (separate beads)\n- URL dispatch (separate bead)\n- Any non-memory backend (separate bead)\nDepends on: RecordBatchView bead",
+    "acceptance": "All existing tests pass unchanged: `cargo test --workspace` green. `cargo llvm-cov --workspace --all-features --fail-under-lines 100` still passes. `scripts/kcat-stress.sh` with default params still green.",
+    "parent": "heimq-2136e295",
+    "labels": [
+      "area:storage",
+      "kind:feat",
+      "phase:backends",
+      "step:3-log-trait"
+    ],
+    "metadata": {
+      "claimed-at": "2026-04-24T01:37:36Z",
+      "claimed-machine": "eitri",
+      "claimed-pid": "196235",
+      "execute-loop-heartbeat-at": "2026-04-24T01:37:36.591362041Z"
+    }
+  },
+  "paths": {
+    "dir": ".ddx/executions/20260424T013736-cd061fd2",
+    "prompt": ".ddx/executions/20260424T013736-cd061fd2/prompt.md",
+    "manifest": ".ddx/executions/20260424T013736-cd061fd2/manifest.json",
+    "result": ".ddx/executions/20260424T013736-cd061fd2/result.json",
+    "checks": ".ddx/executions/20260424T013736-cd061fd2/checks.json",
+    "usage": ".ddx/executions/20260424T013736-cd061fd2/usage.json",
+    "worktree": "tmp/ddx-exec-wt/.execute-bead-wt-heimq-20daf864-20260424T013736-cd061fd2"
+  }
+}
\ No newline at end of file
diff --git a/.ddx/executions/20260424T013736-cd061fd2/result.json b/.ddx/executions/20260424T013736-cd061fd2/result.json
new file mode 100644
index 0000000..8ccbd0e
--- /dev/null
+++ b/.ddx/executions/20260424T013736-cd061fd2/result.json
@@ -0,0 +1,22 @@
+{
+  "bead_id": "heimq-20daf864",
+  "attempt_id": "20260424T013736-cd061fd2",
+  "base_rev": "6de081f61a05d7543bba5aa0ee2f9ec9fd99ef05",
+  "result_rev": "2f9e9d8dec12fab5a1557fbc3c9c322115d59a13",
+  "outcome": "task_succeeded",
+  "status": "success",
+  "detail": "success",
+  "harness": "claude",
+  "session_id": "eb-16919dfc",
+  "duration_ms": 873421,
+  "tokens": 47958,
+  "cost_usd": 9.587653000000001,
+  "exit_code": 0,
+  "execution_dir": ".ddx/executions/20260424T013736-cd061fd2",
+  "prompt_file": ".ddx/executions/20260424T013736-cd061fd2/prompt.md",
+  "manifest_file": ".ddx/executions/20260424T013736-cd061fd2/manifest.json",
+  "result_file": ".ddx/executions/20260424T013736-cd061fd2/result.json",
+  "usage_file": ".ddx/executions/20260424T013736-cd061fd2/usage.json",
+  "started_at": "2026-04-24T01:37:45.573474812Z",
+  "finished_at": "2026-04-24T01:52:18.994480193Z"
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
## Review: heimq-20daf864 iter 1

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
