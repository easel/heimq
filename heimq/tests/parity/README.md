# Parity Harness (`tests/parity/`)

Differential parity harness per HARNESS-001. Runs identical client workloads
against heimq and a Redpanda container, normalizes non-determinism, and asserts
zero behavioral diffs for in-scope APIs. Implements FEAT-003.

## Prerequisites

- Docker daemon running on the host
- `cargo build` completed (for the `heimq` binary)

## Running

```sh
# Default cargo test skips parity tests (Docker not required)
cargo test

# Run parity tests (requires Docker)
PARITY_TESTS=1 cargo test --test parity -- --nocapture
```

## Architecture

```
tests/parity/
├── main.rs           -- harness entry: boot containers, run workloads, report
├── broker.rs         -- BrokerTarget, testcontainers boot, rdkafka probe
├── driver.rs         -- WorkloadDriver trait and Observation types
├── workloads/
│   └── mod.rs        -- workload registry (empty: scaffolding bead)
├── normalize.rs      -- field-level normalization (HARNESS-001 §Normalization)
├── diff.rs           -- diff engine, DiffRecord JSONL schema
├── exemptions.rs     -- exemptions.toml loader with fail-fast validation
└── exemptions.toml   -- known-divergence declarations
```

## Adding Workloads

Subsequent beads add `workloads/produce_fetch.rs` and `workloads/consumer_group.rs`
and register them in `workloads::all()`. Each workload implements `WorkloadDriver`:

```rust
#[async_trait]
impl WorkloadDriver for MyWorkload {
    fn name(&self) -> &'static str { "my_workload" }
    async fn run(&self, target: &BrokerTarget) -> Result<Vec<Observation>> { ... }
}
```

## Diff Output

Diffs are written to `target/parity/<timestamp>-<workload>.jsonl` (gitignored).
Each line is a `DiffRecord` JSON object per HARNESS-001 §Structured Diff Format.

Summary lines are printed to stdout:
```
[PASS] produce_fetch_roundtrip: 0 diffs
[FAIL] consumer_group_lifecycle: 2 diffs (0 exempt, 2 unmatched)
```

Exit code: `0` when all unmatched diffs are zero; `1` otherwise.

## Known Divergences

`exemptions.toml` declares intentional behavioral differences. Each entry must
cite a `prd_ref`. The loader validates uniqueness of ids and non-empty `prd_ref`
at startup. Entries with `status = "future"` are loaded but not enforced until
the referenced workload is active.

## CI

A `parity` GitHub Actions job (added in the FEAT-003 build phase) runs on PRs
touching `src/protocol/`, `src/handler/`, or `tests/parity/`. Docker is
available by default on `ubuntu-latest` runners.
