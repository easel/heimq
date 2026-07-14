<bead-review>
  <bead id="heimq-a5061733" iter=1>
    <title>Frame engine embedding and handler features for FR-12 and FR-13</title>
    <description>
Resolve AR13-01. Add one feature specification under crates/heimq/docs/helix/01-frame/features/ that names the exact PRD subsystem 'Engine trait surface (TRAIT-001)' and covers FR-12 and FR-13. Add separate Broker Builder user stories for backend-trait conformance and request-level handler delegation. Read PRD, TRAIT-001, WIRE-001, the HELIX 0.10.2 feature/user-story templates, and existing FEAT/US files first. Preserve artifact-signal voice and add ddx links. In scope: new FEAT-008 and US-016/US-017 files, plus TP-001 dependency/trace rows if required. Out of scope: Rust implementation changes.
    </description>
    <acceptance>
1. python3 /home/erik/.codex/plugins/cache/helix/helix/0.10.2/scripts/helix_align_check.py --docs-root crates/heimq/docs/helix --code-root . --format json reports no FR-&gt;FEAT or subsystem-&gt;FEAT blocking finding. 2. The new stories name FR-12/FR-13 and each acceptance criterion uses stable Given/When/Then IDs. 3. rg -n 'FR-12|FR-13|Engine trait surface' crates/heimq/docs/helix/01-frame/features crates/heimq/docs/helix/01-frame/user-stories shows feature and story coverage.
    </acceptance>
    <labels>helix, area:specs, kind:planning, action:frame</labels>
  </bead>

  <changed-files>
    <file>crates/heimq/docs/helix/01-frame/features/FEAT-008-engine-embedding-contract.md</file>
    <file>crates/heimq/docs/helix/01-frame/user-stories/US-016-backend-trait-conformance.md</file>
    <file>crates/heimq/docs/helix/01-frame/user-stories/US-017-request-level-handler-delegation.md</file>
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
| AR13-09 | IP-001 and the closed program epic claim downstream Niflheim adoption is complete, while Niflheim still pins v0.1.2 and keeps its in-tree Produce request path | DIVERGENT | IP-001 Slice 9; `heimq-5c906acd` AC; `/home/erik/Projects/niflheim/Cargo.toml`; Niflheim bead `niflheim-ba3e609c` | IP/program status correction and cross-repo follow-up; `align` in Niflheim. Upstream handler extraction is present here and is not a Heimq code blocker. |

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
| AR13-09 | truthful program status and cross-repo handoff | heimq-0e59fef5, heimq-168f24e2, heimq-0362a710; downstream implementation remains niflheim-ba3e609c |

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
| heimq-0e59fef5 | Prove the published upstream handler seam and correct program claims | heimq-f88181b0 | handler tests and exact commit/tag evidence |
| heimq-168f24e2 | Stabilize and document the handler embedder API and SASL ownership | heimq-a5061733, heimq-f88181b0, heimq-ed5504bc | public API tests, rustdoc, workspace tests |
| heimq-0362a710 | Satisfy Niflheim's publication gate with a remotely visible release tag | heimq-a5061733, heimq-f88181b0, heimq-ed5504bc, heimq-168f24e2 | tag contents, remote visibility, downstream pin |

## Issue Coverage

Every non-ALIGNED finding AR13-01 through AR13-09 has executable coverage above. Niflheim request-path delegation is explicitly delegated to `niflheim-ba3e609c`; Heimq owns publication of the prerequisite, not closure of the downstream bead.

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

  <diff rev="a5b5ab13784d6b351f222e2c3df3fec5d30a8c15">
