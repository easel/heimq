# Heimq Release Infrastructure Plan

## Goal

Make heimq release-ready for users who want to install, test, and operate a
single-node Kafka-compatible broker from source, binaries, Docker images, Helm,
or hosted documentation.

The finished state must have:

- a GitHub release workflow that creates release artifacts, not only Actions
  artifacts;
- a published Docker image workflow with release-grade tags and metadata;
- a Helm chart that is linted, packaged, and published from CI;
- a README that supports first-use without chat context;
- a microsite or static docs site published from CI;
- reproducible local and CI test infrastructure with pinned toolchains,
  scripts, and documented evidence commands.

## Current State

Evidence from the repository on 2026-06-16:

- [`.github/workflows/test.yml`](../../.github/workflows/test.yml) runs format,
  clippy, and workspace tests.
- [`.github/workflows/release-binaries.yml`](../../.github/workflows/release-binaries.yml)
  builds `heimq-linux-x86_64` and uploads it as a workflow artifact. It does not
  create a GitHub Release, attach artifacts to a release, or publish provenance.
- [`.github/workflows/docker-image.yml`](../../.github/workflows/docker-image.yml)
  publishes to GHCR for `v*` tags and manual dispatch, but does not document or
  verify the image with a smoke test in the workflow.
- [`.github/workflows/parity.yml`](../../.github/workflows/parity.yml),
  [`bench-smoke.yml`](../../.github/workflows/bench-smoke.yml),
  [`bench-omb.yml`](../../.github/workflows/bench-omb.yml),
  [`postgres-conformance.yml`](../../.github/workflows/postgres-conformance.yml),
  [`stress-matrix.yml`](../../.github/workflows/stress-matrix.yml), and
  [`ecosystem.yml`](../../.github/workflows/ecosystem.yml) provide useful test
  coverage but have duplicated setup and mixed required/optional semantics.
- [README.md](../../README.md) only contains the project title.
- [Dockerfile](../../Dockerfile) builds a runtime image.
- [charts/heimq](../../charts/heimq/Chart.yaml) contains a Helm chart, but there
  is no chart lint/package/publish workflow.
- No `site/`, `website/`, `mkdocs.yml`, `docusaurus.config.*`, `astro.config.*`,
  or `package.json` exists for a microsite.
- `Cargo.lock` exists. `rust-toolchain.toml` and `justfile` now exist; there
  is still no `Makefile`, `compose.yml`, or `.devcontainer` at the repository
  root.

## Base Contract

This repository treats the following as the baseline for release work:

- canonical repository: `easel/heimq`;
- Rust toolchain: `1.85.1` with `clippy` and `rustfmt`;
- Docker builder image: `rust:1.85.1-bookworm`;
- local command surface: `fmt`, `clippy`, `test`, `ci`, `docker-build`,
  `helm-check`, `bench-smoke`, and `parity`;
- release tag mapping: `vX.Y.Z` -> binary `X.Y.Z` -> Docker
  `vX.Y.Z` / `X.Y.Z` / `X.Y` / `X` -> Helm chart `version = X.Y.Z` and
  `appVersion = X.Y.Z`.

## Scope

In scope:

- release workflows under `.github/workflows/`;
- Docker release metadata and smoke verification;
- Helm chart validation and publication;
- root README and chart README improvements;
- a static documentation site and Pages workflow;
- reproducible local test entrypoints and pinned toolchain/config files;
- release runbook documentation.

Out of scope for this plan:

- changing Kafka protocol behavior;
- changing storage semantics;
- making heimq a clustered broker;
- publishing a crates.io release unless a separate bead explicitly adds it;
- modifying sibling repositories such as fjord or niflheim.

## Assumptions

- Release tags use `vMAJOR.MINOR.PATCH`.
- The canonical repository for release artifacts is `easel/heimq`, matching the
  configured `origin` remote. Cargo workspace metadata, README links, release
  workflows, Docker labels, and Helm chart docs must not publish a conflicting
  `github.com/heimq/heimq` namespace.
