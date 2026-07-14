<bead-review>
  <bead id="heimq-23a20c6f" iter=1>
    <title>Make parity, benchmark, and ecosystem reliability targets executable</title>
    <description>
Resolve AR13-07. Decide and document a reproducible evidence window for FEAT-003 less-than-1-percent flake rate and FEAT-004/005 at-least-99-percent pass rates. Prefer a zero-failure required workflow invariant when historical-rate infrastructure would add unjustified machinery; otherwise add a bounded rolling summary. Update FEAT-003/004/005 and TP-001, then implement the corresponding CI ratchet. Do not claim a measured percentage without evidence.
    </description>
    <acceptance>
1. FEAT-003, FEAT-004, FEAT-005, and TP-001 define the same executable reliability rule and evidence window. 2. The relevant workflows enforce or emit that rule with a non-zero failure path. 3. No unsupported pass-rate or flake-rate result is asserted. 4. Workflow YAML parses and existing focused harness syntax checks pass.
    </acceptance>
    <labels>helix, area:testing, area:infra, kind:quality, ac-quality:needs-refinement</labels>
  </bead>

  <changed-files>
    <file>.github/workflows/bench-omb.yml</file>
    <file>.github/workflows/bench-smoke.yml</file>
    <file>.github/workflows/conformance.yml</file>
    <file>.github/workflows/ecosystem.yml</file>
    <file>crates/heimq/docs/helix/01-frame/features/FEAT-003-differential-parity-testing.md</file>
    <file>crates/heimq/docs/helix/01-frame/features/FEAT-004-benchmark-conformance.md</file>
    <file>crates/heimq/docs/helix/01-frame/features/FEAT-005-ecosystem-integrations.md</file>
    <file>crates/heimq/docs/helix/03-test/test-plan/test-plan.md</file>
    <file>scripts/ci/reliability-gate.sh</file>
  </changed-files>

  <governing>
    <ref id="AR-2026-07-13-repo" path="crates/heimq/docs/helix/06-iterate/alignment-reviews/AR-2026-07-13-repo.md" title="Alignment Review: repository">
      <content>
<untrusted-data>
---
ddx:
  id: AR-2026-07-13-repo
  type: alignment-review
  status: complete
  links:
    - kind: derived_from
      to: helix.product-vision
    - kind: derived_from
      to: helix.prd
---

# Alignment Review: repository

**Review Date**: 2026-07-13
**Scope**: repository
**Status**: complete
**Review Epic**: heimq-c7d1cba4
**Primary Governing Artifact**: `crates/heimq/docs/helix/01-frame/prd.md`

## Scope and Governing Artifacts

### Scope

- Product decomposition and bidirectional traceability
- Engine workspace boundaries and accepted ADR decisions
- Acceptance coverage, parity, benchmarks, and ecosystem verification
- Rust/Cargo concern realization, native plugin topology, and release surfaces

### Governing Artifacts

- `crates/heimq/docs/helix/00-discover/product-vision.md`
- `crates/heimq/docs/helix/01-frame/prd.md`
- `crates/heimq/docs/helix/01-frame/features/FEAT-001` through `FEAT-007`
- `crates/heimq/docs/helix/01-frame/user-stories/US-001` through `US-015`
- `crates/heimq/docs/helix/02-design/adr/ADR-001` through `ADR-007`
- `crates/heimq/docs/helix/02-design/contracts/{API-001,CODEC-001,TRAIT-001,WIRE-001}*.md`
- `crates/heimq/docs/helix/03-test/test-plan/test-plan.md`
- `crates/heimq/docs/helix/04-build/implementation-plan.md`

## Intent Summary

- **Vision**: ship both an embeddable Kafka engine and a fast, in-memory single-binary distribution.
- **Requirements**: FR-1 through FR-11 cover the Kafka distribution; FR-12 and FR-13 govern engine traits, request handlers, context propagation, capability gating, and deferred acknowledgements.
- **Features / Stories**: FEAT-001 through FEAT-007 and US-001 through US-015 govern FR-1 through FR-11. No feature or story owns FR-12 or FR-13.
- **Architecture / ADRs**: accepted ADRs govern test clients, close gates, codecs, benchmark versioning, Schema Registry, restart semantics, and workspace boundaries.
- **Technical Design**: WIRE-001 and TRAIT-001 define the engine extension points; SD-003 defines differential parity.
- **Test Plans**: TP-001 allocates protocol, integration, parity, benchmark, ecosystem, and backend conformance tests.
- **Implementation Plans**: IP-001 records the crate split and downstream adoption program, but predates the request-handler extraction.

