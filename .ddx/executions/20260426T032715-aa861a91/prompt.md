<bead-review>
  <bead id="heimq-c894fcda" iter=1>
    <title>Postgres OffsetStore backend (feature-gated)</title>
    <description>
First durable backend. Stores committed offsets in a Postgres database. Feature-gated: `--features backend-postgres`.

Schema:
  CREATE TABLE heimq_committed_offsets (
    group_id TEXT NOT NULL,
    topic TEXT NOT NULL,
    partition INT NOT NULL,
    committed_offset BIGINT NOT NULL,
    metadata TEXT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (group_id, topic, partition)
  );

Implementation:
- sqlx or tokio-postgres dependency, gated by feature.
- PostgresOffsetStore::connect(url) returns Arc&lt;dyn OffsetStore&gt;.
- BackendCapabilities: durability=WalFsync, survives_restart=true, transactions=false (this trait doesn't expose them), read_your_writes=true.
- URL scheme: postgres://user:pw@host:port/db?schema=heimq
- Schema initialize() idempotent at startup.

In-scope files:
- Cargo.toml (feature flag + sqlx dep)
- src/storage/postgres_offsets.rs (new)
- src/storage/dispatch.rs (register postgres://)
- tests/postgres_offsets.rs (integration test, uses a containerized postgres via testcontainers or skip if env var HEIMQ_PG_TEST_URL unset)
Depends on: OffsetStore trait, URL dispatch
    </description>
    <acceptance>
`cargo test --features backend-postgres` green. Integration test: commit an offset via OffsetCommit handler backed by PostgresOffsetStore, restart the TestServer, fetch the same offset, assert it survived. Test gracefully skips if no postgres URL is configured in CI.
    </acceptance>
    <labels>area:storage, kind:feat, phase:backends, step:9-postgres-offsets</labels>
  </bead>

  <changed-files>
    <file>.ddx/executions/20260426T031958-15d6ae2a/manifest.json</file>
    <file>.ddx/executions/20260426T031958-15d6ae2a/result.json</file>
  </changed-files>

  <governing>
    <note>No governing documents found. Evaluate the diff against the acceptance criteria alone.</note>
  </governing>

  <diff rev="c69a136bb4d543846a63ca98309df4f223e0f9ef">
diff --git a/.ddx/executions/20260426T031958-15d6ae2a/manifest.json b/.ddx/executions/20260426T031958-15d6ae2a/manifest.json
new file mode 100644
index 0000000..1af6d1d
--- /dev/null
+++ b/.ddx/executions/20260426T031958-15d6ae2a/manifest.json
@@ -0,0 +1,38 @@
+{
+  "attempt_id": "20260426T031958-15d6ae2a",
+  "bead_id": "heimq-c894fcda",
+  "base_rev": "479ddb9965bb461449ee335f3fe44586b63ced8d",
+  "created_at": "2026-04-26T03:19:58.213271089Z",
+  "requested": {
+    "harness": "claude",
+    "prompt": "synthesized"
+  },
+  "bead": {
+    "id": "heimq-c894fcda",
+    "title": "Postgres OffsetStore backend (feature-gated)",
+    "description": "First durable backend. Stores committed offsets in a Postgres database. Feature-gated: `--features backend-postgres`.\n\nSchema:\n  CREATE TABLE heimq_committed_offsets (\n    group_id TEXT NOT NULL,\n    topic TEXT NOT NULL,\n    partition INT NOT NULL,\n    committed_offset BIGINT NOT NULL,\n    metadata TEXT,\n    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),\n    PRIMARY KEY (group_id, topic, partition)\n  );\n\nImplementation:\n- sqlx or tokio-postgres dependency, gated by feature.\n- PostgresOffsetStore::connect(url) returns Arc\u003cdyn OffsetStore\u003e.\n- BackendCapabilities: durability=WalFsync, survives_restart=true, transactions=false (this trait doesn't expose them), read_your_writes=true.\n- URL scheme: postgres://user:pw@host:port/db?schema=heimq\n- Schema initialize() idempotent at startup.\n\nIn-scope files:\n- Cargo.toml (feature flag + sqlx dep)\n- src/storage/postgres_offsets.rs (new)\n- src/storage/dispatch.rs (register postgres://)\n- tests/postgres_offsets.rs (integration test, uses a containerized postgres via testcontainers or skip if env var HEIMQ_PG_TEST_URL unset)\nDepends on: OffsetStore trait, URL dispatch",
+    "acceptance": "`cargo test --features backend-postgres` green. Integration test: commit an offset via OffsetCommit handler backed by PostgresOffsetStore, restart the TestServer, fetch the same offset, assert it survived. Test gracefully skips if no postgres URL is configured in CI.",
+    "parent": "heimq-2136e295",
+    "labels": [
+      "area:storage",
+      "kind:feat",
+      "phase:backends",
+      "step:9-postgres-offsets"
+    ],
+    "metadata": {
+      "claimed-at": "2026-04-26T03:19:58Z",
+      "claimed-machine": "eitri",
+      "claimed-pid": "4075646",
+      "execute-loop-heartbeat-at": "2026-04-26T03:19:58.125895082Z"
+    }
+  },
+  "paths": {
+    "dir": ".ddx/executions/20260426T031958-15d6ae2a",
+    "prompt": ".ddx/executions/20260426T031958-15d6ae2a/prompt.md",
+    "manifest": ".ddx/executions/20260426T031958-15d6ae2a/manifest.json",
+    "result": ".ddx/executions/20260426T031958-15d6ae2a/result.json",
+    "checks": ".ddx/executions/20260426T031958-15d6ae2a/checks.json",
+    "usage": ".ddx/executions/20260426T031958-15d6ae2a/usage.json",
+    "worktree": "tmp/ddx-exec-wt/.execute-bead-wt-heimq-c894fcda-20260426T031958-15d6ae2a"
+  }
+}
\ No newline at end of file
diff --git a/.ddx/executions/20260426T031958-15d6ae2a/result.json b/.ddx/executions/20260426T031958-15d6ae2a/result.json
new file mode 100644
index 0000000..dbc6fa6
--- /dev/null
+++ b/.ddx/executions/20260426T031958-15d6ae2a/result.json
@@ -0,0 +1,22 @@
+{
+  "bead_id": "heimq-c894fcda",
+  "attempt_id": "20260426T031958-15d6ae2a",
+  "base_rev": "479ddb9965bb461449ee335f3fe44586b63ced8d",
+  "result_rev": "5603c67ba8c12ec3fb7ebae76e550b06a616d4bb",
+  "outcome": "task_succeeded",
+  "status": "success",
+  "detail": "success",
+  "harness": "claude",
+  "session_id": "eb-6e458e9b",
+  "duration_ms": 435860,
+  "tokens": 26121,
+  "cost_usd": 2.8924687499999995,
+  "exit_code": 0,
+  "execution_dir": ".ddx/executions/20260426T031958-15d6ae2a",
+  "prompt_file": ".ddx/executions/20260426T031958-15d6ae2a/prompt.md",
+  "manifest_file": ".ddx/executions/20260426T031958-15d6ae2a/manifest.json",
+  "result_file": ".ddx/executions/20260426T031958-15d6ae2a/result.json",
+  "usage_file": ".ddx/executions/20260426T031958-15d6ae2a/usage.json",
+  "started_at": "2026-04-26T03:19:58.213512713Z",
+  "finished_at": "2026-04-26T03:27:14.0739286Z"
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
