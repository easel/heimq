<bead-review>
  <bead id="heimq-f0049303" iter=1>
    <title>Cite benchmark acceptance coverage</title>
    <description>
Resolve the benchmark portion of AR13-05. Map US-006-AC1 through AC4 and US-007-AC1 through AC3 to the smoke/OMB scripts, checked-in profiles, workload files, and CI workflows. Add exact @covers comments to executable shell/YAML surfaces and add static verification where needed. Do not claim throughput targets. In scope: scripts/bench/, .github/workflows/bench-smoke.yml, .github/workflows/bench-omb.yml. Out of scope: changing benchmark load.
    </description>
    <acceptance>
1. The HELIX alignment checker reports no uncited AC for US-006 or US-007. 2. bash -n scripts/bench/run-smoke.sh scripts/bench/run-omb.sh passes. 3. All cited profiles and workload files exist and workflow YAML parses.
    </acceptance>
    <labels>helix, area:testing, area:infra, kind:traceability, ac-quality:needs-refinement</labels>
  </bead>

  <changed-files>
    <file>.ddx/executions/20260714T054626-9418aab3/helix-align-check.json</file>
    <file>.github/workflows/bench-omb.yml</file>
    <file>.github/workflows/bench-smoke.yml</file>
    <file>scripts/bench/run-omb.sh</file>
    <file>scripts/bench/run-smoke.sh</file>
    <file>scripts/bench/verify-coverage.sh</file>
  </changed-files>

  <governing>
    <ref id="FEAT-004" path="crates/heimq/docs/helix/01-frame/features/FEAT-004-benchmark-conformance.md" title="Feature Specification: FEAT-004 — Standard Kafka Benchmark Conformance">
      <content>
<untrusted-data>
---
ddx:
  id: FEAT-004
  depends_on:
    - helix.prd
    - FEAT-001
    - FEAT-002
    - FEAT-006
  review:
    self_hash: 84531399bf0bf4d4ff962e6f516c45506d67340944a8ba924e6010b4a39b64a6
    deps:
      FEAT-001: 7133c264bc364ec4535c1d6b6187a90c9ba66d1fa3df30731ade260c2e092479
      FEAT-002: 164350929c7bbc09a589f3cd1a80b685e88cce1054445fe5373aec566464636f
      FEAT-006: e59d3b8965ebd35b4bbe9c5302f4218432ad83ec27691989dfd4c345ac2ae004
      helix.prd: 96f0479e307f2c240e8f47b69fff510164d0b9eda132abb22cc4a860932984fe
    reviewed_at: "2026-07-14T05:12:26Z"
---
# Feature Specification: FEAT-004 — Standard Kafka Benchmark Conformance

**Feature ID**: FEAT-004
**Status**: Specified
**Priority**: P0
**Owner**: heimq core
**Covered PRD Subsystem(s)**: Benchmark conformance (FEAT-004)
**Covered PRD Requirements**: FR-9 (standard Kafka benchmark conformance) — PRD P0 #6
**Cross-Subsystem Rationale**: None — single subsystem.

## Overview

heimq runs the standard, commonly-used Kafka benchmarks to completion
without protocol or client errors. This is a conformance test, not a
performance target — it verifies that the API surface and client
state machines exercised by these tools are correct.

## Ideal Future State

`kafka-producer-perf-test`, `kafka-consumer-perf-test`, and the
OpenMessaging Benchmark Kafka driver all run to completion against heimq
at documented, checked-in load profiles with zero protocol/client errors,
exercising the protocol corners (admin RPCs, metadata refresh,
large-batch produce, idempotent/transactional flows under load) that unit
tests miss.

## Problem Statement

- **Current situation**: heimq has no benchmark harness; no standard
  Kafka benchmark has been run end-to-end against it.
- **Pain points**: Standard benchmarks exercise protocol corners that
  unit tests miss (admin RPCs, repeated metadata refresh, large-batch
  produce, idempotent / transactional flows under load). Without them,
  divergences only surface in production tools.
- **Desired outcome**: `kafka-producer-perf-test`,
  `kafka-consumer-perf-test`, and the OpenMessaging Benchmark Kafka
  driver all complete against heimq at a documented load profile,
  reporting zero protocol/client errors.

## Requirements

### Functional Requirements

- **FR-01** — A bench harness (`scripts/bench/`) invokes
  `kafka-producer-perf-test` against heimq with a documented load
  profile (record size, total records, throughput cap) and asserts
  exit 0 with no error lines.
- **FR-02** — Same for `kafka-consumer-perf-test`.
- **FR-03** — A driver runs the **latest released** OpenMessaging Benchmark Kafka
  driver against heimq for at least one documented workload (e.g., 1KB
  records, N partitions, M producers, K consumers, capped duration)
  and asserts it completes without protocol/client errors. Version
  pin: upstream tag `jms` at commit
  `c0e51b8b86a3b0ff50b935152d6e600602a7f0a0`; bumps are tracked as
  ordinary maintenance.
