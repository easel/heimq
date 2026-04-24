<bead-review>
  <bead id="heimq-04def9c7" iter=1>
    <title>Extract OffsetStore trait</title>
    <description>
Commit offsets are today stored inside ConsumerGroupManager state. Extract an OffsetStore trait:

- commit(group, topic, partition, offset, metadata)
- fetch(group, topic, partition) -&gt; Option&lt;CommittedOffset&gt;
- delete_group(group)
- capabilities()

Current implementation becomes MemoryOffsetStore. OffsetCommit/OffsetFetch handlers take Arc&lt;dyn OffsetStore&gt;.

In-scope files:
- src/storage/offset_store.rs (new trait + memory impl)
- src/consumer_group/offset_store.rs (refactor: concrete struct implements new trait)
- src/handler/offset_commit.rs, src/handler/offset_fetch.rs (take Arc&lt;dyn OffsetStore&gt;)
- src/server.rs (construction)
Out of scope:
- Postgres impl (separate bead)
- GroupCoordinatorBackend (separate bead)
    </description>
    <acceptance>
`cargo test --workspace` passes. kcat stress script passes. Offset commit/fetch unit tests remain green.
    </acceptance>
    <labels>area:storage, kind:feat, phase:backends, step:4-offset-trait</labels>
  </bead>

  <governing>
    <note>No governing documents found. Evaluate the diff against the acceptance criteria alone.</note>
  </governing>

  <diff rev="af91382d93047e937eb05f43f89dc7e10e6a5246">
commit af91382d93047e937eb05f43f89dc7e10e6a5246
Author: ddx-land-coordinator <coordinator@ddx.local>
Date:   Thu Apr 23 22:02:42 2026 -0400

    chore: add execution evidence [20260424T015506-]

diff --git a/.ddx/executions/20260424T015506-3887069f/manifest.json b/.ddx/executions/20260424T015506-3887069f/manifest.json
new file mode 100644
index 00000000..791effae
--- /dev/null
+++ b/.ddx/executions/20260424T015506-3887069f/manifest.json
@@ -0,0 +1,38 @@
+{
+  "attempt_id": "20260424T015506-3887069f",
+  "bead_id": "heimq-04def9c7",
+  "base_rev": "fe2f56035b490ad0f0330051d49d0c7093e29703",
+  "created_at": "2026-04-24T01:55:22.735419987Z",
+  "requested": {
+    "harness": "claude",
+    "prompt": "synthesized"
+  },
+  "bead": {
+    "id": "heimq-04def9c7",
+    "title": "Extract OffsetStore trait",
+    "description": "Commit offsets are today stored inside ConsumerGroupManager state. Extract an OffsetStore trait:\n\n- commit(group, topic, partition, offset, metadata)\n- fetch(group, topic, partition) -\u003e Option\u003cCommittedOffset\u003e\n- delete_group(group)\n- capabilities()\n\nCurrent implementation becomes MemoryOffsetStore. OffsetCommit/OffsetFetch handlers take Arc\u003cdyn OffsetStore\u003e.\n\nIn-scope files:\n- src/storage/offset_store.rs (new trait + memory impl)\n- src/consumer_group/offset_store.rs (refactor: concrete struct implements new trait)\n- src/handler/offset_commit.rs, src/handler/offset_fetch.rs (take Arc\u003cdyn OffsetStore\u003e)\n- src/server.rs (construction)\nOut of scope:\n- Postgres impl (separate bead)\n- GroupCoordinatorBackend (separate bead)",
+    "acceptance": "`cargo test --workspace` passes. kcat stress script passes. Offset commit/fetch unit tests remain green.",
+    "parent": "heimq-2136e295",
+    "labels": [
+      "area:storage",
+      "kind:feat",
+      "phase:backends",
+      "step:4-offset-trait"
+    ],
+    "metadata": {
+      "claimed-at": "2026-04-24T01:55:06Z",
+      "claimed-machine": "eitri",
+      "claimed-pid": "196235",
+      "execute-loop-heartbeat-at": "2026-04-24T01:55:06.013751852Z"
+    }
+  },
+  "paths": {
+    "dir": ".ddx/executions/20260424T015506-3887069f",
+    "prompt": ".ddx/executions/20260424T015506-3887069f/prompt.md",
+    "manifest": ".ddx/executions/20260424T015506-3887069f/manifest.json",
+    "result": ".ddx/executions/20260424T015506-3887069f/result.json",
+    "checks": ".ddx/executions/20260424T015506-3887069f/checks.json",
+    "usage": ".ddx/executions/20260424T015506-3887069f/usage.json",
+    "worktree": "tmp/ddx-exec-wt/.execute-bead-wt-heimq-04def9c7-20260424T015506-3887069f"
+  }
+}
\ No newline at end of file
diff --git a/.ddx/executions/20260424T015506-3887069f/result.json b/.ddx/executions/20260424T015506-3887069f/result.json
new file mode 100644
index 00000000..8d53d9ef
--- /dev/null
+++ b/.ddx/executions/20260424T015506-3887069f/result.json
@@ -0,0 +1,22 @@
+{
+  "bead_id": "heimq-04def9c7",
+  "attempt_id": "20260424T015506-3887069f",
+  "base_rev": "fe2f56035b490ad0f0330051d49d0c7093e29703",
+  "result_rev": "71b6d985a9dfad0388429bbf84cfeff23b8ddfe6",
+  "outcome": "task_succeeded",
+  "status": "success",
+  "detail": "success",
+  "harness": "claude",
+  "session_id": "eb-c3c7847b",
+  "duration_ms": 427355,
+  "tokens": 22527,
+  "cost_usd": 3.3405905,
+  "exit_code": 0,
+  "execution_dir": ".ddx/executions/20260424T015506-3887069f",
+  "prompt_file": ".ddx/executions/20260424T015506-3887069f/prompt.md",
+  "manifest_file": ".ddx/executions/20260424T015506-3887069f/manifest.json",
+  "result_file": ".ddx/executions/20260424T015506-3887069f/result.json",
+  "usage_file": ".ddx/executions/20260424T015506-3887069f/usage.json",
+  "started_at": "2026-04-24T01:55:22.735714695Z",
+  "finished_at": "2026-04-24T02:02:30.09163751Z"
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
## Review: heimq-04def9c7 iter 1

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
