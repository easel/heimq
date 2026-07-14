<bead-review>
  <bead id="heimq-a97ba3ff" iter=1>
    <title>Pin the OpenMessaging Benchmark source used by CI</title>
    <description>
Resolve AR13-03. ADR-004 requires the OMB harness to pin the latest released driver, but .github/workflows/bench-omb.yml currently clones default-branch HEAD and scripts/bench/run-omb.sh documents the same floating clone. Determine the current latest released OMB tag from the authoritative upstream repository, pin CI and documentation to that immutable tag or commit, and record the chosen version in ADR-004/FEAT-004. In scope: bench-omb workflow, OMB run docs/scripts, ADR-004 and FEAT-004 version statement. Out of scope: changing workload semantics.
    </description>
    <acceptance>
1. rg -n 'git clone' .github/workflows/bench-omb.yml scripts/bench/run-omb.sh shows an explicit tag/commit checkout and no floating default-branch build. 2. The pin matches the version recorded in ADR-004 and FEAT-004. 3. bash -n scripts/bench/run-omb.sh passes and actionlint .github/workflows/bench-omb.yml passes when actionlint is available.
    </acceptance>
    <labels>helix, area:infra, area:testing, kind:reproducibility</labels>
  </bead>

  <changed-files>
    <file>.github/workflows/bench-omb.yml</file>
    <file>crates/heimq/docs/helix/01-frame/features/FEAT-004-benchmark-conformance.md</file>
    <file>crates/heimq/docs/helix/02-design/adr/ADR-004-openmessaging-benchmark-version.md</file>
    <file>scripts/bench/run-omb.sh</file>
  </changed-files>

  <governing>
    <ref id="ADR-004" path="crates/heimq/docs/helix/02-design/adr/ADR-004-openmessaging-benchmark-version.md" title="ADR-004: OpenMessaging Benchmark Version Targeting">
      <content>
<untrusted-data>
---
ddx:
  id: ADR-004
  status: accepted
  review:
    self_hash: a56e385e34701cc479f3d7ade7959461b3b46777305a0cda3fca01c3a69d0edf
    deps: {}
    reviewed_at: "2026-06-22T21:30:26Z"
---

# ADR-004: OpenMessaging Benchmark Version Targeting

| Date | Status | Deciders | Related | Confidence |
|------|--------|----------|---------|------------|
| 2026-06-11 | Accepted | heimq maintainers | FEAT-004 | High |

> Extracted from PRD §Resolved Decisions during the 2026-06-11 alignment pass.

## Context

FEAT-004 (benchmark conformance) requires heimq to run under the OpenMessaging Benchmark Kafka driver. The PRD ([../../01-frame/prd.md](../../01-frame/prd.md)) needed a fixed answer to which OpenMessaging Benchmark version the bench harness builds against, so that conformance results are reproducible. See [FEAT-004](../../01-frame/features/FEAT-004-benchmark-conformance.md).

## Decision

OpenMessaging Benchmark version: target the latest released driver. The bench harness pins to that release; bumps are tracked as ordinary maintenance.

## Alternatives

No alternatives were recorded in the PRD; the implied alternative — tracking an unpinned/floating driver version — is rejected because unreproducible harness builds would make benchmark regressions unattributable.

## Consequences

| Type | Impact |
|------|--------|
| Positive | Reproducible benchmark runs; clear upgrade path via ordinary maintenance bumps. |
| Negative | The pin can lag upstream releases until a maintenance bump lands. |
| Neutral | Version bumps are routine work items, not architecture changes. |

## References

- [PRD §Resolved Decisions](../../01-frame/prd.md)
- [FEAT-004 benchmark conformance](../../01-frame/features/FEAT-004-benchmark-conformance.md)
</untrusted-data>
      </content>
    </ref>
  </governing>

  <diff rev="ac7a19f4328c5f06371ce089bfc8a0f83e790272">
