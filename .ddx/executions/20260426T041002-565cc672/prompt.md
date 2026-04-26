<bead-review>
  <bead id="heimq-977d7b59" iter=1>
    <title>e2e: record headers round-trip</title>
    <description>
Producer attaches 2-3 headers (k/v byte arrays) to a message. Consumer reads the message and asserts all headers present with correct values and order.
In-scope files:
- tests/integration.rs (new fn test_rdkafka_record_headers_roundtrip)
    </description>
    <acceptance>
`cargo test --test integration test_rdkafka_record_headers_roundtrip -- --ignored --nocapture` passes with header equality assertion.
    </acceptance>
    <labels>area:tests, kind:test, layer:e2e, client:rdkafka, phase:5, tier:p2, topic:client-surface</labels>
  </bead>

  <changed-files>
    <file>.ddx/executions/20260426T040814-5fcc2ea0/manifest.json</file>
    <file>.ddx/executions/20260426T040814-5fcc2ea0/result.json</file>
  </changed-files>

  <governing>
    <note>No governing documents found. Evaluate the diff against the acceptance criteria alone.</note>
  </governing>

  <diff rev="5da09368106c724b6ac9b5cb38fd325aaf6e6dd2">
diff --git a/.ddx/executions/20260426T040814-5fcc2ea0/manifest.json b/.ddx/executions/20260426T040814-5fcc2ea0/manifest.json
new file mode 100644
index 0000000..90a0c3d
--- /dev/null
+++ b/.ddx/executions/20260426T040814-5fcc2ea0/manifest.json
@@ -0,0 +1,42 @@
+{
+  "attempt_id": "20260426T040814-5fcc2ea0",
+  "bead_id": "heimq-977d7b59",
+  "base_rev": "b830a14f39b62f53b0d93d3a62be6ba7cd6a1d6e",
+  "created_at": "2026-04-26T04:08:14.450962973Z",
+  "requested": {
+    "harness": "claude",
+    "prompt": "synthesized"
+  },
+  "bead": {
+    "id": "heimq-977d7b59",
+    "title": "e2e: record headers round-trip",
+    "description": "Producer attaches 2-3 headers (k/v byte arrays) to a message. Consumer reads the message and asserts all headers present with correct values and order.\nIn-scope files:\n- tests/integration.rs (new fn test_rdkafka_record_headers_roundtrip)",
+    "acceptance": "`cargo test --test integration test_rdkafka_record_headers_roundtrip -- --ignored --nocapture` passes with header equality assertion.",
+    "parent": "heimq-700e47b2",
+    "labels": [
+      "area:tests",
+      "kind:test",
+      "layer:e2e",
+      "client:rdkafka",
+      "phase:5",
+      "tier:p2",
+      "topic:client-surface"
+    ],
+    "metadata": {
+      "claimed-at": "2026-04-26T04:08:14Z",
+      "claimed-machine": "eitri",
+      "claimed-pid": "4075646",
+      "execute-loop-heartbeat-at": "2026-04-26T04:08:14.354746948Z",
+      "spec-id": "API-001"
+    }
+  },
+  "paths": {
+    "dir": ".ddx/executions/20260426T040814-5fcc2ea0",
+    "prompt": ".ddx/executions/20260426T040814-5fcc2ea0/prompt.md",
+    "manifest": ".ddx/executions/20260426T040814-5fcc2ea0/manifest.json",
+    "result": ".ddx/executions/20260426T040814-5fcc2ea0/result.json",
+    "checks": ".ddx/executions/20260426T040814-5fcc2ea0/checks.json",
+    "usage": ".ddx/executions/20260426T040814-5fcc2ea0/usage.json",
+    "worktree": "tmp/ddx-exec-wt/.execute-bead-wt-heimq-977d7b59-20260426T040814-5fcc2ea0"
+  }
+}
\ No newline at end of file
diff --git a/.ddx/executions/20260426T040814-5fcc2ea0/result.json b/.ddx/executions/20260426T040814-5fcc2ea0/result.json
new file mode 100644
index 0000000..5707f0a
--- /dev/null
+++ b/.ddx/executions/20260426T040814-5fcc2ea0/result.json
@@ -0,0 +1,22 @@
+{
+  "bead_id": "heimq-977d7b59",
+  "attempt_id": "20260426T040814-5fcc2ea0",
+  "base_rev": "b830a14f39b62f53b0d93d3a62be6ba7cd6a1d6e",
+  "result_rev": "f3945918d9b94d68a093aef9ea50aa783073745c",
+  "outcome": "task_succeeded",
+  "status": "success",
+  "detail": "success",
+  "harness": "claude",
+  "session_id": "eb-b8aa49aa",
+  "duration_ms": 107001,
+  "tokens": 4697,
+  "cost_usd": 0.5535285,
+  "exit_code": 0,
+  "execution_dir": ".ddx/executions/20260426T040814-5fcc2ea0",
+  "prompt_file": ".ddx/executions/20260426T040814-5fcc2ea0/prompt.md",
+  "manifest_file": ".ddx/executions/20260426T040814-5fcc2ea0/manifest.json",
+  "result_file": ".ddx/executions/20260426T040814-5fcc2ea0/result.json",
+  "usage_file": ".ddx/executions/20260426T040814-5fcc2ea0/usage.json",
+  "started_at": "2026-04-26T04:08:14.451218597Z",
+  "finished_at": "2026-04-26T04:10:01.452299244Z"
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
