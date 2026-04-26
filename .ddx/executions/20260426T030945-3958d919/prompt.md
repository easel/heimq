<bead-review>
  <bead id="heimq-fb3e5ca8" iter=1>
    <title>Extract GroupCoordinatorBackend trait</title>
    <description>
Unlike a simple load/save KV, the group coordinator is live state: members, generations, assignments, sticky session behavior, rebalance state machine. Per codex review, this trait must be richer than a snapshot store.

Trait surface (initial):
- join_group(group, member, protocols) -&gt; JoinResult { generation, member_id, leader, ... }
- sync_group(group, generation, member, assignments) -&gt; SyncResult
- heartbeat(group, generation, member) -&gt; HeartbeatResult
- leave_group(group, member) -&gt; LeaveResult
- commit_offset / fetch_offset could either be here OR in OffsetStore — for the first cut, keep OffsetStore separate (already its own bead).
- capabilities()

Memory impl wraps current ConsumerGroupManager logic; no behavior change.

In-scope files:
- src/consumer_group/mod.rs (trait + memory impl wiring)
- src/consumer_group/coordinator.rs (refactor)
- src/handler/{join_group,sync_group,heartbeat,leave_group}.rs (take Arc&lt;dyn GroupCoordinatorBackend&gt;)
Out of scope:
- Any durable group backend (separate future bead)
- Rebalance-delay fix (separate bead, filed as flakiness bug)
    </description>
    <acceptance>
`cargo test --workspace` green. kcat stress script group-consumer flow passes with default stagger.
    </acceptance>
    <labels>area:storage, kind:feat, phase:backends, step:5-group-trait</labels>
  </bead>

  <changed-files>
    <file>.ddx/executions/20260426T030128-76595357/manifest.json</file>
    <file>.ddx/executions/20260426T030128-76595357/result.json</file>
  </changed-files>

  <governing>
    <note>No governing documents found. Evaluate the diff against the acceptance criteria alone.</note>
  </governing>

  <diff rev="c099d19d10524a0e2f7395858db8963f938b6bd1">
diff --git a/.ddx/executions/20260426T030128-76595357/manifest.json b/.ddx/executions/20260426T030128-76595357/manifest.json
new file mode 100644
index 0000000..3029f79
--- /dev/null
+++ b/.ddx/executions/20260426T030128-76595357/manifest.json
@@ -0,0 +1,38 @@
+{
+  "attempt_id": "20260426T030128-76595357",
+  "bead_id": "heimq-fb3e5ca8",
+  "base_rev": "5c8e7606656e88a372c6b8ec637babfe828ba8bd",
+  "created_at": "2026-04-26T03:01:28.478343149Z",
+  "requested": {
+    "harness": "claude",
+    "prompt": "synthesized"
+  },
+  "bead": {
+    "id": "heimq-fb3e5ca8",
+    "title": "Extract GroupCoordinatorBackend trait",
+    "description": "Unlike a simple load/save KV, the group coordinator is live state: members, generations, assignments, sticky session behavior, rebalance state machine. Per codex review, this trait must be richer than a snapshot store.\n\nTrait surface (initial):\n- join_group(group, member, protocols) -\u003e JoinResult { generation, member_id, leader, ... }\n- sync_group(group, generation, member, assignments) -\u003e SyncResult\n- heartbeat(group, generation, member) -\u003e HeartbeatResult\n- leave_group(group, member) -\u003e LeaveResult\n- commit_offset / fetch_offset could either be here OR in OffsetStore — for the first cut, keep OffsetStore separate (already its own bead).\n- capabilities()\n\nMemory impl wraps current ConsumerGroupManager logic; no behavior change.\n\nIn-scope files:\n- src/consumer_group/mod.rs (trait + memory impl wiring)\n- src/consumer_group/coordinator.rs (refactor)\n- src/handler/{join_group,sync_group,heartbeat,leave_group}.rs (take Arc\u003cdyn GroupCoordinatorBackend\u003e)\nOut of scope:\n- Any durable group backend (separate future bead)\n- Rebalance-delay fix (separate bead, filed as flakiness bug)",
+    "acceptance": "`cargo test --workspace` green. kcat stress script group-consumer flow passes with default stagger.",
+    "parent": "heimq-2136e295",
+    "labels": [
+      "area:storage",
+      "kind:feat",
+      "phase:backends",
+      "step:5-group-trait"
+    ],
+    "metadata": {
+      "claimed-at": "2026-04-26T03:01:28Z",
+      "claimed-machine": "eitri",
+      "claimed-pid": "4075646",
+      "execute-loop-heartbeat-at": "2026-04-26T03:01:28.413812934Z"
+    }
+  },
+  "paths": {
+    "dir": ".ddx/executions/20260426T030128-76595357",
+    "prompt": ".ddx/executions/20260426T030128-76595357/prompt.md",
+    "manifest": ".ddx/executions/20260426T030128-76595357/manifest.json",
+    "result": ".ddx/executions/20260426T030128-76595357/result.json",
+    "checks": ".ddx/executions/20260426T030128-76595357/checks.json",
+    "usage": ".ddx/executions/20260426T030128-76595357/usage.json",
+    "worktree": "tmp/ddx-exec-wt/.execute-bead-wt-heimq-fb3e5ca8-20260426T030128-76595357"
+  }
+}
\ No newline at end of file
diff --git a/.ddx/executions/20260426T030128-76595357/result.json b/.ddx/executions/20260426T030128-76595357/result.json
new file mode 100644
index 0000000..b13c9a6
--- /dev/null
+++ b/.ddx/executions/20260426T030128-76595357/result.json
@@ -0,0 +1,22 @@
+{
+  "bead_id": "heimq-fb3e5ca8",
+  "attempt_id": "20260426T030128-76595357",
+  "base_rev": "5c8e7606656e88a372c6b8ec637babfe828ba8bd",
+  "result_rev": "be214d96cacf0e40e6a65630dc7b02585da4454d",
+  "outcome": "task_succeeded",
+  "status": "success",
+  "detail": "success",
+  "harness": "claude",
+  "session_id": "eb-510ea294",
+  "duration_ms": 495874,
+  "tokens": 31474,
+  "cost_usd": 4.767299249999999,
+  "exit_code": 0,
+  "execution_dir": ".ddx/executions/20260426T030128-76595357",
+  "prompt_file": ".ddx/executions/20260426T030128-76595357/prompt.md",
+  "manifest_file": ".ddx/executions/20260426T030128-76595357/manifest.json",
+  "result_file": ".ddx/executions/20260426T030128-76595357/result.json",
+  "usage_file": ".ddx/executions/20260426T030128-76595357/usage.json",
+  "started_at": "2026-04-26T03:01:28.478615941Z",
+  "finished_at": "2026-04-26T03:09:44.352749842Z"
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
