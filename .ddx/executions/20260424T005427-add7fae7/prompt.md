<bead-review>
  <bead id="heimq-65cdaf14" iter=1>
    <title>e2e: consumer resume from committed offset after restart</title>
    <description>
Produce N messages; consumer A in group G commits offset mid-stream, then drops; a new consumer B joins group G with the same group.id, auto.offset.reset=latest (so reset is not the explanation), and should consume only the uncommitted tail. Distinct from test_rdkafka_consumer_group_manual_offset_fetch which tests the fetch API, not resume semantics.
In-scope files:
- tests/integration.rs (new fn test_rdkafka_group_resume_from_committed)
    </description>
    <acceptance>
`cargo test --test integration test_rdkafka_group_resume_from_committed -- --ignored --nocapture` passes and asserts the second consumer sees only the tail after the commit point.
    </acceptance>
    <labels>area:tests, kind:test, layer:e2e, client:rdkafka, phase:5, tier:p0, topic:group-correctness</labels>
  </bead>

  <governing>
    <note>No governing documents found. Evaluate the diff against the acceptance criteria alone.</note>
  </governing>

  <diff rev="bac3173d00801f080198a8c2750458c7ce519c97">
commit bac3173d00801f080198a8c2750458c7ce519c97
Author: ddx-land-coordinator <coordinator@ddx.local>
Date:   Thu Apr 23 20:54:23 2026 -0400

    chore: add execution evidence [20260424T005125-]

diff --git a/.ddx/executions/20260424T005125-b9e66757/manifest.json b/.ddx/executions/20260424T005125-b9e66757/manifest.json
new file mode 100644
index 0000000..ccc3ac8
--- /dev/null
+++ b/.ddx/executions/20260424T005125-b9e66757/manifest.json
@@ -0,0 +1,42 @@
+{
+  "attempt_id": "20260424T005125-b9e66757",
+  "bead_id": "heimq-65cdaf14",
+  "base_rev": "504cbb16e343aaabc3949eb259867c4b8e5e9161",
+  "created_at": "2026-04-24T00:51:30.069280219Z",
+  "requested": {
+    "harness": "claude",
+    "prompt": "synthesized"
+  },
+  "bead": {
+    "id": "heimq-65cdaf14",
+    "title": "e2e: consumer resume from committed offset after restart",
+    "description": "Produce N messages; consumer A in group G commits offset mid-stream, then drops; a new consumer B joins group G with the same group.id, auto.offset.reset=latest (so reset is not the explanation), and should consume only the uncommitted tail. Distinct from test_rdkafka_consumer_group_manual_offset_fetch which tests the fetch API, not resume semantics.\nIn-scope files:\n- tests/integration.rs (new fn test_rdkafka_group_resume_from_committed)",
+    "acceptance": "`cargo test --test integration test_rdkafka_group_resume_from_committed -- --ignored --nocapture` passes and asserts the second consumer sees only the tail after the commit point.",
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
+      "claimed-at": "2026-04-24T00:51:25Z",
+      "claimed-machine": "eitri",
+      "claimed-pid": "196235",
+      "execute-loop-heartbeat-at": "2026-04-24T00:51:25.868172561Z",
+      "spec-id": "API-001"
+    }
+  },
+  "paths": {
+    "dir": ".ddx/executions/20260424T005125-b9e66757",
+    "prompt": ".ddx/executions/20260424T005125-b9e66757/prompt.md",
+    "manifest": ".ddx/executions/20260424T005125-b9e66757/manifest.json",
+    "result": ".ddx/executions/20260424T005125-b9e66757/result.json",
+    "checks": ".ddx/executions/20260424T005125-b9e66757/checks.json",
+    "usage": ".ddx/executions/20260424T005125-b9e66757/usage.json",
+    "worktree": "tmp/ddx-exec-wt/.execute-bead-wt-heimq-65cdaf14-20260424T005125-b9e66757"
+  }
+}
\ No newline at end of file
diff --git a/.ddx/executions/20260424T005125-b9e66757/result.json b/.ddx/executions/20260424T005125-b9e66757/result.json
new file mode 100644
index 0000000..d295eba
--- /dev/null
+++ b/.ddx/executions/20260424T005125-b9e66757/result.json
@@ -0,0 +1,22 @@
+{
+  "bead_id": "heimq-65cdaf14",
+  "attempt_id": "20260424T005125-b9e66757",
+  "base_rev": "504cbb16e343aaabc3949eb259867c4b8e5e9161",
+  "result_rev": "0cb1e5b821748a8c40e4eb5e3e4c052a33bd0b17",
+  "outcome": "task_succeeded",
+  "status": "success",
+  "detail": "success",
+  "harness": "claude",
+  "session_id": "eb-056cba74",
+  "duration_ms": 162809,
+  "tokens": 10327,
+  "cost_usd": 0.9230125000000001,
+  "exit_code": 0,
+  "execution_dir": ".ddx/executions/20260424T005125-b9e66757",
+  "prompt_file": ".ddx/executions/20260424T005125-b9e66757/prompt.md",
+  "manifest_file": ".ddx/executions/20260424T005125-b9e66757/manifest.json",
+  "result_file": ".ddx/executions/20260424T005125-b9e66757/result.json",
+  "usage_file": ".ddx/executions/20260424T005125-b9e66757/usage.json",
+  "started_at": "2026-04-24T00:51:30.069565719Z",
+  "finished_at": "2026-04-24T00:54:12.879207575Z"
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
## Review: heimq-65cdaf14 iter 1

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
