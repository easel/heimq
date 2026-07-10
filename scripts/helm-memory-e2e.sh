#!/usr/bin/env bash
# Run the fixed-memory Helm end-to-end evidence suite.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CHART_DIR="${ROOT_DIR}/charts/heimq"
CLUSTER="${HEIMQ_E2E_KIND_CLUSTER:-heimq-e2e}"
NAMESPACE="${HEIMQ_E2E_NAMESPACE:-heimq-e2e}"
RELEASE="${HEIMQ_E2E_RELEASE:-heimq}"
LOCAL_IMAGE_DEFAULT="heimq:helm-memory-e2e"
IMAGE="${HEIMQ_E2E_IMAGE:-${LOCAL_IMAGE_DEFAULT}}"
USE_CURRENT_CONTEXT="${HEIMQ_E2E_USE_CURRENT_CONTEXT:-0}"
ARTIFACT_ROOT="${HEIMQ_E2E_ARTIFACT_ROOT:-${ROOT_DIR}/target/helm-memory-e2e}"
TIMESTAMP="$(date -u +%Y%m%dT%H%M%SZ)"
ARTIFACT_DIR="${HEIMQ_E2E_ARTIFACT_DIR:-${ARTIFACT_ROOT}/${TIMESTAMP}}"
KAFKA_PORT="${HEIMQ_E2E_KAFKA_PORT:-9092}"
METRICS_PORT="${HEIMQ_E2E_METRICS_PORT:-9093}"
MAX_MEMORY_BYTES="${HEIMQ_E2E_MAX_MEMORY_BYTES:-8388608}"
PF_PID=""
SAMPLE_PID=""

usage() {
    cat <<'EOF'
Usage: scripts/helm-memory-e2e.sh [--help]

Runs the heimq Helm fixed-memory e2e suite and writes raw evidence under:
  target/helm-memory-e2e/<timestamp>/

Required tools:
  docker, kind, kubectl, helm, curl

Default behavior:
  - creates/uses kind cluster heimq-e2e
  - builds heimq:helm-memory-e2e from the local Dockerfile
  - loads that image into kind
  - installs a fresh Helm release for each scenario/case
  - port-forwards 127.0.0.1:9092 and 127.0.0.1:9093
  - runs the containerized harness in tests/conformance/helm_e2e

Environment:
  HEIMQ_E2E_KIND_CLUSTER       kind cluster name (default: heimq-e2e)
  HEIMQ_E2E_NAMESPACE          Kubernetes namespace (default: heimq-e2e)
  HEIMQ_E2E_RELEASE            Helm release name (default: heimq)
  HEIMQ_E2E_IMAGE              Image to deploy (default: heimq:helm-memory-e2e)
  HEIMQ_E2E_SKIP_IMAGE_BUILD   Set to 1 to skip docker build
  HEIMQ_E2E_USE_CURRENT_CONTEXT
                               Set to 1 to use current kubectl context; HEIMQ_E2E_IMAGE
                               must be a pullable image for non-kind clusters.
  HEIMQ_E2E_ARTIFACT_ROOT      Artifact root (default: target/helm-memory-e2e)
  HEIMQ_E2E_ARTIFACT_DIR       Exact artifact dir override
  HEIMQ_E2E_MAX_MEMORY_BYTES   Broker memory cap (default: 8388608)
EOF
}

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
    usage
    exit 0
fi

cleanup_background() {
    stop_cgroup_sampler
    if [[ -n "${PF_PID}" ]]; then
        kill "${PF_PID}" >/dev/null 2>&1 || true
        wait "${PF_PID}" >/dev/null 2>&1 || true
        PF_PID=""
    fi
}

trap cleanup_background EXIT

stop_cgroup_sampler() {
    if [[ -n "${SAMPLE_PID}" ]]; then
        kill "${SAMPLE_PID}" >/dev/null 2>&1 || true
        wait "${SAMPLE_PID}" >/dev/null 2>&1 || true
        SAMPLE_PID=""
    fi
}

require_tool() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "missing required tool: $1" >&2
        exit 2
    fi
}