- **FR-04** — The bench harness is runnable locally and in CI; it is not on the
  default test path but is gated on protocol-touching changes.
- **FR-05** — Idempotent and transactional bench profiles are included (e.g.,
  `kafka-producer-perf-test --producer-props enable.idempotence=true
  transactional.id=...`) and complete without errors.

### Non-Functional Requirements

- **Reliability**: Bench harness pass rate ≥ 99% on the gating workload.
- **Reproducibility**: Each profile is a checked-in script with pinned
  client / tool versions.
- **Performance**: Bench harness wall-clock budget ≤ 30 min on CI
  hardware (this is a budget, not a throughput target — heimq is not
  competing on throughput).

## User Stories

- [US-006 — Run kafka-producer-perf-test against heimq](../user-stories/US-006-kafka-perf-test.md)
- [US-007 — Run OpenMessaging Benchmark against heimq](../user-stories/US-007-openmessaging-benchmark.md)

## Edge Cases and Error Handling

- **Tool requires admin APIs heimq does not implement**: capture the
  required API in a parking-lot item and either implement a minimal
  stub or document the non-support with a workaround flag if the tool
  exposes one.
- **Tool assumes durability**: document as a deliberate divergence
  per PRD non-goal #1.

## Success Metrics

- All listed standard benchmarks complete with zero protocol/client
  errors at their gating profile.
- Each benchmark profile is checked in with its expected exit code and
  acceptable warnings list.

## Constraints and Assumptions

- We are not asserting throughput or latency targets — only conformance
  (the tool runs and exits cleanly).
- Standard Kafka tooling is available in the bench environment.

## Dependencies

