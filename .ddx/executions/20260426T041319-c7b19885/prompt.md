<bead-review>
  <bead id="heimq-76674efa" iter=1>
    <title>e2e: compression codec round-trips (gzip, snappy, lz4, zstd)</title>
    <description>
For each of compression.type = gzip, snappy, lz4, zstd: producer encodes a batch, consumer decodes and asserts payload equality. Exercises the broker's RecordBatch decode paths for each codec (heimq must re-read batch metadata but the compressed payload itself is passed through opaque — confirm behavior).
In-scope files:
- tests/integration.rs (new fn test_rdkafka_compression_codecs, parameterized over codecs)
- Cargo.toml dev-dependencies (enable rdkafka compression features if needed)
    </description>
    <acceptance>
`cargo test --test integration test_rdkafka_compression_codecs -- --ignored --nocapture` passes for all four codecs. If a codec is unsupported, test documents and asserts a clear error rather than silent corruption.
    </acceptance>
    <labels>area:tests, kind:test, layer:e2e, client:rdkafka, phase:5, tier:p2, topic:client-surface</labels>
  </bead>

  <changed-files>
    <file>.ddx/executions/20260426T041021-f5376186/manifest.json</file>
    <file>.ddx/executions/20260426T041021-f5376186/result.json</file>
  </changed-files>

  <governing>
    <note>No governing documents found. Evaluate the diff against the acceptance criteria alone.</note>
  </governing>

  <diff rev="2b8f1ef1ce86b1c603de6a324b45ff9cabbd3ce6">
diff --git a/.ddx/executions/20260426T041021-f5376186/manifest.json b/.ddx/executions/20260426T041021-f5376186/manifest.json
new file mode 100644
index 0000000..33cfbee
--- /dev/null
+++ b/.ddx/executions/20260426T041021-f5376186/manifest.json
@@ -0,0 +1,42 @@
+{
+  "attempt_id": "20260426T041021-f5376186",
+  "bead_id": "heimq-76674efa",
+  "base_rev": "fb881bb8ccf5f5c773a1af618f5a509d7ddcc450",
+  "created_at": "2026-04-26T04:10:21.088875661Z",
+  "requested": {
+    "harness": "claude",
+    "prompt": "synthesized"
+  },
+  "bead": {
+    "id": "heimq-76674efa",
+    "title": "e2e: compression codec round-trips (gzip, snappy, lz4, zstd)",
+    "description": "For each of compression.type = gzip, snappy, lz4, zstd: producer encodes a batch, consumer decodes and asserts payload equality. Exercises the broker's RecordBatch decode paths for each codec (heimq must re-read batch metadata but the compressed payload itself is passed through opaque — confirm behavior).\nIn-scope files:\n- tests/integration.rs (new fn test_rdkafka_compression_codecs, parameterized over codecs)\n- Cargo.toml dev-dependencies (enable rdkafka compression features if needed)",
+    "acceptance": "`cargo test --test integration test_rdkafka_compression_codecs -- --ignored --nocapture` passes for all four codecs. If a codec is unsupported, test documents and asserts a clear error rather than silent corruption.",
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
+      "claimed-at": "2026-04-26T04:10:21Z",
+      "claimed-machine": "eitri",
+      "claimed-pid": "4075646",
+      "execute-loop-heartbeat-at": "2026-04-26T04:10:21.001717089Z",
+      "spec-id": "API-001"
+    }
+  },
+  "paths": {
+    "dir": ".ddx/executions/20260426T041021-f5376186",
+    "prompt": ".ddx/executions/20260426T041021-f5376186/prompt.md",
+    "manifest": ".ddx/executions/20260426T041021-f5376186/manifest.json",
+    "result": ".ddx/executions/20260426T041021-f5376186/result.json",
+    "checks": ".ddx/executions/20260426T041021-f5376186/checks.json",
+    "usage": ".ddx/executions/20260426T041021-f5376186/usage.json",
+    "worktree": "tmp/ddx-exec-wt/.execute-bead-wt-heimq-76674efa-20260426T041021-f5376186"
+  }
+}
\ No newline at end of file
diff --git a/.ddx/executions/20260426T041021-f5376186/result.json b/.ddx/executions/20260426T041021-f5376186/result.json
new file mode 100644
index 0000000..910a227
--- /dev/null
+++ b/.ddx/executions/20260426T041021-f5376186/result.json
@@ -0,0 +1,22 @@
+{
+  "bead_id": "heimq-76674efa",
+  "attempt_id": "20260426T041021-f5376186",
+  "base_rev": "fb881bb8ccf5f5c773a1af618f5a509d7ddcc450",
+  "result_rev": "594e1ee7d1455302a5c64847896f883ec56b097a",
+  "outcome": "task_succeeded",
+  "status": "success",
+  "detail": "success",
+  "harness": "claude",
+  "session_id": "eb-62dc0dcf",
+  "duration_ms": 176645,
+  "tokens": 7799,
+  "cost_usd": 0.9107117500000002,
+  "exit_code": 0,
+  "execution_dir": ".ddx/executions/20260426T041021-f5376186",
+  "prompt_file": ".ddx/executions/20260426T041021-f5376186/prompt.md",
+  "manifest_file": ".ddx/executions/20260426T041021-f5376186/manifest.json",
+  "result_file": ".ddx/executions/20260426T041021-f5376186/result.json",
+  "usage_file": ".ddx/executions/20260426T041021-f5376186/usage.json",
+  "started_at": "2026-04-26T04:10:21.089160202Z",
+  "finished_at": "2026-04-26T04:13:17.734629999Z"
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
