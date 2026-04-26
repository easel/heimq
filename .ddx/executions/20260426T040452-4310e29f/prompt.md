<bead-review>
  <bead id="heimq-620861e8" iter=1>
    <title>Reject unsupported operations at handler entry</title>
    <description>
Complement startup ApiVersions filtering with per-call checks: if a client somehow sends a transactional marker or an oversized batch, the handler returns a proper Kafka error code rather than attempting the op and producing undefined behavior.

In-scope files:
- src/handler/produce.rs (size checks against caps.max_message_size/max_batch_size; transactional flag rejected if caps.transactions=false)
- src/handler/fetch.rs (long-poll falls back to Immediate if caps.fetch_wait doesn't include LongPoll)
- src/handler/*group*.rs (generation semantics check)
Depends on: capability descriptor + trait extractions
    </description>
    <acceptance>
`cargo test` green. New integration test: set a memory backend with max_message_size=1024, produce 2KB message, assert MessageSizeTooLarge error response.
    </acceptance>
    <labels>area:storage, kind:feat, phase:backends, step:8-call-time-rejection</labels>
  </bead>

  <changed-files>
    <file>.ddx/executions/20260426T040014-1acc49c3/manifest.json</file>
    <file>.ddx/executions/20260426T040014-1acc49c3/result.json</file>
  </changed-files>

  <governing>
    <note>No governing documents found. Evaluate the diff against the acceptance criteria alone.</note>
  </governing>

  <diff rev="9d24014778861d884c54fde8ff0e73082b18fb89">
diff --git a/.ddx/executions/20260426T040014-1acc49c3/manifest.json b/.ddx/executions/20260426T040014-1acc49c3/manifest.json
new file mode 100644
index 0000000..cc3b898
--- /dev/null
+++ b/.ddx/executions/20260426T040014-1acc49c3/manifest.json
@@ -0,0 +1,38 @@
+{
+  "attempt_id": "20260426T040014-1acc49c3",
+  "bead_id": "heimq-620861e8",
+  "base_rev": "c16548d5d8b17da19b25f361a87469b5d5e223e4",
+  "created_at": "2026-04-26T04:00:14.840581326Z",
+  "requested": {
+    "harness": "claude",
+    "prompt": "synthesized"
+  },
+  "bead": {
+    "id": "heimq-620861e8",
+    "title": "Reject unsupported operations at handler entry",
+    "description": "Complement startup ApiVersions filtering with per-call checks: if a client somehow sends a transactional marker or an oversized batch, the handler returns a proper Kafka error code rather than attempting the op and producing undefined behavior.\n\nIn-scope files:\n- src/handler/produce.rs (size checks against caps.max_message_size/max_batch_size; transactional flag rejected if caps.transactions=false)\n- src/handler/fetch.rs (long-poll falls back to Immediate if caps.fetch_wait doesn't include LongPoll)\n- src/handler/*group*.rs (generation semantics check)\nDepends on: capability descriptor + trait extractions",
+    "acceptance": "`cargo test` green. New integration test: set a memory backend with max_message_size=1024, produce 2KB message, assert MessageSizeTooLarge error response.",
+    "parent": "heimq-2136e295",
+    "labels": [
+      "area:storage",
+      "kind:feat",
+      "phase:backends",
+      "step:8-call-time-rejection"
+    ],
+    "metadata": {
+      "claimed-at": "2026-04-26T04:00:14Z",
+      "claimed-machine": "eitri",
+      "claimed-pid": "4075646",
+      "execute-loop-heartbeat-at": "2026-04-26T04:00:14.752303226Z"
+    }
+  },
+  "paths": {
+    "dir": ".ddx/executions/20260426T040014-1acc49c3",
+    "prompt": ".ddx/executions/20260426T040014-1acc49c3/prompt.md",
+    "manifest": ".ddx/executions/20260426T040014-1acc49c3/manifest.json",
+    "result": ".ddx/executions/20260426T040014-1acc49c3/result.json",
+    "checks": ".ddx/executions/20260426T040014-1acc49c3/checks.json",
+    "usage": ".ddx/executions/20260426T040014-1acc49c3/usage.json",
+    "worktree": "tmp/ddx-exec-wt/.execute-bead-wt-heimq-620861e8-20260426T040014-1acc49c3"
+  }
+}
\ No newline at end of file
diff --git a/.ddx/executions/20260426T040014-1acc49c3/result.json b/.ddx/executions/20260426T040014-1acc49c3/result.json
new file mode 100644
index 0000000..f83a7fa
--- /dev/null
+++ b/.ddx/executions/20260426T040014-1acc49c3/result.json
@@ -0,0 +1,22 @@
+{
+  "bead_id": "heimq-620861e8",
+  "attempt_id": "20260426T040014-1acc49c3",
+  "base_rev": "c16548d5d8b17da19b25f361a87469b5d5e223e4",
+  "result_rev": "a00914855d7eb644a51294d2d29e8103f4331ca6",
+  "outcome": "task_succeeded",
+  "status": "success",
+  "detail": "success",
+  "harness": "claude",
+  "session_id": "eb-9ab072d0",
+  "duration_ms": 276334,
+  "tokens": 13497,
+  "cost_usd": 2.5512942500000007,
+  "exit_code": 0,
+  "execution_dir": ".ddx/executions/20260426T040014-1acc49c3",
+  "prompt_file": ".ddx/executions/20260426T040014-1acc49c3/prompt.md",
+  "manifest_file": ".ddx/executions/20260426T040014-1acc49c3/manifest.json",
+  "result_file": ".ddx/executions/20260426T040014-1acc49c3/result.json",
+  "usage_file": ".ddx/executions/20260426T040014-1acc49c3/usage.json",
+  "started_at": "2026-04-26T04:00:14.840842534Z",
+  "finished_at": "2026-04-26T04:04:51.175646584Z"
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
