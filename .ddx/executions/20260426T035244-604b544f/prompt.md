<bead-review>
  <bead id="heimq-bbab6a2e" iter=1>
    <title>e2e: produce-while-consume soak</title>
    <description>
Producer and consumer(s) run concurrently for a fixed batch size (e.g. 10k messages total across &gt;=3 partitions). Assert: (a) consumed count == produced count, (b) no duplicate (topic, partition, offset) tuples, (c) per-partition offsets strictly monotonic. Time-bounded (e.g. &lt;30s) so it fits in CI.
In-scope files:
- tests/integration.rs (new fn test_rdkafka_produce_consume_soak)
    </description>
    <acceptance>
`cargo test --test integration test_rdkafka_produce_consume_soak -- --ignored --nocapture` passes within a time budget and all three invariants hold.
    </acceptance>
    <labels>area:tests, kind:test, layer:e2e, client:rdkafka, phase:5, tier:p1, topic:concurrency</labels>
  </bead>

  <changed-files>
    <file>.ddx/executions/20260426T034819-d77a0ee7/manifest.json</file>
    <file>.ddx/executions/20260426T034819-d77a0ee7/result.json</file>
  </changed-files>

  <governing>
    <note>No governing documents found. Evaluate the diff against the acceptance criteria alone.</note>
  </governing>

  <diff rev="896672dc5f7268858e73e2535424b39e64eacb43">
diff --git a/.ddx/executions/20260426T034819-d77a0ee7/manifest.json b/.ddx/executions/20260426T034819-d77a0ee7/manifest.json
new file mode 100644
index 0000000..5e6e067
--- /dev/null
+++ b/.ddx/executions/20260426T034819-d77a0ee7/manifest.json
@@ -0,0 +1,42 @@
+{
+  "attempt_id": "20260426T034819-d77a0ee7",
+  "bead_id": "heimq-bbab6a2e",
+  "base_rev": "0266b0232e1c29ffbe94645f765c338848424558",
+  "created_at": "2026-04-26T03:48:19.634636539Z",
+  "requested": {
+    "harness": "claude",
+    "prompt": "synthesized"
+  },
+  "bead": {
+    "id": "heimq-bbab6a2e",
+    "title": "e2e: produce-while-consume soak",
+    "description": "Producer and consumer(s) run concurrently for a fixed batch size (e.g. 10k messages total across \u003e=3 partitions). Assert: (a) consumed count == produced count, (b) no duplicate (topic, partition, offset) tuples, (c) per-partition offsets strictly monotonic. Time-bounded (e.g. \u003c30s) so it fits in CI.\nIn-scope files:\n- tests/integration.rs (new fn test_rdkafka_produce_consume_soak)",
+    "acceptance": "`cargo test --test integration test_rdkafka_produce_consume_soak -- --ignored --nocapture` passes within a time budget and all three invariants hold.",
+    "parent": "heimq-700e47b2",
+    "labels": [
+      "area:tests",
+      "kind:test",
+      "layer:e2e",
+      "client:rdkafka",
+      "phase:5",
+      "tier:p1",
+      "topic:concurrency"
+    ],
+    "metadata": {
+      "claimed-at": "2026-04-26T03:48:19Z",
+      "claimed-machine": "eitri",
+      "claimed-pid": "4075646",
+      "execute-loop-heartbeat-at": "2026-04-26T03:48:19.546578982Z",
+      "spec-id": "API-001"
+    }
+  },
+  "paths": {
+    "dir": ".ddx/executions/20260426T034819-d77a0ee7",
+    "prompt": ".ddx/executions/20260426T034819-d77a0ee7/prompt.md",
+    "manifest": ".ddx/executions/20260426T034819-d77a0ee7/manifest.json",
+    "result": ".ddx/executions/20260426T034819-d77a0ee7/result.json",
+    "checks": ".ddx/executions/20260426T034819-d77a0ee7/checks.json",
+    "usage": ".ddx/executions/20260426T034819-d77a0ee7/usage.json",
+    "worktree": "tmp/ddx-exec-wt/.execute-bead-wt-heimq-bbab6a2e-20260426T034819-d77a0ee7"
+  }
+}
\ No newline at end of file
diff --git a/.ddx/executions/20260426T034819-d77a0ee7/result.json b/.ddx/executions/20260426T034819-d77a0ee7/result.json
new file mode 100644
index 0000000..aead05b
--- /dev/null
+++ b/.ddx/executions/20260426T034819-d77a0ee7/result.json
@@ -0,0 +1,22 @@
+{
+  "bead_id": "heimq-bbab6a2e",
+  "attempt_id": "20260426T034819-d77a0ee7",
+  "base_rev": "0266b0232e1c29ffbe94645f765c338848424558",
+  "result_rev": "b16413bd5c252dd2f5a0d902aca312e3e2d33f8b",
+  "outcome": "task_succeeded",
+  "status": "success",
+  "detail": "success",
+  "harness": "claude",
+  "session_id": "eb-d460a771",
+  "duration_ms": 263645,
+  "tokens": 10572,
+  "cost_usd": 0.9904217499999999,
+  "exit_code": 0,
+  "execution_dir": ".ddx/executions/20260426T034819-d77a0ee7",
+  "prompt_file": ".ddx/executions/20260426T034819-d77a0ee7/prompt.md",
+  "manifest_file": ".ddx/executions/20260426T034819-d77a0ee7/manifest.json",
+  "result_file": ".ddx/executions/20260426T034819-d77a0ee7/result.json",
+  "usage_file": ".ddx/executions/20260426T034819-d77a0ee7/usage.json",
+  "started_at": "2026-04-26T03:48:19.63490483Z",
+  "finished_at": "2026-04-26T03:52:43.280296561Z"
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