## Dimension Coverage Matrix

| Dimension | In-scope items found | Checked | Findings | Classification | Evidence |
|---|---:|---:|---:|---|---|
| Capability (FR) | 13 | 13 | 2 | INCOMPLETE | `prd.md:284-301`; HELIX deterministic check reports FR-12/FR-13 absent from all FEAT headers |
| Acceptance behavior | 68 | 68 | 53 citation gaps | INCOMPLETE | HELIX deterministic check: 15 cited, 53 uncited, 0 dangling; existing test scenario rows and executable harnesses indicate mostly `UNCITED_COVERAGE` pending exact citation |
| Architecture decision | 7 | 7 | 2 | DIVERGENT | ADR-004 pin contradicted by `.github/workflows/bench-omb.yml:26`; ADR-007 four-crate boundary contradicted by `Cargo.toml:3-10` and `crates/heimq-handlers/` |
| Concern practice | 5 | 5 | 2 | INCOMPLETE | `concerns.md:26-30`; `Cargo.toml`, `rust-toolchain.toml`, `justfile`, `.github/workflows/test.yml` |
| Concern-to-artifact realization | 5 | 5 | 1 | UNDERSPECIFIED | `concerns.md:42-47` leaves Rust and verification overrides at “Needs ADR” |
| Measurable NFR / budget | 9 | 9 | 3 | INCOMPLETE | PRD startup <1s has no check; FEAT-003/004/005 reliability targets have no rolling measurement; origin/main parity/bench/ecosystem workflows are green |
| Decomposition | 7 subsystems / 7 FEATs / 15 stories | 29 | 3 | INCOMPLETE | deterministic subsystem/FR checks; Engine trait surface has no FEAT; FR-12/13 have no stories |
| Slot / instrument | 7 catalog slots / 5 selected concerns | 12 | 0 | ALIGNED | HELIX 0.10.2 catalog binds; `concerns.md:22-40` resolves required and N/A slots without duplicate fillers |

## Planning Stack Findings

| ID | Finding | Classification | Evidence | Destination / deliverable / next mode |
|---|---|---|---|---|
| AR13-01 | Engine trait and request-handler subsystem has no feature or user-story owner | INCOMPLETE | `prd.md:284-301`; FEAT headers under `01-frame/features/` | Feature Specification plus Broker Builder stories for FR-12/13; `frame`; cite PRD, TRAIT-001, WIRE-001 |
| AR13-02 | Workspace decision omits `heimq-handlers` and still calls the result a four-crate workspace | STALE_PLAN | `ADR-007:28-49`; root `Cargo.toml:3-10`; `crates/heimq-handlers/Cargo.toml:1-15` | ADR amendment plus IP-001 crate table and downstream consumption update; `evolve` |
| AR13-03 | Benchmark ADR requires a released-version pin, but the OMB workflow clones default-branch HEAD | DIVERGENT | `ADR-004:23-29`; `.github/workflows/bench-omb.yml:26`; `scripts/bench/run-omb.sh:14-16` | CI/script pin to a named release or immutable revision, recorded in ADR/FEAT; runtime handoff |
| AR13-04 | Distribution test references still name the removed `tests/parity/` and pre-split test locations | STALE_PLAN | `prd.md:72-76`; TP-001 test organization; current `tests/conformance/` and `crates/heimq/tests/` | PRD and TP path/evidence refresh; `evolve` |
| AR13-05 | 53 acceptance criteria lack canonical `@covers` citations | INCOMPLETE | deterministic output; scenario mappings in every US file; current test/harness inventory | Exact citations on tests/scripts that exercise each AC; add tests only where exercise is absent; runtime handoff |
| AR13-06 | Cold-start <1s budget has no executable measurement | INCOMPLETE | `prd.md:76`; no startup timing job/test under `.github/workflows/` or `tests/` | Test Plan row plus bounded startup/first-roundtrip check; runtime handoff |
| AR13-07 | Reliability targets (<1% flake or >=99% pass rate) have no rolling measurement or ratchet | INCOMPLETE | FEAT-003:78-82; FEAT-004:77-84; FEAT-005:96-102 | Define evidence window and CI summary/ratchet, or revise targets to an executable invariant; `evolve` then runtime handoff |
| AR13-08 | Rust concern practices and project deviations are not fully realized or authorized | UNDERSPECIFIED | `concerns.md:26,42-47`; HELIX rust-cargo concern; root Cargo/CI files | ADR for project Rust policy and explicit overrides; align lint/dependency gates or narrow the selected practice; `design` |
| AR13-09 | IP-001 and the closed program epic claimed downstream Niflheim adoption was complete, while Niflheim still pins an older Heimq tag and keeps its in-tree Produce request path | DIVERGENT | IP-001 Slice 9; `heimq-5c906acd` AC; upstream handler extraction `35e8a15`; release tag `v0.1.4` at `48e6e34`; Niflheim bead `niflheim-ba3e609c` | Heimq published the request-level handler seam in `v0.1.4`: `codec.rs` exposes request decode/response encode and `produce.rs` exposes async Produce dispatch over `LogBackend`/`RequestContext`. Niflheim can replace the stale external blocker with that tag after release, but its dependency bump and request-path delegation remain downstream work under `niflheim-ba3e609c`. |