<untrusted-data>
diff --git a/.github/workflows/bench-omb.yml b/.github/workflows/bench-omb.yml
index 5816c7f..9d6bec2 100644
--- a/.github/workflows/bench-omb.yml
+++ b/.github/workflows/bench-omb.yml
@@ -1,7 +1,8 @@
 name: bench-omb
 
 # FEAT-004 FR-03: OpenMessaging Benchmark conformance.
-# Builds OMB from source (cached), then runs a 1-minute smoke workload
+# Builds OMB tag jms (c0e51b8b86a3b0ff50b935152d6e600602a7f0a0) from source
+# (cached), then runs a 1-minute smoke workload
 # against heimq. Not a required gate (slow Docker build), but must be
 # green before FEAT-004 close.
 
@@ -23,7 +24,7 @@ jobs:
         uses: docker/setup-buildx-action@v3
 
       - name: Clone OpenMessaging Benchmark
-        run: git clone --depth 1 https://github.com/openmessaging/benchmark /tmp/ombbench
+        run: git clone --no-checkout --filter=blob:none https://github.com/openmessaging/benchmark /tmp/ombbench && git -C /tmp/ombbench checkout c0e51b8b86a3b0ff50b935152d6e600602a7f0a0
 
       - name: Build OMB Docker image (cached)
         uses: docker/build-push-action@v5
diff --git a/scripts/bench/run-omb.sh b/scripts/bench/run-omb.sh
index 25b2207..1cb8549 100755
--- a/scripts/bench/run-omb.sh
+++ b/scripts/bench/run-omb.sh
@@ -2,7 +2,7 @@
 # OpenMessaging Benchmark conformance run against heimq.
 #
 # Requirements:
-#   - Docker + the ombbuild-heimq image (built once — see below)
+#   - Docker + the ombbuild-heimq image (built once; see below)
 #   - heimq running on BOOTSTRAP (default localhost:9094)
 #
 # Build the image once:
@@ -11,8 +11,8 @@
 #     -t ombbuild-heimq \
 #     /tmp/ombbench
 #
-# (Clone OMB first if needed:
-#   git clone --depth 1 https://github.com/openmessaging/benchmark /tmp/ombbench)
+# (Clone OMB first if needed; this is OMB tag jms pinned to its current commit:
+#   git clone --no-checkout --filter=blob:none https://github.com/openmessaging/benchmark /tmp/ombbench && git -C /tmp/ombbench checkout c0e51b8b86a3b0ff50b935152d6e600602a7f0a0)
 #
 # Usage:
 #   BOOTSTRAP=localhost:9094 ./scripts/bench/run-omb.sh
diff --git a/crates/heimq/docs/helix/02-design/adr/ADR-004-openmessaging-benchmark-version.md b/crates/heimq/docs/helix/02-design/adr/ADR-004-openmessaging-benchmark-version.md
index b207462..d2217ce 100644
--- a/crates/heimq/docs/helix/02-design/adr/ADR-004-openmessaging-benchmark-version.md
+++ b/crates/heimq/docs/helix/02-design/adr/ADR-004-openmessaging-benchmark-version.md
@@ -22,7 +22,7 @@ FEAT-004 (benchmark conformance) requires heimq to run under the OpenMessaging B
 
 ## Decision
 
-OpenMessaging Benchmark version: target the latest released driver. The bench harness pins to that release; bumps are tracked as ordinary maintenance.
+OpenMessaging Benchmark version: pin the latest released upstream tag available from `openmessaging/benchmark`, `jms`, at commit `c0e51b8b86a3b0ff50b935152d6e600602a7f0a0`. The bench harness checks out that commit; bumps are tracked as ordinary maintenance.
 
 ## Alternatives
 
diff --git a/crates/heimq/docs/helix/01-frame/features/FEAT-004-benchmark-conformance.md b/crates/heimq/docs/helix/01-frame/features/FEAT-004-benchmark-conformance.md
index 62b85c0..391b9bc 100644
--- a/crates/heimq/docs/helix/01-frame/features/FEAT-004-benchmark-conformance.md
+++ b/crates/heimq/docs/helix/01-frame/features/FEAT-004-benchmark-conformance.md
@@ -67,7 +67,9 @@ tests miss.
   driver against heimq for at least one documented workload (e.g., 1KB
   records, N partitions, M producers, K consumers, capped duration)
   and asserts it completes without protocol/client errors. Version