preflight() {
    require_tool docker
    require_tool kind
    require_tool kubectl
    require_tool helm
    require_tool curl

    if command -v lsof >/dev/null 2>&1; then
        if lsof -nP -iTCP:"${KAFKA_PORT}" -sTCP:LISTEN >/dev/null 2>&1; then
            echo "local port ${KAFKA_PORT} is already in use" >&2
            exit 2
        fi
        if lsof -nP -iTCP:"${METRICS_PORT}" -sTCP:LISTEN >/dev/null 2>&1; then
            echo "local port ${METRICS_PORT} is already in use" >&2
            exit 2
        fi
    fi

    if [[ "${USE_CURRENT_CONTEXT}" == "1" && -z "${HEIMQ_E2E_IMAGE:-}" ]]; then
        echo "HEIMQ_E2E_USE_CURRENT_CONTEXT=1 requires HEIMQ_E2E_IMAGE to name a pullable image" >&2
        exit 2
    fi
}

image_repo() {
    if [[ -n "${HEIMQ_E2E_IMAGE_REPOSITORY:-}" ]]; then
        printf '%s' "${HEIMQ_E2E_IMAGE_REPOSITORY}"
        return
    fi
    printf '%s' "${IMAGE%:*}"
}

image_tag() {
    if [[ -n "${HEIMQ_E2E_IMAGE_TAG:-}" ]]; then
        printf '%s' "${HEIMQ_E2E_IMAGE_TAG}"
        return
    fi
    if [[ "${IMAGE}" == *:* ]]; then
        printf '%s' "${IMAGE##*:}"
    else
        printf 'latest'
    fi
}

ensure_cluster() {
    if [[ "${USE_CURRENT_CONTEXT}" == "1" ]]; then
        wait_for_kube_api "current-context" 60
        kubectl cluster-info >"${ARTIFACT_DIR}/cluster-info.txt"
        return
    fi
    if ! kind get clusters | grep -qx "${CLUSTER}"; then
        kind create cluster --name "${CLUSTER}"
    fi
    kubectl config use-context "kind-${CLUSTER}" >/dev/null
    if ! wait_for_kube_api "kind-${CLUSTER}" 30; then
        patch_kind_api_to_container_ip
    fi
    if ! wait_for_kube_api "kind-${CLUSTER}" 90; then
        echo "kind cluster ${CLUSTER} did not become API-ready; recreating it" >&2
        kind delete cluster --name "${CLUSTER}" >/dev/null
        kind create cluster --name "${CLUSTER}"
        kubectl config use-context "kind-${CLUSTER}" >/dev/null
        if ! wait_for_kube_api "kind-${CLUSTER}" 30; then
            patch_kind_api_to_container_ip
        fi
        wait_for_kube_api "kind-${CLUSTER}" 120
    fi
    kubectl cluster-info >"${ARTIFACT_DIR}/cluster-info.txt"
}

patch_kind_api_to_container_ip() {
    local container="${CLUSTER}-control-plane"
    local ip
    ip="$(docker inspect -f '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' "${container}" 2>/dev/null || true)"
    if [[ -z "${ip}" ]]; then
        return 0
    fi
    if curl -kfsS "https://${ip}:6443/readyz" >/dev/null 2>&1; then
        echo "Docker loopback publish for kind API is unavailable; using ${ip}:6443 directly" >&2
        kubectl config set-cluster "kind-${CLUSTER}" \
            --server="https://${ip}:6443" \
            --insecure-skip-tls-verify=true >/dev/null
    fi
}

wait_for_kube_api() {
    local label="$1"
    local tries="$2"
    for _ in $(seq 1 "${tries}"); do
        if kubectl get --raw=/readyz >/dev/null 2>&1; then
            return 0
        fi
        sleep 1
    done
    echo "Kubernetes API for ${label} did not become ready" >&2
    return 1
}

build_and_load_image() {
    if [[ "${HEIMQ_E2E_SKIP_IMAGE_BUILD:-0}" != "1" ]]; then
        docker build -t "${IMAGE}" "${ROOT_DIR}"
    fi
    if [[ "${USE_CURRENT_CONTEXT}" != "1" ]]; then
        kind load docker-image "${IMAGE}" --name "${CLUSTER}"
    fi
}

helm_values_args() {
    local broker_retention_ms="$1"
    local pull_policy="IfNotPresent"
    if [[ "${USE_CURRENT_CONTEXT}" == "1" ]]; then
        pull_policy="${HEIMQ_E2E_IMAGE_PULL_POLICY:-Always}"
    fi
    printf '%s\n' \
        --set "image.repository=$(image_repo)" \
        --set "image.tag=$(image_tag)" \
        --set "image.pullPolicy=${pull_policy}" \
        --set "replicaCount=1" \
        --set "fullnameOverride=heimq" \
        --set "heimq.host=0.0.0.0" \
        --set "heimq.advertisedHost=" \
        --set "heimq.memoryOnly=true" \
        --set "heimq.maxMemoryBytes=${MAX_MEMORY_BYTES}" \
        --set "heimq.retentionMs=${broker_retention_ms}" \
        --set "heimq.autoCreateTopics=false" \
        --set "heimq.metrics.enabled=true" \
        --set "heimq.metrics.port=${METRICS_PORT}" \
        --set "resources.requests.memory=64Mi" \
        --set "resources.limits.memory=128Mi" \
        --set "resources.requests.cpu=250m" \
        --set "resources.limits.cpu=1000m"
}