## Implementation Map

| Surface | Evidence | Kind | Governing artifact | Mapping rationale | Classification |
|---|---|---|---|---|---|
| Kafka TCP framing and connection loop | `crates/heimq-wire/src/lib.rs` | capability | FEAT-001, WIRE-001 | wire crate owns framing and per-connection dispatch | ALIGNED |
| Request-level decode/dispatch/encode | `crates/heimq-handlers/src/{codec,lib,produce}.rs` | capability | FR-13, WIRE-001 | embedder-facing handler layer; missing feature/story owner | UNDERSPECIFIED |
| Backend trait families and request context | `crates/heimq-broker/src/{context,storage,consumer_group}/` | capability | FR-12, TRAIT-001 | stable embedder extension points | INCOMPLETE (planning trace only) |
| Per-trait conformance suites | `crates/heimq-testkit/src/suites/` | capability | FR-12, TRAIT-001 | executable backend contract | INCOMPLETE (planning trace only) |
| Standalone distribution | `crates/heimq/src/main.rs`, `server.rs` | capability | Product Vision, FEAT-001..007 | CLI composes engine crates and in-memory defaults | ALIGNED |
| Differential parity | `tests/conformance/` | capability | FEAT-003, SD-003 | two independent clients against Kafka and Redpanda | ALIGNED |
| Standard benchmarks | `scripts/bench/`, `.github/workflows/bench-*.yml` | capability | FEAT-004, ADR-004 | executable benchmark conformance | DIVERGENT (floating OMB source) |
| Ecosystem matrix | `tests/ecosystem/`, `.github/workflows/ecosystem.yml` | capability | FEAT-005 | eight real integration scripts | ALIGNED |
| Postgres offset backend | `crates/heimq/src/storage/postgres_offsets.rs` | capability | FEAT-007, ADR-006 | opt-in durable offsets only | ALIGNED |
| Docker, Helm, release binaries | `Dockerfile`, `charts/heimq/`, release workflows | technical | Product Vision, IP-001 | deployment and distribution projection | ALIGNED |
| Native agent plugins | user-level Codex/Claude plugin registries | technical | `.helix.yml`, AGENTS plugin policy | DDx 1.0.0, easel-skills 0.2.1, HELIX 0.10.2; no repo-local skill payloads | ALIGNED |

## ADR Decision Honoring

| ADR | Decision evidence | Classification |
|---|---|---|
| ADR-001 | rdkafka remains test-only; production crates do not depend on it | ALIGNED |
| ADR-002 | `.ddx/checks/workspace-tests.yaml` plus required GitHub `test` status check | ALIGNED |
| ADR-003 | vendored kafka-protocol 0.15.1 fork and generated version-aware codec | ALIGNED |
| ADR-004 | workflow clones unpinned OMB HEAD | DIVERGENT |
| ADR-005 | `tests/ecosystem/04-schema-registry.sh` uses Confluent Schema Registry 7.6.0 | ALIGNED |
| ADR-006 | producer/transaction state remains in-memory; Postgres persists offsets only | ALIGNED |
| ADR-007 | engine split exists, but handler extraction changed the selected boundary | STALE_PLAN |

## Acceptance Criteria Status

