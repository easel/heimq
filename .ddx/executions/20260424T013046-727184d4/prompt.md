<bead-review>
  <bead id="heimq-9e9f257c" iter=1>
    <title>Add BackendCapabilities descriptor struct</title>
    <description>
Introduce a BackendCapabilities struct exposing what a backend can do: durability (None|Snapshot|WalFsync), atomic_append scope (Partition|Topic|Cluster), survives_restart, compaction, transactions, idempotent_producer, timestamps support, headers, compression codecs, max message/batch/partitions, fetch_wait support, read_your_writes, retention modes, truncate, name, version. Also related enums.

At this step the struct is used by nothing — purely additive — but its shape is the contract future backends will implement. Struct lives in src/storage/capabilities.rs; re-exported from src/storage/mod.rs.

In-scope files:
- src/storage/capabilities.rs (new)
- src/storage/mod.rs (re-export)
Out of scope:
- Wiring into ApiVersions (separate bead)
- Implementing the capability for the existing in-memory storage (separate bead after trait extraction)
    </description>
    <acceptance>
`cargo test --lib storage::capabilities` passes with a basic test constructing a BackendCapabilities and asserting field defaults. `cargo build --release` passes.
    </acceptance>
    <labels>area:storage, kind:feat, phase:backends, step:1-capabilities</labels>
  </bead>

  <governing>
    <note>No governing documents found. Evaluate the diff against the acceptance criteria alone.</note>
  </governing>

  <diff rev="907d90cb0c10905742178acfdf13b703edddd127">
commit 907d90cb0c10905742178acfdf13b703edddd127
Author: ddx-land-coordinator <coordinator@ddx.local>
Date:   Thu Apr 23 21:30:45 2026 -0400

    chore: add execution evidence [20260424T012836-]

diff --git a/.ddx/executions/20260424T012836-8274d170/manifest.json b/.ddx/executions/20260424T012836-8274d170/manifest.json
new file mode 100644
index 0000000..1737890
--- /dev/null
+++ b/.ddx/executions/20260424T012836-8274d170/manifest.json
@@ -0,0 +1,38 @@
+{
+  "attempt_id": "20260424T012836-8274d170",
+  "bead_id": "heimq-9e9f257c",
+  "base_rev": "00aa32492717751ab664a099f79b39864b82f942",
+  "created_at": "2026-04-24T01:28:40.779253662Z",
+  "requested": {
+    "harness": "claude",
+    "prompt": "synthesized"
+  },
+  "bead": {
+    "id": "heimq-9e9f257c",
+    "title": "Add BackendCapabilities descriptor struct",
+    "description": "Introduce a BackendCapabilities struct exposing what a backend can do: durability (None|Snapshot|WalFsync), atomic_append scope (Partition|Topic|Cluster), survives_restart, compaction, transactions, idempotent_producer, timestamps support, headers, compression codecs, max message/batch/partitions, fetch_wait support, read_your_writes, retention modes, truncate, name, version. Also related enums.\n\nAt this step the struct is used by nothing — purely additive — but its shape is the contract future backends will implement. Struct lives in src/storage/capabilities.rs; re-exported from src/storage/mod.rs.\n\nIn-scope files:\n- src/storage/capabilities.rs (new)\n- src/storage/mod.rs (re-export)\nOut of scope:\n- Wiring into ApiVersions (separate bead)\n- Implementing the capability for the existing in-memory storage (separate bead after trait extraction)",
+    "acceptance": "`cargo test --lib storage::capabilities` passes with a basic test constructing a BackendCapabilities and asserting field defaults. `cargo build --release` passes.",
+    "parent": "heimq-2136e295",
+    "labels": [
+      "area:storage",
+      "kind:feat",
+      "phase:backends",
+      "step:1-capabilities"
+    ],
+    "metadata": {
+      "claimed-at": "2026-04-24T01:28:36Z",
+      "claimed-machine": "eitri",
+      "claimed-pid": "196235",
+      "execute-loop-heartbeat-at": "2026-04-24T01:28:36.753379104Z"
+    }
+  },
+  "paths": {
+    "dir": ".ddx/executions/20260424T012836-8274d170",
+    "prompt": ".ddx/executions/20260424T012836-8274d170/prompt.md",
+    "manifest": ".ddx/executions/20260424T012836-8274d170/manifest.json",
+    "result": ".ddx/executions/20260424T012836-8274d170/result.json",
+    "checks": ".ddx/executions/20260424T012836-8274d170/checks.json",
+    "usage": ".ddx/executions/20260424T012836-8274d170/usage.json",
+    "worktree": "tmp/ddx-exec-wt/.execute-bead-wt-heimq-9e9f257c-20260424T012836-8274d170"
+  }
+}
\ No newline at end of file
diff --git a/.ddx/executions/20260424T012836-8274d170/result.json b/.ddx/executions/20260424T012836-8274d170/result.json
new file mode 100644
index 0000000..3833527
--- /dev/null
+++ b/.ddx/executions/20260424T012836-8274d170/result.json
@@ -0,0 +1,22 @@
+{
+  "bead_id": "heimq-9e9f257c",
+  "attempt_id": "20260424T012836-8274d170",
+  "base_rev": "00aa32492717751ab664a099f79b39864b82f942",
+  "result_rev": "be04ca9e8f7736cae9d57916be525bb071a038ad",
+  "outcome": "task_succeeded",
+  "status": "success",
+  "detail": "success",
+  "harness": "claude",
+  "session_id": "eb-dd2079a6",
+  "duration_ms": 115402,
+  "tokens": 5004,
+  "cost_usd": 0.46739375,
+  "exit_code": 0,
+  "execution_dir": ".ddx/executions/20260424T012836-8274d170",
+  "prompt_file": ".ddx/executions/20260424T012836-8274d170/prompt.md",
+  "manifest_file": ".ddx/executions/20260424T012836-8274d170/manifest.json",
+  "result_file": ".ddx/executions/20260424T012836-8274d170/result.json",
+  "usage_file": ".ddx/executions/20260424T012836-8274d170/usage.json",
+  "started_at": "2026-04-24T01:28:40.77949212Z",
+  "finished_at": "2026-04-24T01:30:36.182291063Z"
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
## Review: heimq-9e9f257c iter 1

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
