<bead-review>
  <bead id="heimq-0e59fef5" iter=1>
    <title>Reconcile the Niflheim adoption claim with the published Heimq handler seam</title>
    <description>
Resolve AR13-09 without modifying Niflheim in this Heimq worktree. Verify heimq-handlers exposes request decode, async Produce dispatch over LogBackend/RequestContext, and response encode; verify those APIs are in the release tag being cut. Record that the external blocker named by niflheim-ba3e609c is satisfied upstream, while the downstream dependency bump and in-tree pipeline retirement remain Niflheim work. Update the Heimq alignment report issue coverage and program notes with exact commit/tag evidence. Do not claim niflheim-ba3e609c complete.
    </description>
    <acceptance>
1. cargo test -p heimq-handlers passes. 2. The release candidate contains crates/heimq-handlers/src/codec.rs and produce.rs public request-level APIs. 3. IP-001, AR-2026-07-13-repo, and heimq-5c906acd notes name the upstream commit/tag and the still-open Niflheim bead accurately. 4. Niflheim can replace its stale external-blocker reason with the published Heimq tag after release.
    </acceptance>
    <labels>helix, program, cross-repo, area:niflheim-kafka, kind:release, ac-quality:needs-refinement</labels>
  </bead>

  <changed-files>
    <file>.ddx/executions/20260714T044346-fff36f39/handler-seam-evidence.md</file>
    <file>crates/heimq/docs/helix/04-build/implementation-plan.md</file>
    <file>crates/heimq/docs/helix/06-iterate/alignment-reviews/AR-2026-07-13-repo.md</file>
  </changed-files>

  <governing>
    <ref id="IP-001" path="crates/heimq/docs/helix/04-build/implementation-plan.md" title="Implementation Plan: heimq Engine Restructuring Program">
      <content>
<untrusted-data>
---
ddx:
  id: IP-001
  type: implementation-plan
  status: approved
  depends_on:
    - helix.prd
    - helix.product-vision
  review:
    self_hash: 329b4d9ab3db67b9977ed0bf2c489426542a9e510e26154558f87ba51913fee4
    deps:
      helix.prd: 236574e8f31d3847bb8269d538fe07d0a47376aa7d7e75c30dca783e479ad4ab
      helix.product-vision: 8fda503ba36c48175e42be03782c96af196de39d6e87b6edab88d853ee1857ca
    reviewed_at: "2026-06-22T21:30:26Z"
---
# Implementation Plan: heimq Engine Restructuring Program

**Plan ID**: IP-001
**Program Epic**: heimq-5c906acd
**Date**: 2026-06-12
**Status**: Approved (operator-directed 2026-06-12)
**Owner**: heimq maintainers

## Scope

Restructure heimq from a single-crate, single-node test broker into a
five-member first-party workspace that serves as the shared Kafka broker
engine for three consumer projects, then drive adoption in dependency order:

1. Split heimq into wire / broker / handlers / testkit / cli workspace crates.
2. Fold in niflheim wire-layer improvements.
3. Build out the heimq test/conformance suite.
4. Iterate until it passes.
5. Fold into fjord; 6. iterate on conformance suite.
7. Fold into pqueue; 8. iterate on conformance suite.
9. Fold into niflheim.

Target workspace (this repo, no rename):

| Crate | Contents | Consumers |
|---|---|---|
| `heimq` (bin) | CLI, config, backend dispatch (memory/postgres), packaging — the distribution | end users |
| `heimq-wire` | framing, flexible headers, connection loop, SASL/TLS hooks, frame-size and error-frame policy | niflheim, pqueue, heimq bin |
| `heimq-broker` | produce core, group/idempotence/transaction semantics, capability gating, trait families (`LogBackend`/`TopicLog`/`PartitionLog`, `OffsetStore`, `GroupCoordinatorBackend`, `ClusterView`), in-memory reference backends | fjord, niflheim (produce path over a WAL-backed `TopicLog`), heimq bin; pqueue shape decided at Slice 7 framing |
| `heimq-handlers` | request-level decode/dispatch/encode layer above `heimq-broker`; embedder-facing codec helpers plus typed Produce, ApiVersions, Metadata, and InitProducerId handlers | heimq bin, niflheim downstream delegation |
| `heimq-testkit` | per-trait conformance suites, contract-test pattern, differential parity harness (SD-003), expected-divergence annotations | all consumers |