| Story group | Criteria | Current evidence | Status |
|---|---:|---|---|
| US-001 / US-013 wire and modern clients | 8 | contract, integration, parity suites; 6 unique IDs cited | UNCITED_COVERAGE for remaining IDs |
| US-002 groups | 5 | contract and integration group lifecycle; all IDs cited | SATISFIED |
| US-003 / US-004 idempotence and transactions | 16 | handler, contract, integration, parity tests; several generic `US-004` comments | UNCITED_COVERAGE until exact IDs replace generic comments |
| US-005 parity harness | 7 | Python and Go parity implementations, normalization, exemptions, CI | UNCITED_COVERAGE |
| US-006 / US-007 benchmarks | 7 | `run-smoke.sh`, `run-omb.sh`, profiles and workflows | UNCITED_COVERAGE |
| US-008..US-012 / US-014 ecosystem | 24 | eight executable scripts and ecosystem workflow | UNCITED_COVERAGE |
| US-015 Postgres offsets | 4 | conformance and backend tests plus stress matrix | UNCITED_COVERAGE |

## NFR Evidence

| Target | Evidence | Classification |
|---|---|---|
| 100% in-scope protocol contract coverage | `crates/heimq/tests/API_COVERAGE.md`, contract suite, green `test` workflow | ALIGNED |
| Zero unmatched differential diffs | `tests/conformance/`, green `conformance` workflow on origin/main | ALIGNED |
| Benchmark tools complete without errors | green `bench-smoke` and `bench-omb` workflows on origin/main | ALIGNED |
| Flexible codec property round-trips pass | workspace tests and handler codec tests | ALIGNED |
| Cold start through first roundtrip <1s | no executable timing evidence | INCOMPLETE |
| Parity runtime <10m and flake rate <1% | individual green run, no rolling flake evidence | INCOMPLETE |
| Benchmark runtime <=30m and pass rate >=99% | individual green runs, no rolling pass-rate evidence | INCOMPLETE |
| Ecosystem pass rate >=99% | individual green run, no rolling pass-rate evidence | INCOMPLETE |
| Single self-contained binary | GitHub release binary and Docker smoke evidence | ALIGNED |

## Gap Register

| Finding | Resolution direction | Execution issue |
|---|---|---|
| AR13-01 | plan-to-code traceability | heimq-a5061733 |
| AR13-02 | plan-to-code refresh | heimq-f88181b0 |
| AR13-03 | code-to-plan correction | heimq-a97ba3ff |
| AR13-04 | plan-to-code refresh | heimq-caf7b41c |
| AR13-05 | quality-improvement / traceability | heimq-86f66ee2, heimq-1cdbc19d, heimq-f0049303, heimq-3bfa949e, heimq-ed5504bc |
| AR13-06 | test implementation | heimq-cdd87b6a |
| AR13-07 | decision then measurement ratchet | heimq-23a20c6f |
| AR13-08 | decision and concern realization | heimq-08416254 |
| AR13-09 | truthful program status and cross-repo handoff | heimq-0e59fef5 verifies published upstream seam `35e8a15` / `v0.1.4` (`48e6e34`); heimq-168f24e2 and heimq-0362a710 cover API-stability and final publication gates; downstream implementation remains `niflheim-ba3e609c` |

## Execution Issues Generated

| Issue | Goal | Dependencies | Verification |
|---|---|---|---|
| heimq-a5061733 | Add FEAT-008 and Broker Builder stories for FR-12/13 | none | HELIX decomposition blockers reach zero |
| heimq-f88181b0 | Evolve ADR-007 and IP-001 for heimq-handlers and truthful Niflheim status | none | Cargo members and artifact boundaries agree |
| heimq-caf7b41c | Refresh live artifact paths after the split | none | stale path search and DDx document freshness |
| heimq-a97ba3ff | Pin OMB source | none | shell/action syntax plus immutable version evidence |
| heimq-86f66ee2 | Cite core protocol/semantic ACs | heimq-a5061733 | contract tests plus zero target citation gaps |
| heimq-1cdbc19d | Cite parity ACs | none | Python/Go runner checks plus zero target citation gaps |
| heimq-f0049303 | Cite benchmark ACs | heimq-a97ba3ff | shell/workflow checks plus zero target citation gaps |
| heimq-3bfa949e | Cite ecosystem ACs | none | shell checks plus zero target citation gaps |
| heimq-ed5504bc | Cite Postgres and engine conformance ACs | heimq-a5061733 | broker/handler/testkit tests plus zero target citation gaps |
| heimq-cdd87b6a | Enforce <1s startup through first roundtrip | none | three consecutive bounded real-client runs |
| heimq-23a20c6f | Make reliability targets executable and honest | none | consistent spec rule and CI ratchet |
| heimq-08416254 | Authorize and realize Rust concern policy | none | fmt, clippy, tests, deny, machete |
| heimq-0e59fef5 | Prove the published upstream handler seam and correct program claims | heimq-f88181b0 | `cargo test -p heimq-handlers`; `v0.1.4` contains `crates/heimq-handlers/src/codec.rs` and `produce.rs` public request-level APIs |
| heimq-168f24e2 | Stabilize and document the handler embedder API and SASL ownership | heimq-a5061733, heimq-f88181b0, heimq-ed5504bc | public API tests, rustdoc, workspace tests |
| heimq-0362a710 | Satisfy Niflheim's publication gate with a remotely visible release tag | heimq-a5061733, heimq-f88181b0, heimq-ed5504bc, heimq-168f24e2 | tag contents, remote visibility, downstream pin |

