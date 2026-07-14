---
ddx:
  id: ADR-008
  status: accepted
---

# ADR-008: Rust Cargo Workspace Policy

| Date | Status | Deciders | Related | Confidence |
|------|--------|----------|---------|------------|
| 2026-07-14 | Accepted | heimq maintainers | concerns.md, AR13-08, ADR-007 | High |

## Context

HELIX 0.10.2 selected the `rust-cargo` concern for heimq instead of the
shipped `language-runtime` slot default, `typescript-bun`. The selected concern
expects a pinned stable Rust toolchain, a Cargo workspace, strict formatting and
lint gates, explicit error handling, dependency-health checks, unsafe-code
policy, dependency centralization, lint inheritance, and intentional profile
choices. The repository already has most of this policy in code, but AR13-08
found that the differences from the concern were implicit rather than
authorized.

## Decision

heimq is a Rust Cargo workspace governed by this policy.

| Policy Area | Decision | Repository Evidence |
|-------------|----------|---------------------|
| Runtime concern | `rust-cargo` fills the `language-runtime` slot. `typescript-bun` is not selected because heimq is a headless Kafka-compatible broker and shared Rust engine. | `Cargo.toml`; `crates/heimq/docs/helix/01-frame/concerns.md` |
| Edition | First-party crates use edition 2021. Edition 2024 is not required for the current MSRV and would add migration work without product value. | `Cargo.toml`; first-party `Cargo.toml` manifests |
| MSRV/toolchain | `workspace.package.rust-version` and `rust-toolchain.toml` must stay in lockstep: `1.88` package metadata maps to `1.88.0` toolchain channel. | `Cargo.toml`; `rust-toolchain.toml` |
| Toolchain components | Rustfmt and clippy are required components of the pinned toolchain. | `rust-toolchain.toml`; `.github/workflows/test.yml` |
| Workspace membership | First-party crates are workspace members. The vendored `heimq-protocol` fork is excluded and consumed as a path dependency so it is treated as third-party source under Cargo dependency lint policy. | root `Cargo.toml`; ADR-007 |
| Formatting | `cargo fmt --all -- --check` is the required formatting gate. | `justfile`; `.github/workflows/test.yml` |
| Lints | `cargo clippy --workspace --all-targets -- -D warnings` is the required first-party lint gate. | `justfile`; `.github/workflows/test.yml` |
| Tests | `cargo test --workspace --all-targets` is the required Rust workspace test gate. | `justfile`; `.github/workflows/test.yml` |
| Dependency audit | `cargo deny --all-features check` is the required advisories/license/source/bans gate. | `deny.toml`; `justfile`; `.github/workflows/test.yml` |
| Unused dependencies | `cargo machete` is the required unused-dependency gate for first-party manifests. | `justfile`; `.github/workflows/test.yml` |
| Error handling | Library crates use typed errors (`thiserror`) at boundaries. The `heimq` binary may use `anyhow` at the composition root. | crate manifests and error modules |
| Profiles | Release builds use LTO, one codegen unit, stripped symbols, and aborting panic to favor small deployable binaries. Bench profile mirrors the LTO/codegen settings for comparable benchmark artifacts. | root `Cargo.toml` |

## Authorized Deviations

| Concern Practice | Repository Difference | Authorization |
|------------------|-----------------------|---------------|
| Deny unsafe code by default | `heimq-wire` already forbids unsafe. Some first-party test/helper code uses unsafe for a no-op waker and capture stream test plumbing, and `heimq-protocol` contains upstream vendored unsafe code. | Do not mechanically add workspace-wide `forbid(unsafe_code)` in this bead. Unsafe is allowed only where already present, must stay local and justified by code review, and any production-path expansion requires a new ADR or TD section with safety invariants and tests. |
| Workspace lint inheritance | The workspace does not yet define `[workspace.lints]` or `lints.workspace = true` in every first-party crate. | The enforced source of truth is the clippy command with `-D warnings`, because it covers all workspace targets today without manifest churn. A future crate-publication cleanup may add lint inheritance, but it must not replace the command gate. |
| Dependency centralization | Dependency versions are repeated across crate manifests instead of centralized in `[workspace.dependencies]`. | Repetition is authorized until the crate publication surface stabilizes. Local manifests make embedder-facing dependency intent visible and avoid a broad mechanical rewrite while the split workspace is still changing. Duplicate version drift is guarded by `cargo deny`, `cargo machete`, review, and the workspace lockfile. |
| No `.unwrap()` in library crates | Tests, conformance fixtures, and some byte-decoding internals use `.unwrap()` / `.expect()` where setup failure should panic or where slice lengths have already been checked. | This is authorized for tests/fixtures and for immediately-prechecked internal conversions. New production-path unwraps that can be reached from malformed client input or backend errors are not allowed without local proof and review. |
| Pinned command execution | CI pins the Rust toolchain to `1.88.0`; command-line helper versions are pinned where installed by CI. Local shells may still need `rustup`/PATH configuration to honor `rust-toolchain.toml`. | CI is the authoritative pinned execution surface. Local `just` commands are the same commands and are accepted when run under the pinned Rust toolchain or an equivalent `rustup run 1.88.0 cargo ...` invocation. |
| All-features workspace gates | The standard test and clippy gates are `--workspace --all-targets`, not `--all-features`. | `cargo deny` runs with `--all-features`; feature-specific runtime behavior is covered by targeted conformance workflows. Expanding every fast Rust gate to all features requires a separate compatibility pass because optional Postgres and conformance paths have external-service expectations. |

## Consequences

Positive:

- AR13-08 has a single accepted decision for Rust/Cargo policy.
- CI and local `just` commands now include formatting, clippy, tests, dependency
  audit, and unused-dependency checks.
- Intentional differences from the HELIX concern are explicit instead of
  implicit drift.

Negative:

- Dependency centralization and manifest lint inheritance remain deferred.
- Unsafe-code policy is stricter than status quo for future changes but does
  not remove existing unsafe helper code.

## Validation

The policy is valid when these commands pass, or when the same commands are run
through the pinned Rust toolchain explicitly:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo deny --all-features check
cargo machete
```

`rust-toolchain.toml` and `workspace.package.rust-version` must be reviewed
together on every Rust upgrade.