<untrusted-data>
diff --git a/crates/heimq/docs/helix/01-frame/features/FEAT-008-engine-embedding-contract.md b/crates/heimq/docs/helix/01-frame/features/FEAT-008-engine-embedding-contract.md
new file mode 100644
index 0000000..fa6b467
--- /dev/null
+++ b/crates/heimq/docs/helix/01-frame/features/FEAT-008-engine-embedding-contract.md
@@ -0,0 +1,152 @@
+---
+ddx:
+  id: FEAT-008
+  depends_on:
+    - helix.prd
+    - TRAIT-001
+    - WIRE-001
+  links:
+    - kind: derived_from
+      to: helix.prd
+    - kind: constrained_by
+      to: TRAIT-001
+    - kind: constrained_by
+      to: WIRE-001
+    - kind: resolves
+      to: AR-2026-07-13-repo
+---
+# Feature Specification: FEAT-008 — Engine Embedding Contract
+
+**Feature ID**: FEAT-008
+**Status**: Specified
+**Priority**: P0
+**Owner**: heimq core
+**Covered PRD Subsystem(s)**: Engine trait surface (TRAIT-001)
+**Covered PRD Requirements**: FR-12, FR-13
+**Cross-Subsystem Rationale**: None — single PRD subsystem. FR-13 is included here because the PRD places the handler contract under the Engine trait surface subsystem and delegates normative handler/registry detail to WIRE-001.
+
+## Overview
+
+FEAT-008 defines the engine-embedding capability for Broker Builders who depend
+on `heimq-wire`, `heimq-broker`, and `heimq-testkit` without depending on the
+`heimq` distribution binary. It covers PRD FR-12 and FR-13 by making the trait
+families, conformance suites, capability-gated advertisement, and request-level
+handler delegation visible as one coherent engine contract.
+
+## Ideal Future State
+
+A Broker Builder can embed heimq as a Kafka front end over its own storage and
+coordination plane, implement only the trait families it can serve, and prove
+that implementation with per-trait conformance suites. Request handlers remain
+async and delegate request context, deferred acknowledgements, backpressure, and
+response encoding through the WIRE-001 handler contract, so embedders can add
+durability, tenancy, quota, or WAL work behind the engine without forking the
+Kafka wire surface.
+
+## Problem Statement
+
+- **Current situation**: TRAIT-001 and WIRE-001 define the engine seams, but no
+  feature spec owns the PRD subsystem "Engine trait surface (TRAIT-001)" or
+  traces FR-12/FR-13 into user stories.
+- **Pain points**: Broker Builders cannot tell which frame artifact governs
+  backend trait conformance versus request-level delegation, and deterministic
+  HELIX alignment reports FR-12/FR-13 as uncovered.
+- **Desired outcome**: FEAT-008 becomes the frame-level owner for engine
+  embedding; US-016 covers backend-trait conformance for FR-12, and US-017
+  covers request-level handler delegation for FR-13.
+
+## Functional Areas
+
+| Area | User question or job | Feature responsibility |
+|------|----------------------|------------------------|
+| Backend trait conformance | Can my backend safely implement only the trait families it owns? | Trace FR-12 to stable trait families, per-trait conformance suites, and capability-gated API advertisement in TRAIT-001. |
+| Request-level handler delegation | Can I run product work before responses without owning Kafka framing? | Trace FR-13 to async handlers, request context, deferred acks, backpressure mapping, cancellation, and response-encoding ownership in WIRE-001. |
+
+## Requirements
+
+### Functional Requirements by Area
+
+#### Backend trait conformance
+
+- **EMB-01** — The engine must expose the trait families named by FR-12
+  (`TopicLog`/`LogBackend`/`PartitionLog`, `OffsetStore`,
+  `GroupCoordinatorBackend`, and `ClusterView`) through the normative surface
+  in TRAIT-001.
+- **EMB-02** — Each trait family must have a corresponding conformance suite in
+  `heimq-testkit` so an embedder can verify its implementation without using
+  the `heimq` bin crate.
+- **EMB-03** — API advertisement must be capability-gated: absent or
+  unsupported backend families remove only the Kafka APIs that require that
+  family, while unrelated APIs remain advertised.
+
+#### Request-level handler delegation
+
+- **EMB-04** — Request handlers must be async and must receive the per-request
+  context required by WIRE-001 and TRAIT-001 before delegating to backend trait
+  calls.
+- **EMB-05** — Produce and other state-changing handlers must be able to defer
+  responses until required pre-ack work completes, preserving per-connection
+  response ordering.
+- **EMB-06** — Handler overload and backpressure must be reported through the
+  WIRE-001 error-mapping contract rather than by handlers writing raw frames or
+  embedding ad hoc Kafka error codes.
+- **EMB-07** — The gateway, not request handlers, must own response header
+  selection, body encoding, and length-prefix framing per WIRE-001.
+
+### Non-Functional Requirements
+
+- **Compatibility**: A backend that passes the relevant `heimq-testkit`
+  conformance suite must interoperate with the in-scope Kafka API surface for
+  that family.
+- **Reliability**: Capability-gated advertisement must be covered by tests that
+  fail if APIs are advertised for absent backend families.
+- **Security**: Per-request principal context must be threadable to backend
+  trait calls while authentication and authorization policy remain embedder
+  responsibilities.
+
+## User Stories
+
+- [US-016 — Backend trait conformance for embedders](../user-stories/US-016-backend-trait-conformance.md)
+- [US-017 — Request-level handler delegation for embedders](../user-stories/US-017-request-level-handler-delegation.md)
+
+## Edge Cases and Error Handling
+
+- **Partial backend implementation**: unsupported families remove only their
+  dependent Kafka APIs from ApiVersions; unrelated APIs still work.
+- **Pre-ack work fails**: the handler returns the standard Kafka error response
+  defined by WIRE-001 and does not acknowledge the request as successful.
+- **Connection closes while deferred work is pending**: cancellation follows the
+  WIRE-001 request-context contract; backend policy determines rollback or
+  completion semantics.
+
+## Success Metrics
+
+- HELIX alignment reports no FR-to-FEAT or subsystem-to-FEAT blocking finding
+  for FR-12, FR-13, or "Engine trait surface (TRAIT-001)".
+- `heimq-testkit` exposes per-trait conformance suites for every FR-12 trait
+  family.
+- Handler contract tests cover async delegation, deferred acknowledgements,
+  per-request context, and WIRE-001 backpressure mapping.
+
+## Constraints and Assumptions
+
+- Broker Builders depend on the engine crates only: `heimq-wire`,
+  `heimq-broker`, and `heimq-testkit`.
+- TRAIT-001 owns exact trait signatures and conformance obligations.
+- WIRE-001 owns exact handler, registry, context, cancellation, backpressure,
+  and response-encoding surface.
+
+## Dependencies
+
+- **Other features**: FEAT-001 for ApiVersions advertisement and protocol
+  response compatibility.
+- **External services**: None required by the feature; backend implementations
+  may bring their own storage or coordination systems.
+- **PRD requirements**: FR-12, FR-13.
+
+## Out of Scope
+
+- Rust implementation changes to trait signatures or handlers.
+- Durable message logs, distributed coordination, replication, or KRaft.
+- SASL/TLS as a heimq distribution feature; WIRE-001 capability-gates that
+  surface for embedders.
diff --git a/crates/heimq/docs/helix/01-frame/user-stories/US-016-backend-trait-conformance.md b/crates/heimq/docs/helix/01-frame/user-stories/US-016-backend-trait-conformance.md
new file mode 100644
index 0000000..f1e893a
--- /dev/null
+++ b/crates/heimq/docs/helix/01-frame/user-stories/US-016-backend-trait-conformance.md
@@ -0,0 +1,96 @@
+---
+ddx:
+  id: US-016
+  depends_on:
+    - FEAT-008
+  links:
+    - kind: derived_from
+      to: FEAT-008
+    - kind: constrained_by
+      to: TRAIT-001
+    - kind: covers
+      to: FR-12
+---
+# US-016 — Backend trait conformance for embedders
+
+**Feature**: FEAT-008 — Engine Embedding Contract
+**Feature Requirements**: EMB-01, EMB-02, EMB-03
+**PRD Requirements**: FR-12
+**Priority**: P0
+**Status**: Specified
+
+## Story
+
+**As a** Broker Builder,
+**I want** to implement the heimq backend trait families and run their
+conformance suites,
+**So that** my storage or coordination plane can serve only the Kafka APIs it
+actually supports while remaining compatible with the engine contract.
+
+## Context
+
+Broker Builders embed `heimq-wire`, `heimq-broker`, and `heimq-testkit` rather
+than the `heimq` bin crate. FR-12 requires stable trait families, per-trait
+conformance suites, and capability-gated API advertisement; TRAIT-001 owns the
+normative trait and capability rules. This story covers the backend side of
+FEAT-008.
+
+## Walkthrough
+
+1. Broker Builder selects the trait families their backend can serve.
+2. Builder implements those families against the TRAIT-001 contract.
+3. Builder runs the corresponding `heimq-testkit` conformance suites.
+4. Builder starts the embedded engine with those capabilities.
+5. Standard clients see only APIs backed by implemented trait families in
+   ApiVersions, while unrelated APIs remain available.
+
+## Acceptance Criteria
+
+- [ ] **US-016-AC1** — Given a backend implements a TRAIT-001 trait family, when
+  its matching `heimq-testkit` conformance suite runs, then the suite verifies
+  the family behavior required by FR-12 without depending on the `heimq` bin
+  crate.
+- [ ] **US-016-AC2** — Given a backend does not implement a trait family, when
+  ApiVersions is computed, then Kafka APIs requiring that family are not
+  advertised.
+- [ ] **US-016-AC3** — Given a backend omits one trait family but implements
+  another, when ApiVersions is computed, then only APIs gated by the missing
+  family are removed and unrelated supported APIs remain advertised.
+- [ ] **US-016-AC4** — Given backend trait methods serve a client request, when
+  the engine delegates to them, then the request context defined by TRAIT-001 is
+  available to the backend without the engine enforcing tenancy, quota, or RBAC.
+
+## Edge Cases
+
+- **Unknown or placeholder capability**: the family is treated as absent for
+  advertisement, preventing clients from selecting unsupported APIs.
+- **Conformance failure**: the backend is not considered compatible with that
+  trait family until the failing behavior is fixed or explicitly marked out of
+  scope by capability gating.
+- **Partial implementation**: unsupported families do not disable unrelated API
+  groups.
+
+## Test Scenarios
+
+| Scenario | AC ID | Input / State | Action | Expected Result |
+|----------|-------|---------------|--------|-----------------|
+| Trait suite passes | US-016-AC1 | Backend implements `OffsetStore` per TRAIT-001 | Run the offset-store conformance suite | Suite passes without importing the `heimq` bin crate |
+| Missing family is gated | US-016-AC2 | Backend has no group-coordinator family | Compute ApiVersions | Group APIs are absent from the advertised set |
+| Gating is narrow | US-016-AC3 | Backend has log and offset families, no group family | Compute ApiVersions | Produce/fetch/offset APIs remain advertised; group APIs are absent |
+| Context reaches backend | US-016-AC4 | Request context has principal and client id | Execute a client-serving trait method | Backend observes the context values |
+
+## Dependencies
+
+- **Stories**: None.
+- **Feature Spec**: FEAT-008
+- **Feature Requirements**: EMB-01, EMB-02, EMB-03
+- **PRD Requirements**: FR-12
+- **External**: TRAIT-001 for exact trait families, capability rules, and
+  conformance obligations.
+
+## Out of Scope
+
+- Changing trait signatures.
+- Implementing a new durable backend.
+- Defining Kafka wire framing or handler registry behavior; US-017 and
+  WIRE-001 own that path.
diff --git a/crates/heimq/docs/helix/01-frame/user-stories/US-017-request-level-handler-delegation.md b/crates/heimq/docs/helix/01-frame/user-stories/US-017-request-level-handler-delegation.md
new file mode 100644
index 0000000..257267f
--- /dev/null
+++ b/crates/heimq/docs/helix/01-frame/user-stories/US-017-request-level-handler-delegation.md
@@ -0,0 +1,103 @@
+---
+ddx:
+  id: US-017
+  depends_on:
+    - FEAT-008
+  links:
+    - kind: derived_from
+      to: FEAT-008
+    - kind: constrained_by
+      to: WIRE-001
+    - kind: constrained_by
+      to: TRAIT-001
+    - kind: covers
+      to: FR-13
+---
+# US-017 — Request-level handler delegation for embedders
+
+**Feature**: FEAT-008 — Engine Embedding Contract
+**Feature Requirements**: EMB-04, EMB-05, EMB-06, EMB-07
+**PRD Requirements**: FR-13
+**Priority**: P0
+**Status**: Specified
+
+## Story
+
+**As a** Broker Builder,
+**I want** request handlers to delegate async work, request context, deferred
+acknowledgements, and response encoding through the engine contract,
+**So that** my embedded broker can perform product-specific pre-response work
+without owning Kafka framing or forking handler behavior.
+
+## Context
+
+FR-13 requires async handlers, deferred produce acknowledgements, per-request
+principal context, backpressure mapping, cancellation, response-encoding
+ownership, and SASL/TLS capability gating. WIRE-001 owns the handler and
+registry contract; TRAIT-001 owns the backend request-context expectations.
+This story covers the request-level delegation side of FEAT-008.
+
+## Walkthrough
+
+1. Broker Builder configures an embedded engine with backend trait
+   implementations.
+2. A standard Kafka client sends a request through `heimq-wire`.
+3. The gateway decodes the typed request, creates request context, and invokes
+   the async handler.
+4. The handler delegates to backend trait calls and may await pre-ack work
+   before returning a response.
+5. The gateway encodes the response and preserves per-connection ordering.
+
+## Acceptance Criteria
+
+- [ ] **US-017-AC1** — Given a typed Kafka request covered by WIRE-001, when the
+  gateway dispatches it, then the async handler receives the decoded request
+  and request context before backend delegation.
+- [ ] **US-017-AC2** — Given produce handling requires pre-ack backend work, when
+  the handler returns a deferred acknowledgement, then the gateway waits for
+  that work before sending the Produce response.
+- [ ] **US-017-AC3** — Given multiple requests are queued on one connection, when
+  an earlier request is waiting on deferred work, then later responses are not
+  sent before the earlier response.
+- [ ] **US-017-AC4** — Given a handler reports overload, when the gateway builds
+  the response, then WIRE-001 backpressure mapping determines the Kafka error
+  and the handler does not write a raw frame.
+- [ ] **US-017-AC5** — Given a handler returns a typed response, when the gateway
+  sends it, then response header selection, body encoding, and frame prefixing
+  are owned by the gateway per WIRE-001.
+
+## Edge Cases
+
+- **Connection closes during pending work**: the request context cancellation
+  signal is set and backend completion policy follows the embedder's trait
+  implementation.
+- **Raw escape hatch request**: raw handlers remain explicit and are used only
+  for API keys not covered by typed handlers.
+- **SASL/TLS gate off**: SASL APIs are not advertised and request context may
+  carry no principal.
+
+## Test Scenarios
+
+| Scenario | AC ID | Input / State | Action | Expected Result |
+|----------|-------|---------------|--------|-----------------|
+| Typed dispatch | US-017-AC1 | Typed Produce request and request context | Dispatch through the gateway | Handler receives decoded request plus context |
+| Deferred produce ack | US-017-AC2 | Backend append future pending on pre-ack work | Handle Produce | Response is sent only after the future completes |
+| FIFO with deferred work | US-017-AC3 | Produce followed by Metadata on same connection | Delay Produce completion | Metadata response is not sent before Produce response |
+| Overload mapping | US-017-AC4 | Handler returns overload during Produce | Gateway builds error response | Produce receives WIRE-001 storage-error mapping; handler writes no frame |
+| Gateway encoding | US-017-AC5 | Handler returns typed response | Gateway sends response | Response header, body, and length prefix are encoded by the gateway |
+
+## Dependencies
+
+- **Stories**: US-016 for backend trait families that handlers delegate into.
+- **Feature Spec**: FEAT-008
+- **Feature Requirements**: EMB-04, EMB-05, EMB-06, EMB-07
+- **PRD Requirements**: FR-13
+- **External**: WIRE-001 for exact handler, registry, cancellation,
+  backpressure, and response-encoding rules; TRAIT-001 for backend
+  request-context propagation.
+
+## Out of Scope
+
+- Defining exact handler signatures or trait method signatures inline.
+- Implementing SASL/TLS in the `heimq` distribution.
+- Changing Kafka protocol codec rules owned by CODEC-001.
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