## Issue Coverage

Every non-ALIGNED finding AR13-01 through AR13-09 has executable coverage above. For AR13-09, Heimq's upstream prerequisite is satisfied by handler extraction commit `35e8a15` and release tag `v0.1.4` at `48e6e34`: the tag contains `crates/heimq-handlers/src/codec.rs` request decode/response encode helpers and `produce.rs` async Produce dispatch over `LogBackend`/`RequestContext`. Niflheim request-path delegation is explicitly delegated to `niflheim-ba3e609c`; Heimq owns publication of the prerequisite, not closure of the downstream bead.

## Execution Order

1. Establish missing engine feature/story authority and update the workspace ADR.
2. Refresh stale plan/test paths and exact acceptance citations.
3. Correct the OMB pin and add startup/reliability evidence gates.
4. Resolve Rust concern overrides and gates.
5. Re-run HELIX strict alignment, full release validation, and a fresh-eyes review.
6. Stabilize the Niflheim-facing embedder contract and record SASL as embedder-owned.
7. Publish the release tag, verify it remotely, and hand the exact tag to Niflheim without claiming its downstream integration as complete.

**Critical Path**: authority -> traceability/tests -> strict alignment -> release.

## Open Decisions

| Decision | Why open | Governing artifacts | Owner |
|---|---|---|---|
| Whether reliability percentages remain rolling-rate targets or become zero-failure gating invariants | no current evidence window is defined | FEAT-003/004/005 | heimq maintainers |
| Whether Niflheim adoption remains part of Heimq IP-001 completion or is explicitly delegated to the Niflheim tracker | current Heimq epic auto-closed despite unmet cross-repo AC | IP-001, heimq-5c906acd, niflheim-ba3e609c | Heimq and Niflheim maintainers |
</untrusted-data>
      </content>
    </ref>
  </governing>

  <diff rev="67290e3fcc2e79a475cd177f340a604ec7e15acb">
<untrusted-data>
diff --git a/.github/workflows/bench-omb.yml b/.github/workflows/bench-omb.yml
index ae2a8cc..7e57421 100644
--- a/.github/workflows/bench-omb.yml
+++ b/.github/workflows/bench-omb.yml
@@ -51,7 +51,7 @@ jobs:
       - name: Run OMB conformance
         # @covers US-007-AC1
         # @covers US-007-AC3
-        run: BOOTSTRAP=localhost:9094 bash scripts/bench/run-omb.sh
+        run: scripts/ci/reliability-gate.sh feat-004-bench-omb -- bash -c 'BOOTSTRAP=localhost:9094 bash scripts/bench/run-omb.sh'
 
       - name: Stop heimq
         if: always()
diff --git a/.github/workflows/bench-smoke.yml b/.github/workflows/bench-smoke.yml
index b6c6183..bd5f7f7 100644
--- a/.github/workflows/bench-smoke.yml
+++ b/.github/workflows/bench-smoke.yml
@@ -34,7 +34,7 @@ jobs:
         # @covers US-006-AC2
         # @covers US-006-AC3
         # @covers US-006-AC4
-        run: BOOTSTRAP=localhost:9094 bash scripts/bench/run-smoke.sh
+        run: scripts/ci/reliability-gate.sh feat-004-bench-smoke -- bash -c 'BOOTSTRAP=localhost:9094 bash scripts/bench/run-smoke.sh'
 
       - name: Stop heimq
         if: always()
diff --git a/.github/workflows/conformance.yml b/.github/workflows/conformance.yml
index 5bcf325..3596f3b 100644
--- a/.github/workflows/conformance.yml
+++ b/.github/workflows/conformance.yml
@@ -31,7 +31,7 @@ jobs:
           docker pull apache/kafka:3.9.0
 
       - name: Run parity (python + go runners)
