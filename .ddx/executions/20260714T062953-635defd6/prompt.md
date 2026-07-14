<bead-review>
  <bead id="heimq-08416254" iter=1>
    <title>Authorize and realize the Heimq Rust workspace concern policy</title>
    <description>
Resolve AR13-08. Add an ADR that records Heimq Rust/Cargo policy and explicit project overrides from the HELIX 0.10.2 rust-cargo concern. Reconcile edition/MSRV/toolchain lockstep, workspace lints, unsafe policy, lint inheritance, dependency centralization, cargo deny/machete, pinned command execution, and profile choices. Implement safe missing gates where feasible; explicitly authorize intentional deviations with rationale rather than silently weakening them. Update concerns.md Project Overrides and artifact-impact evidence. In scope: ADR, concerns.md, root Cargo.toml, manifests, justfile, CI, and a pinned wrapper if adopted. Out of scope: behavior changes unrelated to build policy.
    </description>
    <acceptance>
1. The new accepted ADR accounts for every rust-cargo constraint and practice that differs from the repository. 2. concerns.md no longer contains Needs ADR for selected concern overrides. 3. cargo fmt --all -- --check, cargo clippy --workspace --all-targets -- -D warnings, cargo test --workspace --all-targets, cargo deny --all-features check, and cargo machete pass or an ADR-authorized equivalent command passes. 4. rust-toolchain.toml and workspace.package.rust-version remain in lockstep.
    </acceptance>
    <labels>helix, area:architecture, area:infra, kind:quality</labels>
  </bead>

  <changed-files>
    <file>.github/workflows/test.yml</file>
    <file>Cargo.lock</file>
    <file>crates/heimq-testkit/Cargo.toml</file>
    <file>crates/heimq/Cargo.toml</file>
    <file>crates/heimq/docs/helix/01-frame/concerns.md</file>
    <file>crates/heimq/docs/helix/02-design/adr/ADR-008-rust-cargo-policy.md</file>
    <file>fuzz/Cargo.toml</file>
    <file>justfile</file>
  </changed-files>

  <governing>
    <ref id="helix.concerns" path="crates/heimq/docs/helix/01-frame/concerns.md" title="Project Concerns">
      <content>
<untrusted-data>
---
ddx:
  id: helix.concerns
  depends_on:
    - helix.prd
    - helix.product-vision
  review:
    self_hash: a0172ef88e0fe7e93e4c789e8f33b2d1a33cde54b7c032516a20c0dbaeca4049
    deps:
      helix.prd: 96f0479e307f2c240e8f47b69fff510164d0b9eda132abb22cc4a860932984fe
      helix.product-vision: 8fda503ba36c48175e42be03782c96af196de39d6e87b6edab88d853ee1857ca
    reviewed_at: "2026-07-14T05:12:26Z"
---

# Project Concerns

Project Concerns declare active cross-cutting context for heimq. They are not
principles, requirements, ADRs, test plans, or implementation tasks. All slot
decisions below are inferred from the codebase, product vision, and PRD
(source: assumption); none has an operator-confirmed `concerns.local.yml`.

## Active Concerns

