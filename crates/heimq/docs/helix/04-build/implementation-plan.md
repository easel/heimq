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
four-crate workspace that serves as the shared Kafka broker engine for three
consumer projects, then drive adoption in dependency order:

1. Split heimq into wire / broker / cli workspace crates.
2. Fold in niflheim wire-layer improvements.
3. Build out the heimq test/conformance suite.
4. Iterate until it passes.
5. Fold into fjord; 6. iterate on conformance suite.
7. Fold into pqueue; 8. iterate on conformance suite.
9. Fold into niflheim.

Target workspace (this repo, no rename):

| Crate | Contents | Consumers |
|---|---|---|
| `heimq-wire` | framing, codec, flexible headers, connection loop, SASL/TLS hooks, error-frame policy, handler registry | niflheim, pqueue, (broker) |
| `heimq-broker` | handlers, group/idempotence/transaction semantics, capability gating, trait families (`LogBackend`/`TopicLog`/`PartitionLog`, `OffsetStore`, `GroupCoordinatorBackend`, `ClusterView`), in-memory reference backends | fjord, niflheim (produce path over a WAL-backed `TopicLog`), heimq bin; pqueue shape decided at Slice 7 framing |
| `heimq-testkit` | per-trait conformance suites, contract-test pattern, differential parity harness (SD-003), expected-divergence annotations | all consumers |
| `heimq` (bin) | CLI, config, backend dispatch (memory/postgres), packaging — the distribution | end users |

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
- Flatten `heimq/` into `crates/{heimq-wire,heimq-broker,heimq-testkit,heimq}`
  workspace (testkit created here as manifest + public API skeleton,
  populated in Slice 3); CI (`test.yml`, stress-matrix) updated for
  workspace paths.
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