- **Other features**: FEAT-001 (wire protocol), FEAT-002 (idempotent /
  transactional bench profiles), FEAT-006 (flexible-version protocol —
  `kafka-producer-perf-test` / `kafka-consumer-perf-test` / OMB ship the
  modern Java client, which default-negotiates flexible versions; see
  PRD P0 #1).
- **External services**: Apache Kafka tooling distribution (for
  `kafka-*-perf-test`); OpenMessaging Benchmark repository.
- **PRD requirements**: P0 #6.

## Out of Scope

- Throughput / latency targets relative to Kafka or Redpanda.
- Benchmarks that target out-of-scope APIs (e.g., share-group
  benchmarks).
</untrusted-data>
      </content>
    </ref>
  </governing>

  <diff rev="11b9dc6d140798a40ad627acc0dd39eae32dd5ef">
<untrusted-data>
diff --git a/scripts/bench/run-omb.sh b/scripts/bench/run-omb.sh
index 1cb8549..c501864 100755
--- a/scripts/bench/run-omb.sh
+++ b/scripts/bench/run-omb.sh
@@ -5,6 +5,10 @@
 #   - Docker + the ombbuild-heimq image (built once; see below)
 #   - heimq running on BOOTSTRAP (default localhost:9094)
 #
+# @covers US-007-AC1
+# @covers US-007-AC2
+# @covers US-007-AC3
+#
 # Build the image once:
 #   docker build --network host \
 #     -f scripts/bench/openmessaging/Dockerfile.omb-heimq \
@@ -56,6 +60,7 @@ if echo "$omb_out" | grep -qiE "ERROR Benchmark|Exception in thread|FATAL"; then
     exit 1
 fi
 
+# @covers US-007-AC1
 if ! echo "$omb_out" | grep -q "Writing test result"; then
     echo "FAIL: OMB did not complete (no result file written, exit=$omb_exit)" >&2
     exit 1
diff --git a/scripts/bench/run-smoke.sh b/scripts/bench/run-smoke.sh
index 1d7b176..caab546 100755
--- a/scripts/bench/run-smoke.sh
+++ b/scripts/bench/run-smoke.sh
@@ -3,6 +3,11 @@
 # Requirements: Kafka CLI tools on PATH, heimq running on localhost:9094.
 # Exit code: 0 on success, non-zero on any protocol/client error line.
 #
+# @covers US-006-AC1
+# @covers US-006-AC2
+# @covers US-006-AC3
+# @covers US-006-AC4
+#
 # Usage: BOOTSTRAP=localhost:9094 ./scripts/bench/run-smoke.sh
 
 set -euo pipefail
@@ -31,6 +36,7 @@ echo "==> [1/3] Non-idempotent producer + consumer"
 kafka-topics.sh --bootstrap-server "$BOOTSTRAP" --create --topic "$TOPIC" \
     --partitions 1 --replication-factor 1
 
+# @covers US-006-AC1
 run_check kafka-producer-perf-test.sh \
     --producer-props bootstrap.servers="$BOOTSTRAP" acks=1 enable.idempotence=false \
     --topic "$TOPIC" \
@@ -38,6 +44,7 @@ run_check kafka-producer-perf-test.sh \
     --record-size "$RECORD_SIZE" \
     --throughput -1
 
+# @covers US-006-AC2
 run_check kafka-consumer-perf-test.sh \
     --bootstrap-server "$BOOTSTRAP" \
     --consumer.config "$SCRIPT_DIR/profiles/consumer-smoke.properties" \
@@ -56,6 +63,7 @@ echo "==> [2/3] Idempotent producer (enable.idempotence=true)"
 kafka-topics.sh --bootstrap-server "$BOOTSTRAP" --create --topic "$TOPIC_IDEM" \
     --partitions 1 --replication-factor 1
 
+# @covers US-006-AC3
 run_check kafka-producer-perf-test.sh \
     --producer.config "$SCRIPT_DIR/profiles/producer-idempotent.properties" \
     --topic "$TOPIC_IDEM" \
@@ -75,6 +83,7 @@ echo "==> [3/3] Transactional producer (transactional.id)"
 kafka-topics.sh --bootstrap-server "$BOOTSTRAP" --create --topic "$TOPIC_TXN" \
     --partitions 1 --replication-factor 1
 
+# @covers US-006-AC3
 run_check kafka-producer-perf-test.sh \
     --producer.config "$SCRIPT_DIR/profiles/producer-transactional.properties" \
     --topic "$TOPIC_TXN" \
diff --git a/.ddx/executions/20260714T054626-9418aab3/helix-align-check.json b/.ddx/executions/20260714T054626-9418aab3/helix-align-check.json
new file mode 100644
index 0000000..e87df7e
--- /dev/null
+++ b/.ddx/executions/20260714T054626-9418aab3/helix-align-check.json
@@ -0,0 +1,579 @@
+{
+  "docs_root": "/home/erik/.cache/ddx/exec-wt/.execute-bead-wt-heimq-f0049303-20260714T054626-9418aab3/crates/heimq/docs/helix",
+  "code_root": "/home/erik/.cache/ddx/exec-wt/.execute-bead-wt-heimq-f0049303-20260714T054626-9418aab3",
+  "prd": "/home/erik/.cache/ddx/exec-wt/.execute-bead-wt-heimq-f0049303-20260714T054626-9418aab3/crates/heimq/docs/helix/01-frame/prd.md",
+  "parser_warnings": [],
+  "dimensions": {
+    "decomposition": {
+      "subsystems": [
+        "Wire protocol (FEAT-001 + FEAT-006)",
+        "Core semantics (FEAT-002)",
+        "Differential parity (FEAT-003)",
+        "Benchmark conformance (FEAT-004)",
+        "Ecosystem integrations (FEAT-005)",
+        "Durable offset backend (FEAT-007)",
+        "Engine trait surface (TRAIT-001)"
+      ],
+      "frs": [
+        "FR-1",
+        "FR-2",
+        "FR-3",
+        "FR-4",
+        "FR-5",
+        "FR-6",
+        "FR-7",
+        "FR-8",
+        "FR-9",
+        "FR-10",
+        "FR-11",
+        "FR-12",
+        "FR-13"
+      ],
+      "feats": {
+        "FEAT-001": {
+          "file": "/home/erik/.cache/ddx/exec-wt/.execute-bead-wt-heimq-f0049303-20260714T054626-9418aab3/crates/heimq/docs/helix/01-frame/features/FEAT-001-wire-protocol-compatibility.md",
+          "subsystems": [
+            "Wire protocol (FEAT-001 + FEAT-006)"
+          ],
+          "requirements": [
+            "FR-1",
+            "FR-2",
+            "FR-3",
+            "FR-4"
+          ],
+          "cross_rationale": true
+        },
+        "FEAT-002": {
+          "file": "/home/erik/.cache/ddx/exec-wt/.execute-bead-wt-heimq-f0049303-20260714T054626-9418aab3/crates/heimq/docs/helix/01-frame/features/FEAT-002-core-kafka-semantics.md",
+          "subsystems": [
+            "Core semantics (FEAT-002)"
+          ],
+          "requirements": [
+            "FR-5",
+            "FR-6",
+            "FR-7"
+          ],
+          "cross_rationale": true
+        },
+        "FEAT-003": {
+          "file": "/home/erik/.cache/ddx/exec-wt/.execute-bead-wt-heimq-f0049303-20260714T054626-9418aab3/crates/heimq/docs/helix/01-frame/features/FEAT-003-differential-parity-testing.md",
+          "subsystems": [
+            "Differential parity (FEAT-003)"
+          ],
+          "requirements": [
+            "FR-8"
+          ],
+          "cross_rationale": true
+        },
+        "FEAT-004": {
+          "file": "/home/erik/.cache/ddx/exec-wt/.execute-bead-wt-heimq-f0049303-20260714T054626-9418aab3/crates/heimq/docs/helix/01-frame/features/FEAT-004-benchmark-conformance.md",
+          "subsystems": [
+            "Benchmark conformance (FEAT-004)"
+          ],
+          "requirements": [
+            "FR-9"
+          ],
+          "cross_rationale": true
+        },
+        "FEAT-005": {
+          "file": "/home/erik/.cache/ddx/exec-wt/.execute-bead-wt-heimq-f0049303-20260714T054626-9418aab3/crates/heimq/docs/helix/01-frame/features/FEAT-005-ecosystem-integrations.md",
+          "subsystems": [
+            "Ecosystem integrations (FEAT-005)"
+          ],
+          "requirements": [
+            "FR-10"
+          ],
+          "cross_rationale": true
+        },
+        "FEAT-006": {
+          "file": "/home/erik/.cache/ddx/exec-wt/.execute-bead-wt-heimq-f0049303-20260714T054626-9418aab3/crates/heimq/docs/helix/01-frame/features/FEAT-006-flexible-version-protocol.md",
+          "subsystems": [
+            "Wire protocol (FEAT-001 + FEAT-006)"
+          ],
+          "requirements": [
+            "FR-4"
+          ],
+          "cross_rationale": true
+        },
+        "FEAT-007": {
+          "file": "/home/erik/.cache/ddx/exec-wt/.execute-bead-wt-heimq-f0049303-20260714T054626-9418aab3/crates/heimq/docs/helix/01-frame/features/FEAT-007-durable-offset-backend.md",
+          "subsystems": [
+            "Durable offset backend (FEAT-007)"
+          ],
+          "requirements": [
+            "FR-11"
+          ],
+          "cross_rationale": true
+        },
+        "FEAT-008": {
+          "file": "/home/erik/.cache/ddx/exec-wt/.execute-bead-wt-heimq-f0049303-20260714T054626-9418aab3/crates/heimq/docs/helix/01-frame/features/FEAT-008-engine-embedding-contract.md",
+          "subsystems": [
+            "Engine trait surface (TRAIT-001)"
+          ],
+          "requirements": [
+            "FR-12",
+            "FR-13"
+          ],
+          "cross_rationale": true
+        }
+      },
+      "stories": {
+        "US-001": {
+          "feature": "FEAT-001",
+          "acs": [
+            "US-001-AC1",
+            "US-001-AC2",
+            "US-001-AC3",
+            "US-001-AC4"
+          ]
+        },
+        "US-002": {
+          "feature": "FEAT-002",
+          "acs": [
+            "US-002-AC1",
+            "US-002-AC2",
+            "US-002-AC3",
+            "US-002-AC4",
+            "US-002-AC5"
+          ]
+        },
+        "US-003": {
+          "feature": "FEAT-002",
+          "acs": [
+            "US-003-AC1",
+            "US-003-AC2",
+            "US-003-AC3",
+            "US-003-AC4",
+            "US-003-AC5"
+          ]
+        },
+        "US-004": {
+          "feature": "FEAT-002",
+          "acs": [
+            "US-004-AC1",
+            "US-004-AC10",
+            "US-004-AC11",
+            "US-004-AC2",
+            "US-004-AC3",
+            "US-004-AC4",
+            "US-004-AC5",
+            "US-004-AC6",
+            "US-004-AC7",
+            "US-004-AC8",
+            "US-004-AC9"
+          ]
+        },
+        "US-005": {
+          "feature": "FEAT-003",
+          "acs": [
+            "US-005-AC1",
+            "US-005-AC2",
+            "US-005-AC3",
+            "US-005-AC4",
+            "US-005-AC5",
+            "US-005-AC6",
+            "US-005-AC7"
+          ]
+        },
+        "US-006": {
+          "feature": "FEAT-004",
+          "acs": [
+            "US-006-AC1",
+            "US-006-AC2",
+            "US-006-AC3",
+            "US-006-AC4"
+          ]
+        },
+        "US-007": {
+          "feature": "FEAT-004",
+          "acs": [
+            "US-007-AC1",
+            "US-007-AC2",
+            "US-007-AC3"
+          ]
+        },
+        "US-008": {
+          "feature": "FEAT-005",
+          "acs": [
+            "US-008-AC1",
+            "US-008-AC2",
+            "US-008-AC3",
+            "US-008-AC4"
+          ]
+        },
+        "US-009": {
+          "feature": "FEAT-005",
+          "acs": [
+            "US-009-AC1",
+            "US-009-AC2",
+            "US-009-AC3"
+          ]
+        },
+        "US-010": {
+          "feature": "FEAT-005",
+          "acs": [
+            "US-010-AC1",
+            "US-010-AC2",
+            "US-010-AC3"
+          ]
+        },
+        "US-011": {
+          "feature": "FEAT-005",
+          "acs": [
+            "US-011-AC1",
+            "US-011-AC2",
+            "US-011-AC3",
+            "US-011-AC4"
+          ]
+        },
+        "US-012": {
+          "feature": "FEAT-005",
+          "acs": [
+            "US-012-AC1",
+            "US-012-AC2",
+            "US-012-AC3",
+            "US-012-AC4"
+          ]
+        },
+        "US-013": {
+          "feature": "FEAT-006",
+          "acs": [
+            "US-013-AC1",
+            "US-013-AC2",
+            "US-013-AC3",
+            "US-013-AC4"
+          ]
+        },
+        "US-014": {
+          "feature": "FEAT-005",
+          "acs": [
+            "US-014-AC1",
+            "US-014-AC2",
+            "US-014-AC3"
+          ]
+        },
+        "US-015": {
+          "feature": "FEAT-007",
+          "acs": [
+            "US-015-AC1",
+            "US-015-AC2",
+            "US-015-AC3",
+            "US-015-AC4"
+          ]
+        },
+        "US-016": {
+          "feature": "FEAT-008",
+          "acs": [
+            "US-016-AC1",
+            "US-016-AC2",
+            "US-016-AC3",
+            "US-016-AC4"
+          ]
+        },
+        "US-017": {
+          "feature": "FEAT-008",
+          "acs": [
+            "US-017-AC1",
+            "US-017-AC2",
+            "US-017-AC3",
+            "US-017-AC4",
+            "US-017-AC5"
+          ]
+        }
+      }
+    },
+    "acceptance": {
+      "acs_found": 77,
+      "acs_cited": [
+        "US-001-AC1",
+        "US-001-AC4",
+        "US-002-AC1",
+        "US-002-AC2",
+        "US-002-AC3",
+        "US-002-AC4",
+        "US-002-AC5",
+        "US-003-AC1",
+        "US-003-AC2",
+        "US-003-AC5",
+        "US-004-AC1",
+        "US-004-AC10",
+        "US-004-AC11",
+        "US-004-AC2",
+        "US-004-AC3",
+        "US-004-AC4",
+        "US-004-AC5",
+        "US-004-AC6",
+        "US-004-AC7",
+        "US-004-AC8",
+        "US-004-AC9",
+        "US-005-AC1",
+        "US-005-AC2",
+        "US-005-AC3",
+        "US-005-AC4",
+        "US-005-AC5",
+        "US-005-AC6",
+        "US-005-AC7",
+        "US-006-AC1",
+        "US-006-AC2",
+        "US-006-AC3",
+        "US-006-AC4",
+        "US-007-AC1",
+        "US-007-AC2",
+        "US-007-AC3",
+        "US-008-AC1",
+        "US-008-AC2",
+        "US-008-AC3",
+        "US-008-AC4",
+        "US-009-AC1",
+        "US-009-AC2",
+        "US-009-AC3",
+        "US-010-AC1",
+        "US-010-AC2",
+        "US-010-AC3",
+        "US-011-AC1",
+        "US-011-AC2",
+        "US-011-AC3",
+        "US-011-AC4",
+        "US-012-AC1",
+        "US-012-AC2",
+        "US-012-AC3",
+        "US-012-AC4",
+        "US-013-AC3",
+        "US-014-AC1",
+        "US-014-AC2",
+        "US-014-AC3",
+        "US-015-AC1",
+        "US-015-AC3",
+        "US-016-AC1",
+        "US-016-AC2",
+        "US-016-AC3",
+        "US-016-AC4",
+        "US-017-AC2",
+        "US-017-AC4",
+        "US-017-AC5"
+      ],
+      "acs_uncited": [
+        "US-001-AC2",
+        "US-001-AC3",
+        "US-003-AC3",
+        "US-003-AC4",
+        "US-013-AC1",
+        "US-013-AC2",
+        "US-013-AC4",
+        "US-015-AC2",
+        "US-015-AC4",
+        "US-017-AC1",
+        "US-017-AC3"
+      ],
+      "dangling_citations": []
+    },
+    "adr": {
+      "found": 7,
+      "by_status": {
+        "accepted": 7
+      },
+      "items": [
+        {
+          "id": "ADR-001",
+          "file": "/home/erik/.cache/ddx/exec-wt/.execute-bead-wt-heimq-f0049303-20260714T054626-9418aab3/crates/heimq/docs/helix/02-design/adr/ADR-001-kafka-client-for-testing.md",
+          "status": "accepted"
+        },
+        {
+          "id": "ADR-002",
+          "file": "/home/erik/.cache/ddx/exec-wt/.execute-bead-wt-heimq-f0049303-20260714T054626-9418aab3/crates/heimq/docs/helix/02-design/adr/ADR-002-bead-close-test-gate.md",
+          "status": "accepted"
+        },
+        {
+          "id": "ADR-003",
+          "file": "/home/erik/.cache/ddx/exec-wt/.execute-bead-wt-heimq-f0049303-20260714T054626-9418aab3/crates/heimq/docs/helix/02-design/adr/ADR-003-flexible-version-codec-strategy.md",
+          "status": "accepted"
+        },
+        {
+          "id": "ADR-004",
+          "file": "/home/erik/.cache/ddx/exec-wt/.execute-bead-wt-heimq-f0049303-20260714T054626-9418aab3/crates/heimq/docs/helix/02-design/adr/ADR-004-openmessaging-benchmark-version.md",
+          "status": "accepted"
+        },
+        {
+          "id": "ADR-005",
+          "file": "/home/erik/.cache/ddx/exec-wt/.execute-bead-wt-heimq-f0049303-20260714T054626-9418aab3/crates/heimq/docs/helix/02-design/adr/ADR-005-schema-registry-target.md",
+          "status": "accepted"
+        },
+        {
+          "id": "ADR-006",
+          "file": "/home/erik/.cache/ddx/exec-wt/.execute-bead-wt-heimq-f0049303-20260714T054626-9418aab3/crates/heimq/docs/helix/02-design/adr/ADR-006-no-state-across-restart.md",
+          "status": "accepted"
+        },
+        {
+          "id": "ADR-007",
+          "file": "/home/erik/.cache/ddx/exec-wt/.execute-bead-wt-heimq-f0049303-20260714T054626-9418aab3/crates/heimq/docs/helix/02-design/adr/ADR-007-workspace-split-engine-crates.md",
+          "status": "accepted"
+        }
+      ],
+      "naming_violations": []
+    },
+    "nfr": {
+      "candidate_artifacts": [
+        "/home/erik/.cache/ddx/exec-wt/.execute-bead-wt-heimq-f0049303-20260714T054626-9418aab3/crates/heimq/docs/helix/01-frame/features/FEAT-001-wire-protocol-compatibility.md",
+        "/home/erik/.cache/ddx/exec-wt/.execute-bead-wt-heimq-f0049303-20260714T054626-9418aab3/crates/heimq/docs/helix/01-frame/features/FEAT-002-core-kafka-semantics.md",
+        "/home/erik/.cache/ddx/exec-wt/.execute-bead-wt-heimq-f0049303-20260714T054626-9418aab3/crates/heimq/docs/helix/01-frame/features/FEAT-003-differential-parity-testing.md",
+        "/home/erik/.cache/ddx/exec-wt/.execute-bead-wt-heimq-f0049303-20260714T054626-9418aab3/crates/heimq/docs/helix/01-frame/features/FEAT-004-benchmark-conformance.md",
+        "/home/erik/.cache/ddx/exec-wt/.execute-bead-wt-heimq-f0049303-20260714T054626-9418aab3/crates/heimq/docs/helix/01-frame/features/FEAT-005-ecosystem-integrations.md",
+        "/home/erik/.cache/ddx/exec-wt/.execute-bead-wt-heimq-f0049303-20260714T054626-9418aab3/crates/heimq/docs/helix/01-frame/features/FEAT-006-flexible-version-protocol.md",
+        "/home/erik/.cache/ddx/exec-wt/.execute-bead-wt-heimq-f0049303-20260714T054626-9418aab3/crates/heimq/docs/helix/01-frame/features/FEAT-007-durable-offset-backend.md",
+        "/home/erik/.cache/ddx/exec-wt/.execute-bead-wt-heimq-f0049303-20260714T054626-9418aab3/crates/heimq/docs/helix/01-frame/features/FEAT-008-engine-embedding-contract.md",
+        "/home/erik/.cache/ddx/exec-wt/.execute-bead-wt-heimq-f0049303-20260714T054626-9418aab3/crates/heimq/docs/helix/01-frame/user-stories/US-002-consumer-group-rebalance.md",
+        "/home/erik/.cache/ddx/exec-wt/.execute-bead-wt-heimq-f0049303-20260714T054626-9418aab3/crates/heimq/docs/helix/03-test/test-plan/test-plan.md",
+        "/home/erik/.cache/ddx/exec-wt/.execute-bead-wt-heimq-f0049303-20260714T054626-9418aab3/crates/heimq/docs/helix/06-iterate/alignment-reviews/AR-2026-07-13-repo.md"
+      ]
+    }
+  },
+  "coverage_matrix_inputs": [
+    {
+      "dimension": "Capability (FR)",
+      "computed": true,
+      "found": 13,
+      "checked": 13,
+      "blocking_findings": 0,
+      "advisory_findings": 0,
+      "structural_status": "clean (structural floor)",
+      "detail": "13 FRs; 13 covered by a FEAT",
+      "model_completes": "verify each FR is implemented in code (code->spec surface map)"
+    },
+    {
+      "dimension": "Acceptance behavior",
+      "computed": true,
+      "found": 77,
+      "checked": 77,
+      "blocking_findings": 0,
+      "advisory_findings": 11,
+      "structural_status": "advisory-findings",
+      "detail": "66/77 ACs have a @covers citation; 0 dangling",
+      "model_completes": "verify each cited test EXERCISES that exact AC; classify uncited ACs UNTESTED vs UNCITED_COVERAGE"
+    },
+    {
+      "dimension": "Architecture decision (ADR)",
+      "computed": true,
+      "found": 7,
+      "checked": 7,
+      "blocking_findings": 0,
+      "advisory_findings": 0,
+      "structural_status": "clean (structural floor)",
+      "detail": "7 ADRs; status {'accepted': 7}",
+      "model_completes": "verify each accepted ADR's decision is honored in code; no contradicting surface"
+    },
+    {
+      "dimension": "Concern practice",
+      "computed": false,
+      "found": null,
+      "checked": null,
+      "blocking_findings": 0,
+      "advisory_findings": 0,
+      "structural_status": "model-required",
+      "detail": "this checker does not inspect concern practices",
+      "model_completes": "verify each active concern's behavioral practices are realized in code (not just tooling wired)"
+    },
+    {
+      "dimension": "Measurable NFR / budget",
+      "computed": false,
+      "found": null,
+      "checked": null,
+      "blocking_findings": 0,
+      "advisory_findings": 0,
+      "structural_status": "model-required",
+      "detail": "11 candidate artifact(s) mention NFR / non-functional (search pointer, NOT an NFR-item count)",
+      "model_completes": "extract stated NFR targets and verify each is met / has evidence"
+    },
+    {
+      "dimension": "Decomposition",
+      "computed": true,
+      "found": 32,
+      "checked": 32,
+      "blocking_findings": 0,
+      "advisory_findings": 0,
+      "structural_status": "clean (structural floor)",
+      "detail": "7 subsystems / 8 FEATs / 17 stories",
+      "model_completes": null
+    },
+    {
+      "dimension": "Slot / instrument",
+      "computed": false,
+      "found": null,
+      "checked": null,
+      "blocking_findings": 0,
+      "advisory_findings": 0,
+      "structural_status": "model-required",
+      "detail": "Slot-Registry + Instrument-Integrity read the workflows/concerns tree, not this docs-root",
+      "model_completes": "run the deterministic Slot-Registry Integrity check (slots.yml vs concern ## Slot) separately"
+    }
+  ],
+  "findings": [
+    {
+      "check": "ac-citation",
+      "severity": "advisory",
+      "classification": "NO_CITATION",
+      "detail": "US-001-AC2 has no @covers citation in the scanned code tree \u2014 deterministic fact only; the model classifies UNTESTED vs UNCITED_COVERAGE by checking for an exercising test"
+    },
+    {
+      "check": "ac-citation",
+      "severity": "advisory",
+      "classification": "NO_CITATION",
+      "detail": "US-001-AC3 has no @covers citation in the scanned code tree \u2014 deterministic fact only; the model classifies UNTESTED vs UNCITED_COVERAGE by checking for an exercising test"
+    },
+    {
+      "check": "ac-citation",
+      "severity": "advisory",
+      "classification": "NO_CITATION",
+      "detail": "US-003-AC3 has no @covers citation in the scanned code tree \u2014 deterministic fact only; the model classifies UNTESTED vs UNCITED_COVERAGE by checking for an exercising test"
+    },
+    {
+      "check": "ac-citation",
+      "severity": "advisory",
+      "classification": "NO_CITATION",
+      "detail": "US-003-AC4 has no @covers citation in the scanned code tree \u2014 deterministic fact only; the model classifies UNTESTED vs UNCITED_COVERAGE by checking for an exercising test"
+    },
+    {
+      "check": "ac-citation",
+      "severity": "advisory",
+      "classification": "NO_CITATION",
+      "detail": "US-013-AC1 has no @covers citation in the scanned code tree \u2014 deterministic fact only; the model classifies UNTESTED vs UNCITED_COVERAGE by checking for an exercising test"
+    },
+    {
+      "check": "ac-citation",
+      "severity": "advisory",
+      "classification": "NO_CITATION",
+      "detail": "US-013-AC2 has no @covers citation in the scanned code tree \u2014 deterministic fact only; the model classifies UNTESTED vs UNCITED_COVERAGE by checking for an exercising test"
+    },
+    {
+      "check": "ac-citation",
+      "severity": "advisory",
+      "classification": "NO_CITATION",
+      "detail": "US-013-AC4 has no @covers citation in the scanned code tree \u2014 deterministic fact only; the model classifies UNTESTED vs UNCITED_COVERAGE by checking for an exercising test"
+    },
+    {
+      "check": "ac-citation",
+      "severity": "advisory",
+      "classification": "NO_CITATION",
+      "detail": "US-015-AC2 has no @covers citation in the scanned code tree \u2014 deterministic fact only; the model classifies UNTESTED vs UNCITED_COVERAGE by checking for an exercising test"
+    },
+    {
+      "check": "ac-citation",
+      "severity": "advisory",
+      "classification": "NO_CITATION",
+      "detail": "US-015-AC4 has no @covers citation in the scanned code tree \u2014 deterministic fact only; the model classifies UNTESTED vs UNCITED_COVERAGE by checking for an exercising test"
+    },
+    {
+      "check": "ac-citation",
+      "severity": "advisory",
+      "classification": "NO_CITATION",
+      "detail": "US-017-AC1 has no @covers citation in the scanned code tree \u2014 deterministic fact only; the model classifies UNTESTED vs UNCITED_COVERAGE by checking for an exercising test"
+    },
+    {
+      "check": "ac-citation",
+      "severity": "advisory",
+      "classification": "NO_CITATION",
+      "detail": "US-017-AC3 has no @covers citation in the scanned code tree \u2014 deterministic fact only; the model classifies UNTESTED vs UNCITED_COVERAGE by checking for an exercising test"
+    }
+  ],
+  "blocking_count": 0,
+  "note": "Deterministic structural floor only \u2014 these are INPUTS to the Dimension Coverage Matrix, not the matrix itself. The script assigns no Step 4 classification. Semantic verdicts (cited-test-exercises-that-AC, ADR-decision-honored, concern-behavior-realized, NFR-target-met) and the model-required rows are the model's job in reconcile-alignment STEP 3."
+}
diff --git a/.github/workflows/bench-omb.yml b/.github/workflows/bench-omb.yml
index 9d6bec2..ae2a8cc 100644
--- a/.github/workflows/bench-omb.yml
+++ b/.github/workflows/bench-omb.yml
@@ -27,6 +27,8 @@ jobs:
         run: git clone --no-checkout --filter=blob:none https://github.com/openmessaging/benchmark /tmp/ombbench && git -C /tmp/ombbench checkout c0e51b8b86a3b0ff50b935152d6e600602a7f0a0
 
       - name: Build OMB Docker image (cached)
+        # @covers US-007-AC2
+        # @covers US-007-AC3
         uses: docker/build-push-action@v5
         with:
           context: /tmp/ombbench
@@ -47,6 +49,8 @@ jobs:
           timeout 15 bash -c 'until nc -z localhost 9094; do sleep 0.2; done'
 
       - name: Run OMB conformance
+        # @covers US-007-AC1
+        # @covers US-007-AC3
         run: BOOTSTRAP=localhost:9094 bash scripts/bench/run-omb.sh
 
       - name: Stop heimq
diff --git a/.github/workflows/bench-smoke.yml b/.github/workflows/bench-smoke.yml
index 1d9d732..b6c6183 100644
--- a/.github/workflows/bench-smoke.yml
+++ b/.github/workflows/bench-smoke.yml
@@ -30,6 +30,10 @@ jobs:
           timeout 15 bash -c 'until nc -z localhost 9094; do sleep 0.2; done'
 
       - name: Run bench smoke
+        # @covers US-006-AC1
+        # @covers US-006-AC2
+        # @covers US-006-AC3
+        # @covers US-006-AC4
         run: BOOTSTRAP=localhost:9094 bash scripts/bench/run-smoke.sh
 
       - name: Stop heimq
diff --git a/scripts/bench/verify-coverage.sh b/scripts/bench/verify-coverage.sh
new file mode 100755
index 0000000..fa6a7cf
--- /dev/null
+++ b/scripts/bench/verify-coverage.sh
@@ -0,0 +1,54 @@
+#!/usr/bin/env bash
+# Static traceability checks for benchmark acceptance coverage.
+
+set -euo pipefail
+
+ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
+cd "$ROOT"
+
+required_acs=(
+    US-006-AC1
+    US-006-AC2
+    US-006-AC3
+    US-006-AC4
+    US-007-AC1
+    US-007-AC2
+    US-007-AC3
+)
+
+for ac in "${required_acs[@]}"; do
+    if ! grep -R -E "@covers[[:space:]]+$ac([^0-9]|$)" \
+        scripts/bench .github/workflows/bench-smoke.yml .github/workflows/bench-omb.yml \
+        >/dev/null; then
+        echo "missing @covers citation for $ac" >&2
+        exit 1
+    fi
+done
+
+required_files=(
+    scripts/bench/profiles/producer-smoke.properties
+    scripts/bench/profiles/consumer-smoke.properties
+    scripts/bench/profiles/producer-idempotent.properties
+    scripts/bench/profiles/producer-transactional.properties
+    scripts/bench/openmessaging/driver-heimq.yaml
+    scripts/bench/openmessaging/workload-smoke.yaml
+    scripts/bench/openmessaging/Dockerfile.omb-heimq
+)
+
+for path in "${required_files[@]}"; do
+    if [[ ! -f "$path" ]]; then
+        echo "required benchmark file does not exist: $path" >&2
+        exit 1
+    fi
+done
+
+ruby -e '
+  require "yaml"
+  ARGV.each { |path| YAML.load_file(path) }
+' \
+    .github/workflows/bench-smoke.yml \
+    .github/workflows/bench-omb.yml \
+    scripts/bench/openmessaging/driver-heimq.yaml \
+    scripts/bench/openmessaging/workload-smoke.yaml
+
+echo "benchmark coverage citations and YAML parse checks passed"
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