-  bumps are tracked as ordinary maintenance.
+  pin: upstream tag `jms` at commit
+  `c0e51b8b86a3b0ff50b935152d6e600602a7f0a0`; bumps are tracked as
+  ordinary maintenance.
 - **FR-04** — The bench harness is runnable locally and in CI; it is not on the
   default test path but is gated on protocol-touching changes.
 - **FR-05** — Idempotent and transactional bench profiles are included (e.g.,
</untrusted-data>
  </diff>

  <strictness-mode mode="strict">strict — each AC must be anchored to a named Test* function or a diff-touched symbol; file-only evidence is insufficient.</strictness-mode>

  <instructions>
You are reviewing a bead implementation against its acceptance criteria.

## AC-Check Ratification

When an &lt;ac-check&gt; section is present, ratify the mechanical results rather
than re-verifying them independently from the diff:

- result="pass": confirm the evidence is credible. Override to fail only if
  the evidence is fabricated — include judgment_override_reason and a diff
  citation (file:line) in the per_ac evidence string.
- result="fail": mechanically verified failure. Grade as fail and BLOCK unless
  the commit message contains an explicit AC-Waive trailer for this AC.
- result="needs_judgment": adjudicate from the diff. If you cannot determine
  pass/fail without additional bead context from the operator, use
  REQUEST_CLARIFICATION for that AC item.
- result="error": treat as needs_judgment.

Overriding a mechanical grade (pass→fail or fail→pass) requires an explicit
judgment_override_reason note and a concrete diff citation in the evidence.

## Strictness Mode

The &lt;strictness-mode&gt; tag specifies per-bead evidence requirements:

- strict (kind:fix, kind:feat): each AC must be anchored to a named Test*
  function or a diff-touched symbol; file-only evidence is insufficient.
- behavior-light (kind:refactor, kind:chore): build green plus file/symbol
  evidence suffices; test-name match required only when an AC explicitly
  names a Test* function.
- mechanical (kind:doc, kind:mechanical): file presence, renames, or symbol
  evidence only; no test-name or runtime evidence required.

## Verdicts

For each acceptance-criteria (AC) item, decide whether it is implemented
correctly, then assign one overall verdict:

- APPROVE — every AC item is fully and correctly implemented.
- REQUEST_CHANGES — some AC items are partial or have fixable minor issues.
- BLOCK — at least one AC item is not implemented or incorrectly implemented;
  or the diff is insufficient to evaluate.
- REQUEST_CLARIFICATION — you cannot adjudicate one or more needs_judgment AC
  items without operator clarification. Use this ONLY when the item is
  ambiguous even given the full diff. This verdict does NOT block the queue;
  it routes to the operator lane for input.

## Required output format (schema_version: 1)

Respond with EXACTLY one JSON object as your final response, fenced as a single ```json … ``` code block. Do not include any prose outside the fenced block. The JSON must match this schema:

```json
{
  "schema_version": 1,
  "verdict": "APPROVE",
  "summary": "≤300 char human-readable verdict justification",
  "per_ac": [
    { "number": 1, "item": "acceptance criterion text", "grade": "pass", "evidence": "file:line or test evidence" }
  ],
  "findings": [
    { "severity": "info", "summary": "what is wrong or notable", "location": "path/to/file.go:42" }
  ]
}
```

Rules:
- "verdict" must be exactly one of "APPROVE", "REQUEST_CHANGES", "BLOCK", "REQUEST_CLARIFICATION".
- "severity" must be exactly one of "info", "warn", "block".
- Output the JSON object inside ONE fenced ```json … ``` block. No additional prose, no extra fences, no markdown headings.
- Do not echo this template back. Do not write the verdict value anywhere except as the JSON value of the verdict field.
  </instructions>
</bead-review>