- The default public image is `ghcr.io/easel/heimq`. Workflows may compute this
  from `github.repository_owner`, but checked-in docs and chart defaults must
  use the canonical public image unless the repository owner changes it in this
  plan first.
- Git tag to artifact mapping:
  - git tag: `vX.Y.Z`;
  - binary/package version: `X.Y.Z`;
  - Docker tags: `vX.Y.Z`, `X.Y.Z`, `X.Y`, `X`; publish `latest` only for stable
    non-prerelease tags;
  - Helm chart `version`: `X.Y.Z`;
  - Helm chart `appVersion`: `X.Y.Z`;
  - Helm default image tag: chart `appVersion`, which is valid because Docker
    also publishes `X.Y.Z`.
- The Helm chart publishes as an OCI chart to GHCR unless the repository owner
  chooses GitHub Pages chart indexes later.
- The microsite uses checked-in static HTML/CSS under `site/` and a GitHub Pages
  workflow that uploads that directory. Do not introduce Node, Docusaurus,
  MkDocs, or Astro unless a later bead changes this plan with evidence.
- Rust is pinned with `rust-toolchain.toml` using exact channel `1.85.1` and
  components `clippy` and `rustfmt`. CI workflows must consume this file or
  mirror the same exact version. The Docker builder image must use
  `rust:1.85.1-bookworm` or document why an equivalent digest-pinned image is
  used.
- GitHub release workflows must have their own tag-triggered validation job that
  runs the same required checks as `test.yml`, because the existing `test.yml`
  only runs on pushes to `main`, pull requests, and manual dispatch.
- Docker-based parity and ecosystem jobs remain optional unless branch
  protection is changed by an operator.

## Work Breakdown

### 1. Release Binary Workflow

Upgrade `release-binaries.yml` from workflow-artifact upload to a real GitHub
Release publisher.

Required outcomes:

- run on `v*` tags and manual dispatch;
- build at least Linux x86_64;
- attach binary and SHA256 checksum to a GitHub Release;
- include release notes generated from tag/commit metadata or a checked-in
  template;
- include a tag-triggered validation job that runs format, clippy, and workspace
  tests before any release artifact is published.
- use the canonical `easel/heimq` release namespace in release URLs and metadata.

Validation:

- `cargo build -p heimq --release`;
- workflow syntax review;
- dry-run or manual-dispatch instructions documented if GitHub Release creation
  cannot be fully tested locally.

### 2. Docker Image Release

Harden `docker-image.yml` and Docker usage docs.

Required outcomes:

- semver tags for release pushes: `vX.Y.Z`, `X.Y.Z`, `X.Y`, `X`, and `latest`
  for stable non-prerelease tags;
- OCI labels from `docker/metadata-action`;
- image digest surfaced in workflow output;
- smoke test that runs the built image, waits for TCP readiness, and then
  verifies Kafka usability with an explicit metadata or produce/fetch check using
  the image's advertised host settings before pushing or as a post-build check.

Validation:

- `docker build -t heimq:local .`;
- `docker run --rm -p 9092:9092 heimq:local` smoke instructions;
- workflow syntax review.

### 3. Helm Chart CI And Publication

Make the chart installable and release-published.

Required outcomes:

- CI job for `helm lint charts/heimq`;
- CI job for `helm template heimq charts/heimq`;
- release workflow packages the chart;
- chart version and `appVersion` are updated or validated against release tags;
- chart publish path is documented, preferably GHCR OCI.
- chart image defaults resolve to a Docker tag published by the same release
  workflow (`X.Y.Z` from chart `appVersion`).

Validation:

- `helm lint charts/heimq`;
- `helm template heimq charts/heimq`;
- `helm package charts/heimq`.

### 4. README

Replace the placeholder root README.

Required outcomes:

