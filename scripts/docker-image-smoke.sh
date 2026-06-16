#!/usr/bin/env bash
# Smoke-test a built heimq Docker image with a real Kafka client.
#
# The workflow loads the image locally, starts it in Docker, and then uses a
# helper Go container that shares the broker's network namespace to wait for
# TCP readiness and run the checked-in Sarama oracle against 127.0.0.1:9092.
#
# Usage:
#   ./scripts/docker-image-smoke.sh [image-ref]

set -euo pipefail

IMAGE_REF="${1:-heimq:local}"
BOOTSTRAP="127.0.0.1:9092"
TOPIC="docker-image-smoke-$$"
CONTAINER="heimq-image-smoke-$$"

cleanup() {
    docker rm -f "${CONTAINER}" >/dev/null 2>&1 || true
}

trap cleanup EXIT

docker run -d --rm \
    --name "${CONTAINER}" \
    -e HEIMQ_HOST=0.0.0.0 \
    -e HEIMQ_ADVERTISED_HOST=127.0.0.1 \
    -e HEIMQ_MEMORY_ONLY=true \
    "${IMAGE_REF}" >/dev/null

if ! docker run --rm \
    --network "container:${CONTAINER}" \
    -v "$PWD:/repo" \
    -w /repo/crates/heimq/tests/compat/sarama_oracle \
    golang:1.24-bookworm \
    bash -lc "set -euo pipefail
        export PATH=/usr/local/go/bin:\$PATH
        ready=0
        for _ in \$(seq 1 60); do
            if (echo > /dev/tcp/127.0.0.1/9092) >/dev/null 2>&1; then
                ready=1
                break
            fi
            sleep 1
        done
        if [[ \"\$ready\" -ne 1 ]]; then
            echo 'heimq image did not become ready on 127.0.0.1:9092' >&2
            exit 1
        fi
        timeout 10m go run . '${BOOTSTRAP}' '${TOPIC}'
    "; then
    docker logs "${CONTAINER}" >&2 || true
    exit 1
fi