-        run: ./tests/conformance/run.sh
+        run: scripts/ci/reliability-gate.sh feat-003-parity -- ./tests/conformance/run.sh
 
       - name: Upload diff records
         if: always()
diff --git a/.github/workflows/ecosystem.yml b/.github/workflows/ecosystem.yml
index e281158..196c7c1 100644
--- a/.github/workflows/ecosystem.yml
+++ b/.github/workflows/ecosystem.yml
@@ -33,17 +33,17 @@ jobs:
       - name: Test Python confluent-kafka
         # @covers US-012-AC2
         # @covers US-012-AC4
-        run: BOOTSTRAP=localhost:9094 bash tests/ecosystem/01-librdkafka-python.sh
+        run: scripts/ci/reliability-gate.sh feat-005-librdkafka-python -- bash -c 'BOOTSTRAP=localhost:9094 bash tests/ecosystem/01-librdkafka-python.sh'
 
       - name: Test Go confluent-kafka-go
         # @covers US-012-AC1
         # @covers US-012-AC4
-        run: BOOTSTRAP=localhost:9094 bash tests/ecosystem/02-librdkafka-go.sh
+        run: scripts/ci/reliability-gate.sh feat-005-librdkafka-go -- bash -c 'BOOTSTRAP=localhost:9094 bash tests/ecosystem/02-librdkafka-go.sh'
 
       - name: Test Node node-rdkafka
         # @covers US-012-AC3
         # @covers US-012-AC4
-        run: BOOTSTRAP=localhost:9094 bash tests/ecosystem/03-librdkafka-node.sh
+        run: scripts/ci/reliability-gate.sh feat-005-node-rdkafka -- bash -c 'BOOTSTRAP=localhost:9094 bash tests/ecosystem/03-librdkafka-node.sh'
 
       - name: Stop heimq
         if: always()
@@ -72,20 +72,20 @@ jobs:
         # @covers US-011-AC2
         # @covers US-011-AC3
         # @covers US-011-AC4
-        run: BOOTSTRAP=localhost:9094 bash tests/ecosystem/04-schema-registry.sh
+        run: scripts/ci/reliability-gate.sh feat-005-schema-registry -- bash -c 'BOOTSTRAP=localhost:9094 bash tests/ecosystem/04-schema-registry.sh'
 
       - name: Test Kafka Connect (source + sink)
         # @covers US-008-AC1
         # @covers US-008-AC2
         # @covers US-008-AC3
         # @covers US-008-AC4
-        run: BOOTSTRAP=localhost:9094 bash tests/ecosystem/05-kafka-connect.sh
+        run: scripts/ci/reliability-gate.sh feat-005-kafka-connect -- bash -c 'BOOTSTRAP=localhost:9094 bash tests/ecosystem/05-kafka-connect.sh'
 
       - name: Test ksqlDB
         # @covers US-014-AC1
         # @covers US-014-AC2
         # @covers US-014-AC3
-        run: BOOTSTRAP=localhost:9094 bash tests/ecosystem/06-ksqldb.sh
+        run: scripts/ci/reliability-gate.sh feat-005-ksqldb -- bash -c 'BOOTSTRAP=localhost:9094 bash tests/ecosystem/06-ksqldb.sh'
 
       - name: Stop heimq
         if: always()
@@ -113,13 +113,13 @@ jobs:
         # @covers US-010-AC1
         # @covers US-010-AC2
         # @covers US-010-AC3
-        run: BOOTSTRAP=localhost:9094 bash tests/ecosystem/07-debezium.sh
+        run: scripts/ci/reliability-gate.sh feat-005-debezium -- bash -c 'BOOTSTRAP=localhost:9094 bash tests/ecosystem/07-debezium.sh'
 
       - name: Test Apache Flink (Kafka source + sink, EOS)
         # @covers US-009-AC1
         # @covers US-009-AC2
         # @covers US-009-AC3
-        run: BOOTSTRAP=localhost:9094 bash tests/ecosystem/08-flink.sh
+        run: scripts/ci/reliability-gate.sh feat-005-flink -- bash -c 'BOOTSTRAP=localhost:9094 bash tests/ecosystem/08-flink.sh'
 
       - name: Stop heimq
         if: always()
