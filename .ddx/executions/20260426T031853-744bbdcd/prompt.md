<bead-review>
  <bead id="heimq-ed70c613" iter=1>
    <title>URL-scheme config + runtime dispatch for storage backends</title>
    <description>
Add a [storage] config block with three URL fields (log, offsets, groups), all defaulting to memory://. Introduce a dispatch fn per trait that parses the URL scheme and returns Arc&lt;dyn ...&gt;. Default config produces three memory backends; existing behavior unchanged.

Only the memory:// scheme resolves at this step. Unknown schemes return a configuration error at startup (fail fast, not at first request).

In-scope files:
- src/config.rs (new StorageConfig struct, feeding three URLs)
- src/storage/dispatch.rs (new — scheme -&gt; Arc&lt;dyn LogBackend&gt;, OffsetStore, GroupCoordinatorBackend)
- src/server.rs (construct backends via dispatch)
- tests/config tests
Out of scope:
- Postgres or any non-memory scheme (separate bead)
Depends on: LogBackend + OffsetStore + GroupCoordinatorBackend traits (beads 3, 4, 5)
    </description>
    <acceptance>
`cargo test` green. Starting heimq with default config uses three memory backends. Starting with an unknown scheme (e.g. `log = "weird://"`) fails at startup with a clear error. kcat stress passes.
    </acceptance>
    <labels>area:storage, kind:feat, phase:backends, step:6-url-dispatch</labels>
  </bead>

  <changed-files>
    <file>.ddx/executions/20260426T031251-426aca26/manifest.json</file>
    <file>.ddx/executions/20260426T031251-426aca26/result.json</file>
  </changed-files>

  <governing>
    <note>No governing documents found. Evaluate the diff against the acceptance criteria alone.</note>
  </governing>

  <diff rev="250485398c6662fe50f8dcfb3b7e36416a5e80f6">