| Concern | Source | Areas | Why Active | Key Practices |
|---------|--------|-------|------------|---------------|
| `rust-cargo` | library (source: assumption) | all | Fills the `language-runtime` slot. heimq is a Rust Cargo workspace (`Cargo.toml` at repo root) shipping a single binary. Overrides the shipped slot default `typescript-bun`. | Workspace lints with clippy `-D warnings`; `cargo fmt` check; `thiserror` for library errors, `anyhow` in binaries; no `.unwrap()` in library crates; pinned toolchain; `cargo deny` / `cargo machete` clean. |
| `testing` | library (source: assumption) | all | PRD success metrics are test-defined: contract tests per in-scope API, property tests (`proptest`), integration tests via `rdkafka`. | Multi-layer coverage (unit/property/contract/integration); stubs over mocks; generated data over fixtures; every acceptance criterion traced to a test; flaky tests treated as bugs. |
| `verification` | library (source: assumption) | all | The differential parity harness vs Redpanda (PRD FEAT-003) is the whole-stack evidence surface: work is not done until the parity, benchmark (FEAT-004), and ecosystem (FEAT-005) harnesses run with recorded evidence. | Recorded command + exit status for parity/bench/ecosystem runs; adversarial re-review against acceptance criteria; never report an unobserved result. |
| `single-binary-broker` | project-local (source: assumption) | all | Fills the `architecture-style` slot (no shipped default; no library layering style matches a protocol server). heimq is a single-process, single-node, single-binary broker — no replication, no controller quorum (PRD Constraints). | Keep all coordinators (group, txn, idempotence) single-process; no distributed-consensus abstractions; fast cold start and small footprint are product properties, not optimizations. |
| `in-memory-datastore` | project-local (source: assumption) | `area:data` | Fills the `datastore` slot (library has no members for this slot). Primary state is in-memory and may be lost on restart (PRD Non-Goal 1); Postgres is an opt-in offsets backend only (`HEIMQ_STORAGE_OFFSETS=postgres`, PRD Technical Context). | In-memory is the default and the correctness baseline; durable backends are opt-in slices that must not be required for in-scope feature correctness; restart presents as a fresh broker per Kafka semantics. |

Slot decisions explicitly **not applicable** (source: assumption): heimq is a
headless wire-protocol server plus test tooling — not a UI, account, or
operator-console product. Therefore the `frontend-framework` slot (default
`react-nextjs`), the `e2e-framework` browser slot (default `e2e-playwright`),
and the `auth-provider` slot (default `auth-local-sessions`; the PRD lists
security/SASL/ACLs as Non-Goal 4) are vacant, and the composable
`admin-console` and `sample-data` UI concerns are not selected. Whole-stack
e2e is owned by the differential parity, benchmark, and ecosystem harnesses
instead of a browser runner.

## Project Overrides

| Concern | Practice | Override | Authority |
|---------|----------|----------|-----------|
| `rust-cargo` | Shipped `language-runtime` slot default is `typescript-bun`. | `rust-cargo` fills the slot; the codebase is a Rust Cargo workspace. | Needs ADR (source: assumption) |
| `verification` | Whole-stack evidence via browser e2e flows. | Whole-stack evidence is the differential parity harness vs Redpanda plus benchmark and ecosystem harness runs (PRD FEAT-003/004/005). | Needs ADR (source: assumption) |

## Area Labels

This project uses the following area labels for concern scoping
(source: assumption — no UI area; tailored from the default set):

- `area:api` — Kafka wire-protocol surface: codecs, version negotiation, request handlers
- `area:data` — in-memory log/group/txn state; opt-in Postgres offsets backend
- `area:infra` — CI, Docker harnesses (Redpanda, ecosystem tools), packaging/release
- `area:cli` — heimq binary startup, configuration, runtime flags
- `area:testing` — parity, contract, property, benchmark, and ecosystem suites

## Concern Conflicts

