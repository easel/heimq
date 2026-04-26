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

  <diff rev="23eafba4c437887435538797770449a3b3f3c497">
commit 23eafba4c437887435538797770449a3b3f3c497
Author: ddx-land-coordinator <coordinator@ddx.local>
Date:   Thu Apr 23 22:07:33 2026 -0400

    chore: add execution evidence [20260424T020456-]

diff --git a/.ddx/executions/20260424T020456-be39bf0c/manifest.json b/.ddx/executions/20260424T020456-be39bf0c/manifest.json
new file mode 100644
index 0000000..ccf40b7
--- /dev/null
+++ b/.ddx/executions/20260424T020456-be39bf0c/manifest.json
@@ -0,0 +1,38 @@
+{
+  "attempt_id": "20260424T020456-be39bf0c",
+  "bead_id": "heimq-04def9c7",
+  "base_rev": "d40f24df8c9662f9a613bdb796a3e6c3543e2d6c",
+  "created_at": "2026-04-24T02:05:11.989307108Z",
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
+      "claimed-at": "2026-04-24T02:04:56Z",
+      "claimed-machine": "eitri",
+      "claimed-pid": "196235",
+      "execute-loop-heartbeat-at": "2026-04-24T02:04:56.112017159Z"
+    }
+  },
+  "paths": {
+    "dir": ".ddx/executions/20260424T020456-be39bf0c",
+    "prompt": ".ddx/executions/20260424T020456-be39bf0c/prompt.md",
+    "manifest": ".ddx/executions/20260424T020456-be39bf0c/manifest.json",
+    "result": ".ddx/executions/20260424T020456-be39bf0c/result.json",
+    "checks": ".ddx/executions/20260424T020456-be39bf0c/checks.json",
+    "usage": ".ddx/executions/20260424T020456-be39bf0c/usage.json",
+    "worktree": "tmp/ddx-exec-wt/.execute-bead-wt-heimq-04def9c7-20260424T020456-be39bf0c"
+  }
+}
\ No newline at end of file
diff --git a/.ddx/executions/20260424T020456-be39bf0c/result.json b/.ddx/executions/20260424T020456-be39bf0c/result.json
new file mode 100644
index 0000000..cf275fd
--- /dev/null
+++ b/.ddx/executions/20260424T020456-be39bf0c/result.json
@@ -0,0 +1,22 @@
+{
+  "bead_id": "heimq-04def9c7",
+  "attempt_id": "20260424T020456-be39bf0c",
+  "base_rev": "d40f24df8c9662f9a613bdb796a3e6c3543e2d6c",
+  "result_rev": "9362ca39ad456c93e84d87a70d9d4a4c38153cc5",
+  "outcome": "task_succeeded",
+  "status": "success",
+  "detail": "success",
+  "harness": "claude",
+  "session_id": "eb-6fae48d6",
+  "duration_ms": 112035,
+  "tokens": 4781,
+  "cost_usd": 0.609821,
+  "exit_code": 0,
+  "execution_dir": ".ddx/executions/20260424T020456-be39bf0c",
+  "prompt_file": ".ddx/executions/20260424T020456-be39bf0c/prompt.md",
+  "manifest_file": ".ddx/executions/20260424T020456-be39bf0c/manifest.json",
+  "result_file": ".ddx/executions/20260424T020456-be39bf0c/result.json",
+  "usage_file": ".ddx/executions/20260424T020456-be39bf0c/usage.json",
+  "started_at": "2026-04-24T02:05:11.989557983Z",
+  "finished_at": "2026-04-24T02:07:04.025455301Z"
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
