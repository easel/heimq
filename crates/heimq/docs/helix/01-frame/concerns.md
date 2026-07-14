---
ddx:
  id: helix.concerns
  depends_on:
    - helix.prd
    - helix.product-vision
  review:
    self_hash: 2e97e292fbf1bb7c94f8a3476e4bc6e9133a2a685354601a6209a5438f024d42
    deps:
      helix.prd: debc0a32007f0c42db51e82f47848c7b988c3f67f6f97171069170492f9b5b95
      helix.product-vision: 8fda503ba36c48175e42be03782c96af196de39d6e87b6edab88d853ee1857ca
    reviewed_at: "2026-07-14T06:48:37Z"
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
| `rust-cargo` | Shipped `language-runtime` slot default is `typescript-bun`; HELIX rust-cargo practices expect pinned Rust, Cargo workspace gates, dependency checks, lint policy, and unsafe/dependency governance. | `rust-cargo` fills the slot; edition 2021, Rust 1.88/1.88.0 lockstep, clippy/fmt/test/deny/machete gates, excluded vendored `heimq-protocol`, and authorized deviations for unsafe helper code, manifest lint inheritance, dependency centralization, and local pinned-command execution are governed by ADR-008. | ADR-008 (source: assumption) |
| `verification` | Whole-stack evidence via browser e2e flows. | Whole-stack evidence is the differential parity harness vs Redpanda plus benchmark and ecosystem harness runs (PRD FEAT-003/004/005). | ADR-008 for the runtime-slot conflict; PRD FEAT-003/004/005 for harness authority (source: assumption) |

## Artifact Impact Evidence

| Finding | Impacted Artifacts | Resolution Evidence |
|---------|--------------------|---------------------|
| AR13-08 | `Cargo.toml`, `rust-toolchain.toml`, crate manifests, `justfile`, `.github/workflows/test.yml`, `deny.toml`, ADR set | ADR-008 accepts the Rust/Cargo workspace policy, records intentional deviations from the HELIX 0.10.2 `rust-cargo` concern, and makes the missing `cargo machete` gate explicit in local and CI commands. |

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