| Conflict | Resolution |
|----------|------------|
| `testing` (real dependencies via Docker: Redpanda, ecosystem tools) vs. fast hermetic CI (the product's own value proposition) | Unit, property, and contract layers stay hermetic and Docker-free; only parity, benchmark, and ecosystem suites require Docker and may be gated to dedicated CI jobs. (source: assumption) |
| `in-memory-datastore` (state lost on restart) vs. `verification` (evidence of restart/recovery behavior) | Verify restart behavior as Kafka-spec behavior (fresh broker, `UNKNOWN_PRODUCER_ID`, re-init via `InitProducerId`) per PRD Resolved Decisions — do not add heimq-specific recovery to satisfy evidence collection. (source: assumption) |
</untrusted-data>
      </content>
    </ref>
  </governing>

  <diff rev="7a14b0783e2c6344338711308cc7f3a4454b6505">
<untrusted-data>
diff --git a/crates/heimq/docs/helix/01-frame/concerns.md b/crates/heimq/docs/helix/01-frame/concerns.md
index 8eac701..2d07185 100644
--- a/crates/heimq/docs/helix/01-frame/concerns.md
+++ b/crates/heimq/docs/helix/01-frame/concerns.md
@@ -43,8 +43,14 @@ instead of a browser runner.
 
 | Concern | Practice | Override | Authority |
 |---------|----------|----------|-----------|
-| `rust-cargo` | Shipped `language-runtime` slot default is `typescript-bun`. | `rust-cargo` fills the slot; the codebase is a Rust Cargo workspace. | Needs ADR (source: assumption) |
-| `verification` | Whole-stack evidence via browser e2e flows. | Whole-stack evidence is the differential parity harness vs Redpanda plus benchmark and ecosystem harness runs (PRD FEAT-003/004/005). | Needs ADR (source: assumption) |
+| `rust-cargo` | Shipped `language-runtime` slot default is `typescript-bun`; HELIX rust-cargo practices expect pinned Rust, Cargo workspace gates, dependency checks, lint policy, and unsafe/dependency governance. | `rust-cargo` fills the slot; edition 2021, Rust 1.88/1.88.0 lockstep, clippy/fmt/test/deny/machete gates, excluded vendored `heimq-protocol`, and authorized deviations for unsafe helper code, manifest lint inheritance, dependency centralization, and local pinned-command execution are governed by ADR-008. | ADR-008 (source: assumption) |
+| `verification` | Whole-stack evidence via browser e2e flows. | Whole-stack evidence is the differential parity harness vs Redpanda plus benchmark and ecosystem harness runs (PRD FEAT-003/004/005). | ADR-008 for the runtime-slot conflict; PRD FEAT-003/004/005 for harness authority (source: assumption) |
+
+## Artifact Impact Evidence
+
+| Finding | Impacted Artifacts | Resolution Evidence |
+|---------|--------------------|---------------------|
+| AR13-08 | `Cargo.toml`, `rust-toolchain.toml`, crate manifests, `justfile`, `.github/workflows/test.yml`, `deny.toml`, ADR set | ADR-008 accepts the Rust/Cargo workspace policy, records intentional deviations from the HELIX 0.10.2 `rust-cargo` concern, and makes the missing `cargo machete` gate explicit in local and CI commands. |
 
 ## Area Labels
 
diff --git a/.github/workflows/test.yml b/.github/workflows/test.yml
index 20354b3..a452195 100644
--- a/.github/workflows/test.yml
+++ b/.github/workflows/test.yml
@@ -31,7 +31,7 @@ jobs:
       - uses: actions/checkout@v4
 
       - name: Install Rust toolchain
-        uses: dtolnay/rust-toolchain@stable
+        uses: dtolnay/rust-toolchain@1.88.0
         with:
           components: clippy, rustfmt
 
@@ -46,3 +46,9 @@ jobs:
 
       - name: Run workspace tests
         run: cargo test --workspace --all-targets
+
+      - name: Install cargo-machete
+        run: cargo install cargo-machete --locked --version 0.9.1
+
+      - name: Check unused dependencies
+        run: cargo machete
diff --git a/Cargo.lock b/Cargo.lock
index d42c73c..0df7e67 100644
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -514,38 +514,6 @@ dependencies = [
  "cfg-if",
 ]
 
-[[package]]
-name = "crossbeam"
-version = "0.8.4"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "1137cd7e7fc0fb5d3c5a8678be38ec56e819125d8d7907411fe24ccb943faca8"
-dependencies = [
- "crossbeam-channel",
- "crossbeam-deque",
- "crossbeam-epoch",
- "crossbeam-queue",
- "crossbeam-utils",
-]
-
-[[package]]
-name = "crossbeam-channel"
-version = "0.5.16"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "d85363c37faeca707aef026efa9f3b34d077bce547e48f770770625c6013679e"
-dependencies = [
- "crossbeam-utils",
-]
-
-[[package]]
-name = "crossbeam-deque"
-version = "0.8.7"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "5181e0de7b61eb03a81e347d6dd8797bae9da5146707b51077e2d71a54ec0ceb"
-dependencies = [
- "crossbeam-epoch",
- "crossbeam-utils",
-]
-
 [[package]]
 name = "crossbeam-epoch"
 version = "0.9.20"
@@ -555,15 +523,6 @@ dependencies = [
  "crossbeam-utils",
 ]
 
-[[package]]
-name = "crossbeam-queue"
-version = "0.3.13"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "803d13fb3b09d88be9f4dbc29062c66b19bf7170867ceb746d2a8689bf6c7a26"
-dependencies = [
- "crossbeam-utils",
-]
-
 [[package]]
 name = "crossbeam-utils"
 version = "0.8.22"
@@ -1005,17 +964,12 @@ dependencies = [
  "bytes",
  "chrono",
  "clap",
- "crc32c",
- "crossbeam",
  "dashmap",
- "flate2",
  "heimq",
  "heimq-broker",
  "heimq-handlers",
  "heimq-protocol",
  "heimq-wire",
- "lz4",
- "memmap2",
  "metrics",
  "metrics-exporter-prometheus",
  "parking_lot",
@@ -1023,14 +977,11 @@ dependencies = [
  "proptest",
  "serde",
  "serde_json",
- "snap",
  "tempfile",
- "thiserror",
  "tokio",
  "tokio-test",
  "tracing",
  "tracing-subscriber",
- "zstd",
 ]
 
 [[package]]
@@ -1082,7 +1033,6 @@ dependencies = [
 name = "heimq-testkit"
 version = "0.1.0"
 dependencies = [
- "anyhow",
  "heimq",
  "heimq-protocol",
  "testcontainers",
@@ -1611,15 +1561,6 @@ version = "2.8.3"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "cf8baf1c55e62ffcace7a9f06f4bd9cd3f0c4beb022d3b367256b91b87513d98"
 
-[[package]]
-name = "memmap2"
-version = "0.9.11"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "d1219ed1b7f229ee7104d281dd01d6802fe28bb6e95d292942c4daacdeb798c0"
-dependencies = [
- "libc",
-]
-
 [[package]]
 name = "metrics"
 version = "0.24.6"
diff --git a/crates/heimq-testkit/Cargo.toml b/crates/heimq-testkit/Cargo.toml
index 7fa0d7d..46e4a5a 100644
--- a/crates/heimq-testkit/Cargo.toml
+++ b/crates/heimq-testkit/Cargo.toml
@@ -10,7 +10,6 @@ description = "Per-trait conformance suites and differential parity harness for
 
 [dependencies]
 heimq = { path = "../heimq", version = "0.1.4", features = ["test-support"] }
-anyhow = "1.0"
 # Vendored fork of kafka-protocol 0.15.1; imported as `heimq_protocol`.
 heimq-protocol = { path = "../heimq-protocol", version = "0.15.1" }
 
diff --git a/crates/heimq/Cargo.toml b/crates/heimq/Cargo.toml
index c7baa79..3f41c90 100644
--- a/crates/heimq/Cargo.toml
+++ b/crates/heimq/Cargo.toml
@@ -46,29 +46,15 @@ tracing-subscriber = { version = "0.3", features = ["env-filter"] }
 clap = { version = "4.5", features = ["derive", "env"] }
 
 # Error handling
-thiserror = "2.0"
 anyhow = "1.0"
 
 # Data structures
 dashmap = "6.1"
 parking_lot = "0.12"
-crossbeam = "0.8"
 
 # Time
 chrono = "0.4"
 
-# CRC for record batches
-crc32c = "0.6"
-
-# Compression support
-flate2 = "1.0"          # gzip
-snap = "1.1"            # snappy
-lz4 = "1.28"            # lz4
-zstd = "0.13"           # zstd
-
-# Optional persistence
-memmap2 = "0.9"
-
 # Metrics
 metrics = "0.24"
 metrics-exporter-prometheus = "0.18"
diff --git a/crates/heimq/docs/helix/02-design/adr/ADR-008-rust-cargo-policy.md b/crates/heimq/docs/helix/02-design/adr/ADR-008-rust-cargo-policy.md
new file mode 100644
index 0000000..d52bdd8
--- /dev/null
+++ b/crates/heimq/docs/helix/02-design/adr/ADR-008-rust-cargo-policy.md
@@ -0,0 +1,84 @@
+---
+ddx:
+  id: ADR-008
+  status: accepted
+---
+
+# ADR-008: Rust Cargo Workspace Policy
+
+| Date | Status | Deciders | Related | Confidence |
+|------|--------|----------|---------|------------|
+| 2026-07-14 | Accepted | heimq maintainers | concerns.md, AR13-08, ADR-007 | High |
+
+## Context
+
+HELIX 0.10.2 selected the `rust-cargo` concern for heimq instead of the
+shipped `language-runtime` slot default, `typescript-bun`. The selected concern
+expects a pinned stable Rust toolchain, a Cargo workspace, strict formatting and
+lint gates, explicit error handling, dependency-health checks, unsafe-code
+policy, dependency centralization, lint inheritance, and intentional profile
+choices. The repository already has most of this policy in code, but AR13-08
+found that the differences from the concern were implicit rather than
+authorized.
+
+## Decision
+
+heimq is a Rust Cargo workspace governed by this policy.
+
+| Policy Area | Decision | Repository Evidence |
+|-------------|----------|---------------------|
+| Runtime concern | `rust-cargo` fills the `language-runtime` slot. `typescript-bun` is not selected because heimq is a headless Kafka-compatible broker and shared Rust engine. | `Cargo.toml`; `crates/heimq/docs/helix/01-frame/concerns.md` |
+| Edition | First-party crates use edition 2021. Edition 2024 is not required for the current MSRV and would add migration work without product value. | `Cargo.toml`; first-party `Cargo.toml` manifests |
+| MSRV/toolchain | `workspace.package.rust-version` and `rust-toolchain.toml` must stay in lockstep: `1.88` package metadata maps to `1.88.0` toolchain channel. | `Cargo.toml`; `rust-toolchain.toml` |
+| Toolchain components | Rustfmt and clippy are required components of the pinned toolchain. | `rust-toolchain.toml`; `.github/workflows/test.yml` |
+| Workspace membership | First-party crates are workspace members. The vendored `heimq-protocol` fork is excluded and consumed as a path dependency so it is treated as third-party source under Cargo dependency lint policy. | root `Cargo.toml`; ADR-007 |
+| Formatting | `cargo fmt --all -- --check` is the required formatting gate. | `justfile`; `.github/workflows/test.yml` |
+| Lints | `cargo clippy --workspace --all-targets -- -D warnings` is the required first-party lint gate. | `justfile`; `.github/workflows/test.yml` |
+| Tests | `cargo test --workspace --all-targets` is the required Rust workspace test gate. | `justfile`; `.github/workflows/test.yml` |
+| Dependency audit | `cargo deny --all-features check` is the required advisories/license/source/bans gate. | `deny.toml`; `justfile`; `.github/workflows/test.yml` |
+| Unused dependencies | `cargo machete` is the required unused-dependency gate for first-party manifests. | `justfile`; `.github/workflows/test.yml` |
+| Error handling | Library crates use typed errors (`thiserror`) at boundaries. The `heimq` binary may use `anyhow` at the composition root. | crate manifests and error modules |
+| Profiles | Release builds use LTO, one codegen unit, stripped symbols, and aborting panic to favor small deployable binaries. Bench profile mirrors the LTO/codegen settings for comparable benchmark artifacts. | root `Cargo.toml` |
+
+## Authorized Deviations
+
+| Concern Practice | Repository Difference | Authorization |
+|------------------|-----------------------|---------------|
+| Deny unsafe code by default | `heimq-wire` already forbids unsafe. Some first-party test/helper code uses unsafe for a no-op waker and capture stream test plumbing, and `heimq-protocol` contains upstream vendored unsafe code. | Do not mechanically add workspace-wide `forbid(unsafe_code)` in this bead. Unsafe is allowed only where already present, must stay local and justified by code review, and any production-path expansion requires a new ADR or TD section with safety invariants and tests. |
+| Workspace lint inheritance | The workspace does not yet define `[workspace.lints]` or `lints.workspace = true` in every first-party crate. | The enforced source of truth is the clippy command with `-D warnings`, because it covers all workspace targets today without manifest churn. A future crate-publication cleanup may add lint inheritance, but it must not replace the command gate. |
+| Dependency centralization | Dependency versions are repeated across crate manifests instead of centralized in `[workspace.dependencies]`. | Repetition is authorized until the crate publication surface stabilizes. Local manifests make embedder-facing dependency intent visible and avoid a broad mechanical rewrite while the split workspace is still changing. Duplicate version drift is guarded by `cargo deny`, `cargo machete`, review, and the workspace lockfile. |
+| No `.unwrap()` in library crates | Tests, conformance fixtures, and some byte-decoding internals use `.unwrap()` / `.expect()` where setup failure should panic or where slice lengths have already been checked. | This is authorized for tests/fixtures and for immediately-prechecked internal conversions. New production-path unwraps that can be reached from malformed client input or backend errors are not allowed without local proof and review. |
+| Pinned command execution | CI pins the Rust toolchain to `1.88.0`; command-line helper versions are pinned where installed by CI. Local shells may still need `rustup`/PATH configuration to honor `rust-toolchain.toml`. | CI is the authoritative pinned execution surface. Local `just` commands are the same commands and are accepted when run under the pinned Rust toolchain or an equivalent `rustup run 1.88.0 cargo ...` invocation. |
+| All-features workspace gates | The standard test and clippy gates are `--workspace --all-targets`, not `--all-features`. | `cargo deny` runs with `--all-features`; feature-specific runtime behavior is covered by targeted conformance workflows. Expanding every fast Rust gate to all features requires a separate compatibility pass because optional Postgres and conformance paths have external-service expectations. |
+
+## Consequences
+
+Positive:
+
+- AR13-08 has a single accepted decision for Rust/Cargo policy.
+- CI and local `just` commands now include formatting, clippy, tests, dependency
+  audit, and unused-dependency checks.
+- Intentional differences from the HELIX concern are explicit instead of
+  implicit drift.
+
+Negative:
+
+- Dependency centralization and manifest lint inheritance remain deferred.
+- Unsafe-code policy is stricter than status quo for future changes but does
+  not remove existing unsafe helper code.
+
+## Validation
+
+The policy is valid when these commands pass, or when the same commands are run
+through the pinned Rust toolchain explicitly:
+
+```bash
+cargo fmt --all -- --check
+cargo clippy --workspace --all-targets -- -D warnings
+cargo test --workspace --all-targets
+cargo deny --all-features check
+cargo machete
+```
+
+`rust-toolchain.toml` and `workspace.package.rust-version` must be reviewed
+together on every Rust upgrade.
diff --git a/fuzz/Cargo.toml b/fuzz/Cargo.toml
index 07c51f5..c28d6d1 100644
--- a/fuzz/Cargo.toml
+++ b/fuzz/Cargo.toml
@@ -10,7 +10,6 @@ cargo-fuzz = true
 [dependencies]
 libfuzzer-sys = "0.4"
 heimq = { path = "../crates/heimq", features = ["test-support"] }
-heimq-broker = { path = "../crates/heimq-broker" }
 
 [[bin]]
 name = "route"
diff --git a/justfile b/justfile
index 49dd9d2..d8d9067 100644
--- a/justfile
+++ b/justfile
@@ -13,6 +13,9 @@ test:
 deny:
     cargo deny --all-features check
 
+machete:
+    cargo machete
+
 docker-build:
     docker build -t heimq:local .
 
@@ -34,7 +37,7 @@ bench-smoke:
     timeout 15 bash -c 'until nc -z localhost 9094; do sleep 0.2; done'
     PATH="$tmpdir/kafka_2.13-4.3.0/bin:$PATH" BOOTSTRAP=localhost:9094 bash scripts/bench/run-smoke.sh
 
-ci: fmt clippy test deny helm-check
+ci: fmt clippy test deny machete helm-check
 
 release-check:
     cargo build -p heimq --release
@@ -42,5 +45,6 @@ release-check:
     cargo clippy --workspace --all-targets -- -D warnings
     cargo test --workspace --all-targets
     cargo deny --all-features check
+    cargo machete
     helm lint charts/heimq
     helm template heimq charts/heimq >/dev/null
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