- project purpose and non-goals;
- quick start from source;
- Docker quick start;
- Helm quick start;
- basic producer/consumer smoke test;
- compatibility and known limits;
- release artifact links;
- test and benchmark commands;
- contribution/development setup.

Validation:

- README commands are copy-pasteable and reference existing files or workflows;
- links to chart, workflows, and scripts resolve.

### 5. Microsite

Add a lightweight static site and publish it from CI.

Required outcomes:

- static source under `site/`;
- pages for overview, quickstart, deployment, compatibility/testing, and
  architecture;
- GitHub Pages workflow builds and publishes the site on `main`;
- site content does not diverge from README install commands.
- no package manager or JavaScript build dependency is introduced in the first
  pass; use plain HTML/CSS and copied content links.

Validation:

- local site build command passes;
- CI workflow syntax exists;
- generated site is ignored or intentionally committed according to the chosen
  generator.

### 6. Reproducible Test Infrastructure

Add one local command surface that mirrors CI.

Required outcomes:

- pinned Rust toolchain via `rust-toolchain.toml`;
- a `justfile` or `Makefile` with `fmt`, `clippy`, `test`, `ci`, `docker-build`,
  `helm-check`, `bench-smoke`, and `parity` targets;
- pinned external image/tool versions collected in docs or scripts;
- optional Docker Compose file for local Postgres/parity dependencies if it
  reduces setup drift;
- CI workflows call the same scripts where practical.

Validation:

- `cargo fmt --all -- --check`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- `cargo test --workspace --all-targets`;
- `rustup show active-toolchain` reports the pinned toolchain when run from the
  repository, or CI logs prove the pinned toolchain was installed;
- `helm lint charts/heimq` and `helm template heimq charts/heimq` when Helm is
  installed;
- `docker build -t heimq:local .` when Docker is installed.

### 7. Release Runbook

Document release operation.

Required outcomes:

- tag naming;
- pre-release verification;
- release workflow behavior;
- artifact verification;
- Docker and Helm publish locations;
- rollback/recovery for a bad release.

Validation:

- runbook references exact workflows and commands;
- release checklist can be followed without hidden chat context.

## Validation

Every implementation bead must include at least one command-based acceptance
criterion. The final parent bead closes only when:

- release, Docker, Helm, README, microsite, and reproducible-test child beads are
  closed;
- release artifact namespace is consistent across Cargo metadata, workflows,
  README, Docker labels, Helm defaults, and site content;
- `vX.Y.Z` tag mapping to binary, Docker, and Helm versions is documented and
  enforced by workflows or release-check scripts;
- `cargo fmt --all -- --check` passes;
- `cargo clippy --workspace --all-targets -- -D warnings` passes;
- `cargo test --workspace --all-targets` passes;
- Helm chart lint/template commands pass or the bead documents that Helm is not
  available locally and CI covers it;
- Docker build/smoke commands pass or the bead documents that Docker is not
  available locally and CI covers it;
- README and site commands agree.

## Risks

- GitHub Release creation cannot be fully proven locally; use `workflow_dispatch`
  dry-run paths or document the exact tag-based test.
- Helm publishing needs a registry decision. Default to GHCR OCI unless a
  stronger project convention exists.
- Microsite tooling can add unnecessary JavaScript/build complexity. Prefer a
  static, low-dependency tool.
- Docker, parity, and ecosystem tests may be slow or unavailable on some local
  machines. Keep fast required gates separate from slow optional gates.

## Open Questions

- Should Docker `latest` be published on every stable tag, or avoided until the
  first public release?
- Should Helm charts publish to GHCR OCI only, or also a GitHub Pages chart
  repository?
- Should release binaries include macOS and arm64 in the first release hardening
  pass, or remain Linux x86_64 until demand is clear?

## Handoff

Break this plan into DDx beads under one epic. Each child bead must inline the
relevant current-state evidence, list in-scope and out-of-scope files, and use
command-based acceptance criteria. Use `ddx work` to execute the queue and
monitor project-scoped workers with `ddx work status`.