The workspace also carries `crates/heimq-protocol`, a vendored fork of
`kafka-protocol` 0.15.1, as an excluded path dependency rather than a
first-party workspace member.

Status update (2026-07-14): upstream handler extraction into
`heimq-handlers` landed in commit `35e8a15` and release `v0.1.4`. That
satisfies Heimq's side of the request-level seam, but Niflheim's request-path
delegation remains downstream work tracked by `niflheim-ba3e609c`.

Governing decisions recorded 2026-06-12 (formalized in Slice 0): single name
`heimq` for engine and distribution; conformance on the in-memory reference
broker is the proof mechanism for trait shapes; consumers never depend on the
bin crate.

**Out of scope**: publishing to crates.io (deferred, names verified
available); fjord/pqueue/niflheim product semantics (their repos, their
trackers); Kafka APIs beyond the current in-scope surface plus FEAT-002/006
completion.

## Implementation Slices

### Slice 0 — Governing specs (prerequisite to all build work)

Evolve heimq's stack top-down to carry the broadened product:
- Product vision rewrite: engine + distribution dual identity; conformance-
  proves-the-traits principle; multi-node-capable via pluggable coordination.
- PRD evolve: new personas (broker-builder embedding the engine; existing
  test-author persona retained), FR additions for trait families and
  per-trait conformance, capability-gated advertisement as a product
  requirement.
- ADR-007: workspace split and crate boundaries (supersedes the single-crate
  assumption; records the niflheim/fjord/pqueue consumption model and the
  `kafka-protocol` single-pin policy).
- WIRE-001 contract (seeded from CODEC-001): framing, header versions
  (delegating to `kafka-protocol` `ApiKey::{request,response}_header_version`),
  error-frame policy, frame-size cap, malformed-request behavior; **the
  handler/registry contract** — async handler shape, typed-vs-raw body
  policy, response-encoding ownership, per-request auth/principal context,
  backpressure-to-error mapping (ING-6 → KAFKA_STORAGE_ERROR), cancellation,
  deferred produce acks; **security scope resolution** — SASL PLAIN/TLS as a
  gated wire capability (SaslHandshake/SaslAuthenticate surface per the
  niflheim implementation), disabled in the heimq distribution; PRD non-goal
  reworded from "out of scope" to "wire-crate capability, off in heimq bin".
- TRAIT-001 contract: the storage/offset/group/coordination seam — trait
  signatures, capability structs, durability-boundary semantics (ADR-006),
  ack-mode requirements (complete-work-before-ack: an impl may run product
  work — e.g. niflheim materialization enqueue — before the produce ack
  resolves), per-request principal/tenant context threaded to trait calls,
  partial-implementation rules via capability gating (produce-only impls
  advertise no Fetch/groups), per-trait conformance obligations.
- Cross-repo: supersession note filed against fjord ADR-002 (extraction
  precondition met); niflheim and pqueue adoption ADRs deferred to their
  slices.

**Exit gate**: validate-clean on all touched artifacts; no DIVERGENT findings
between vision/PRD and FEAT-001..007.

### Slice 1 — Split heimq (program step 1)

Mechanical workspace restructure plus seam-debt payoff at the boundary:
- Flatten `heimq/` into the first-party
  `crates/{heimq,heimq-wire,heimq-broker,heimq-handlers,heimq-testkit}`
  workspace (testkit created here as manifest + public API skeleton,
  populated in Slice 3; `heimq-handlers` owns request-level
  decode/dispatch/encode above `heimq-broker`); CI (`test.yml`,
  stress-matrix) updated for workspace paths.
