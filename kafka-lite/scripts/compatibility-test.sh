#!/bin/bash
# Compatibility test script
# Runs kafka-lite against a real Kafka/Redpanda broker and compares behavior

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Check dependencies
check_deps() {
    if ! command -v docker &> /dev/null; then
        log_error "Docker is required for compatibility tests"
        exit 1
    fi

    if ! command -v kcat &> /dev/null && ! command -v kafkacat &> /dev/null; then
        log_warn "kcat/kafkacat not found - some tests may be skipped"
    fi
}

# Start a Redpanda container for comparison
start_redpanda() {
    log_info "Starting Redpanda container..."
    docker run -d --name kafka-lite-test-redpanda \
        -p 19092:9092 \
        docker.redpanda.com/redpandadata/redpanda:latest \
        redpanda start --smp 1 --memory 512M \
        --overprovisioned \
        --kafka-addr 0.0.0.0:9092 \
        --advertise-kafka-addr localhost:19092

    # Wait for Redpanda to be ready
    sleep 5
    log_info "Redpanda started on port 19092"
}

# Stop Redpanda container
stop_redpanda() {
    log_info "Stopping Redpanda container..."
    docker stop kafka-lite-test-redpanda 2>/dev/null || true
    docker rm kafka-lite-test-redpanda 2>/dev/null || true
}

# Start kafka-lite
start_kafka_lite() {
    log_info "Building kafka-lite..."
    cd "$PROJECT_DIR"
    cargo build --release

    log_info "Starting kafka-lite on port 29092..."
    ./target/release/kafka-lite --port 29092 --memory-only &
    KAFKA_LITE_PID=$!
    sleep 2
    log_info "kafka-lite started (PID: $KAFKA_LITE_PID)"
}

# Stop kafka-lite
stop_kafka_lite() {
    if [ -n "$KAFKA_LITE_PID" ]; then
        log_info "Stopping kafka-lite..."
        kill $KAFKA_LITE_PID 2>/dev/null || true
    fi
}

# Test produce/consume with kcat
test_with_kcat() {
    if ! command -v kcat &> /dev/null; then
        log_warn "Skipping kcat tests (kcat not installed)"
        return
    fi

    log_info "Testing with kcat..."

    # Test Redpanda
    echo "test-message-1" | kcat -b localhost:19092 -t kcat-test -P
    kcat -b localhost:19092 -t kcat-test -C -c 1 -e | grep -q "test-message-1" && \
        log_info "Redpanda: kcat produce/consume OK" || \
        log_error "Redpanda: kcat produce/consume FAILED"

    # Test kafka-lite
    echo "test-message-2" | kcat -b localhost:29092 -t kcat-test -P 2>/dev/null
    kcat -b localhost:29092 -t kcat-test -C -c 1 -e 2>/dev/null | grep -q "test-message-2" && \
        log_info "kafka-lite: kcat produce/consume OK" || \
        log_error "kafka-lite: kcat produce/consume FAILED"
}

# Run Rust integration tests
run_rust_tests() {
    log_info "Running Rust integration tests..."
    cd "$PROJECT_DIR"
    cargo test --test integration -- --ignored || true
}

# Run Rust compatibility tests
run_compatibility_tests() {
    log_info "Running Rust compatibility tests..."
    cd "$PROJECT_DIR"
    cargo test --test compatibility -- --ignored || true
}

# Cleanup on exit
cleanup() {
    stop_kafka_lite
    stop_redpanda
}

trap cleanup EXIT

# Main
main() {
    check_deps

    log_info "=== Kafka-lite Compatibility Tests ==="

    # Start services
    start_redpanda
    start_kafka_lite

    # Run tests
    test_with_kcat
    run_rust_tests
    run_compatibility_tests

    log_info "=== Tests Complete ==="
}

main "$@"
