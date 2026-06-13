#!/usr/bin/env bash
# Shared utilities for ecosystem integration tests.
# Source this file; do not execute directly.

BOOTSTRAP="${BOOTSTRAP:-localhost:9094}"

# In OrbStack / Docker-in-VM environments, 'localhost' inside a Docker container
# refers to the Docker host's loopback, not the OrbStack VM's loopback.
# DOCKER_BOOTSTRAP replaces localhost/127.0.0.1 with the actual host IP so that
# Docker containers launched by these scripts can reach heimq.
HOST_IP=$(hostname -I | awk '{print $1}')
DOCKER_BOOTSTRAP="${BOOTSTRAP/localhost/$HOST_IP}"
DOCKER_BOOTSTRAP="${DOCKER_BOOTSTRAP/127.0.0.1/$HOST_IP}"
export DOCKER_BOOTSTRAP

_eco_pass=0
_eco_fail=0
_eco_skip=0

eco_pass() { echo "PASS: $1"; _eco_pass=$((_eco_pass + 1)); }
eco_fail() { echo "FAIL: $1" >&2; _eco_fail=$((_eco_fail + 1)); }
eco_skip() { echo "SKIP: $1"; _eco_skip=$((_eco_skip + 1)); }

eco_summary() {
    echo "==> ecosystem results: ${_eco_pass} passed, ${_eco_fail} failed, ${_eco_skip} skipped"
    [ "$_eco_fail" -eq 0 ]
}

# Wait for a TCP port to accept connections (timeout in seconds).
wait_for_port() {
    local host=$1 port=$2 timeout=${3:-30}
    local deadline=$((SECONDS + timeout))
    while ! nc -z "$host" "$port" 2>/dev/null; do
        [ $SECONDS -ge $deadline ] && return 1
        sleep 0.5
    done
}

# Wait for an HTTP endpoint to return HTTP 200.
wait_for_http() {
    local url=$1 timeout=${2:-60}
    local deadline=$((SECONDS + timeout))
    while ! curl -sf "$url" >/dev/null 2>&1; do
        [ $SECONDS -ge $deadline ] && return 1
        sleep 1
    done
}

# Unique suffix for topic names per test run (avoids cross-test collisions).
RUN_ID="${RUN_ID:-$$}"