- Seam-debt beads (fix while drawing the boundary, not after):
  router holds concrete `ConsumerGroupManager` → trait object;
  `Config` reach-through in Metadata/FindCoordinator → `ClusterView` trait
  with single-node impl; frame-size cap (none today); malformed requests
  swallowed without error response → WIRE-001 error-frame policy;
  **manual-parser conversion — checked inventory (2026-06-12), all 11
  converted to `kafka-protocol` decode in this slice** (every handler except
  `api_versions` and `produce`, which already conform): create_topics,
  delete_topics, fetch, heartbeat, join_group, leave_group, list_offsets,
  metadata, offset_commit, offset_fetch, sync_group — one bead each, gated
  by existing handler tests + contract tests;
  `FLEXIBLE_VERSION_MIN` → delegate to `kafka-protocol` (CODEC-001 tests
  re-pin upstream behavior).

**Exit gate**: full workspace suite + contract tests green; parity harness
boots; required `test` check green; behavior unchanged for valid requests
within advertised versions — intentional deltas (frame-size cap, typed
malformed-request error frames) enumerated in WIRE-001 and pinned by
contract tests.

### Slice 2 — Fold in niflheim wire improvements (program step 2)

Port from `niflheim-protocol/src/kafka/` (same owner; copy with attribution):
- Reader/writer task split with bounded channel (pipelining + deferred acks).
- SASL PLAIN session handling + TLS acceptor (incl. provider-init idempotency
  fix); auth gate blocking all APIs except ApiVersions pre-auth.
- Consecutive-error policy and best-effort typed error frames.
- Record-batch helpers: CRC-validated legacy v0/v1 message-set decoder, v2
  wire-size calculator.
- Single `kafka-protocol` pin for the workspace (0.15 → current; ADR-007).

**Exit gate**: wire-level tests for each ported behavior green; SASL/TLS
behavior present only behind the WIRE-001 capability gate (off by default in
heimq bin); produce path supports deferred acks (ING-6 shape) demonstrated
by test; consumer-shaped adapter tests (niflheim-shape product handler,
pqueue-shape producer front-end) compile and pass against the handler
contract before heimq-wire is declared stable.

### Slice 3 — Conformance suite build-out (program step 3)

- Per-trait conformance suites in `heimq-testkit`: TopicLog, OffsetStore
  (commit/fetch/restart matrix — Postgres impl is the second executor),
  GroupCoordinatorBackend (rebalance traces), ClusterView.
- Whole-broker: SD-003 parity workloads implemented (currently scaffolding-
  only), parity CI job (FEAT-003 FR6 — still missing), bench harness OMB
  integration (FEAT-004), client-matrix smoke (FEAT-005 subset), multi-node
  in-memory cluster harness exercising ClusterView with standard clients.
- `@covers US-<n>-AC<m>` retrofit + test-plan AC layer allocation (existing
  bead heimq-1ff758fc folds in here).
- **Consumer-shaped fixture backends** (small, in-repo, not products):
  object-store-shape `TopicLog` (manifest-ordered append), WAL-shape
  `TopicLog` with deferred acks + pre-ack work hook (niflheim shape),
  queue-enqueue produce sink (pqueue shape). Each must pass the per-trait
  suites — trait shapes are not "proven" until these pass.
- Known limit recorded in TP-001 successor: in-memory conformance cannot
  prove durability-boundary semantics; those live in per-trait suites only.

**Exit gate**: suites runnable and red/green honest — failures enumerate real
engine gaps, no phantom passes.

### Slice 4 — Iterate until green (program step 4)

Drain conformance failures; the two known large items are FEAT-006 remainder
(advertised maxima bump, flexible bodies end-to-end with modern clients) and
FEAT-002 (idempotent producer + transactions/EOS).

