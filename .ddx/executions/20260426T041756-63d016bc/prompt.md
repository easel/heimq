<bead-review>
  <bead id="heimq-0c5afc66" iter=1>
    <title>e2e: pause/resume on assigned partitions</title>
    <description>
Consumer assigned to 2 partitions. Pause partition 0, produce to both, poll, assert only partition 1 messages arrive. Resume partition 0, poll, assert partition 0 backlog drains.
In-scope files:
- tests/integration.rs (new fn test_rdkafka_pause_resume_partitions)
    </description>
    <acceptance>
`cargo test --test integration test_rdkafka_pause_resume_partitions -- --ignored --nocapture` passes with both paused-then-resumed assertions.
    </acceptance>
    <labels>area:tests, kind:test, layer:e2e, client:rdkafka, phase:5, tier:p2, topic:client-surface</labels>
  </bead>

  <changed-files>
    <file>.ddx/executions/20260426T041553-cb86080d/manifest.json</file>
    <file>.ddx/executions/20260426T041553-cb86080d/result.json</file>
  </changed-files>

  <governing>
    <note>No governing documents found. Evaluate the diff against the acceptance criteria alone.</note>
  </governing>

  <diff rev="fe36451aceba7d7df1e14eab0139839a09dbbed2">
diff --git a/.ddx/executions/20260426T041553-cb86080d/manifest.json b/.ddx/executions/20260426T041553-cb86080d/manifest.json
new file mode 100644
index 0000000..e63994e
--- /dev/null
+++ b/.ddx/executions/20260426T041553-cb86080d/manifest.json
@@ -0,0 +1,42 @@
+{
+  "attempt_id": "20260426T041553-cb86080d",
+  "bead_id": "heimq-0c5afc66",
+  "base_rev": "c0539e9bd30f4c3456f7614016e7ac805a6f3787",
+  "created_at": "2026-04-26T04:15:53.784967781Z",
+  "requested": {
+    "harness": "claude",
+    "prompt": "synthesized"
+  },
+  "bead": {
+    "id": "heimq-0c5afc66",
+    "title": "e2e: pause/resume on assigned partitions",
+    "description": "Consumer assigned to 2 partitions. Pause partition 0, produce to both, poll, assert only partition 1 messages arrive. Resume partition 0, poll, assert partition 0 backlog drains.\nIn-scope files:\n- tests/integration.rs (new fn test_rdkafka_pause_resume_partitions)",
+    "acceptance": "`cargo test --test integration test_rdkafka_pause_resume_partitions -- --ignored --nocapture` passes with both paused-then-resumed assertions.",
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
+      "claimed-at": "2026-04-26T04:15:53Z",
+      "claimed-machine": "eitri",
+      "claimed-pid": "4075646",
+      "execute-loop-heartbeat-at": "2026-04-26T04:15:53.693611836Z",
+      "spec-id": "API-001"
+    }
+  },
+  "paths": {
+    "dir": ".ddx/executions/20260426T041553-cb86080d",
+    "prompt": ".ddx/executions/20260426T041553-cb86080d/prompt.md",
+    "manifest": ".ddx/executions/20260426T041553-cb86080d/manifest.json",
+    "result": ".ddx/executions/20260426T041553-cb86080d/result.json",
+    "checks": ".ddx/executions/20260426T041553-cb86080d/checks.json",
+    "usage": ".ddx/executions/20260426T041553-cb86080d/usage.json",
+    "worktree": "tmp/ddx-exec-wt/.execute-bead-wt-heimq-0c5afc66-20260426T041553-cb86080d"
+  }
+}
\ No newline at end of file
diff --git a/.ddx/executions/20260426T041553-cb86080d/result.json b/.ddx/executions/20260426T041553-cb86080d/result.json
new file mode 100644
index 0000000..4da0b3b
--- /dev/null
+++ b/.ddx/executions/20260426T041553-cb86080d/result.json
@@ -0,0 +1,22 @@
+{
+  "bead_id": "heimq-0c5afc66",
+  "attempt_id": "20260426T041553-cb86080d",
+  "base_rev": "c0539e9bd30f4c3456f7614016e7ac805a6f3787",
+  "result_rev": "ea726a6e47a2d57cba1d9269b5379d20afcc68c6",
+  "outcome": "task_succeeded",
+  "status": "success",
+  "detail": "success",
+  "harness": "claude",
+  "session_id": "eb-e35fc1d6",
+  "duration_ms": 121759,
+  "tokens": 7554,
+  "cost_usd": 0.7925444999999999,
+  "exit_code": 0,
+  "execution_dir": ".ddx/executions/20260426T041553-cb86080d",
+  "prompt_file": ".ddx/executions/20260426T041553-cb86080d/prompt.md",
+  "manifest_file": ".ddx/executions/20260426T041553-cb86080d/manifest.json",
+  "result_file": ".ddx/executions/20260426T041553-cb86080d/result.json",
+  "usage_file": ".ddx/executions/20260426T041553-cb86080d/usage.json",
+  "started_at": "2026-04-26T04:15:53.785251947Z",
+  "finished_at": "2026-04-26T04:17:55.544303595Z"
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
