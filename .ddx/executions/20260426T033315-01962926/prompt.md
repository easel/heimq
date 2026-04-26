<bead-review>
  <bead id="heimq-4ab3d5d7" iter=1>
    <title>e2e: long-poll fetch respects fetch.min.bytes + fetch.max.wait.ms</title>
    <description>
Consumer with fetch.min.bytes=1024 and fetch.max.wait.ms=500 polls an empty-ish topic. Case A: no new data — poll returns after ~500ms with empty batch (not 0ms, not hung). Case B: producer writes enough bytes mid-wait — poll returns immediately after threshold reached.
In-scope files:
- tests/integration.rs (new fn test_rdkafka_fetch_long_poll)
    </description>
    <acceptance>
`cargo test --test integration test_rdkafka_fetch_long_poll -- --ignored --nocapture` passes. Both timing assertions hold within a reasonable tolerance (e.g. 300ms &lt; poll_a &lt; 800ms; poll_b &lt; 300ms after produce).
    </acceptance>
    <labels>area:tests, kind:test, layer:e2e, client:rdkafka, phase:5, tier:p1, topic:fetch-wait</labels>
  </bead>

  <changed-files>
    <file>.ddx/executions/20260426T032832-01207fec/manifest.json</file>
    <file>.ddx/executions/20260426T032832-01207fec/result.json</file>
  </changed-files>

  <governing>
    <note>No governing documents found. Evaluate the diff against the acceptance criteria alone.</note>
  </governing>

  <diff rev="c18c7e4f0ef7ad80264a6d3516ec75be7d097060">
diff --git a/.ddx/executions/20260426T032832-01207fec/manifest.json b/.ddx/executions/20260426T032832-01207fec/manifest.json
new file mode 100644
index 0000000..cb0efba
--- /dev/null
+++ b/.ddx/executions/20260426T032832-01207fec/manifest.json
@@ -0,0 +1,42 @@
+{
+  "attempt_id": "20260426T032832-01207fec",
+  "bead_id": "heimq-4ab3d5d7",
+  "base_rev": "caa19c97bc7c249b653bce6276ea1081df9fa744",
+  "created_at": "2026-04-26T03:28:32.349011956Z",
+  "requested": {
+    "harness": "claude",
+    "prompt": "synthesized"
+  },
+  "bead": {
+    "id": "heimq-4ab3d5d7",
+    "title": "e2e: long-poll fetch respects fetch.min.bytes + fetch.max.wait.ms",
+    "description": "Consumer with fetch.min.bytes=1024 and fetch.max.wait.ms=500 polls an empty-ish topic. Case A: no new data — poll returns after ~500ms with empty batch (not 0ms, not hung). Case B: producer writes enough bytes mid-wait — poll returns immediately after threshold reached.\nIn-scope files:\n- tests/integration.rs (new fn test_rdkafka_fetch_long_poll)",
+    "acceptance": "`cargo test --test integration test_rdkafka_fetch_long_poll -- --ignored --nocapture` passes. Both timing assertions hold within a reasonable tolerance (e.g. 300ms \u003c poll_a \u003c 800ms; poll_b \u003c 300ms after produce).",
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
+      "claimed-at": "2026-04-26T03:28:32Z",
+      "claimed-machine": "eitri",
+      "claimed-pid": "4075646",
+      "execute-loop-heartbeat-at": "2026-04-26T03:28:32.267200864Z",
+      "spec-id": "API-001"
+    }
+  },
+  "paths": {
+    "dir": ".ddx/executions/20260426T032832-01207fec",
+    "prompt": ".ddx/executions/20260426T032832-01207fec/prompt.md",
+    "manifest": ".ddx/executions/20260426T032832-01207fec/manifest.json",
+    "result": ".ddx/executions/20260426T032832-01207fec/result.json",
+    "checks": ".ddx/executions/20260426T032832-01207fec/checks.json",
+    "usage": ".ddx/executions/20260426T032832-01207fec/usage.json",
+    "worktree": "tmp/ddx-exec-wt/.execute-bead-wt-heimq-4ab3d5d7-20260426T032832-01207fec"
+  }
+}
\ No newline at end of file
diff --git a/.ddx/executions/20260426T032832-01207fec/result.json b/.ddx/executions/20260426T032832-01207fec/result.json
new file mode 100644
index 0000000..41b5558
--- /dev/null
+++ b/.ddx/executions/20260426T032832-01207fec/result.json
@@ -0,0 +1,22 @@
+{
+  "bead_id": "heimq-4ab3d5d7",
+  "attempt_id": "20260426T032832-01207fec",
+  "base_rev": "caa19c97bc7c249b653bce6276ea1081df9fa744",
+  "result_rev": "ecba96433a663f4b36acb8da598ab44b358c0094",
+  "outcome": "task_succeeded",
+  "status": "success",
+  "detail": "success",
+  "harness": "claude",
+  "session_id": "eb-c051bf3f",
+  "duration_ms": 280087,
+  "tokens": 19245,
+  "cost_usd": 1.2295942499999997,
+  "exit_code": 0,
+  "execution_dir": ".ddx/executions/20260426T032832-01207fec",
+  "prompt_file": ".ddx/executions/20260426T032832-01207fec/prompt.md",
+  "manifest_file": ".ddx/executions/20260426T032832-01207fec/manifest.json",
+  "result_file": ".ddx/executions/20260426T032832-01207fec/result.json",
+  "usage_file": ".ddx/executions/20260426T032832-01207fec/usage.json",
+  "started_at": "2026-04-26T03:28:32.349232914Z",
+  "finished_at": "2026-04-26T03:33:12.436839806Z"
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
