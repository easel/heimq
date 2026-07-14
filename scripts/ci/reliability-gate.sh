#!/usr/bin/env bash
# Execute one CI reliability target and emit the project's executable
# evidence-window rule before returning the target command's status.

set -euo pipefail

usage() {
    cat >&2 <<'EOF'
usage: scripts/ci/reliability-gate.sh <target-name> -- <command> [args...]

Runs <command> as one reliability evidence attempt. The current GitHub Actions
workflow run is the evidence window, and zero target failures are allowed.
EOF
}

if [[ $# -lt 3 || "${2:-}" != "--" ]]; then
    usage
    exit 64
fi

target="$1"
shift 2

echo "reliability_target=$target"
echo "reliability_rule=zero_failures"
echo "evidence_window=current_github_actions_workflow_run"
echo "required_attempts=1"
echo "allowed_failures=0"
echo "command=$*"

if "$@"; then
    echo "reliability_result=pass"
else
    status=$?
    echo "reliability_result=fail"
    echo "failing_target=$target"
    exit "$status"
fi
