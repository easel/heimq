<bead-review>
  <bead id="heimq-3af6652c" iter=1>
    <title>Integration test matrix: kcat-stress against each backend combo</title>
    <description>
Run the kcat-stress script against each sensible backend combination and assert behavior is consistent.

Combinations initially:
- log=memory, offsets=memory, groups=memory (default)
- log=memory, offsets=postgres, groups=memory (durability slice)

Script extension: a wrapper scripts/stress-matrix.sh that loops over combos, setting HEIMQ_STORAGE_LOG / HEIMQ_STORAGE_OFFSETS / HEIMQ_STORAGE_GROUPS env vars per iteration, running the existing kcat-stress.sh at moderate volume (e.g. 5 topics x 10k msgs).

In-scope files:
- scripts/stress-matrix.sh (new)
- docs/helix/03-test/test-plan/test-plan.md (add Phase 6: backend matrix)
Depends on: postgres backend bead
    </description>
    <acceptance>
stress-matrix.sh runs all configured combos and exits 0. If POSTGRES_URL is unset, the postgres row is skipped with a clear SKIPPED line (not FAIL). CI gets a new (optional) job that runs the matrix against a postgres service container.
    </acceptance>
    <labels>area:storage, kind:feat, phase:backends, step:10-test-matrix</labels>
  </bead>

  <changed-files>
    <file>.ddx/executions/20260426T040547-a01a840d/manifest.json</file>
    <file>.ddx/executions/20260426T040547-a01a840d/result.json</file>
  </changed-files>

  <governing>
    <note>No governing documents found. Evaluate the diff against the acceptance criteria alone.</note>
  </governing>

  <diff rev="45793cc242fa185d7c8dc3642aed174c73725abf">
diff --git a/.ddx/executions/20260426T040547-a01a840d/manifest.json b/.ddx/executions/20260426T040547-a01a840d/manifest.json
new file mode 100644
index 0000000..d6baef9
--- /dev/null
+++ b/.ddx/executions/20260426T040547-a01a840d/manifest.json
@@ -0,0 +1,38 @@
+{
+  "attempt_id": "20260426T040547-a01a840d",
+  "bead_id": "heimq-3af6652c",
+  "base_rev": "d22c8050e8c7fd5ddc35f7130854f071a14fd03d",
+  "created_at": "2026-04-26T04:05:47.661535757Z",
+  "requested": {
+    "harness": "claude",
+    "prompt": "synthesized"
+  },
+  "bead": {
+    "id": "heimq-3af6652c",
+    "title": "Integration test matrix: kcat-stress against each backend combo",
+    "description": "Run the kcat-stress script against each sensible backend combination and assert behavior is consistent.\n\nCombinations initially:\n- log=memory, offsets=memory, groups=memory (default)\n- log=memory, offsets=postgres, groups=memory (durability slice)\n\nScript extension: a wrapper scripts/stress-matrix.sh that loops over combos, setting HEIMQ_STORAGE_LOG / HEIMQ_STORAGE_OFFSETS / HEIMQ_STORAGE_GROUPS env vars per iteration, running the existing kcat-stress.sh at moderate volume (e.g. 5 topics x 10k msgs).\n\nIn-scope files:\n- scripts/stress-matrix.sh (new)\n- docs/helix/03-test/test-plan/test-plan.md (add Phase 6: backend matrix)\nDepends on: postgres backend bead",
+    "acceptance": "stress-matrix.sh runs all configured combos and exits 0. If POSTGRES_URL is unset, the postgres row is skipped with a clear SKIPPED line (not FAIL). CI gets a new (optional) job that runs the matrix against a postgres service container.",
+    "parent": "heimq-2136e295",
+    "labels": [
+      "area:storage",
+      "kind:feat",
+      "phase:backends",
+      "step:10-test-matrix"
+    ],
+    "metadata": {
+      "claimed-at": "2026-04-26T04:05:47Z",
+      "claimed-machine": "eitri",
+      "claimed-pid": "4075646",
+      "execute-loop-heartbeat-at": "2026-04-26T04:05:47.566816149Z"
+    }
+  },
+  "paths": {
+    "dir": ".ddx/executions/20260426T040547-a01a840d",
+    "prompt": ".ddx/executions/20260426T040547-a01a840d/prompt.md",
+    "manifest": ".ddx/executions/20260426T040547-a01a840d/manifest.json",
+    "result": ".ddx/executions/20260426T040547-a01a840d/result.json",
+    "checks": ".ddx/executions/20260426T040547-a01a840d/checks.json",
+    "usage": ".ddx/executions/20260426T040547-a01a840d/usage.json",
+    "worktree": "tmp/ddx-exec-wt/.execute-bead-wt-heimq-3af6652c-20260426T040547-a01a840d"
+  }
+}
\ No newline at end of file
diff --git a/.ddx/executions/20260426T040547-a01a840d/result.json b/.ddx/executions/20260426T040547-a01a840d/result.json
new file mode 100644
index 0000000..4d2e467
--- /dev/null
+++ b/.ddx/executions/20260426T040547-a01a840d/result.json
@@ -0,0 +1,22 @@
+{
+  "bead_id": "heimq-3af6652c",
+  "attempt_id": "20260426T040547-a01a840d",
+  "base_rev": "d22c8050e8c7fd5ddc35f7130854f071a14fd03d",
+  "result_rev": "390b2b00aebd964c69fbb2241bc3cd3b7dd49e26",
+  "outcome": "task_succeeded",
+  "status": "success",
+  "detail": "success",
+  "harness": "claude",
+  "session_id": "eb-974260a4",
+  "duration_ms": 114375,
+  "tokens": 7094,
+  "cost_usd": 0.7414562500000001,
+  "exit_code": 0,
+  "execution_dir": ".ddx/executions/20260426T040547-a01a840d",
+  "prompt_file": ".ddx/executions/20260426T040547-a01a840d/prompt.md",
+  "manifest_file": ".ddx/executions/20260426T040547-a01a840d/manifest.json",
+  "result_file": ".ddx/executions/20260426T040547-a01a840d/result.json",
+  "usage_file": ".ddx/executions/20260426T040547-a01a840d/usage.json",
+  "started_at": "2026-04-26T04:05:47.661795673Z",
+  "finished_at": "2026-04-26T04:07:42.037294766Z"
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
