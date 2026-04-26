# ADR-002: Enforce Workspace Test Gate at Bead Close

**Status**: Accepted
**Date**: 2026-04-26
**Bead**: heimq-20730055 (process retro)

## Context

Two beads in the storage-backends epic shipped (state = `closed`) while
`tests/contract.rs::contract_api_versions_matches_supported_range` was red:

| Bead | Title | Acceptance Criterion |
|------|-------|----------------------|
| heimq-20daf864 (commit `2f9e9d8`) | refactor(storage): extract LogBackend/TopicLog/PartitionLog traits | "cargo test --workspace passes" |
| heimq-04def9c7 (commits `71b6d98`, `9362ca3`) | refactor(storage): extract OffsetStore trait | "cargo test --workspace passes" |

The contract test had been red since commit `331b93b` ("fix(protocol): cap
API versions below flexible-version boundaries"), which tightened the
runtime ApiVersions response without updating the corresponding
hard-coded assertion in `tests/contract.rs`. The drift was not a
consequence of either trait-extraction bead — those refactors were
correct — but both beads carried an explicit AC ("cargo test
--workspace passes") that was provably unmet at close time. The reviewer
either did not run the AC command, or ran it and accepted a red result.

The drift was eventually fixed by heimq-3141d5dd (commit `d421e38`)
which derives the contract assertion from `SUPPORTED_APIS` directly,
removing the duplicated source of truth.

## Decision

**Adopt a two-layer test gate that prevents a bead from being closed
while the workspace test suite is red.**

### Layer 1 — Tracker close-hook (primary, immediate)

The bead-tracker `close` operation runs:

```
cargo test --workspace --all-features --no-fail-fast
```

against the worktree at the closing revision **before** flipping the
bead state to `closed`. A non-zero exit blocks the close and surfaces
the failing test names in the operator's terminal. This catches the
exact failure mode that occurred in heimq-20daf864 / heimq-04def9c7:
an AC literally states "tests pass", and the close action verifies it.

The hook is implemented in the DDx tracker layer (not in this repo's
build), so it applies uniformly to every bead regardless of which agent
or operator closes it.

### Layer 2 — Required CI status check (defense-in-depth)

A `test` GitHub Actions workflow runs `cargo test --workspace
--all-features` on every push and pull request and is marked **required**
in branch protection. This catches drift introduced by changes that
land outside the bead-close path (e.g. checkpoint commits, manual
merges, hotfixes). The existing `stress-matrix.yml` job stays
`continue-on-error: true` since it covers a separate Postgres-parity
concern; the new `test` job is the hard gate.

### Why both

The close-hook gives the operator immediate, local feedback at the
moment a bead is being closed (the action that flips a bead from
"trust-but-verify" to "shipped"). CI is the backstop for cases the
hook cannot see: branch state that diverges from any individual bead
worktree, environmental flakiness on the operator's machine, or beads
closed via tooling that bypasses the hook.

## Alternatives Considered

### CI gate alone (Rejected — too late)

Without a close-hook, a red CI on `main` does not prevent a bead from
being marked `closed` in the tracker. The bead state and the actual
build state can diverge until someone notices, exactly the gap the
retro is addressing.

### Close-hook alone (Rejected — too narrow)

The hook only fires on the close action. Drift introduced by
checkpoint commits, alignment-review merges, or tracker-update
commits would land on `main` un-gated until the next bead close.

### Reviewer discipline ("operators must run the AC command")
(Rejected — already the de-facto policy)

This is the policy that failed in heimq-20daf864 / heimq-04def9c7.
Re-asserting it without a mechanical check produces no behavior
change.

## Consequences

### Positive

- A bead with a "tests pass" AC cannot be closed while tests are red.
- CI surfaces drift that is not tied to any specific bead close.
- The two layers are independent: a bug in one does not silently
  disable the other.

### Negative

- Bead close becomes slower (full `cargo test --workspace` runtime)
  on the operator's machine. Mitigation: the hook can be configured
  to use `cargo nextest` or a cached build dir if runtime becomes
  painful.
- Adds a required CI check, which can block emergency merges.
  Mitigation: standard GitHub admin override applies; use sparingly
  and document.

### Neutral

- Beads whose AC does *not* mention `cargo test` still run the gate.
  This is intentional — "tests pass" is an implicit invariant for
  every code-change bead even when not stated explicitly.

## Implementation

1. **Tracker close-hook** — wire `cargo test --workspace
   --all-features --no-fail-fast` into the DDx bead-tracker `close`
   action. Block close on non-zero exit. Tracked separately from
   this repo (DDx tooling layer).
2. **CI workflow** — add `.github/workflows/test.yml` running
   `cargo test --workspace --all-features` on push and pull-request,
   then mark it required in branch protection on `main`.

Both are out of scope for the retro bead itself (heimq-20730055),
which only documents the decision. Implementation is to be filed as
separate beads referencing this ADR.

## References

- `tests/contract.rs` — site of the stale assertion
- Commit `331b93b` — change that introduced the drift
- Commit `d421e38` (bead heimq-3141d5dd) — fix that removed the
  duplicated source of truth
- Beads heimq-20daf864, heimq-04def9c7 — closed-while-red examples
- `docs/helix/06-iterate/alignment-reviews/AR-2026-04-26-repo.md` —
  repo-level alignment review covering the affected epic