fresh_install() {
    local case_name="$1"
    local broker_retention_ms="$2"
    local case_dir="${ARTIFACT_DIR}/${case_name}"
    mkdir -p "${case_dir}"
    cleanup_background
    helm -n "${NAMESPACE}" uninstall "${RELEASE}" >/dev/null 2>&1 || true
    kubectl delete namespace "${NAMESPACE}" --ignore-not-found=true --wait=true >/dev/null
    kubectl create namespace "${NAMESPACE}" >/dev/null

    mapfile -t values < <(helm_values_args "${broker_retention_ms}")
    helm upgrade --install "${RELEASE}" "${CHART_DIR}" \
        -n "${NAMESPACE}" \
        "${values[@]}" \
        >"${case_dir}/helm-install.txt"
    helm get manifest "${RELEASE}" -n "${NAMESPACE}" >"${case_dir}/helm-manifest.yaml"
    helm get values "${RELEASE}" -n "${NAMESPACE}" --all >"${case_dir}/helm-values.yaml"
    kubectl -n "${NAMESPACE}" rollout status deploy/heimq --timeout=120s >"${case_dir}/rollout.txt"
    kubectl -n "${NAMESPACE}" get pods -o wide >"${case_dir}/pods.txt"
    kubectl -n "${NAMESPACE}" describe pod -l app.kubernetes.io/name=heimq >"${case_dir}/pod-describe.txt"
    assert_cgroup_v2 "${case_name}"
}

pod_name() {
    kubectl -n "${NAMESPACE}" get pod -l app.kubernetes.io/name=heimq \
        -o jsonpath='{.items[0].metadata.name}'
}

assert_cgroup_v2() {
    local case_name="$1"
    local pod
    pod="$(pod_name)"
    if ! kubectl -n "${NAMESPACE}" exec "${pod}" -- test -f /sys/fs/cgroup/memory.current; then
        echo "${case_name}: pod does not expose cgroup v2 memory.current" >&2
        exit 3
    fi
}

start_port_forward() {
    local case_name="$1"
    local case_dir="${ARTIFACT_DIR}/${case_name}"
    kubectl -n "${NAMESPACE}" port-forward svc/heimq \
        "${KAFKA_PORT}:9092" "${METRICS_PORT}:${METRICS_PORT}" \
        --address 127.0.0.1 \
        >"${case_dir}/port-forward.log" 2>&1 &
    PF_PID="$!"
    wait_for_tcp "127.0.0.1" "${KAFKA_PORT}" 60
    wait_for_tcp "127.0.0.1" "${METRICS_PORT}" 60
}

wait_for_tcp() {
    local host="$1"
    local port="$2"
    local tries="$3"
    for _ in $(seq 1 "${tries}"); do
        if (echo >"/dev/tcp/${host}/${port}") >/dev/null 2>&1; then
            return 0
        fi
        sleep 1
    done
    echo "timed out waiting for ${host}:${port}" >&2
    exit 3
}

scrape_metrics() {
    local case_name="$1"
    local label="$2"
    local case_dir="${ARTIFACT_DIR}/${case_name}"
    curl -fsS "http://127.0.0.1:${METRICS_PORT}/metrics" >"${case_dir}/metrics-${label}.prom"
}

sample_cgroup_once() {
    local case_name="$1"
    local label="$2"
    local case_dir="${ARTIFACT_DIR}/${case_name}"
    local pod
    pod="$(pod_name)"
    kubectl -n "${NAMESPACE}" exec "${pod}" -- cat /sys/fs/cgroup/memory.current \
        >"${case_dir}/cgroup-memory-${label}.txt"
}