**Gate A — adoption-ready** (unblocks Slices 5–9): parity zero diffs at
non-transactional gating workloads; perf tests clean; per-trait suites green
on memory + Postgres impls + the three fixture backends; **both FEAT-002
capabilities explicitly gated off at Gate A** — transactions AND idempotent
producer (neither advertised; `enable.idempotence` clients are rejected with
the standard error, matching today's behavior). Consumers needing either
wait for the capability, not for Gate B as a whole. Gate A's "perf tests
clean" means exactly: `kafka-producer-perf-test` (non-idempotent config) and
`kafka-consumer-perf-test` against the checked-in `scripts/bench/profiles/`
load profile, exit 0, zero protocol/client error lines; OMB and the
idempotent/transactional bench profiles (FEAT-004 FR-03/FR-04 portions)
belong to Gate B.

**Gate B — heimq PRD success** (full): Gate A plus FEAT-002 idempotent
producer parity-clean, FEAT-002 transactions/EOS parity-clean, OMB complete,
≥3-language client matrix green. Gate B is the distribution's bar and does
not block consumer adoption.

### Slices 5–6 — fjord adoption + iterate (program steps 5–6)

- Starts at Slice 4 **Gate A** (fjord excludes EOS v1; transactions stay
  capability-gated off).
- fjord gateway built on `heimq-broker` (TD-001 revised to integration
  design); object-log backends implement TopicLog/OffsetStore; fjord
  metadata plane implements ClusterView; fjord ADR-002 superseded.
- fjord runs per-trait suites against its backends + parity with
  expected-divergence annotations (ack-after-commit, manifest ordering).
- Consumer-matrix CI introduced in heimq: candidate changes build/test fjord
  (and later pqueue/niflheim) via patch overrides before tagging.
- Gaps found feed back as heimq trait/engine beads under semver discipline
  (additive trait evolution preferred).

**Exit gate**: fjord skeleton passes wire conformance + per-trait suites on
its backends; no unrecorded divergences.

### Slices 7–8 — pqueue adoption + iterate (program steps 7–8)

- **Decided scope (operator, 2026-06-12)**: pqueue adopts a Kafka
  **producer interface** — wire-only, produce path, niflheim-shaped
  (ApiVersions/Metadata/Produce over `heimq-wire`, produce mapped onto
  pqueue enqueue semantics). Consumer-side Kafka APIs stay out of scope.
- **Named prerequisite**: pqueue's PRD currently lists Kafka compatibility
  as a v1 non-goal (`pqueue/docs/helix/01-frame/prd.md`); the slice opens
  with the planned PRD promotion scoping the producer front-end — a
  recorded evolve, not an open conflict.
- Iterate: wire-suite gaps feed back to heimq.

**Exit gate**: pqueue PRD promotion merged; producer front-end passes the
wire + produce-path conformance subset; pqueue suite green.

### Slice 9 — niflheim adoption (program step 9)

- niflheim adopts `heimq-wire` AND the `heimq-broker` produce path: its
  WAL becomes a `TopicLog` (append-side) implementation with deferred acks
  and pre-ack materialization enqueue per TRAIT-001; tenancy/RBAC/quota move
  into the impl + per-request principal context; schema parsing
  (record → WalEntry) lives inside the TopicLog impl. Capability gating
  keeps advertisement produce-only. The WAL TopicLog runs the per-trait
  conformance suites.
- **Divergence risk management**: diff niflheim's wire layer against the
  Slice-2 port baseline first; forward-port anything that changed since.
- niflheim adoption ADR; its contract tests re-pointed at heimq-wire.
- Current external remainder: Heimq has shipped the request-level handler seam
  in `35e8a15` / `v0.1.4`; Niflheim still needs to delegate its request path
  to that seam under `niflheim-ba3e609c`.

**Exit gate**: niflheim workspace suite + kafka contract tests green on
heimq-wire; WAL-backed `TopicLog` passes the per-trait conformance suite;
produce-path tests cover deferred acks, pre-ack materialization enqueue, and
principal-context propagation; LOC delta recorded (expected: net deletion in
niflheim).

## Issue Decomposition

- Program epic: **heimq-5c906acd** (this plan).
- One phase bead per slice, dependency-chained **strictly serially in
  program order**: 0 → 1 → 2 → 3 → 4 → (5-6) → (7-8) → 9. The chain is the
  executable ordering contract for fleet/DDx executors — no slice starts
  before its predecessor's exit gate. (Gate A vs Gate B nuance lives inside
  Slice 4's exit criteria, not in the chain.) Consumer-side work is
  additionally tracked in fjord/pqueue/niflheim trackers at slice start —
  cross-repo beads reference this epic id.
- Slices file detailed sub-beads at start (progressive elaboration); Slice 1
  seam-debt items enumerated above become individual beads at that point.
- Existing open beads absorbed: heimq-11399a35 (FEAT-006 epic → Slice 4),
  heimq-1ff758fc (@covers → Slice 3), heimq-5453072e (ADR-002 close-hook —
  independent, unaffected).

## Validation Plan

- Every slice exits through the ADR-002 gate (required `test` check) plus the
  slice-specific exit gate above; gates are intrinsic (build/test/conformance)
  — external review is advisory.
- From Slice 3 on, the conformance suite is the standing definition of done;
  "passes" claims require recorded evidence (command, exit status, target)
  per the verification concern.
- From Slice 5 on, heimq changes additionally pass the consumer matrix before
  tagging; consumers pin tags, never branches.
- Spec changes validate against templates (helix validate) before slice exit.

## Risks and Rollbacks

| Risk | Impact | Mitigation / rollback |
|---|---|---|
| Framework trap: traits generalized beyond the three known consumers | API churn, delay | Trait changes require a named consumer need; rule-of-three already satisfied — no speculative hooks |
| FEAT-002 (transactions) is the largest unknown in Slice 4 | Schedule | Gate A/B split makes the deferral structural: adoption proceeds at Gate A with transactions capability-gated off; FEAT-002 lands toward Gate B on its own clock |
| Code-copy drift: fixes land on niflheim's copy of the wire scaffolding between the Slice-2 port and Slice-9 swap | Missed fixes in the fork | Slice 9 starts with a forward-port diff against the Slice-2 baseline commit (recorded in ADR-007); niflheim wire layer is currently low-churn |
| pqueue PRD conflict unresolved | Slice 7 blocked | Prerequisite named here and in the slice bead; program proceeds through Slice 6 regardless |
| `kafka-protocol` major upgrade (0.15→current) breaks codecs | Slice 2 stall | Upgrade isolated in its own bead with contract tests as the gate; rollback = stay on 0.15 for Slice 1–2, upgrade in Slice 3 |
| Parity-suite flakiness (containers, timing) | False reds erode the gate | Normalization + exemptions already designed (SD-003); flake budget tracked; exemptions require id + prd_ref |
| Workspace split churns every open PR/bead path reference | Confusion | Slice 1 lands as one reviewed change; tracker paths updated in same commit; old paths git-mv'd for history |

Rollback posture: Slices 0–3 are additive or behavior-preserving under the
Slice-1 compatibility contract (unchanged for valid requests within
advertised versions; WIRE-001-enumerated intentional deltas excluded); each
lands as an independently revertable commit series on `main` behind the
required check. Consumer adoptions are
pinned-version bumps in their repos — rollback is reverting the pin.
</untrusted-data>
      </content>
    </ref>
  </governing>

  <diff rev="ba28a9c3cb7df495656613a251cd974710a7ab57">
<untrusted-data>
diff --git a/crates/heimq/docs/helix/04-build/implementation-plan.md b/crates/heimq/docs/helix/04-build/implementation-plan.md
index cd80f4e..ba4255a 100644
--- a/crates/heimq/docs/helix/04-build/implementation-plan.md
+++ b/crates/heimq/docs/helix/04-build/implementation-plan.md
@@ -50,9 +50,13 @@ The workspace also carries `crates/heimq-protocol`, a vendored fork of
 first-party workspace member.
 
 Status update (2026-07-14): upstream handler extraction into
-`heimq-handlers` landed in commit `35e8a15` and release `v0.1.4`. That
-satisfies Heimq's side of the request-level seam, but Niflheim's request-path
-delegation remains downstream work tracked by `niflheim-ba3e609c`.
+`heimq-handlers` landed in commit `35e8a15` and release `v0.1.4`
+(`48e6e34`). That tag contains `crates/heimq-handlers/src/codec.rs` with
+request decode and response encode helpers, and `produce.rs` with async
+Produce dispatch over `LogBackend` plus `RequestContext`. Heimq's side of the
+request-level seam is therefore published; Niflheim's dependency bump and
+request-path delegation remain downstream work tracked by
+`niflheim-ba3e609c`.
 
 Governing decisions recorded 2026-06-12 (formalized in Slice 0): single name
 `heimq` for engine and distribution; conformance on the in-memory reference
@@ -240,8 +244,12 @@ wire + produce-path conformance subset; pqueue suite green.
   Slice-2 port baseline first; forward-port anything that changed since.
 - niflheim adoption ADR; its contract tests re-pointed at heimq-wire.
 - Current external remainder: Heimq has shipped the request-level handler seam
-  in `35e8a15` / `v0.1.4`; Niflheim still needs to delegate its request path
-  to that seam under `niflheim-ba3e609c`.
+  in `35e8a15` / `v0.1.4` (`48e6e34`): `codec.rs` exposes request decode and
+  response encode helpers, and `produce.rs` exposes async Produce dispatch
+  through `LogBackend` and `RequestContext`. Niflheim can replace its stale
+  external-blocker reason with the published Heimq tag after release, but it
+  still needs to bump its Heimq dependencies and delegate its request path
+  under `niflheim-ba3e609c`.
 
 **Exit gate**: niflheim workspace suite + kafka contract tests green on
 heimq-wire; WAL-backed `TopicLog` passes the per-trait conformance suite;
diff --git a/.ddx/executions/20260714T044346-fff36f39/handler-seam-evidence.md b/.ddx/executions/20260714T044346-fff36f39/handler-seam-evidence.md
new file mode 100644
index 0000000..9c8f58a
--- /dev/null
+++ b/.ddx/executions/20260714T044346-fff36f39/handler-seam-evidence.md
@@ -0,0 +1,21 @@
+# Handler Seam Evidence
+
+Bead: `heimq-0e59fef5`
+
+Evidence:
+
+- Extraction commit: `35e8a15` (`refactor(handlers): extract request-level handlers into heimq-handlers`).
+- Published tag: `v0.1.4` at `48e6e34`.
+- Current release-candidate baseline: `d000a24` (`git describe`: `v0.1.4-29-gd000a24`).
+- `crates/heimq-handlers/src/codec.rs` exposes `decode_request`, `decode_request_bytes`, `encode_response`, and `encode_response_body`.
+- `crates/heimq-handlers/src/produce.rs` exposes `handle_async`, `handle_async_with_config_store`, and `handle_async_with_context_and_config_store`; async dispatch calls `append_records_async` with `LogBackend` and `RequestContext`.
+- Downstream Niflheim work remains open in `niflheim-ba3e609c`: bump to the published Heimq tag and retire the in-tree request path. This bead does not claim that downstream bead complete.
+
+Verification command:
+
+```sh
+cargo test -p heimq-handlers
+```
+
+Result: passed on 2026-07-14 in this execution worktree; 27 unit tests passed
+and doc-tests passed with 0 tests.
diff --git a/crates/heimq/docs/helix/06-iterate/alignment-reviews/AR-2026-07-13-repo.md b/crates/heimq/docs/helix/06-iterate/alignment-reviews/AR-2026-07-13-repo.md
index 5de0ed1..f4e5eb9 100644
--- a/crates/heimq/docs/helix/06-iterate/alignment-reviews/AR-2026-07-13-repo.md
+++ b/crates/heimq/docs/helix/06-iterate/alignment-reviews/AR-2026-07-13-repo.md
@@ -73,7 +73,7 @@ ddx:
 | AR13-06 | Cold-start <1s budget has no executable measurement | INCOMPLETE | `prd.md:76`; no startup timing job/test under `.github/workflows/` or `tests/` | Test Plan row plus bounded startup/first-roundtrip check; runtime handoff |
 | AR13-07 | Reliability targets (<1% flake or >=99% pass rate) have no rolling measurement or ratchet | INCOMPLETE | FEAT-003:78-82; FEAT-004:77-84; FEAT-005:96-102 | Define evidence window and CI summary/ratchet, or revise targets to an executable invariant; `evolve` then runtime handoff |
 | AR13-08 | Rust concern practices and project deviations are not fully realized or authorized | UNDERSPECIFIED | `concerns.md:26,42-47`; HELIX rust-cargo concern; root Cargo/CI files | ADR for project Rust policy and explicit overrides; align lint/dependency gates or narrow the selected practice; `design` |
-| AR13-09 | IP-001 and the closed program epic claim downstream Niflheim adoption is complete, while Niflheim still pins v0.1.2 and keeps its in-tree Produce request path | DIVERGENT | IP-001 Slice 9; `heimq-5c906acd` AC; `/home/erik/Projects/niflheim/Cargo.toml`; Niflheim bead `niflheim-ba3e609c` | IP/program status correction and cross-repo follow-up; `align` in Niflheim. Upstream handler extraction is present here and is not a Heimq code blocker. |
+| AR13-09 | IP-001 and the closed program epic claimed downstream Niflheim adoption was complete, while Niflheim still pins an older Heimq tag and keeps its in-tree Produce request path | DIVERGENT | IP-001 Slice 9; `heimq-5c906acd` AC; upstream handler extraction `35e8a15`; release tag `v0.1.4` at `48e6e34`; Niflheim bead `niflheim-ba3e609c` | Heimq published the request-level handler seam in `v0.1.4`: `codec.rs` exposes request decode/response encode and `produce.rs` exposes async Produce dispatch over `LogBackend`/`RequestContext`. Niflheim can replace the stale external blocker with that tag after release, but its dependency bump and request-path delegation remain downstream work under `niflheim-ba3e609c`. |
 
 ## Implementation Map
 
@@ -141,7 +141,7 @@ ddx:
 | AR13-06 | test implementation | heimq-cdd87b6a |
 | AR13-07 | decision then measurement ratchet | heimq-23a20c6f |
 | AR13-08 | decision and concern realization | heimq-08416254 |
-| AR13-09 | truthful program status and cross-repo handoff | heimq-0e59fef5, heimq-168f24e2, heimq-0362a710; downstream implementation remains niflheim-ba3e609c |
+| AR13-09 | truthful program status and cross-repo handoff | heimq-0e59fef5 verifies published upstream seam `35e8a15` / `v0.1.4` (`48e6e34`); heimq-168f24e2 and heimq-0362a710 cover API-stability and final publication gates; downstream implementation remains `niflheim-ba3e609c` |
 
 ## Execution Issues Generated
 
@@ -159,13 +159,13 @@ ddx:
 | heimq-cdd87b6a | Enforce <1s startup through first roundtrip | none | three consecutive bounded real-client runs |
 | heimq-23a20c6f | Make reliability targets executable and honest | none | consistent spec rule and CI ratchet |
 | heimq-08416254 | Authorize and realize Rust concern policy | none | fmt, clippy, tests, deny, machete |
-| heimq-0e59fef5 | Prove the published upstream handler seam and correct program claims | heimq-f88181b0 | handler tests and exact commit/tag evidence |
+| heimq-0e59fef5 | Prove the published upstream handler seam and correct program claims | heimq-f88181b0 | `cargo test -p heimq-handlers`; `v0.1.4` contains `crates/heimq-handlers/src/codec.rs` and `produce.rs` public request-level APIs |
 | heimq-168f24e2 | Stabilize and document the handler embedder API and SASL ownership | heimq-a5061733, heimq-f88181b0, heimq-ed5504bc | public API tests, rustdoc, workspace tests |
 | heimq-0362a710 | Satisfy Niflheim's publication gate with a remotely visible release tag | heimq-a5061733, heimq-f88181b0, heimq-ed5504bc, heimq-168f24e2 | tag contents, remote visibility, downstream pin |
 
 ## Issue Coverage
 
-Every non-ALIGNED finding AR13-01 through AR13-09 has executable coverage above. Niflheim request-path delegation is explicitly delegated to `niflheim-ba3e609c`; Heimq owns publication of the prerequisite, not closure of the downstream bead.
+Every non-ALIGNED finding AR13-01 through AR13-09 has executable coverage above. For AR13-09, Heimq's upstream prerequisite is satisfied by handler extraction commit `35e8a15` and release tag `v0.1.4` at `48e6e34`: the tag contains `crates/heimq-handlers/src/codec.rs` request decode/response encode helpers and `produce.rs` async Produce dispatch over `LogBackend`/`RequestContext`. Niflheim request-path delegation is explicitly delegated to `niflheim-ba3e609c`; Heimq owns publication of the prerequisite, not closure of the downstream bead.
 
 ## Execution Order
 
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