diff --git a/crates/heimq/docs/helix/01-frame/features/FEAT-003-differential-parity-testing.md b/crates/heimq/docs/helix/01-frame/features/FEAT-003-differential-parity-testing.md
index 78dc4d8..fd049dd 100644
--- a/crates/heimq/docs/helix/01-frame/features/FEAT-003-differential-parity-testing.md
+++ b/crates/heimq/docs/helix/01-frame/features/FEAT-003-differential-parity-testing.md
@@ -79,7 +79,13 @@ with Redpanda is observable rather than asserted.
   workload produce the same diff (after normalization).
 - **Performance**: Harness completes its gating workload in under 10
   minutes on CI hardware.
-- **Reliability**: Less than 1% flake rate on the gating workload.
+- **Reliability**: The executable reliability rule is a zero-failure
+  invariant over the current GitHub Actions workflow run. The FEAT-003
+  evidence window is the `conformance` workflow's `parity` job: one
+  required parity-harness attempt, zero allowed failures, emitted and
+  enforced by `scripts/ci/reliability-gate.sh feat-003-parity -- ./tests/conformance/run.sh`.
+  Historical flake-rate percentages are not claimed until a separate
+  rolling measurement store exists.
 
 ## User Stories
 
@@ -98,6 +104,9 @@ with Redpanda is observable rather than asserted.
 
 - Zero unmatched diffs at the gating workload across in-scope APIs.
 - Differential harness gates CI for protocol-touching changes.
+- Current-workflow reliability evidence for FEAT-003 shows
+  `reliability_rule=zero_failures`, `required_attempts=1`,
+  `allowed_failures=0`, and a passing `feat-003-parity` target.
 
 ## Constraints and Assumptions
 
diff --git a/crates/heimq/docs/helix/01-frame/features/FEAT-004-benchmark-conformance.md b/crates/heimq/docs/helix/01-frame/features/FEAT-004-benchmark-conformance.md
index 539d67f..8f879ae 100644
--- a/crates/heimq/docs/helix/01-frame/features/FEAT-004-benchmark-conformance.md
+++ b/crates/heimq/docs/helix/01-frame/features/FEAT-004-benchmark-conformance.md
@@ -78,7 +78,14 @@ tests miss.
 
 ### Non-Functional Requirements
 
-- **Reliability**: Bench harness pass rate ≥ 99% on the gating workload.
+- **Reliability**: The executable reliability rule is a zero-failure
+  invariant over the current GitHub Actions workflow run. The FEAT-004
+  evidence window is the `bench-smoke` workflow's `feat-004-bench-smoke`
+  target plus the `bench-omb` workflow's `feat-004-bench-omb` target:
+  one required attempt per target, zero allowed failures, emitted and
+  enforced by `scripts/ci/reliability-gate.sh`. Historical pass-rate
+  percentages are not claimed until a separate rolling measurement
+  store exists.
 - **Reproducibility**: Each profile is a checked-in script with pinned
   client / tool versions.
 - **Performance**: Bench harness wall-clock budget ≤ 30 min on CI
@@ -105,6 +112,10 @@ tests miss.
   errors at their gating profile.
 - Each benchmark profile is checked in with its expected exit code and
   acceptable warnings list.
+- Current-workflow reliability evidence for FEAT-004 shows
+  `reliability_rule=zero_failures`, `required_attempts=1`,
+  `allowed_failures=0`, and passing `feat-004-bench-smoke` and
+  `feat-004-bench-omb` targets.
 
 ## Constraints and Assumptions
 
diff --git a/crates/heimq/docs/helix/01-frame/features/FEAT-005-ecosystem-integrations.md b/crates/heimq/docs/helix/01-frame/features/FEAT-005-ecosystem-integrations.md
index bbf175b..bbf9e2a 100644
--- a/crates/heimq/docs/helix/01-frame/features/FEAT-005-ecosystem-integrations.md
+++ b/crates/heimq/docs/helix/01-frame/features/FEAT-005-ecosystem-integrations.md
@@ -95,8 +95,16 @@ single script per target.
 
 ### Non-Functional Requirements
 