diff --git a/.ddx/executions/20260426T031251-426aca26/manifest.json b/.ddx/executions/20260426T031251-426aca26/manifest.json
new file mode 100644
index 0000000..52a859b
--- /dev/null
+++ b/.ddx/executions/20260426T031251-426aca26/manifest.json
@@ -0,0 +1,38 @@
+{
+  "attempt_id": "20260426T031251-426aca26",
+  "bead_id": "heimq-ed70c613",
+  "base_rev": "fbae963750d873fdb75e1235c5bbf34ae1715162",
+  "created_at": "2026-04-26T03:12:51.228851803Z",
+  "requested": {
+    "harness": "claude",
+    "prompt": "synthesized"
+  },
+  "bead": {
+    "id": "heimq-ed70c613",
+    "title": "URL-scheme config + runtime dispatch for storage backends",
+    "description": "Add a [storage] config block with three URL fields (log, offsets, groups), all defaulting to memory://. Introduce a dispatch fn per trait that parses the URL scheme and returns Arc\u003cdyn ...\u003e. Default config produces three memory backends; existing behavior unchanged.\n\nOnly the memory:// scheme resolves at this step. Unknown schemes return a configuration error at startup (fail fast, not at first request).\n\nIn-scope files:\n- src/config.rs (new StorageConfig struct, feeding three URLs)\n- src/storage/dispatch.rs (new — scheme -\u003e Arc\u003cdyn LogBackend\u003e, OffsetStore, GroupCoordinatorBackend)\n- src/server.rs (construct backends via dispatch)\n- tests/config tests\nOut of scope:\n- Postgres or any non-memory scheme (separate bead)\nDepends on: LogBackend + OffsetStore + GroupCoordinatorBackend traits (beads 3, 4, 5)",
+    "acceptance": "`cargo test` green. Starting heimq with default config uses three memory backends. Starting with an unknown scheme (e.g. `log = \"weird://\"`) fails at startup with a clear error. kcat stress passes.",
+    "parent": "heimq-2136e295",
+    "labels": [
+      "area:storage",
+      "kind:feat",
+      "phase:backends",
+      "step:6-url-dispatch"
+    ],
+    "metadata": {
+      "claimed-at": "2026-04-26T03:12:51Z",
+      "claimed-machine": "eitri",
+      "claimed-pid": "4075646",
+      "execute-loop-heartbeat-at": "2026-04-26T03:12:51.159280792Z"
+    }
+  },
+  "paths": {
+    "dir": ".ddx/executions/20260426T031251-426aca26",
+    "prompt": ".ddx/executions/20260426T031251-426aca26/prompt.md",
+    "manifest": ".ddx/executions/20260426T031251-426aca26/manifest.json",
+    "result": ".ddx/executions/20260426T031251-426aca26/result.json",
+    "checks": ".ddx/executions/20260426T031251-426aca26/checks.json",
+    "usage": ".ddx/executions/20260426T031251-426aca26/usage.json",
+    "worktree": "tmp/ddx-exec-wt/.execute-bead-wt-heimq-ed70c613-20260426T031251-426aca26"
+  }
+}
\ No newline at end of file
diff --git a/.ddx/executions/20260426T031251-426aca26/result.json b/.ddx/executions/20260426T031251-426aca26/result.json
new file mode 100644
index 0000000..7180a78
--- /dev/null
+++ b/.ddx/executions/20260426T031251-426aca26/result.json
@@ -0,0 +1,22 @@
+{
+  "bead_id": "heimq-ed70c613",
+  "attempt_id": "20260426T031251-426aca26",
+  "base_rev": "fbae963750d873fdb75e1235c5bbf34ae1715162",
+  "result_rev": "2c338a9a6c5fc7e97e6984a51a9d30f4760c7e8c",
+  "outcome": "task_succeeded",
+  "status": "success",
+  "detail": "success",
+  "harness": "claude",
+  "session_id": "eb-abe7ad5d",
+  "duration_ms": 360523,
+  "tokens": 21613,
+  "cost_usd": 3.1770337499999997,
+  "exit_code": 0,
+  "execution_dir": ".ddx/executions/20260426T031251-426aca26",
+  "prompt_file": ".ddx/executions/20260426T031251-426aca26/prompt.md",
+  "manifest_file": ".ddx/executions/20260426T031251-426aca26/manifest.json",
+  "result_file": ".ddx/executions/20260426T031251-426aca26/result.json",
+  "usage_file": ".ddx/executions/20260426T031251-426aca26/usage.json",
+  "started_at": "2026-04-26T03:12:51.229126344Z",
+  "finished_at": "2026-04-26T03:18:51.752659985Z"
+}
\ No newline at end of file
  </diff>

  <instructions>
You are reviewing a bead implementation against its acceptance criteria.

For each acceptance-criteria (AC) item, decide whether it is implemented correctly, then assign one overall verdict:

- APPROVE — every AC item is fully and correctly implemented.
- REQUEST_CHANGES — some AC items are partial or have fixable minor issues.
- BLOCK — at least one AC item is not implemented or incorrectly implemented; or the diff is insufficient to evaluate.

## Required output format (schema_version: 1)

Respond with EXACTLY one JSON object as your final response, fenced as a single ```json … ``` code block. Do not include any prose outside the fenced block. The JSON must match this schema:

```json
{
  "schema_version": 1,
  "verdict": "APPROVE",
  "summary": "≤300 char human-readable verdict justification",
  "findings": [
    { "severity": "info", "summary": "what is wrong or notable", "location": "path/to/file.go:42" }
  ]
}
```

Rules:
- "verdict" must be exactly one of "APPROVE", "REQUEST_CHANGES", "BLOCK".
- "severity" must be exactly one of "info", "warn", "block".
- Output the JSON object inside ONE fenced ```json … ``` block. No additional prose, no extra fences, no markdown headings.
- Do not echo this template back. Do not write the words APPROVE, REQUEST_CHANGES, or BLOCK anywhere except as the JSON value of the verdict field.
  </instructions>
</bead-review>