start_cgroup_sampler() {
    local case_name="$1"
    local case_dir="${ARTIFACT_DIR}/${case_name}"
    local pod
    pod="$(pod_name)"
    (
        while true; do
            local ts
            ts="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
            local value
            value="$(kubectl -n "${NAMESPACE}" exec "${pod}" -- cat /sys/fs/cgroup/memory.current 2>/dev/null || true)"
            printf '{"ts":"%s","memory_current_bytes":%s}\n' "${ts}" "${value:-null}"
            sleep 1
        done
    ) >"${case_dir}/cgroup-memory-samples.jsonl" &
    SAMPLE_PID="$!"
}

E2E_IMAGE_TAG="heimq-helm-e2e:local"

build_e2e_image() {
    docker build -t "${E2E_IMAGE_TAG}" "${ROOT_DIR}/tests/conformance/helm_e2e" >/dev/null
}

# The harness runs in a container and reaches the port-forwarded broker on the
# host, so nothing but Docker is required on the runner. It used to be a Rust
# test linking librdkafka into the heimq crate.
run_e2e_scenario() {
    local case_name="$1"
    local scenario="$2"
    local a_case="${3:-}"
    local case_dir="${ARTIFACT_DIR}/${case_name}"
    local docker_env=(
        -e "HEIMQ_E2E_BOOTSTRAP=127.0.0.1:${KAFKA_PORT}"
        -e "HEIMQ_E2E_METRICS=127.0.0.1:${METRICS_PORT}"
        -e "HEIMQ_E2E_ARTIFACT_DIR=/out"
        -e "HEIMQ_E2E_SCENARIO=${scenario}"
    )
    if [[ -n "${a_case}" ]]; then
        docker_env+=(-e "HEIMQ_E2E_A_CASE=${a_case}")
    fi
    docker run --rm --network host \
        "${docker_env[@]}" \
        --user "$(id -u):$(id -g)" \
        -v "${case_dir}:/out" \
        "${E2E_IMAGE_TAG}" 2>&1 | tee "${case_dir}/e2e.log"
}

capture_after() {
    local case_name="$1"
    local case_dir="${ARTIFACT_DIR}/${case_name}"
    scrape_metrics "${case_name}" "after"
    sample_cgroup_once "${case_name}" "after"
    kubectl -n "${NAMESPACE}" get pods -o wide >"${case_dir}/pods-after.txt"
    kubectl -n "${NAMESPACE}" logs deploy/heimq >"${case_dir}/heimq.log" 2>&1 || true
    kubectl -n "${NAMESPACE}" describe pod -l app.kubernetes.io/name=heimq >"${case_dir}/pod-describe-after.txt" || true
}

run_case() {
    local case_name="$1"
    local scenario="$2"
    local broker_retention_ms="$3"
    local a_case="${4:-}"
    echo "==> ${case_name}"
    fresh_install "${case_name}" "${broker_retention_ms}"
    start_port_forward "${case_name}"
    sample_cgroup_once "${case_name}" "before"
    scrape_metrics "${case_name}" "before"
    if [[ "${scenario}" == "B" ]]; then
        start_cgroup_sampler "${case_name}"
    fi
    run_e2e_scenario "${case_name}" "${scenario}" "${a_case}"
    stop_cgroup_sampler
    capture_after "${case_name}"
    cleanup_background
}

main() {
    mkdir -p "${ARTIFACT_DIR}"
    preflight
    {
        echo "timestamp=${TIMESTAMP}"
        echo "image=${IMAGE}"
        echo "namespace=${NAMESPACE}"
        echo "release=${RELEASE}"
        echo "kind_cluster=${CLUSTER}"
        echo "use_current_context=${USE_CURRENT_CONTEXT}"
        echo "max_memory_bytes=${MAX_MEMORY_BYTES}"
        uname -a || true
        docker version || true
        kubectl version --client=true || true
        helm version || true
    } >"${ARTIFACT_DIR}/run-metadata.txt" 2>&1

    ensure_cluster
    build_and_load_image
    build_e2e_image

    run_case "scenario-a-1-topic" "A" "604800000" "A-1-topic"
    run_case "scenario-a-10-topics" "A" "604800000" "A-10-topics"
    run_case "scenario-a-100-topics" "A" "604800000" "A-100-topics"
    run_case "scenario-a2-throughput" "A2" "604800000"
    run_case "scenario-b-plateau" "B" "604800000"
    run_case "scenario-c-retention-ms" "C" "604800000"
    run_case "scenario-d-backpressure" "D" "1000"
    run_case "scenario-e-retention-bytes" "E" "604800000"

    echo "${ARTIFACT_DIR}" >"${ARTIFACT_ROOT}/latest.txt"
    echo "artifacts: ${ARTIFACT_DIR}"
}

main "$@"
