<bead-review>
  <bead id="heimq-24c6cd00" iter=1>
    <title>Derive ApiVersions response from backend capabilities</title>
    <description>
Today SUPPORTED_APIS in src/protocol/mod.rs is static. Now that each backend exposes capabilities, compute the effective advertised ApiVersions at startup by intersecting static protocol support with backend caps (e.g. don't advertise InitProducerId if no backend has transactions; don't advertise OffsetCommit if offsets backend isn't writable).

Per codex review: intersection is per-API, not one global meet — GroupCoordinatorBackend capabilities must not constrain plain Produce/Fetch.

In-scope files:
- src/protocol/mod.rs (SUPPORTED_APIS becomes a computation, or a wrapper that filters)
- src/handler/api_versions.rs (reads computed set)
- src/server.rs (composes on startup)
Depends on: capability descriptor, all three trait extractions
    </description>
    <acceptance>
`cargo test` green. A manual test (or integration test) with a backend that declares compaction=false verifies no compaction-specific APIs leak into ApiVersions. Existing ApiVersions contract tests still pass for the memory default.
    </acceptance>
    <labels>area:storage, kind:feat, phase:backends, step:7-apiversions-from-caps</labels>
  </bead>

  <changed-files>
    <file>.ddx/executions/20260426T035326-1afb8e0b/manifest.json</file>
    <file>.ddx/executions/20260426T035326-1afb8e0b/result.json</file>
  </changed-files>

  <governing>
    <note>No governing documents found. Evaluate the diff against the acceptance criteria alone.</note>
  </governing>

  <diff rev="3763968138e776ea4447e8296ece0c63f77ce78f">
diff --git a/.ddx/executions/20260426T035326-1afb8e0b/manifest.json b/.ddx/executions/20260426T035326-1afb8e0b/manifest.json
new file mode 100644
index 0000000..ff92468
--- /dev/null
+++ b/.ddx/executions/20260426T035326-1afb8e0b/manifest.json
@@ -0,0 +1,38 @@
+{
+  "attempt_id": "20260426T035326-1afb8e0b",
+  "bead_id": "heimq-24c6cd00",
+  "base_rev": "b9d09f679ea8ef59b792f1625fc855a0fa85b3c8",
+  "created_at": "2026-04-26T03:53:26.137832049Z",
+  "requested": {
+    "harness": "claude",
+    "prompt": "synthesized"
+  },
+  "bead": {
+    "id": "heimq-24c6cd00",
+    "title": "Derive ApiVersions response from backend capabilities",
+    "description": "Today SUPPORTED_APIS in src/protocol/mod.rs is static. Now that each backend exposes capabilities, compute the effective advertised ApiVersions at startup by intersecting static protocol support with backend caps (e.g. don't advertise InitProducerId if no backend has transactions; don't advertise OffsetCommit if offsets backend isn't writable).\n\nPer codex review: intersection is per-API, not one global meet — GroupCoordinatorBackend capabilities must not constrain plain Produce/Fetch.\n\nIn-scope files:\n- src/protocol/mod.rs (SUPPORTED_APIS becomes a computation, or a wrapper that filters)\n- src/handler/api_versions.rs (reads computed set)\n- src/server.rs (composes on startup)\nDepends on: capability descriptor, all three trait extractions",
+    "acceptance": "`cargo test` green. A manual test (or integration test) with a backend that declares compaction=false verifies no compaction-specific APIs leak into ApiVersions. Existing ApiVersions contract tests still pass for the memory default.",
+    "parent": "heimq-2136e295",
+    "labels": [
+      "area:storage",
+      "kind:feat",
+      "phase:backends",
+      "step:7-apiversions-from-caps"
+    ],
+    "metadata": {
+      "claimed-at": "2026-04-26T03:53:26Z",
+      "claimed-machine": "eitri",
+      "claimed-pid": "4075646",
+      "execute-loop-heartbeat-at": "2026-04-26T03:53:26.051600876Z"
+    }
+  },
+  "paths": {
+    "dir": ".ddx/executions/20260426T035326-1afb8e0b",
+    "prompt": ".ddx/executions/20260426T035326-1afb8e0b/prompt.md",
+    "manifest": ".ddx/executions/20260426T035326-1afb8e0b/manifest.json",
+    "result": ".ddx/executions/20260426T035326-1afb8e0b/result.json",
+    "checks": ".ddx/executions/20260426T035326-1afb8e0b/checks.json",
+    "usage": ".ddx/executions/20260426T035326-1afb8e0b/usage.json",
+    "worktree": "tmp/ddx-exec-wt/.execute-bead-wt-heimq-24c6cd00-20260426T035326-1afb8e0b"
+  }
+}
\ No newline at end of file
diff --git a/.ddx/executions/20260426T035326-1afb8e0b/result.json b/.ddx/executions/20260426T035326-1afb8e0b/result.json
new file mode 100644
index 0000000..ab9f543
--- /dev/null
+++ b/.ddx/executions/20260426T035326-1afb8e0b/result.json
@@ -0,0 +1,22 @@
+{
+  "bead_id": "heimq-24c6cd00",
+  "attempt_id": "20260426T035326-1afb8e0b",
+  "base_rev": "b9d09f679ea8ef59b792f1625fc855a0fa85b3c8",
+  "result_rev": "78c69c90a9939c81272f539d4adc5f2baf9b01f0",
+  "outcome": "task_succeeded",
+  "status": "success",
+  "detail": "success",
+  "harness": "claude",
+  "session_id": "eb-537b74f6",
+  "duration_ms": 380245,
+  "tokens": 21997,
+  "cost_usd": 2.8852347499999995,
+  "exit_code": 0,
+  "execution_dir": ".ddx/executions/20260426T035326-1afb8e0b",
+  "prompt_file": ".ddx/executions/20260426T035326-1afb8e0b/prompt.md",
+  "manifest_file": ".ddx/executions/20260426T035326-1afb8e0b/manifest.json",
+  "result_file": ".ddx/executions/20260426T035326-1afb8e0b/result.json",
+  "usage_file": ".ddx/executions/20260426T035326-1afb8e0b/usage.json",
+  "started_at": "2026-04-26T03:53:26.138154424Z",
+  "finished_at": "2026-04-26T03:59:46.38344785Z"
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
