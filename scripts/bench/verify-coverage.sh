#!/usr/bin/env bash
# Static traceability checks for benchmark acceptance coverage.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

required_acs=(
    US-006-AC1
    US-006-AC2
    US-006-AC3
    US-006-AC4
    US-007-AC1
    US-007-AC2
    US-007-AC3
)

for ac in "${required_acs[@]}"; do
    if ! grep -R -E "@covers[[:space:]]+$ac([^0-9]|$)" \
        scripts/bench .github/workflows/bench-smoke.yml .github/workflows/bench-omb.yml \
        >/dev/null; then
        echo "missing @covers citation for $ac" >&2
        exit 1
    fi
done

required_files=(
    scripts/bench/profiles/producer-smoke.properties
    scripts/bench/profiles/consumer-smoke.properties
    scripts/bench/profiles/producer-idempotent.properties
    scripts/bench/profiles/producer-transactional.properties
    scripts/bench/openmessaging/driver-heimq.yaml
    scripts/bench/openmessaging/workload-smoke.yaml
    scripts/bench/openmessaging/Dockerfile.omb-heimq
)

for path in "${required_files[@]}"; do
    if [[ ! -f "$path" ]]; then
        echo "required benchmark file does not exist: $path" >&2
        exit 1
    fi
done

ruby -e '
  require "yaml"
  ARGV.each { |path| YAML.load_file(path) }
' \
    .github/workflows/bench-smoke.yml \
    .github/workflows/bench-omb.yml \
    scripts/bench/openmessaging/driver-heimq.yaml \
    scripts/bench/openmessaging/workload-smoke.yaml

echo "benchmark coverage citations and YAML parse checks passed"
