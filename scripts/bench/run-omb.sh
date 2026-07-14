#!/usr/bin/env bash
# OpenMessaging Benchmark conformance run against heimq.
#
# Requirements:
#   - Docker + the ombbuild-heimq image (built once; see below)
#   - heimq running on BOOTSTRAP (default localhost:9094)
#
# Build the image once:
#   docker build --network host \
#     -f scripts/bench/openmessaging/Dockerfile.omb-heimq \
#     -t ombbuild-heimq \
#     /tmp/ombbench
#
# (Clone OMB first if needed; this is OMB tag jms pinned to its current commit:
#   git clone --no-checkout --filter=blob:none https://github.com/openmessaging/benchmark /tmp/ombbench && git -C /tmp/ombbench checkout c0e51b8b86a3b0ff50b935152d6e600602a7f0a0)
#
# Usage:
#   BOOTSTRAP=localhost:9094 ./scripts/bench/run-omb.sh

set -euo pipefail

BOOTSTRAP="${BOOTSTRAP:-localhost:9094}"

echo "==> OpenMessaging Benchmark conformance against heimq ($BOOTSTRAP)"

# In OrbStack / Docker-in-VM environments, 'localhost' inside a container
# refers to the Docker host's loopback, not the OrbStack VM's loopback.
# Replace localhost/127.0.0.1 with the actual host IP so containers can reach it.
HOST_IP=$(hostname -I | awk '{print $1}')
CONTAINER_BOOTSTRAP="${BOOTSTRAP/localhost/$HOST_IP}"
CONTAINER_BOOTSTRAP="${CONTAINER_BOOTSTRAP/127.0.0.1/$HOST_IP}"

echo "    container bootstrap: $CONTAINER_BOOTSTRAP"

# Replace the HEIMQ_BOOTSTRAP placeholder in the baked-in driver config,
# then run the benchmark. --network host lets the container reach heimq via host IP.
# OMB's scheduled-metrics thread pool is never shut down, so the JVM hangs
# after writing the result file. We give 150s (1min test + overhead), then
# treat the exit as a success if the result file was written.
omb_exit=0
omb_out=$(timeout --kill-after=10 150 docker run --rm \
    --network host \
    --env HEAP_OPTS="-Xms256m -Xmx512m" \
    ombbuild-heimq \
    bash -c "
        sed -i 's|bootstrap.servers=HEIMQ_BOOTSTRAP|bootstrap.servers=${CONTAINER_BOOTSTRAP}|' /benchmark/driver-heimq.yaml
        bash /benchmark/bin/benchmark \
            --drivers /benchmark/driver-heimq.yaml \
            /benchmark/workload-smoke.yaml
    " 2>&1) || omb_exit=$?

echo "$omb_out"

if echo "$omb_out" | grep -qiE "ERROR Benchmark|Exception in thread|FATAL"; then
    echo "FAIL: OMB output contained error lines" >&2
    exit 1
fi

if ! echo "$omb_out" | grep -q "Writing test result"; then
    echo "FAIL: OMB did not complete (no result file written, exit=$omb_exit)" >&2
    exit 1
fi

echo "==> PASS: OpenMessaging Benchmark conformance completed"
