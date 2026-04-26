<bead-review>
  <bead id="heimq-979c6bea" iter=1>
    <title>e2e: empty-topic consume timeout</title>
    <description>
Consumer subscribes to an empty topic, polls with a short timeout (e.g. 500ms), expects a clean None/timeout result — not an error, not a hang. Catches broken long-poll responses where broker returns malformed empty FetchResponse.
In-scope files:
- tests/integration.rs (new fn test_rdkafka_empty_topic_poll_timeout)
    </description>
    <acceptance>
`cargo test --test integration test_rdkafka_empty_topic_poll_timeout -- --ignored --nocapture` passes and asserts poll() returns None (no error) within the configured timeout.
    </acceptance>
    <labels>area:tests, kind:test, layer:e2e, client:rdkafka, phase:5, tier:p1, topic:fetch-wait</labels>
  </bead>

  <changed-files>
    <file>.ddx/executions/20260426T033401-38d0e687/manifest.json</file>
    <file>.ddx/executions/20260426T033401-38d0e687/result.json</file>
  </changed-files>

  <governing>
    <note>No governing documents found. Evaluate the diff against the acceptance criteria alone.</note>
  </governing>

  <diff rev="3e6dcdf92a722840dcedc1c8a944896edf932db3">
diff --git a/.ddx/executions/20260426T033401-38d0e687/manifest.json b/.ddx/executions/20260426T033401-38d0e687/manifest.json
new file mode 100644
index 0000000..ff1b314
--- /dev/null
+++ b/.ddx/executions/20260426T033401-38d0e687/manifest.json
@@ -0,0 +1,42 @@
+{
+  "attempt_id": "20260426T033401-38d0e687",
+  "bead_id": "heimq-979c6bea",
+  "base_rev": "6ba6375f27253501e524f97add8138b32bf33897",
+  "created_at": "2026-04-26T03:34:01.156969285Z",
+  "requested": {
+    "harness": "claude",
+    "prompt": "synthesized"
+  },
+  "bead": {
+    "id": "heimq-979c6bea",
+    "title": "e2e: empty-topic consume timeout",
+    "description": "Consumer subscribes to an empty topic, polls with a short timeout (e.g. 500ms), expects a clean None/timeout result — not an error, not a hang. Catches broken long-poll responses where broker returns malformed empty FetchResponse.\nIn-scope files:\n- tests/integration.rs (new fn test_rdkafka_empty_topic_poll_timeout)",
+    "acceptance": "`cargo test --test integration test_rdkafka_empty_topic_poll_timeout -- --ignored --nocapture` passes and asserts poll() returns None (no error) within the configured timeout.",
+    "parent": "heimq-700e47b2",
+    "labels": [
+      "area:tests",
+      "kind:test",
+      "layer:e2e",
+      "client:rdkafka",
+      "phase:5",
+      "tier:p1",
+      "topic:fetch-wait"
+    ],
+    "metadata": {
+      "claimed-at": "2026-04-26T03:34:01Z",
+      "claimed-machine": "eitri",
+      "claimed-pid": "4075646",
+      "execute-loop-heartbeat-at": "2026-04-26T03:34:01.068084571Z",
+      "spec-id": "API-001"
+    }
+  },
+  "paths": {
+    "dir": ".ddx/executions/20260426T033401-38d0e687",
+    "prompt": ".ddx/executions/20260426T033401-38d0e687/prompt.md",
+    "manifest": ".ddx/executions/20260426T033401-38d0e687/manifest.json",
+    "result": ".ddx/executions/20260426T033401-38d0e687/result.json",
+    "checks": ".ddx/executions/20260426T033401-38d0e687/checks.json",
+    "usage": ".ddx/executions/20260426T033401-38d0e687/usage.json",
+    "worktree": "tmp/ddx-exec-wt/.execute-bead-wt-heimq-979c6bea-20260426T033401-38d0e687"
+  }
+}
\ No newline at end of file
diff --git a/.ddx/executions/20260426T033401-38d0e687/result.json b/.ddx/executions/20260426T033401-38d0e687/result.json
new file mode 100644
index 0000000..d074ee6
--- /dev/null
+++ b/.ddx/executions/20260426T033401-38d0e687/result.json
@@ -0,0 +1,22 @@
+{
+  "bead_id": "heimq-979c6bea",
+  "attempt_id": "20260426T033401-38d0e687",
+  "base_rev": "6ba6375f27253501e524f97add8138b32bf33897",
+  "result_rev": "b9e803d05238cd10af13caf4c3344762ec3ffc50",
+  "outcome": "task_succeeded",
+  "status": "success",
+  "detail": "success",
+  "harness": "claude",
+  "session_id": "eb-0faf8353",
+  "duration_ms": 118296,
+  "tokens": 5015,
+  "cost_usd": 0.7128679999999998,
+  "exit_code": 0,
+  "execution_dir": ".ddx/executions/20260426T033401-38d0e687",
+  "prompt_file": ".ddx/executions/20260426T033401-38d0e687/prompt.md",
+  "manifest_file": ".ddx/executions/20260426T033401-38d0e687/manifest.json",
+  "result_file": ".ddx/executions/20260426T033401-38d0e687/result.json",
+  "usage_file": ".ddx/executions/20260426T033401-38d0e687/usage.json",
+  "started_at": "2026-04-26T03:34:01.15723816Z",
+  "finished_at": "2026-04-26T03:35:59.454082219Z"
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
