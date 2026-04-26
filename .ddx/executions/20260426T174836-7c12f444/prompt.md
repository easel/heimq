<bead-review>
  <bead id="heimq-17753e60" iter=1>
    <title>docs(design): refresh API-001 Support Matrix to reflect flexible-version cap</title>
    <description>
API-001 Support Matrix table (heimq/docs/helix/02-design/contracts/API-001-kafka-protocol.md lines 50-66) advertises max versions (ListOffsets 0-7, OffsetFetch 0-7, FindCoordinator 0-3, JoinGroup 0-8, Heartbeat 0-4, LeaveGroup 0-4, SyncGroup 0-4, ApiVersions 0-3, CreateTopics 0-6, DeleteTopics 0-5) that pre-date commit 331b93b which capped versions below the flexible-version boundary in src/protocol/mod.rs. Update the matrix to match the authoritative SUPPORTED_APIS values: ListOffsets 0-5, OffsetFetch 0-5, FindCoordinator 0-2, JoinGroup 0-5, Heartbeat 0-3, LeaveGroup 0-3, SyncGroup 0-3, ApiVersions 0-2, CreateTopics 0-4, DeleteTopics 0-3. Add a brief note on the flexible-version policy and link to ADR/protocol/mod.rs comment.
    </description>
    <acceptance>
API-001 table values match SUPPORTED_APIS exactly. Doc explicitly states the flexible-version-boundary policy.
    </acceptance>
    <labels>helix, phase:design, kind:docs, area:protocol</labels>
  </bead>

  <changed-files>
    <file>.ddx/executions/20260426T174734-7ea9cbe6/manifest.json</file>
    <file>.ddx/executions/20260426T174734-7ea9cbe6/result.json</file>
  </changed-files>

  <governing>
    <note>No governing documents found. Evaluate the diff against the acceptance criteria alone.</note>
  </governing>

  <diff rev="c3d7cbd48551471e4fa8d125a20cd4bdeb113ec1">
diff --git a/.ddx/executions/20260426T174734-7ea9cbe6/manifest.json b/.ddx/executions/20260426T174734-7ea9cbe6/manifest.json
new file mode 100644
index 0000000..455dd72
--- /dev/null
+++ b/.ddx/executions/20260426T174734-7ea9cbe6/manifest.json
@@ -0,0 +1,40 @@
+{
+  "attempt_id": "20260426T174734-7ea9cbe6",
+  "bead_id": "heimq-17753e60",
+  "base_rev": "8202c8c991f9b8954364b861c5766db0bd2f0c92",
+  "created_at": "2026-04-26T17:47:34.55999507Z",
+  "requested": {
+    "harness": "claude",
+    "prompt": "synthesized"
+  },
+  "bead": {
+    "id": "heimq-17753e60",
+    "title": "docs(design): refresh API-001 Support Matrix to reflect flexible-version cap",
+    "description": "API-001 Support Matrix table (heimq/docs/helix/02-design/contracts/API-001-kafka-protocol.md lines 50-66) advertises max versions (ListOffsets 0-7, OffsetFetch 0-7, FindCoordinator 0-3, JoinGroup 0-8, Heartbeat 0-4, LeaveGroup 0-4, SyncGroup 0-4, ApiVersions 0-3, CreateTopics 0-6, DeleteTopics 0-5) that pre-date commit 331b93b which capped versions below the flexible-version boundary in src/protocol/mod.rs. Update the matrix to match the authoritative SUPPORTED_APIS values: ListOffsets 0-5, OffsetFetch 0-5, FindCoordinator 0-2, JoinGroup 0-5, Heartbeat 0-3, LeaveGroup 0-3, SyncGroup 0-3, ApiVersions 0-2, CreateTopics 0-4, DeleteTopics 0-3. Add a brief note on the flexible-version policy and link to ADR/protocol/mod.rs comment.",
+    "acceptance": "API-001 table values match SUPPORTED_APIS exactly. Doc explicitly states the flexible-version-boundary policy.",
+    "parent": "heimq-ef45a81b",
+    "labels": [
+      "helix",
+      "phase:design",
+      "kind:docs",
+      "area:protocol"
+    ],
+    "metadata": {
+      "claimed-at": "2026-04-26T17:47:34Z",
+      "claimed-machine": "eitri",
+      "claimed-pid": "4075646",
+      "discovered-from": "heimq-ef45a81b",
+      "execute-loop-heartbeat-at": "2026-04-26T17:47:34.446766289Z",
+      "spec-id": "API-001"
+    }
+  },
+  "paths": {
+    "dir": ".ddx/executions/20260426T174734-7ea9cbe6",
+    "prompt": ".ddx/executions/20260426T174734-7ea9cbe6/prompt.md",
+    "manifest": ".ddx/executions/20260426T174734-7ea9cbe6/manifest.json",
+    "result": ".ddx/executions/20260426T174734-7ea9cbe6/result.json",
+    "checks": ".ddx/executions/20260426T174734-7ea9cbe6/checks.json",
+    "usage": ".ddx/executions/20260426T174734-7ea9cbe6/usage.json",
+    "worktree": "tmp/ddx-exec-wt/.execute-bead-wt-heimq-17753e60-20260426T174734-7ea9cbe6"
+  }
+}
\ No newline at end of file
diff --git a/.ddx/executions/20260426T174734-7ea9cbe6/result.json b/.ddx/executions/20260426T174734-7ea9cbe6/result.json
new file mode 100644
index 0000000..19b4dd0
--- /dev/null
+++ b/.ddx/executions/20260426T174734-7ea9cbe6/result.json
@@ -0,0 +1,22 @@
+{
+  "bead_id": "heimq-17753e60",
+  "attempt_id": "20260426T174734-7ea9cbe6",
+  "base_rev": "8202c8c991f9b8954364b861c5766db0bd2f0c92",
+  "result_rev": "20e8155083aeb543468c09b5bc3392e2e391ec65",
+  "outcome": "task_succeeded",
+  "status": "success",
+  "detail": "success",
+  "harness": "claude",
+  "session_id": "eb-43f75b9b",
+  "duration_ms": 59915,
+  "tokens": 3842,
+  "cost_usd": 0.44381575,
+  "exit_code": 0,
+  "execution_dir": ".ddx/executions/20260426T174734-7ea9cbe6",
+  "prompt_file": ".ddx/executions/20260426T174734-7ea9cbe6/prompt.md",
+  "manifest_file": ".ddx/executions/20260426T174734-7ea9cbe6/manifest.json",
+  "result_file": ".ddx/executions/20260426T174734-7ea9cbe6/result.json",
+  "usage_file": ".ddx/executions/20260426T174734-7ea9cbe6/usage.json",
+  "started_at": "2026-04-26T17:47:34.560564732Z",
+  "finished_at": "2026-04-26T17:48:34.475657398Z"
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
