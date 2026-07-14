---
ddx:
  id: ADR-007
  status: accepted
  review:
    self_hash: 44b47ae3485b6c355c48380610ad1ae6d2cb3779c8ea5d2f0b96910993826500
    deps: {}
    reviewed_at: "2026-06-22T21:30:26Z"
---

# ADR-007: Workspace Split and Engine Consumption Model

| Date | Status | Deciders | Related | Confidence |
|------|--------|----------|---------|------------|
| 2026-06-12 | Accepted | heimq maintainers | IP-001, WIRE-001, TRAIT-001 | High |

## Context

| Aspect | Description |
|--------|-------------|
| Problem | heimq is a single-crate broker used only as a standalone test tool. Three consumer projects (fjord, niflheim, pqueue) each need Kafka wire and/or broker semantics; without a shared engine they each implement or re-implement the stack independently. |
| Current State | Workspace split has landed as five first-party members (`heimq`, `heimq-wire`, `heimq-broker`, `heimq-handlers`, `heimq-testkit`) plus an excluded vendored `heimq-protocol` fork; niflheim still has downstream request-path delegation work tracked by `niflheim-ba3e609c`. |
| Requirements | Consumers must be able to embed wire and/or broker crates independently. Trait conformance must be provable once and reused. Binary distribution (`heimq` CLI) must remain independently releasable. |
| Decision Drivers | niflheim and pqueue are real consumers with concrete requirements now; heimq already has the most complete implementation and test assets; trait conformance proved once on the in-memory backend eliminates per-consumer re-verification. |

## Decision

We will maintain heimq as a Cargo workspace within this repo (no rename), with
five first-party workspace members and one excluded vendored protocol fork.
The current request-handler extraction landed upstream in commit `35e8a15` and
release `v0.1.4`; Niflheim's request-path delegation remains downstream work
tracked by `niflheim-ba3e609c`.

**First-party workspace members:**

| Crate | Contents |
|---|---|
| `heimq` (bin) | CLI, config, backend dispatch — the distribution; consumers never depend on this crate |
| `heimq-wire` | Framing, flexible headers, connection loop, SASL/TLS as a gated capability, frame-size and error-frame policy |
| `heimq-broker` | Produce core, group/idempotence/transaction semantics, capability gating, trait families (`LogBackend`/`TopicLog`/`PartitionLog`, `OffsetStore`, `GroupCoordinatorBackend`, `ClusterView`), in-memory reference backends |
| `heimq-handlers` | Request-level decode/dispatch/encode layer above `heimq-broker`; exposes embedder-facing codec helpers and typed handlers for Produce, ApiVersions, Metadata, and InitProducerId |
| `heimq-testkit` | Per-trait conformance suites, contract-test pattern, differential parity harness |

**Authentication ownership:** TLS/SASL authentication is embedder-owned for the
engine consumption model. `heimq-handlers` does not own authentication, does not
run SASL handshakes, and does not dispatch SaslHandshake/SaslAuthenticate as
handler APIs. An embedding gateway authenticates the connection before handler
dispatch and threads any principal, tenant, or client identity through
`RequestContext`. The standalone `heimq` distribution and future wire gateways
may choose their own TLS/SASL policy outside the `heimq-handlers` crate.

**Excluded vendored dependency:**

| Crate | Status |
|---|---|
| `heimq-protocol` | Vendored fork of `kafka-protocol` 0.15.1, excluded from the workspace so it builds as third-party dependency code under dependency lint policy; first-party members depend on it by path. |

**Consumption model:**
- fjord embeds `heimq-broker` (object-log backends + `ClusterView`)
- niflheim embeds `heimq-wire`, `heimq-handlers`, and `heimq-broker`'s produce path via a WAL-backed `TopicLog`; upstream handler extraction is present in `35e8a15` / `v0.1.4`, while the downstream Niflheim request-path delegation remains tracked by `niflheim-ba3e609c`
- pqueue embeds `heimq-wire` as a producer front-end (shape finalized at Slice 7 framing)
- Consumers pin git tags, never branches

**`kafka-protocol` single-pin policy:** one workspace-wide pinned version,
currently 0.15, upgraded to the current release during IP-001 Slice 2 and
recorded here. Header-version selection delegates to
`ApiKey::{request,response}_header_version`; no hand-rolled version tables.

**Niflheim port baseline:** `762b210` (2026-06-13) — Slice 9 forward-port diffs against this commit.

## Alternatives

| Option | Pros | Cons | Evaluation |
|--------|------|------|------------|
| New repo + new name (`kafrost`) | Clean namespace; clear brand separation | Two brands to maintain; repo migration cost; loses history, CI, and tracker continuity | Rejected: migration cost not justified; single-repo history is an asset |
| Extract shared crate into fjord's planned repo (per fjord ADR-002) | Keeps fjord repo self-contained | heimq already has the more complete implementation and test assets; two real consumers (niflheim, pqueue) exist today and need the engine from heimq's tree | Rejected: moves the engine away from where the work already is |
| Wire-only shared crate without broker engine | Smaller shared surface; less coupling | Every consumer re-implements produce/group semantics; niflheim has explicitly decided to adopt heimq-broker's produce path | Rejected: conformance-once principle requires the broker engine to be shared |
| **Five-member first-party workspace in this repo** | One engine; request handlers have a stable embedder layer; conformance proved once; consumers delete wire code; no migration | heimq repo becomes a multi-consumer dependency — semver discipline and consumer-matrix CI required | **Selected**: benefit outweighs the governance overhead; governance is manageable with pinned tags and additive trait evolution |

## Consequences

| Type | Impact |
|------|--------|
| Positive | One engine implementation; conformance suites run once and reused by all consumers; niflheim and pqueue delete their independent wire-layer code |
| Negative | heimq repo becomes a shared dependency: semver discipline required; trait changes must evolve additively; consumer-matrix CI needed to catch breaking changes |
| Neutral | crates.io publication deferred; crate names verified available; fjord ADR-002's extraction precondition is now met and is superseded by fjord ADR-003 in the fjord repo |

## Risks

| Risk | Prob | Impact | Mitigation |
|------|------|--------|------------|
| Trait API churn breaks pinned consumers | M | H | Additive-only trait evolution policy; consumers pin tags; consumer-matrix CI in heimq |
| `kafka-protocol` upgrade breaks codec behavior | L | M | CODEC-001 contract tests re-pin upstream behavior; pin upgrade is a dedicated Slice 2 bead |
| Workspace restructure regresses existing behavior | L | H | Full workspace suite + parity harness must be green at Slice 1 exit gate |

## Validation

| Success Metric | Review Trigger |
|----------------|----------------|
| All three consumers compile and pass their suites against pinned heimq tags | Any consumer unable to upgrade within two tags |
| `kafka-protocol` single-pin holds across Slices 1–9 | Request to introduce a second version pin |
| Niflheim port baseline commit recorded at Slice 2 | Slice 2 exit gate |

## Supersession

- **Supersedes**: None (establishes new workspace structure; see fjord repo for fjord ADR-002 supersession by fjord ADR-003)
- **Superseded by**: None

## References

- [IP-001 Implementation Plan](../../04-build/implementation-plan.md)
- [WIRE-001](../contracts/WIRE-001-wire-scaffolding.md)
- [TRAIT-001](../contracts/TRAIT-001-backend-traits.md)
- fjord ADR-002 (in fjord repo) — superseded by fjord ADR-003 now that the heimq extraction precondition is met
