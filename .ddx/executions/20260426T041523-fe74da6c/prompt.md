<bead-review>
  <bead id="heimq-cd222bbb" iter=1>
    <title>e2e: seek to end and seek to arbitrary offset</title>
    <description>
Complements existing test_rdkafka_consumer_seek_to_beginning. Two scenarios: (1) seek to end — subsequent poll sees only new messages produced after the seek; (2) seek to a specific mid-range offset — consumer sees messages from that offset forward.
In-scope files:
- tests/integration.rs (new fns test_rdkafka_seek_to_end, test_rdkafka_seek_to_offset)
    </description>
    <acceptance>
`cargo test --test integration test_rdkafka_seek_to -- --ignored --nocapture` runs both and asserts consumed offsets match expectation.
    </acceptance>
    <labels>area:tests, kind:test, layer:e2e, client:rdkafka, phase:5, tier:p2, topic:client-surface</labels>
  </bead>

  <changed-files>
    <file>.ddx/executions/20260426T041347-b97824af/manifest.json</file>
    <file>.ddx/executions/20260426T041347-b97824af/result.json</file>
  </changed-files>

  <governing>
    <note>No governing documents found. Evaluate the diff against the acceptance criteria alone.</note>
  </governing>

  <diff rev="8a4cdadc0c71825c244ee9adbcb612cab452c200">
diff --git a/.ddx/executions/20260426T041347-b97824af/manifest.json b/.ddx/executions/20260426T041347-b97824af/manifest.json
new file mode 100644
index 0000000..5e5dacc
--- /dev/null
+++ b/.ddx/executions/20260426T041347-b97824af/manifest.json
@@ -0,0 +1,42 @@
+{
+  "attempt_id": "20260426T041347-b97824af",
+  "bead_id": "heimq-cd222bbb",
+  "base_rev": "9e13707eb53e0f5c7c0909205d0b1438c1e524dd",
+  "created_at": "2026-04-26T04:13:47.825080085Z",
+  "requested": {
+    "harness": "claude",
+    "prompt": "synthesized"
+  },
+  "bead": {
+    "id": "heimq-cd222bbb",
+    "title": "e2e: seek to end and seek to arbitrary offset",
+    "description": "Complements existing test_rdkafka_consumer_seek_to_beginning. Two scenarios: (1) seek to end — subsequent poll sees only new messages produced after the seek; (2) seek to a specific mid-range offset — consumer sees messages from that offset forward.\nIn-scope files:\n- tests/integration.rs (new fns test_rdkafka_seek_to_end, test_rdkafka_seek_to_offset)",
+    "acceptance": "`cargo test --test integration test_rdkafka_seek_to -- --ignored --nocapture` runs both and asserts consumed offsets match expectation.",
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
+      "claimed-at": "2026-04-26T04:13:47Z",
+      "claimed-machine": "eitri",
+      "claimed-pid": "4075646",
+      "execute-loop-heartbeat-at": "2026-04-26T04:13:47.733254183Z",
+      "spec-id": "API-001"
+    }
+  },
+  "paths": {
+    "dir": ".ddx/executions/20260426T041347-b97824af",
+    "prompt": ".ddx/executions/20260426T041347-b97824af/prompt.md",
+    "manifest": ".ddx/executions/20260426T041347-b97824af/manifest.json",
+    "result": ".ddx/executions/20260426T041347-b97824af/result.json",
+    "checks": ".ddx/executions/20260426T041347-b97824af/checks.json",
+    "usage": ".ddx/executions/20260426T041347-b97824af/usage.json",
+    "worktree": "tmp/ddx-exec-wt/.execute-bead-wt-heimq-cd222bbb-20260426T041347-b97824af"
+  }
+}
\ No newline at end of file
diff --git a/.ddx/executions/20260426T041347-b97824af/result.json b/.ddx/executions/20260426T041347-b97824af/result.json
new file mode 100644
index 0000000..d61d33e
--- /dev/null
+++ b/.ddx/executions/20260426T041347-b97824af/result.json
@@ -0,0 +1,22 @@
+{
+  "bead_id": "heimq-cd222bbb",
+  "attempt_id": "20260426T041347-b97824af",
+  "base_rev": "9e13707eb53e0f5c7c0909205d0b1438c1e524dd",
+  "result_rev": "71a61e00df8312db8f6a1bb2f60b0778ae04a22b",
+  "outcome": "task_succeeded",
+  "status": "success",
+  "detail": "success",
+  "harness": "claude",
+  "session_id": "eb-857959fc",
+  "duration_ms": 94058,
+  "tokens": 5247,
+  "cost_usd": 0.44504350000000004,
+  "exit_code": 0,
+  "execution_dir": ".ddx/executions/20260426T041347-b97824af",
+  "prompt_file": ".ddx/executions/20260426T041347-b97824af/prompt.md",
+  "manifest_file": ".ddx/executions/20260426T041347-b97824af/manifest.json",
+  "result_file": ".ddx/executions/20260426T041347-b97824af/result.json",
+  "usage_file": ".ddx/executions/20260426T041347-b97824af/usage.json",
+  "started_at": "2026-04-26T04:13:47.825468502Z",
+  "finished_at": "2026-04-26T04:15:21.88385765Z"
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