-- **Reliability**: Each integration test pass rate ≥ 99% on its gating
-  profile.
+- **Reliability**: The executable reliability rule is a zero-failure
+  invariant over the current GitHub Actions workflow run. The FEAT-005
+  evidence window is the `ecosystem` workflow: one required attempt for
+  each integration target, zero allowed failures, emitted and enforced
+  by `scripts/ci/reliability-gate.sh`. The target names are
+  `feat-005-librdkafka-python`, `feat-005-librdkafka-go`,
+  `feat-005-node-rdkafka`, `feat-005-schema-registry`,
+  `feat-005-kafka-connect`, `feat-005-ksqldb`, `feat-005-debezium`,
+  and `feat-005-flink`. Historical pass-rate percentages are not
+  claimed until a separate rolling measurement store exists.
 - **Reproducibility**: Pinned tool versions per integration; one script
   per integration that brings up dependencies, runs the test, tears
   down.
@@ -127,6 +135,9 @@ single script per target.
   CI.
 - The integration matrix is documented in the test plan with each
   target's status (green / yellow / parked).
+- Current-workflow reliability evidence for FEAT-005 shows
+  `reliability_rule=zero_failures`, `required_attempts=1`,
+  `allowed_failures=0`, and a passing result for every ecosystem target.
 
 ## Constraints and Assumptions
 
diff --git a/crates/heimq/docs/helix/03-test/test-plan/test-plan.md b/crates/heimq/docs/helix/03-test/test-plan/test-plan.md
index 082785c..1b21048 100644
--- a/crates/heimq/docs/helix/03-test/test-plan/test-plan.md
+++ b/crates/heimq/docs/helix/03-test/test-plan/test-plan.md
@@ -408,6 +408,29 @@ test:
     - cargo llvm-cov --workspace --all-features --fail-under-lines 100 --fail-under-regions 100
 ```
 
+### Reliability Evidence Window
+
+FEAT-003, FEAT-004, and FEAT-005 use the same executable reliability
+rule: zero target failures in the current GitHub Actions workflow run.
+The evidence window is deliberately the current workflow run, not a
+historical percentage window, because the repository does not maintain a
+rolling result store for flake-rate or pass-rate aggregation. No
+less-than-1-percent flake-rate or at-least-99-percent pass-rate result is
+claimed by this plan.
+
+The CI ratchet is `scripts/ci/reliability-gate.sh <target> -- <command>`.
+It emits `reliability_rule=zero_failures`,
+`evidence_window=current_github_actions_workflow_run`,
+`required_attempts=1`, and `allowed_failures=0`, runs the target command,
+then exits non-zero when the command fails. The required target windows
+are:
+
+| Feature | Workflow target(s) | Required attempts | Allowed failures |
+| --- | --- | ---: | ---: |
+| FEAT-003 | `feat-003-parity` in `.github/workflows/conformance.yml` | 1 | 0 |
+| FEAT-004 | `feat-004-bench-smoke` in `.github/workflows/bench-smoke.yml`; `feat-004-bench-omb` in `.github/workflows/bench-omb.yml` | 1 per target | 0 |
+| FEAT-005 | `feat-005-librdkafka-python`, `feat-005-librdkafka-go`, `feat-005-node-rdkafka`, `feat-005-schema-registry`, `feat-005-kafka-connect`, `feat-005-ksqldb`, `feat-005-debezium`, `feat-005-flink` in `.github/workflows/ecosystem.yml` | 1 per target | 0 |
+
 ## Risk Assessment
 
 | Risk | Impact | Probability | Mitigation |
diff --git a/scripts/ci/reliability-gate.sh b/scripts/ci/reliability-gate.sh
new file mode 100755
index 0000000..626638f
--- /dev/null
+++ b/scripts/ci/reliability-gate.sh
@@ -0,0 +1,38 @@
+#!/usr/bin/env bash
+# Execute one CI reliability target and emit the project's executable
+# evidence-window rule before returning the target command's status.
+
+set -euo pipefail
+
+usage() {
+    cat >&2 <<'EOF'
+usage: scripts/ci/reliability-gate.sh <target-name> -- <command> [args...]
+
+Runs <command> as one reliability evidence attempt. The current GitHub Actions
+workflow run is the evidence window, and zero target failures are allowed.
+EOF
+}
+
+if [[ $# -lt 3 || "${2:-}" != "--" ]]; then
+    usage
+    exit 64
+fi
+
+target="$1"
+shift 2
+
+echo "reliability_target=$target"
+echo "reliability_rule=zero_failures"
+echo "evidence_window=current_github_actions_workflow_run"
+echo "required_attempts=1"
+echo "allowed_failures=0"
+echo "command=$*"
+
+if "$@"; then
+    echo "reliability_result=pass"
+else
+    status=$?
+    echo "reliability_result=fail"
+    echo "failing_target=$target"
+    exit "$status"
+fi
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
