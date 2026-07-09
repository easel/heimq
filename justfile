set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

fmt:
    cargo fmt --all -- --check

clippy:
    cargo clippy --workspace --all-targets -- -D warnings

test:
    cargo test --workspace --all-targets

# Supply-chain audit: advisories, licenses, banned crates, source origins.
deny:
    cargo deny --all-features check

docker-build:
    docker build -t heimq:local .

helm-check:
    helm lint charts/heimq
    helm template heimq charts/heimq >/dev/null

parity:
    PARITY_TESTS=1 cargo test -p heimq --test parity -- --test-threads=1

bench-smoke:
    cargo build -p heimq --bin heimq
    tmpdir="$(mktemp -d)"
    trap 'kill "${HEIMQ_PID:-}" 2>/dev/null || true; rm -rf "$tmpdir"' EXIT
    curl -fsSL https://downloads.apache.org/kafka/4.3.0/kafka_2.13-4.3.0.tgz -o "$tmpdir/kafka.tgz"
    tar -xzf "$tmpdir/kafka.tgz" -C "$tmpdir"
    ./target/debug/heimq --host 0.0.0.0 --port 9094 --memory-only &
    HEIMQ_PID=$!
    timeout 15 bash -c 'until nc -z localhost 9094; do sleep 0.2; done'
    PATH="$tmpdir/kafka_2.13-4.3.0/bin:$PATH" BOOTSTRAP=localhost:9094 bash scripts/bench/run-smoke.sh

ci: fmt clippy test deny helm-check

release-check:
    cargo build -p heimq --release
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace --all-targets
    cargo deny --all-features check
    helm lint charts/heimq
    helm template heimq charts/heimq >/dev/null
