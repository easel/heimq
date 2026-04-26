<bead-review>
  <bead id="heimq-cd905f86" iter=1>
    <title>e2e: oversized message vs message.max.bytes</title>
    <description>
Producer attempts to send a message larger than the broker's configured max. Assert a bounded, observable failure (error on delivery report or client-side rejection), not a hang or partial produce. If heimq has no enforced max, document that — if it does enforce one, assert the boundary behavior.
In-scope files:
- tests/integration.rs (new fn test_rdkafka_oversized_message)
- src/config.rs (only if a max-size config knob needs reading; do not add one)
    </description>
    <acceptance>
`cargo test --test integration test_rdkafka_oversized_message -- --ignored --nocapture` passes. If heimq enforces no limit, test documents that and asserts the large payload round-trips.
    </acceptance>
    <labels>area:tests, kind:test, layer:e2e, client:rdkafka, phase:5, tier:p1, topic:error-paths</labels>
  </bead>

  <changed-files>
    <file>.ddx/executions/20260426T033916-e3545f1c/manifest.json</file>
    <file>.ddx/executions/20260426T033916-e3545f1c/result.json</file>
  </changed-files>

  <governing>
    <note>No governing documents found. Evaluate the diff against the acceptance criteria alone.</note>
  </governing>

  <diff rev="eb5c52c1044b89ed78b82aa15e45f987b8a59479">
diff --git a/.ddx/executions/20260426T033916-e3545f1c/manifest.json b/.ddx/executions/20260426T033916-e3545f1c/manifest.json
new file mode 100644
index 0000000..d8e71aa
--- /dev/null
+++ b/.ddx/executions/20260426T033916-e3545f1c/manifest.json
@@ -0,0 +1,42 @@
+{
+  "attempt_id": "20260426T033916-e3545f1c",
+  "bead_id": "heimq-cd905f86",
+  "base_rev": "e9bc88c7e66ed56bf620ed28bffc979557da7f62",
+  "created_at": "2026-04-26T03:39:16.674565256Z",
+  "requested": {
+    "harness": "claude",
+    "prompt": "synthesized"
+  },
+  "bead": {
+    "id": "heimq-cd905f86",
+    "title": "e2e: oversized message vs message.max.bytes",
+    "description": "Producer attempts to send a message larger than the broker's configured max. Assert a bounded, observable failure (error on delivery report or client-side rejection), not a hang or partial produce. If heimq has no enforced max, document that — if it does enforce one, assert the boundary behavior.\nIn-scope files:\n- tests/integration.rs (new fn test_rdkafka_oversized_message)\n- src/config.rs (only if a max-size config knob needs reading; do not add one)",
+    "acceptance": "`cargo test --test integration test_rdkafka_oversized_message -- --ignored --nocapture` passes. If heimq enforces no limit, test documents that and asserts the large payload round-trips.",
+    "parent": "heimq-700e47b2",
+    "labels": [
+      "area:tests",
+      "kind:test",
+      "layer:e2e",
+      "client:rdkafka",
+      "phase:5",
+      "tier:p1",
+      "topic:error-paths"
+    ],
+    "metadata": {
+      "claimed-at": "2026-04-26T03:39:16Z",
+      "claimed-machine": "eitri",
+      "claimed-pid": "4075646",
+      "execute-loop-heartbeat-at": "2026-04-26T03:39:16.591588623Z",
+      "spec-id": "API-001"
+    }
+  },
+  "paths": {
+    "dir": ".ddx/executions/20260426T033916-e3545f1c",
+    "prompt": ".ddx/executions/20260426T033916-e3545f1c/prompt.md",
+    "manifest": ".ddx/executions/20260426T033916-e3545f1c/manifest.json",
+    "result": ".ddx/executions/20260426T033916-e3545f1c/result.json",
+    "checks": ".ddx/executions/20260426T033916-e3545f1c/checks.json",
+    "usage": ".ddx/executions/20260426T033916-e3545f1c/usage.json",
+    "worktree": "tmp/ddx-exec-wt/.execute-bead-wt-heimq-cd905f86-20260426T033916-e3545f1c"
+  }
+}
\ No newline at end of file
diff --git a/.ddx/executions/20260426T033916-e3545f1c/result.json b/.ddx/executions/20260426T033916-e3545f1c/result.json
new file mode 100644
index 0000000..782a839
--- /dev/null
+++ b/.ddx/executions/20260426T033916-e3545f1c/result.json
@@ -0,0 +1,22 @@
+{
+  "bead_id": "heimq-cd905f86",
+  "attempt_id": "20260426T033916-e3545f1c",
+  "base_rev": "e9bc88c7e66ed56bf620ed28bffc979557da7f62",
+  "result_rev": "779e93aaf2b7443663965dc58fe28e3fce4ad5ed",
+  "outcome": "task_succeeded",
+  "status": "success",
+  "detail": "success",
+  "harness": "claude",
+  "session_id": "eb-8b80fb7a",
+  "duration_ms": 178290,
+  "tokens": 7766,
+  "cost_usd": 1.0697407500000002,
+  "exit_code": 0,
+  "execution_dir": ".ddx/executions/20260426T033916-e3545f1c",
+  "prompt_file": ".ddx/executions/20260426T033916-e3545f1c/prompt.md",
+  "manifest_file": ".ddx/executions/20260426T033916-e3545f1c/manifest.json",
+  "result_file": ".ddx/executions/20260426T033916-e3545f1c/result.json",
+  "usage_file": ".ddx/executions/20260426T033916-e3545f1c/usage.json",
+  "started_at": "2026-04-26T03:39:16.674811922Z",
+  "finished_at": "2026-04-26T03:42:14.965533887Z"
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
